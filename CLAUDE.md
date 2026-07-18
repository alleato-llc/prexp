# CLAUDE.md (root — cross-cutting)

This repo is a **modular monolith** modeled on the sibling `soroban` project: two
*independent* implementations of the same tool (a process / file-descriptor explorer),
one per language, that **never link against each other**. There is no FFI, C-ABI, or
shared binary between them — each side reimplements the domain natively.

```
fdtop/
├── rust/     Rust ecosystem — see rust/CLAUDE.md (AUTHORITATIVE for Rust work)
├── swift/    Swift ecosystem — see swift/CLAUDE.md (AUTHORITATIVE for Swift work)  [in progress]
├── spec/     shared behavior spec — the parity oracle both sides run  [planned]
└── docs/     shared cross-language docs  [planned]
```

**When working inside `rust/` or `swift/`, that directory's `CLAUDE.md` is authoritative.**
This root file only covers what spans both.

## The two ecosystems

Each side is layered the same way: `core → front-ends`.

- **`rust/`** — a Cargo workspace: `prexp-ffi` (macOS libproc/Mach FFI) → `prexp-core`
  (models, `ProcessSource` trait, formatters) → `prexp` (TUI) and `prexp-desktop`
  (iced/`rime` GUI, **excluded from the workspace**, built standalone). Build/test from
  `rust/`. Full detail in `rust/CLAUDE.md`.
- **`swift/`** — a parallel Swift port _(in progress)_: `Core` (a native `ProcessSource`
  over the same libproc/Mach APIs via a small C shim + Codable models + formatters), `TUI`
  (on the sibling `tint` immediate-mode kit), and `App` (a native SwiftUI GUI). Full detail
  in `swift/CLAUDE.md`.

## Cross-cutting rules

- **No Rust↔Swift linkage.** The Swift side is a reimplementation, not a binding. The only
  contract between them is behavioral: matching domain model field names / JSON shapes, and
  (eventually) the shared `spec/`.
- **Keep the two in parity.** Both front-end families (TUI, GUI) exist on each side on
  purpose. A domain behavior change should land on both sides (and, once it exists, as a
  `spec/` edit first — the soroban rule).
- **Parity is checked live, not by a fixture spec.** fdtop's interesting surface is the native
  libproc/Mach data extraction, which a canned-fixture Gherkin oracle can't reach (and where a
  real divergence already hid — a Rust struct bug that mislabeled every TCP state). So the
  parity guardrail is `scripts/parity.py`: it runs both implementations, joins their process
  snapshots on PID, and checks stable fields (name/ppid/state/accessible), fd-path overlap, and
  a sample of network tables — with thresholds and a CI exit code. Run it after any change to
  either side's data layer. The pure logic (formatters/sort/filter) is separately unit-tested on
  both sides. A shared Gherkin `spec/` remains possible but is low-value for this domain.

## Sibling repos (checked out next to this one)

- `../rime` — iced component kit used by the Rust GUI (`prexp-desktop`).
- `../tint` — Swift immediate-mode TUI kit used by the Swift TUI.
- `../soroban` — the reference modular-monolith this layout mirrors.
