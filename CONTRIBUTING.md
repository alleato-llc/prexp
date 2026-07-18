# Contributing to prexp

prexp is an ecosystem-first monorepo: two independent implementations of the same
process explorer — a **Rust** stack and a **Swift** stack — that never link against
each other. Read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) first for the big
picture.

## The golden rule: keep the two implementations at parity

The two implementations have no shared binary — their only contract is behavioral.
So a change to the data layer must land on **both** sides and stay in agreement:

1. change `prexp-ffi` / `prexp-core` (Rust) **and** `PrexpCore` (Swift),
2. keep the domain-model shapes / JSON output matching,
3. run the parity check.

```sh
scripts/parity.py        # diffs both implementations against the live system
```

Unlike `soroban`, prexp keeps parity with a **live diff**, not a shared Gherkin
spec — its data is non-deterministic, and the interesting surface is the native
libproc/Mach layer a fixture oracle can't reach. See [docs/PARITY.md](docs/PARITY.md)
(a real Rust struct-layout bug was caught this way). Front-end-only or pure-logic
changes don't need both sides, but must keep their own tests green.

## Where things live

- **Shared design & reference:** [docs/](docs/) — architecture, parity, FFI, testing.
- **Rust:** [rust/README.md](rust/README.md) (+ [rust/CLAUDE.md](rust/CLAUDE.md)).
- **Swift:** [swift/README.md](swift/README.md) (+ [swift/CLAUDE.md](swift/CLAUDE.md)).
- **Sibling kits:** `../tint` (Swift TUI), `../rime` (Rust GUI) — checked out next
  to this repo. Changes that need a new widget land in those repos (e.g. `tint`'s
  `Popup`/`TextInput` were added there).

When working inside `rust/` or `swift/`, that directory's `CLAUDE.md` is
authoritative; the repo-root `CLAUDE.md` covers only cross-cutting concerns.

## Build & test (quick reference)

```sh
# Rust
cd rust && cargo test
cd rust/crates/prexp-desktop && cargo test   # the GUI (excluded from the workspace)

# Swift
cd swift/Core && swift test
cd swift/TUI && swift test
cd swift/App && xcodegen generate && xcodebuild -scheme Prexp build

# Parity
scripts/parity.py
```

Full instructions and the testing strategy: [docs/TESTING.md](docs/TESTING.md) and
each ecosystem's `README.md`.

## Conventions

- **Branching:** work on a feature branch; the default branch is `main`.
- **Commits:** imperative subject; explain the *why* in the body. Keep a data-layer
  change and its parity fix in the same commit when they're one logical change.
- **Docs live with code:** update the relevant `README.md` / `docs/` / `CLAUDE.md`
  in the same change. Document what exists, not what's planned.
- **Errors:** `thiserror` in Rust libraries, `anyhow` in `main`; a `PrexpError` enum
  in Swift. All `unsafe` stays in `prexp-ffi`.

There is no release/CI pipeline set up yet; this is source-first.
