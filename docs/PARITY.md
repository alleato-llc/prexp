# Parity — keeping the two implementations honest

prexp is two independent implementations (Rust and Swift) of the same tool, with
[no FFI between them](ARCHITECTURE.md#two-independent-implementations--no-ffi). The
only contract is behavioral, so something has to verify they actually agree. This
page is that story.

## Why not a shared Gherkin spec (soroban's approach)

The sibling `soroban` project keeps its two implementations in lockstep with a
shared Gherkin `spec/` that both sides run over canned fixtures. That works there
because soroban's domain is *pure, deterministic computation* (a calculator
language) — almost everything is fixture-able.

prexp is different. Its data is **live and non-deterministic** — real processes
that come and go, CPU/memory that changes moment to moment. A fixture-based oracle
could only assert the thin **pure-logic** layer (formatters, sorting, filtering,
CPU-delta math), and that layer is *already* unit-tested on both sides. Worse, it
would be blind to the part most likely to drift: the **native libproc/Mach data
extraction**. That's not hypothetical — a real Rust struct-layout bug (`InSockInfo`
16 bytes short) mislabeled *every* TCP connection as `CLOSED`, and no fixture spec
could have caught it. A live diff did.

So prexp's parity guardrail is a **live cross-implementation diff**, not a Gherkin
harness.

## `scripts/parity.py`

Runs both implementations against the live system, joins their process snapshots on
PID, and checks that they agree — with per-metric thresholds and a CI-friendly exit
code.

```sh
scripts/parity.py                 # build both, run the full check
scripts/parity.py --no-build      # skip the builds
scripts/parity.py --verbose       # print every mismatch
scripts/parity.py --sample 12     # network-check more processes
```

What it compares:

| Check | How | Default gate |
|---|---|---|
| **stable fields** | `name`, `ppid`, `state`, `accessible` per common PID | ≥ 98% exact |
| **open fd paths** | Jaccard overlap of each process's resource path set | ≥ 0.90 mean |
| **network tables** | connection tuples for a sample of socket-holding processes | ≥ 0.60 mean Jaccard |

It **ignores time-variant fields** (CPU%, memory, disk I/O) — the two runs happen a
moment apart, so those legitimately differ — and **tolerates churn** by comparing
only the PID intersection and reporting rates, not equality. It shells out to the
Swift smoke tool (`prexp-smoke`) and the Rust CLI (`prexp --output json` /
`--info network`).

A healthy run looks like:

```
snapshot: swift=965  rust=967  common=964
  name/ppid/state/accessible   99.90%   [ok ≥98%]
  fd paths  Jaccard 0.998 over 603      [ok ≥0.90]
  network   8 sampled, all Jaccard 1.00 [ok ≥0.60]
RESULT: PASS
```

## When to run it

After **any change to either side's data layer** — `prexp-ffi` / `prexp-core`
(Rust) or `PrexpCore` (Swift). If a field's match rate drops, one side has drifted
(or a genuine bug surfaced on one side, as with the TCP-state fix). The pure logic
is separately covered by unit tests on both sides (see [TESTING.md](TESTING.md)).

## The complementary layers

| Layer | Verified by |
|---|---|
| pure logic (formatters, sort, filter, CPU-delta, network formatting) | unit tests, each ecosystem |
| native data extraction (libproc/Mach) + end-to-end shape | `scripts/parity.py` (live diff) |
| front-end behavior (state machine, rendering) | ecosystem tests (Swift render/behavior tests, Rust app-state tests) |

A formal shared Gherkin `spec/` remains *possible* for the pure-logic layer, but is
low-value for this domain and is not maintained.
