# CLAUDE.md — Rust ecosystem (authoritative for `rust/`)

> This is the **Rust** half of a modular monolith (see the repo-root `CLAUDE.md` for the
> cross-cutting overview and the parallel `swift/` ecosystem). The Cargo workspace lives in
> this `rust/` directory — **run all `cargo` commands from here.** The Swift side is an
> independent reimplementation; there is no FFI between them.
>
> Shared design/parity/FFI docs: [`../docs/`](../docs/README.md) — start with
> [`ARCHITECTURE.md`](../docs/ARCHITECTURE.md); the native-API reference is
> [`FFI.md`](../docs/FFI.md). User-facing usage: [`README.md`](README.md).

## Project Overview

prexp (process explorer) displays open file descriptors, CPU usage, and memory per process. It targets developers and power users who need to inspect what files, sockets, and pipes a process has open. Inspired by htop but focused on file descriptor visibility. Two front-ends share one core: a TUI (`prexp`) and a native desktop GUI (`prexp-desktop`).

## Build & Test

```bash
# From rust/ (the workspace root). prexp-desktop is EXCLUDED from the workspace.
cargo build                 # Build all workspace crates (prexp, prexp-core, prexp-ffi)
cargo test                  # Run all tests
cargo test -- --ignored     # Also run FFI smoke tests
cargo run -p prexp          # Run the TUI app
cargo run -p prexp -- --output json   # JSON output
cargo run -p prexp -- --output tsv    # TSV output
cargo run -p prexp -- --help          # CLI usage

# prexp-desktop is built/run standalone from its own directory (heavy iced/rime deps):
cd crates/prexp-desktop && cargo run                    # desktop GUI (2s refresh)
cd crates/prexp-desktop && cargo run -- --interval 1    # custom refresh cadence (seconds)
cd crates/prexp-desktop && cargo test -- --ignored      # GUI view snapshots (wgpu)
```

## Architecture

- **Workspace**: 4 crates (3 in the workspace + `prexp-desktop` excluded, built standalone). Dependency chain `prexp` → `prexp-core` → `prexp-ffi`, plus `prexp` → `prexp-ffi` (direct); and `prexp-desktop` → `prexp-core` + the sibling `rime` crate (`../../../../rime/rime`). `prexp-desktop` is excluded from the workspace (heavy iced/rime graph) — build it from `crates/prexp-desktop`. Both binaries hold the backend behind `prexp-core`'s `ProcessSource` trait.
- **prexp-ffi**: Raw FFI bindings + safe wrappers for macOS libproc and Mach APIs. All unsafe code contained here. Handles Mach timebase conversion for CPU time on Apple Silicon.
- **prexp-core**: Domain models (`ProcessSnapshot`, `OpenResource`, `ResourceKind`), `ProcessSource` trait, platform backends (macOS, Linux stub), output formatters (JSON, TSV).
- **prexp**: TUI binary — CLI parsing (clap), TUI (ratatui + crossterm), application state, event handling, themes.
- **prexp-desktop**: GUI binary — a native iced app on the sibling `rime` component kit. See `crates/prexp-desktop/README.md` and the "prexp-desktop (GUI)" section below.

### Key Design Decisions

- No `lsof` — native platform APIs only
- No `libproc` crate — custom FFI in prexp-ffi
- `ProcessSource` trait enables backend swapping and test doubles
- All unsafe contained in prexp-ffi; downstream crates are safe Rust
- `thiserror` for library errors, `anyhow` in main.rs only
- CPU% computed via delta between refreshes (Mach absolute time → nanoseconds via `mach_timebase_info`)
- Memory: RSS from `proc_taskinfo`, physical footprint from `task_info(TASK_VM_INFO)` via `task_name_for_pid`
- Process tree built from PPID relationships; sorting reorders only roots, children stay grouped
- Anchor-based position tracking preserves selection across refreshes (PID for processes, path for files)
- Esc key does NOT quit from main view (prevents false exits from rapid arrow key escape sequence splitting)

## TUI Keybindings

