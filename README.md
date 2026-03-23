# prexp

Process explorer — a terminal UI for inspecting open file descriptors, CPU usage, and memory per process. Native macOS backend via libproc FFI, no dependency on `lsof`.

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
│       └── safe.rs               # Safe wrappers, Mach timebase conversion
├── prexp-core/                   # Core library
│   └── src/
│       ├── models.rs             # ProcessSnapshot, OpenResource, ResourceKind
│       ├── source.rs             # ProcessSource trait
│       ├── error.rs              # FdtopError (thiserror)
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
            ├── app.rs            # App state, CPU%, sorting, tree, file view, column config
            ├── ui.rs             # ratatui rendering
            ├── event.rs          # Key binding dispatch
            └── theme.rs          # 9 color themes
```

## TUI Usage

### Views

- **Process view** (default) — tree of processes with CPU%, memory (RSS + private), thread count, and fd breakdown
- **File view** (`v`) — deduplicated list of all open file paths with process count
- **Detail overlay** (`Enter`) — shows fds for a process, or processes for a file

### Keybindings

| Key | Action |
|-----|--------|
| `q` | Quit (closes overlay first) |
| `Esc` | Close overlay / clear search |
| `j/k` / arrows | Navigate |
| `Enter` | Open detail overlay |
| `v` | Toggle process / file view |
| `/` | Search |
| `s` / `S` | Cycle sort field / reverse direction |
| `t` | Open theme picker (live preview) |
| `c` | Configure visible columns |
| `r` | Reverse lookup (process view) |
| `a` | Toggle show-all (include inaccessible processes) |
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
