# CLAUDE.md — Swift ecosystem (authoritative for `swift/`)

> This is the **Swift** half of a modular monolith (see the repo-root `CLAUDE.md` for
> the cross-cutting overview and the parallel `rust/` ecosystem). It is an **independent
> reimplementation** of prexp — there is **no FFI to the Rust core**. The native source
> talks straight to macOS libproc/Mach via Swift's `Darwin` module.

## Layout

```
swift/
├── Core/     SwiftPM package `PrexpCore` — the data layer + shared pure logic (DONE)
├── TUI/      SwiftPM `prexp` executable + `PrexpTUI` lib, on sibling `tint` (DONE)
└── App/      xcodegen SwiftUI GUI `Prexp` (DONE)
```

Shared pure logic (`Format`, `CpuTracker`/`CoreUsage`, `SortField`/`sortProcesses`/
`matchesQuery`) lives in **`PrexpCore`** so both front-ends use one implementation.

Mirrors the Rust layering: `Core` (models + `ProcessSource` + native source + formatters)
is the analogue of `prexp-core` + `prexp-ffi`; `TUI` and `App` are the two front-ends
(analogues of `prexp` and `prexp-desktop`).

## Build & Test

```bash
cd swift/Core
swift build
swift test                                  # pure-logic unit tests (canned data)
.build/debug/prexp-smoke json               # dump all processes as JSON
.build/debug/prexp-smoke tsv                # TSV
.build/debug/prexp-smoke pid <PID>          # one process
.build/debug/prexp-smoke detail <PID>       # process detail (incl. network)
.build/debug/prexp-smoke lookup <PATH>      # reverse lookup (processes holding PATH)
.build/debug/prexp-smoke system             # system metrics summary
```

Toolchain: **Swift 6.2+**, **macOS 14+** (matches `tint`).

## Core design (`swift/Core`)

- **No C shim.** Every libproc/Mach API the Rust `prexp-ffi` wraps is reachable directly
  from `import Darwin` (verified: proc enumeration, fd listing, `task_info` phys footprint,
  `proc_pid_rusage`, `host_statistics64`, `proc_listpidspath`, IOKit battery). So `Core`
  is a single pure-Swift target — no module map, no `CProc` target.
- **`ProcessSource` protocol** (`Sources/PrexpCore/ProcessSource.swift`) — the Swift analogue
  of Rust's trait. `NativeSource` implements it; test doubles conform with canned data.
  Default methods throw `.unsupported`, and `killProcess` is a POSIX `kill(2)` default.
- **Models** (`Models.swift`) are `Codable` and mirror `rust/crates/prexp-core/src/models.rs`
  field-for-field, with `CodingKeys` matching serde's snake_case JSON. `OpenResource` encodes
  `path` even when nil (serde emits `null`); `EnvVar` encodes as a `[key, value]` array.
- **Fidelity to the Rust FFI arithmetic** is exact: Mach-timebase tick→ns conversion
  (`Darwin+Helpers.swift`), page-size memory scaling, `pbi_status`→state codes, the
  `proc_listallpids` **count** vs `proc_listpidspath` **bytes** distinction (a real libproc
  inconsistency — see `NativeSource.listAllPids`).

### Parity contract with the Rust side

The two implementations agree **structurally**, not byte-for-byte: Foundation's `JSONEncoder`
does not preserve field declaration order (serde does), so raw JSON bytes differ. What matches
is the shape and values — verified by diffing `prexp-smoke json` against
`cargo run -p prexp -- --output json` and joining on PID: identical process count, and 100%
agreement on name / ppid / state / accessibility and open-fd paths across the table. The
eventual shared `spec/` oracle compares parsed structure, not bytes.

### Known gaps (tracked)

- `networkCounters` / `diskCounters` inherit the `.unsupported` default for now (system-wide
  rate niceties; not needed by the data layer).

`processDetail.network` IS parsed (TCP/UDP via `PROC_PIDFDSOCKETINFO`, mirroring Rust exactly —
including the raw-port/no-`ntohs` quirk). Verifying it caught a latent Rust bug: `prexp-ffi`'s
`InSockInfo` was 16 bytes short, so `tcpsi_state` read the wrong offset (every TCP conn → CLOSED);
fixed by padding the struct to the real size. A good example of parity verification paying off.

## TUI (`swift/TUI`)

Immediate-mode on `tint` (`.package(path: "../../../tint")`, product `Tint`). lib/bin split:
`PrexpTUI` (the `AppModel` state machine + pure `View.render`) is unit-tested by rendering to an
offscreen `Buffer` and inspecting text; `prexp` is the thin executable wiring `Application.run`.
A background `DispatchSource` timer refreshes the (expensive) full snapshot on the `--interval`
cadence; tint redraws the current model each frame. `AppModel` is `@unchecked Sendable` (single
main-run-loop). Preview without a TTY: `.build/debug/prexp --snapshot 110x30 [--stats]`.

Delivered: live process table (PID/NAME/CPU%/MEM/PMEM/THR/FILES/TOTAL), CPU% deltas, sort cycle
(`s`/`S`), `/` search, detail + help overlays, stats header (per-core P/E, memory gauge, load),
row load-coloring, PID-anchored selection.

**Additions made to `tint`** (we own it): `Rect.centered(width:height:)`, a `Clear` widget, a
`Popup` (centered modal + backdrop-clear), and a `TextInput` widget — tint shipped neither modal
centering nor text input. All with tests under `tint/Tests/TintTests/Widget/`.

Deferred TUI features (follow-ups): file view (`v`), signal picker (`K`), reverse lookup (`r`),
theme picker (`t`), column config (`c`), process-tree grouping.

## Milestones

- **Core** — DONE (native `ProcessSource`, formatters, smoke tool, unit tests, verified parity).
- **TUI** — DONE (see above).
- **App** — DONE (see below).

## App (`swift/App`)

Native SwiftUI, macOS 14+, assembled by **xcodegen** from `project.yml` (the `.xcodeproj` is
generated and gitignored — regenerate with `xcodegen generate`). Depends on the `PrexpCore`
SwiftPM package. **Not sandboxed** and ad-hoc signed (`CODE_SIGN_IDENTITY "-"`) — a process
explorer must read other processes via libproc/Mach, which the app sandbox forbids.

```bash
cd swift/App
xcodegen generate
xcodebuild -project Prexp.xcodeproj -scheme Prexp -configuration Debug build
open ~/Library/Developer/Xcode/DerivedData/Prexp-*/Build/Products/Debug/Prexp.app
```

`AppState` is `@MainActor @Observable`; the heavy `ProcessSource` calls hop to detached tasks,
a `Task`-driven timer refreshes on `interval`, and CPU%/per-core deltas + a rolling CPU-history
buffer feed the view (same delta approach as the TUI, via the shared `PrexpCore` helpers).

Delivered: live process table (PID/NAME/CPU%/MEM/PMEM/THR/FILES/FDs), sort picker + reverse,
filter field, stats header (memory bar, per-core P/E grid, load, battery, Swift Charts CPU
history), detail pane (open resources), send-signal (SIGTERM/INT/KILL) via a confirmation
dialog, the **info panel** — a sheet (⌘I / double-click) with Overview / Resources / Network /
Environment tabs over `processDetail`, loaded off the main actor — and **reverse path lookup**
(⌘F, or right-click a resource in the detail pane → "Find processes with this path"), which
runs `findByPath` off the main actor and lists the holders. Deferred: theme picker.
