# Migration — the modular-monolith refactor

The decision record and history behind the current layout: why prexp is a
`rust/` + `swift/` monorepo of two independent implementations. The "why the layout
is this way" reference.

## Context

prexp began as a single Cargo workspace (`crates/{prexp-ffi, prexp-core, prexp,
prexp-desktop}`) — one Rust implementation with a TUI and an iced GUI. It was
refactored to mirror the sibling **`soroban`** project's ecosystem-first modular
monolith and to add a parallel **Swift** implementation.

## Decisions

- **Faithful mirror of soroban's layout.** Sibling top-level `rust/` + `swift/`,
  with shared `docs/` and cross-cutting `scripts/`. The whole Cargo workspace moved
  under `rust/`.
- **Two independent implementations, no FFI.** Like soroban, the Swift side is a
  native reimplementation, not a binding over the Rust core. It reaches libproc/Mach
  straight from `Darwin` — a de-risking probe confirmed **no C shim is needed**. See
  [ARCHITECTURE.md](ARCHITECTURE.md#two-independent-implementations--no-ffi).
- **Keep both GUIs.** The Rust iced/`rime` `prexp-desktop` is preserved alongside
  the new native SwiftUI app — parallel ecosystems, exactly like soroban.
- **`prexp-desktop` excluded from the workspace.** Its heavy iced + sibling-`rime`
  dependency graph builds standalone (mirrors soroban excluding its GUI crate).
- **Parity via live diff, not a shared Gherkin spec.** prexp's live, non-
  deterministic data makes a fixture oracle low-value; `scripts/parity.py` diffs the
  two implementations against the real system instead. See [PARITY.md](PARITY.md).

## History

| Milestone | What landed |
|---|---|
| **M0** | Restructure into `rust/` + `swift/`; split `CLAUDE.md` (root cross-cutting + per-ecosystem); exclude `prexp-desktop`. |
| **M1** | Swift **Core** (`PrexpCore`) — native `ProcessSource`, models, formatters. Verified vs Rust: 989 processes, 100% match on name/ppid/state/accessibility + fd paths. |
| **M2** | Swift **TUI** on `tint` — process table, CPU% deltas, sort/search/detail/help/stats header. Added `Popup`/`Clear`/`TextInput`/`Rect.centered` to `tint`. |
| **M3** | Swift **App** (SwiftUI) — table, stats header + Swift Charts CPU history, detail pane, send-signal. |
| post-M3 | Core **network parsing** (found + fixed a latent Rust `InSockInfo` bug); **`scripts/parity.py`**; GUI **info panel** + **reverse lookup**. |

## Sibling repositories

Checked out next to this one:

- **`../tint`** — the Swift immediate-mode TUI kit the Swift TUI is built on.
- **`../rime`** — the iced component kit the Rust GUI (`prexp-desktop`) is built on.
- **`../soroban`** — the reference modular-monolith this layout mirrors.

## Not done / deferred

A shared Gherkin `spec/` (superseded by the live parity check); Core system-wide
network/disk counters; some front-end features (theme pickers; the Swift TUI's
signal picker / reverse lookup / file view). Tracked in the ecosystem `CLAUDE.md`s.
