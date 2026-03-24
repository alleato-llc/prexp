# prexp

Process explorer — a terminal UI for inspecting open file descriptors, CPU usage, and memory per process. Native macOS backend via libproc FFI, no dependency on `lsof`.

```
┌ Processes [/zed] ───────────────────────────────────────────────────────────────────────────────────┐
│  PID      NAME                        CPU%   MEM     PMEM    THR  FILES  SOCKS  PIPES  OTHER  TOTAL │
│▶ 698      zed                         0.1    11.3M   7.8M    1    3      2      0      1      6     │
│  597      zed                         28.4   1.5G    1.1G    55   251    41     56     22     370   │
│                                                                                                     │
│                                                                                                     │
│                                                                                                     │
│                                                                                                     │
└─────────────────────────────────────────────────────────────────────────────────────────────────────┘
 / zed█  (Enter to confirm, Esc to cancel)
 ```

## Prerequisites

- Rust 1.70+ (stable)
- macOS (Linux backend is stubbed)

## Quickstart

```bash
# Build
cargo build

# Run the TUI
cargo run -p prexp

# Run tests
cargo test

# CLI output modes
cargo run -p prexp -- --output json              # JSON output
cargo run -p prexp -- --output tsv               # Tab-separated values
cargo run -p prexp -- --output json --pid 1234   # Single process
cargo run -p prexp -- --output json --path /dev/null  # Reverse lookup

# Process info (JSON)
cargo run -p prexp -- --pid 1234 --info              # All info tabs
cargo run -p prexp -- --pid 1234 --info overview     # Identity only
cargo run -p prexp -- --pid 1234 --info resources    # Resources only
cargo run -p prexp -- --pid 1234 --info network      # Network connections
cargo run -p prexp -- --pid 1234 --info env          # Environment variables
```

## Architecture

prexp is a Cargo workspace with three crates:

```
prexp (binary) ──> prexp-core (library) ──> prexp-ffi (FFI)
       └──────────────────────────────────> prexp-ffi (direct)
```

- **prexp-ffi** — Raw FFI bindings and safe Rust wrappers for macOS `libproc.h` and Mach APIs. All `unsafe` code is contained here.
- **prexp-core** — Platform-agnostic domain models, `ProcessSource` trait, backend implementations, and output formatters (JSON, TSV).
- **prexp** — Binary crate with CLI argument parsing (clap), TUI (ratatui + crossterm), and theming.

## Project Structure

```
crates/
├── prexp-ffi/                    # FFI crate (macOS)
│   └── src/
│       ├── raw.rs                # extern "C", #[repr(C)] structs, Mach API bindings
│       ├── error.rs              # FfiError, errno helpers, Mach timebase conversion
│       ├── process.rs            # Process APIs (list_all_pids, get_process_info, list_fds, resolve_fd)
│       └── system.rs             # System APIs (get_cpu_ticks, get_memory_info)
├── prexp-core/                   # Core library
│   └── src/
│       ├── models.rs             # ProcessSnapshot, OpenResource, ResourceKind
│       ├── source.rs             # ProcessSource trait
│       ├── error.rs              # PrexpError (thiserror)
│       ├── backend/
│       │   ├── macos.rs          # MacosProcessSource
│       │   └── linux.rs          # LinuxProcessSource (stub)
│       └── output/
│           ├── json.rs           # JSON formatter
│           └── tsv.rs            # TSV formatter
└── prexp/                        # Binary crate
    └── src/
        ├── main.rs               # Entry point
        ├── cli.rs                # Clap argument parsing
        └── tui/
            ├── app/
            │   ├── mod.rs        # App state, navigation, rebuild, overlays
            │   ├── sorting.rs    # Sort field cycling, direction
            │   ├── search.rs     # Search, reverse lookup, clipboard
            │   ├── stats.rs      # CPU%, system stats, memory formatting
            │   └── tree.rs       # Process tree builder
            ├── ui/
            │   ├── mod.rs        # Draw dispatcher, status bar
            │   ├── process_list.rs  # Process table
            │   ├── file_list.rs     # File table + detail
            │   └── overlays.rs      # Summary, help, theme, config, process detail
            ├── event.rs          # Key binding dispatch
            └── theme.rs          # 9 color themes
```

## TUI Usage

### Views

- **Process view** (default) — tree of processes with CPU%, memory (RSS + private), thread count, and fd breakdown
- **File view** (`v`) — deduplicated list of all open file paths with process count
- **Detail overlay** (`Enter`) — shows fds for a process, or processes for a file
- **System summary** (`g`) — per-CPU core usage bars, memory usage bar, process/thread/fd totals

### Keybindings

| Key | Action |
|-----|--------|
| `q` | Quit (closes overlay first) |
| `Esc` | Close overlay / clear search |
| `j/k` / arrows | Navigate |
| `Enter` | Open detail overlay (or clear active search) |
| `v` | Toggle process / file view |
| `/` | Search (Enter to confirm, `n` for next match) |
| `s` / `S` | Cycle sort field / reverse direction |
| `t` | Open theme picker (live preview) |
| `c` | Configure visible columns |
| `i` | Process info panel (4 tabs) |
| `r` | Reverse lookup (process view) |
| `a` | Toggle show-all (include inaccessible processes) |
| `g` | Toggle system summary header |
| `R` | Force refresh |
| `y` | Copy path to clipboard |
| `h/l` | Horizontal scroll (detail overlay) |
| `?` | Help legend |

### Process View Columns

All configurable via `c`:

`PID` `NAME` `CPU%` `MEM` `PMEM` `THR` `FILES` `SOCKS` `PIPES` `OTHER` `TOTAL`

- **CPU%** — per-core percentage (100% = one full core), computed from delta between refreshes
- **MEM** — resident set size (RSS), matches `top`/htop
- **PMEM** — physical footprint (private memory), matches Activity Monitor

### Themes

9 built-in themes, selectable via `t` with live preview:

Default, Nord, Dracula, Solarized, Monokai, Gruvbox, Tokyo Night, Retro, Royal Purple

## Documentation

See `CLAUDE.md` for architecture details, FFI specifics, and development conventions. Skills are available in `.claude/skills/` for project patterns.