### Main views
| Key | Action |
|-----|--------|
| `q` | Quit (closes overlay first) |
| `Esc` | Close overlay / clear search |
| `j/k` / arrows | Navigate |
| `Enter` | Open detail overlay (or clear active search) |
| `/` | Search (filters by name/pid or path) |
| `n` | Next search match (when search is active) |
| `v` | Toggle between process and file views |
| `s` / `S` | Cycle sort field / reverse direction |
| `t` | Open theme picker |
| `c` | Open column configuration |
| `i` | Process info panel (Overview, Resources, Network, Environment) |
| `r` | Reverse lookup (process view only) |
| `a` | Toggle show-all (include inaccessible processes) |
| `K` | Send signal to selected process (signal picker + confirmation) |
| `g` | Toggle system summary (CPU cores, memory, totals) |
| `R` | Force refresh |
| `y` | Copy selected path to clipboard (file view / detail) |
| `?` | Open help legend |

### Detail overlay
| Key | Action |
|-----|--------|
| `q` / `Esc` | Close overlay (clears search first) |
| `j/k` / arrows | Navigate resources |
| `h/l` / left/right | Horizontal scroll |
| `/` | Search resources by path or kind |
| `f` | Filter by kind (File, Socket, Pipe, Device, Kqueue) |
| `y` | Copy selected path to clipboard |

### Theme picker (`t`)
| Key | Action |
|-----|--------|
| `j/k` / arrows | Navigate and live preview |
| `Enter` / `q` / `Esc` | Close and apply |

### Info panel (`i`)
| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Cycle tabs forward / backward |
| `1`/`2`/`3`/`4` | Jump to tab (Overview, Resources, Network, Environment) |
| `j/k` / arrows | Scroll within tab (selects row in Environment tab) |
| `y` | Copy selected environment variable (`KEY=VALUE`) |
| `Y` | Copy all environment variables to clipboard |
| `q` / `Esc` / `i` | Close panel |

### Column config (`c`)
| Key | Action |
|-----|--------|
| `j/k` / arrows | Navigate columns |
| `Enter` / `Space` | Toggle column on/off |
| `q` / `Esc` / `c` | Close config |

### Process view columns (all configurable via `c`)
PID, NAME (always shown), CPU%, MEM (RSS), PMEM (private), THR, FILES, SOCKS, PIPES, OTHER, TOTAL

### Process view sort modes (`s` cycles)
Unsorted (tree) → PID → Name → Total → Unsorted

### File view sort modes (`s` cycles)
Process count (default, desc) → Filename → Process count

### Themes (`t` opens picker with live preview)
Default, Nord, Dracula, Solarized, Monokai, Gruvbox, Tokyo Night, Retro, Royal Purple

## Key Files

- `crates/prexp-ffi/src/raw.rs` — extern "C" bindings, #[repr(C)] structs, Mach API bindings
- `crates/prexp-ffi/src/error.rs` — FfiError type, errno helpers, Mach timebase conversion
- `crates/prexp-ffi/src/process.rs` — process-level APIs: `list_all_pids`, `get_process_info`, `list_fds`, `resolve_fd`, `list_pids_by_path`
- `crates/prexp-ffi/src/system.rs` — system-level APIs: `get_cpu_ticks`, `get_memory_info`
- `crates/prexp-core/src/models.rs` — `ProcessSnapshot`, `OpenResource`, `ResourceKind`
- `crates/prexp-core/src/source.rs` — `ProcessSource` trait
- `crates/prexp-core/src/error.rs` — `PrexpError` (thiserror)
- `crates/prexp-core/src/backend/macos.rs` — `MacosProcessSource` implementation
- `crates/prexp-core/src/output/` — JSON, TSV formatters
- `crates/prexp/src/cli.rs` — CLI argument parsing (clap)
- `crates/prexp/src/tui/app/mod.rs` — App state, navigation, rebuild logic, overlays
- `crates/prexp/src/tui/app/sorting.rs` — Sort field cycling, direction, process index sorting
- `crates/prexp/src/tui/app/search.rs` — Search, reverse lookup, clipboard
- `crates/prexp/src/tui/app/stats.rs` — CPU%, system stats, memory formatting
- `crates/prexp/src/tui/app/tree.rs` — Process tree builder
- `crates/prexp/src/tui/ui/mod.rs` — Draw dispatcher, status bar
- `crates/prexp/src/tui/ui/process_list.rs` — Process table rendering
- `crates/prexp/src/tui/ui/file_list.rs` — File table + file detail overlay
- `crates/prexp/src/tui/ui/overlays.rs` — Summary header, process detail, help, theme picker, column config
- `crates/prexp/src/tui/event.rs` — Keybinding dispatch
- `crates/prexp/src/tui/theme.rs` — 9 color themes (Default, Nord, Dracula, Solarized, Monokai, Gruvbox, Tokyo Night, Retro, Royal Purple)
- `crates/prexp-desktop/src/lib.rs` — iced application wiring (`run`)
- `crates/prexp-desktop/src/app/mod.rs` — GUI `App` state, `update`, subscription, background tasks
- `crates/prexp-desktop/src/app/system.rs` — `Sample` + `SystemStats` (per-core CPU delta, CPU history)
- `crates/prexp-desktop/src/view/` — table, detail pane, stats header (chart), info/signal/lookup modals
- `crates/prexp-desktop/src/snapshot.rs` — headless view snapshot tests (wgpu)

