#!/usr/bin/env python3
"""Live parity check: Swift PrexpCore vs Rust prexp.

The two ecosystems are independent reimplementations with no shared binary, so
their contract is behavioral. This script verifies it against the *live* system —
the surface a fixture-based (Gherkin) oracle can't reach, and exactly where a real
divergence already hid (a Rust struct-layout bug that mislabeled every TCP state).

It runs both implementations, joins their process snapshots on PID, and checks:
  * stable per-process fields (name, ppid, state, accessible) — exact match rate
  * open file-descriptor path sets — mean Jaccard overlap
  * a sample of processes' TCP/UDP connection tables — mean Jaccard overlap

Time-variant fields (CPU%, memory, disk I/O) are ignored: the two runs happen a
moment apart, so those legitimately differ. Process churn between the runs is
tolerated by comparing only the PID intersection and reporting rates, not equality.

Exit code 0 if every metric clears its threshold, 1 otherwise (CI-friendly).

Usage:
    scripts/parity.py [--sample N] [--no-build] [--verbose]
                      [--field-threshold F] [--path-threshold F] [--net-threshold F]
"""
from __future__ import annotations

import argparse
import json
import subprocess
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SWIFT_SMOKE = REPO / "swift" / "Core" / ".build" / "debug" / "prexp-smoke"
RUST_BIN = REPO / "rust" / "target" / "debug" / "prexp"


def run(cmd: list[str], cwd: Path | None = None) -> str:
    r = subprocess.run(cmd, cwd=cwd, capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError(f"command failed ({r.returncode}): {' '.join(map(str, cmd))}\n{r.stderr[:400]}")
    return r.stdout


def build() -> None:
    print("building both implementations…", file=sys.stderr)
    run(["swift", "build", "--package-path", str(REPO / "swift" / "Core")])
    run(["cargo", "build", "-q", "-p", "prexp"], cwd=REPO / "rust")


# ---- snapshot loaders -------------------------------------------------------

def swift_snapshot() -> dict[int, dict]:
    data = json.loads(run([str(SWIFT_SMOKE), "json"]))
    return {p["pid"]: p for p in data}


def rust_snapshot() -> dict[int, dict]:
    data = json.loads(run([str(RUST_BIN), "--output", "json"]))
    return {p["pid"]: p for p in data}


def swift_network(pid: int) -> list[dict]:
    return json.loads(run([str(SWIFT_SMOKE), "detail", str(pid)])).get("network", [])


def rust_network(pid: int) -> list[dict]:
    out = run([str(RUST_BIN), "--pid", str(pid), "--info", "network"])
    data = json.loads(out)
    return data if isinstance(data, list) else data.get("network", [])


# ---- comparison helpers -----------------------------------------------------

def jaccard(a: set, b: set) -> float:
    if not a and not b:
        return 1.0
    return len(a & b) / len(a | b)


def paths(proc: dict) -> set[str]:
    return {r["path"] for r in proc["resources"] if r.get("path")}


def conn_key(c: dict) -> tuple:
    return (c.get("proto"), c.get("local_addr"), c.get("remote_addr"), c.get("state"))


class Metric:
    def __init__(self, name: str, threshold: float, higher_is_better: bool = True):
        self.name, self.threshold = name, threshold
        self.value = 0.0
        self.detail = ""

    @property
    def passed(self) -> bool:
        return self.value >= self.threshold


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--sample", type=int, default=8, help="processes to network-check (default 8)")
    ap.add_argument("--field-threshold", type=float, default=0.98, help="min stable-field match rate")
    ap.add_argument("--path-threshold", type=float, default=0.90, help="min mean fd-path Jaccard")
    ap.add_argument("--net-threshold", type=float, default=0.60, help="min mean connection Jaccard")
    ap.add_argument("--no-build", action="store_true", help="skip building the binaries")
    ap.add_argument("--verbose", action="store_true", help="print per-mismatch detail")
    args = ap.parse_args()

    if not args.no_build:
        build()
    if not SWIFT_SMOKE.exists() or not RUST_BIN.exists():
        print("error: binaries missing — run without --no-build first", file=sys.stderr)
        return 2

    sw, rs = swift_snapshot(), rust_snapshot()
    common = sorted(set(sw) & set(rs))

    print("prexp parity — Swift PrexpCore vs Rust prexp")
    print("=" * 52)
    print(f"snapshot: swift={len(sw)}  rust={len(rs)}  common={len(common)}\n")

    metrics: list[Metric] = []

    # stable fields
    fields = ["name", "ppid", "state", "accessible"]
    print(f"stable fields (over {len(common)} common processes)")
    for f in fields:
        matches = sum(1 for pid in common if sw[pid].get(f) == rs[pid].get(f))
        rate = matches / len(common) if common else 1.0
        m = Metric(f, args.field_threshold)
        m.value = rate
        metrics.append(m)
        flag = "ok " if m.passed else "FAIL"
        print(f"  {f:<12} {matches}/{len(common)}  {rate*100:6.2f}%  [{flag} ≥{args.field_threshold*100:.0f}%]")
        if args.verbose and matches < len(common):
            for pid in common:
                if sw[pid].get(f) != rs[pid].get(f):
                    print(f"      pid {pid}: swift={sw[pid].get(f)!r} rust={rs[pid].get(f)!r}")

    # fd paths
    js = [jaccard(paths(sw[pid]), paths(rs[pid])) for pid in common if paths(sw[pid]) or paths(rs[pid])]
    mean_paths = sum(js) / len(js) if js else 1.0
    mp = Metric("fd-paths", args.path_threshold)
    mp.value = mean_paths
    metrics.append(mp)
    print(f"  {'fd paths':<12} Jaccard mean {mean_paths:.3f} over {len(js)}  "
          f"[{'ok ' if mp.passed else 'FAIL'} ≥{args.path_threshold:.2f}]\n")

    # network sample: processes with the most sockets, present in both snapshots
    def socket_count(p: dict) -> int:
        return sum(1 for r in p["resources"] if r.get("kind") == "socket")

    candidates = sorted((pid for pid in common if sw[pid].get("accessible") and socket_count(sw[pid]) > 0),
                        key=lambda pid: socket_count(sw[pid]), reverse=True)[: args.sample]
    print(f"network ({len(candidates)} sampled processes with sockets)")
    net_scores = []
    for pid in candidates:
        try:
            a = {conn_key(c) for c in swift_network(pid)}
            b = {conn_key(c) for c in rust_network(pid)}
        except RuntimeError:
            continue  # process may have exited
        j = jaccard(a, b)
        net_scores.append(j)
        name = sw[pid].get("name", "?")
        print(f"  pid {pid:<7} {name[:24]:<24} swift={len(a):<3} rust={len(b):<3} Jaccard={j:.2f}")
        if args.verbose and j < 1.0:
            for c in sorted(a - b):
                print(f"      swift-only: {c}")
            for c in sorted(b - a):
                print(f"      rust-only:  {c}")
    mean_net = sum(net_scores) / len(net_scores) if net_scores else 1.0
    mn = Metric("network", args.net_threshold)
    mn.value = mean_net
    metrics.append(mn)
    print(f"  mean connection Jaccard {mean_net:.3f}  "
          f"[{'ok ' if mn.passed else 'FAIL'} ≥{args.net_threshold:.2f}]\n")

    ok = all(m.passed for m in metrics)
    print("RESULT:", "PASS" if ok else "FAIL")
    if not ok:
        print("failing metrics:", ", ".join(m.name for m in metrics if not m.passed))
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
