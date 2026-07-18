# prexp — Rust ecosystem

The Rust implementation of prexp: a Cargo workspace with two front-ends over one
core. Native macOS backend via libproc FFI, no dependency on `lsof`.

For the shared design, start with [../docs/ARCHITECTURE.md](../docs/ARCHITECTURE.md);
the native APIs are in [../docs/FFI.md](../docs/FFI.md). This page is the Rust
ecosystem entry point. The exhaustive agent-facing notes (every keybinding, key
files) are in [CLAUDE.md](CLAUDE.md).

## The workspace

| Crate | What it is | In workspace? |
|---|---|---|
| `crates/prexp-ffi/` | Raw `extern "C"` bindings + safe wrappers for libproc/Mach. All `unsafe` lives here. | yes |
| `crates/prexp-core/` | Domain models, the `ProcessSource` trait, backends, JSON/TSV formatters. | yes |
| `crates/prexp/` | The **TUI** binary (ratatui + crossterm) and the CLI output modes. | yes |
| `crates/prexp-desktop/` | The **GUI** binary — a native iced app on the sibling [`rime`](../../rime) kit. | **excluded** |

**`prexp-desktop` is excluded from the workspace** (`exclude` in `Cargo.toml`): it
pulls in iced/wgpu + the sibling `rime` crate (`../../../../rime/rime`), so it builds
and tests standalone with its own target dir — never via `--workspace`. Full
GUI docs: [crates/prexp-desktop/README.md](crates/prexp-desktop/README.md).

## Build & run

```sh
cd rust                       # the workspace root — run cargo commands here

cargo build
cargo run -p prexp            # the TUI
cargo test                    # see ../docs/TESTING.md

# CLI output modes
cargo run -p prexp -- --output json                     # grouped JSON
cargo run -p prexp -- --output tsv                      # one row per open resource
cargo run -p prexp -- --output json --pid 1234          # a single process
cargo run -p prexp -- --output json --path /dev/null    # reverse lookup
cargo run -p prexp -- --pid 1234 --info network         # a detail tab as JSON

# The GUI is built standalone (heavy iced/rime deps)
cd crates/prexp-desktop && cargo run                 # 2s refresh
cd crates/prexp-desktop && cargo run -- --interval 1 # custom cadence
```

Prerequisites: Rust 1.80+ (stable), macOS (the Linux backend is a stub). The GUI
also needs the sibling `rime` crate checked out at `../rime`.

## TUI at a glance

A live process/file explorer. Full keybinding tables (main view, detail overlay,
info panel, theme/column pickers) are in [CLAUDE.md](CLAUDE.md); the essentials:

| Key | Action |
|---|---|
| `j`/`k`, arrows | navigate · `Enter` detail overlay |
| `/` | search (name/pid or path) · `n` next match |
| `v` | toggle process / file view |
| `s` / `S` | cycle sort field / reverse |
| `i` | process info panel (Overview / Resources / Network / Environment) |
| `r` | reverse lookup · `K` send signal · `a` show inaccessible |
| `g` | system summary · `t` themes · `c` columns · `?` help · `q` quit |

- **Columns** (configurable via `c`): PID, NAME, CPU%, MEM, PMEM, THR, FILES, SOCKS,
  PIPES, OTHER, TOTAL.
- **Themes** (`t`, live preview): Default, Nord, Dracula, Solarized, Monokai,
  Gruvbox, Tokyo Night, Retro, Royal Purple.
