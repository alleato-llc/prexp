# prexp

A process explorer focused on **open file descriptors** — see the files, sockets,
and pipes each process holds, alongside CPU and memory. Inspired by htop, but built
around fd visibility. Native macOS APIs (libproc/Mach), no dependency on `lsof`.

```
╭─ Processes  ·  CPU% ▼ ───────────────────────────────────────────────────────────────╮
│ PID      NAME                         CPU%   MEM      PMEM     THR  FILES  TOTAL       │
│ 597      zed                          28.4   1.5G     1.1G     55   251    370         │
│ 698      zed                          0.1    11.3M    7.8M     1    3      6           │
│ 4961     com.apple.WebKit.Networking  0.1    455.2M   89.1M    8    99     109         │
╰──────────────────────────────────────────────────────────────────────────────────────╯
```

## A monorepo of two implementations

prexp is **ecosystem-first**: two *independent* implementations of the same tool —
one Rust, one Swift — that never link against each other (no FFI). Each is layered
`core → front-ends`, and a live diff keeps them honest. Start with
**[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** for the common design.

| | |
|---|---|
| **[rust/](rust/README.md)** | The Rust workspace: `prexp-ffi` + `prexp-core`, a ratatui **TUI** (`prexp`), and an iced **GUI** (`prexp-desktop`) on the sibling `rime` kit. |
| **[swift/](swift/README.md)** | The Swift stack: `PrexpCore` (native, no C shim), a **TUI** on the sibling [`tint`](../tint) kit, and a native SwiftUI **app**. |
| **[docs/](docs/README.md)** | Shared design, parity, and native-API docs. |
| **[scripts/parity.py](scripts/parity.py)** | The parity oracle — a live cross-implementation diff (see [docs/PARITY.md](docs/PARITY.md)). |

Contributing? See **[CONTRIBUTING.md](CONTRIBUTING.md)**. What prexp can access and
how to report issues: **[SECURITY.md](SECURITY.md)**.

## Quickstart

```sh
# Rust (from rust/)
cd rust && cargo run -p prexp                # the TUI
cd rust && cargo run -p prexp -- --output json

# Swift
cd swift/TUI && swift run prexp              # the TUI, on tint
cd swift/App && xcodegen generate && xcodebuild -scheme Prexp build   # the SwiftUI app
```

Full build/run/usage per ecosystem: [rust/README.md](rust/README.md) ·
[swift/README.md](swift/README.md). Prerequisites: macOS 14+, Rust 1.80+ and/or
Swift 6.2+; the GUIs need the sibling `rime` (Rust) / `tint` (Swift) kits checked out
next to this repo.

## Documentation

- **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** — the common design (start here).
- **[docs/PARITY.md](docs/PARITY.md)** — how the two implementations are kept in sync.
- **[docs/FFI.md](docs/FFI.md)** — the native macOS APIs both sides reimplement.
- **[docs/TESTING.md](docs/TESTING.md)** · **[docs/MIGRATION.md](docs/MIGRATION.md)**
- Ecosystem entry points: [rust/README.md](rust/README.md) ·
  [swift/README.md](swift/README.md). Agent context: [CLAUDE.md](CLAUDE.md).

## License

See [LICENSE](LICENSE).
