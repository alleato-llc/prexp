# prexp — Swift ecosystem

The Swift implementation of prexp: a native core over macOS libproc/Mach, a terminal
UI on the sibling [`tint`](../../tint) kit, and a native SwiftUI app. An
**independent reimplementation** — there is no FFI to the Rust side (see
[../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md)).

For the shared design start with [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md)
and the native-API reference [../docs/FFI.md](../docs/FFI.md). This page is the
ecosystem entry point; the agent-facing notes are in [CLAUDE.md](CLAUDE.md).

## What's here

| Path | What it is |
|---|---|
| `Core/` | SwiftPM package **`PrexpCore`** — the data layer: Codable models, the `ProcessSource` protocol, a native source over `Darwin` (no C shim), JSON/TSV formatters, and the shared pure logic (CPU-delta, sort, format). Plus the `prexp-smoke` tool. |
| `TUI/` | SwiftPM package — the **`prexp`** executable on `tint`, plus the testable `PrexpTUI` library (state machine + pure `View.render`). |
| `App/` | The native SwiftUI app **`Prexp`**, assembled by **xcodegen** from `project.yml` (the `.xcodeproj` is generated + gitignored). |

Toolchain: **Swift 6.2+**, **macOS 14+** (matches `tint`).

## Build & run

```sh
# Core — library + smoke tool
cd swift/Core && swift build && swift test
.build/debug/prexp-smoke json        # dump live processes as JSON
.build/debug/prexp-smoke lookup /dev/null   # reverse lookup

# TUI — on tint
cd swift/TUI && swift run prexp
.build/debug/prexp --snapshot 110x30 --stats   # render one frame to stdout (no TTY)

# App — SwiftUI (not sandboxed: a process explorer must read other processes)
cd swift/App && xcodegen generate
xcodebuild -project Prexp.xcodeproj -scheme Prexp -configuration Debug build
open ~/Library/Developer/Xcode/DerivedData/Prexp-*/Build/Products/Debug/Prexp.app
```

Regenerate the Xcode project (`xcodegen generate`) after adding/removing files under
`App/Sources` or editing `project.yml`.

## Features

- **TUI** — live process table (PID/NAME/CPU%/MEM/PMEM/THR/FILES/TOTAL), CPU% deltas,
  sort cycle (`s`/`S`), `/` search, detail + help overlays, a stats header (per-core
  P/E usage, memory gauge, load), PID-anchored selection.
- **App** — process table, stats header (memory, per-core P/E grid, load, battery,
  Swift Charts CPU-history chart), detail pane, send-signal (⌘-menu / context menu),
  the **info panel** (⌘I — Overview / Resources / Network / Environment), and
  **reverse path lookup** (⌘F, or right-click a resource → find its holders).

The two things `tint` didn't ship — a text-input widget and a centered modal — were
added to `tint` itself (`Popup`/`Clear`/`TextInput`/`Rect.centered`). Deferred
features are tracked in [CLAUDE.md](CLAUDE.md).
