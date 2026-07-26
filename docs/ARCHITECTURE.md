# prexp architecture — the common design

The design ideas that hold across the whole project, independent of which
implementation you're reading. Each ecosystem's own docs (`rust/`, `swift/`)
describe how *that* implementation realizes them; this page is the shared mental
model. Start here.

## Two independent implementations — no FFI

prexp is **not** a core-with-language-bindings project. There is no shared binary,
no C-ABI, no `cbindgen`/`uniffi`, no xcframework. Following the sibling `soroban`
project, prexp is **two fully independent implementations** of the same tool — one
Rust, one Swift — that never link against each other:

- The **Rust** side reaches macOS process APIs through its own `extern "C"` FFI
  layer (`prexp-ffi`).
- The **Swift** side reaches the *same* APIs natively through the `Darwin` module.

Neither calls the other. The only contract between them is **behavioral**: matching
domain-model shapes / JSON output, verified live (see [PARITY.md](PARITY.md)).

Why do it twice instead of binding one core? Because the interesting, platform-
specific surface *is* the native data extraction, and each language expresses it
idiomatically and safely on its own — Swift talks to `libproc`/Mach directly with
no unsafe shim; Rust contains all `unsafe` in one crate. Two clean implementations
beat one core plus a lossy binding layer, and keeping them honest is a solved
problem (a live diff).

## The layered design — core → front-ends

Both ecosystems are split the same way:

1. **The core (data layer).** Domain models (`ProcessSnapshot`, `OpenResource`,
   `ResourceKind`, `ProcessDetail`, system types), the **`ProcessSource`** trait/
   protocol, a native source that implements it over libproc/Mach, and output
   formatters (JSON, TSV). Knows nothing about terminals or windows.
   - Rust: `prexp-core` (+ `prexp-ffi` for the raw bindings).
   - Swift: `PrexpCore` (native source + shared pure logic, no C shim).
2. **The front-ends.** Thin presentation layers over the core:
   - a **TUI** — Rust `prexp` (ratatui), Swift `prexp` (on the sibling `tint` kit);
   - a **GUI** — Rust `prexp-desktop` (iced + the sibling `rime` kit), Swift `Prexp`
     (SwiftUI).

Each front-end holds the backend behind the `ProcessSource` seam, so the core is
shared within an ecosystem and every front-end is a thin view. Pure view-logic
(CPU%-delta tracking, sorting, filtering, byte/percent formatting) lives in the
core so both front-ends share one implementation.

### The `ProcessSource` seam

The load-bearing abstraction (inversion of control). One trait/protocol exposes the
whole domain — `snapshotAll`, `snapshotPid`, `processDetail`, `findByPath`,
`killProcess`, `cpuTicks`, `memoryInfo`, `cpuPerfLevels`, battery/load/boot, … —
so front-ends depend on the contract, not the backend, and tests inject canned
doubles. The two languages define the *same* surface independently.

## Ecosystem-first monorepo

```
fdtop/
├── docs/     # shared design/parity/FFI docs (this file lives here)
├── scripts/  # cross-cutting tooling — parity.py (the parity oracle)
├── rust/     # one cargo workspace: prexp-ffi, prexp-core, prexp, prexp-desktop
└── swift/    # SwiftPM + xcodegen: Core, TUI (on tint), App (SwiftUI)
```

Two independent implementations of the same tool live side by side. When working
inside `rust/` or `swift/`, that directory's `CLAUDE.md` is authoritative; the
repo-root `CLAUDE.md` covers only what spans both.

## The parity model — live diff, not a fixture spec

`soroban` keeps its two implementations honest with a shared Gherkin `spec/` both
run. prexp's data is **live and non-deterministic** (real processes), so a canned-
fixture oracle can only touch the thin pure-logic layer — and would miss the native
FFI layer that's most likely to drift (a real Rust struct-layout bug hid exactly
there). So prexp's parity guardrail is **`scripts/parity.py`**: it runs both
implementations against the live system and diffs their output. Full detail in
[PARITY.md](PARITY.md).

## Native platform access

Both sides read the same macOS APIs — `libproc` (process/fd enumeration), Mach
(`task_info`, `host_*`, timebase), `sysctl`, and IOKit (battery). The Rust side
wraps them in `prexp-ffi`; the Swift side reaches them straight from `Darwin` with
no C shim. The shared reference — every API, the exact arithmetic, and the gotchas
(Mach timebase, the `proc_listallpids` count-vs-bytes quirk) — is
[FFI.md](FFI.md).

## Key design decisions

- **No `lsof`, no `libproc` crate** — native platform APIs only.
- **All `unsafe` contained** in `prexp-ffi` (Rust); the Swift native source is
  ordinary Swift over `Darwin`.
- **Errors:** `thiserror` in Rust libraries / `anyhow` in `main`; a `PrexpError`
  enum in Swift.
- **CPU%** is a delta between refreshes (Mach absolute time → ns via
  `mach_timebase_info`).
- **Memory:** RSS from `proc_taskinfo`; physical footprint from
  `task_info(TASK_VM_INFO)` via `task_name_for_pid` (no root).
- **Selection is anchored** by a stable key (PID for processes, path for files) so
  it survives refreshes.
- **Both GUIs are kept** (Rust iced + Swift SwiftUI) as parallel ecosystems, and
  each GUI is excluded from / separate from the shared build graph because of its
  heavy, sibling-kit dependencies.