## prexp-desktop (GUI)

A native iced app on the sibling `rime` component kit (`rime = { path = "../../../../rime/rime" }`; iced 0.14 re-exported through rime). Full details in `crates/prexp-desktop/README.md`. Conventions worth knowing:

- **Source behind a trait object**: `App` holds `Arc<dyn ProcessSource + Send + Sync>` (IoC — swappable for a test double, shareable into tasks). Native backend is `prexp_core::backend::NativeSource`.
- **Heavy work off the UI thread**: `snapshot_all` + system metrics are gathered in a background `iced::Task` (`sample_task`) → `Message::Refreshed(Sample)`; `process_detail` and `find_by_path` run in their own tasks; only `kill_process` runs inline (fast syscall). A timer subscription (`iced::time::every`) drives the `--interval` cadence.
- **Delta metrics**: per-process and per-core CPU% from `cpu_time_ns`/tick deltas between refreshes, plus a rolling CPU-history buffer for the chart — mirrors the TUI's `stats.rs`.
- **Controls** live in the title strip: a sort dropdown + **Info / Signal / Find / Stats / Theme** buttons; row click selects a process (fills the detail pane).
- **Testing**: behavioral tests drive the public `App` message API against a `Send + Sync` `ProcessSource` double (`tests/`); headless view snapshots (`src/snapshot.rs`, via `iced_test`) render to PNGs and are `#[ignore]`d. **Snapshots render on wgpu** (the default), not `tiny-skia` — the headless `tiny-skia` backend doesn't rasterize the stats-header canvas chart's strokes. Run: `cargo test -p prexp-desktop -- --ignored snapshot`; delete a PNG to re-baseline. Both test doubles override `kill_process` → `Ok(())` so tests never signal a real process.

## FFI Details (macOS)

### libproc APIs used
- `proc_listallpids` — enumerate all PIDs
- `proc_pidinfo(PROC_PIDTBSDINFO)` — PPID, process name (32 chars via `pbi_name`)
- `proc_pidinfo(PROC_PIDTASKINFO)` — thread count, RSS, CPU time, page faults, context switches, syscalls
- `proc_pid_rusage(RUSAGE_INFO_V4)` — disk I/O bytes read/written
- `proc_pidinfo(PROC_PIDLISTFDS)` — list open file descriptors
- `proc_pidfdinfo` — resolve fd details (vnode path, socket info, pipe)
- `proc_listpidspath` — reverse lookup (PIDs with a given path open)

### Mach APIs used
- `mach_timebase_info` — convert Mach ticks to nanoseconds (cached via OnceLock, handles Apple Silicon ratio 125:3)
- `task_name_for_pid` — get task port without root
- `task_info(TASK_VM_INFO)` — physical footprint (private memory, matches Activity Monitor)
- `host_processor_info(PROCESSOR_CPU_LOAD_INFO)` — per-CPU core tick counts (user/system/idle/nice)
- `host_statistics64(HOST_VM_INFO64)` — system memory stats (free/active/wired/compressed pages)

### sysctl APIs used
- `sysctlbyname("hw.memsize")` — total physical memory
- `sysctlbyname("hw.pagesize")` — page size for VM stats conversion

## Skills

Available skills in `.claude/skills/`:

### Core
- **project-structure** — Domain-oriented module layout, lib/bin split
- **component-design** — Services, repositories, clients, calculators
- **error-handling** — thiserror for library errors, anyhow for application
- **inversion-of-control** — Traits as contracts, trait objects for DI

### Testing
- **adding-unit-tests** — Tests for pure business logic
- **adding-integration-tests** — Tests with test doubles
- **testing-boundaries** — Trait-based test doubles, RefCell/Rc patterns
- **test-data-isolation** — Fresh state per test

### Documentation
- **project-documentation** — Documentation structure and conventions
