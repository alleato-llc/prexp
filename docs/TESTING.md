# Testing

Testing is layered to match the [architecture](ARCHITECTURE.md): pure logic is
unit-tested inside each ecosystem, native extraction + end-to-end shape is verified
by the cross-implementation [parity check](PARITY.md), and front-ends have their own
behavioral/rendering tests.

## Rust (`rust/`)

```sh
cd rust
cargo test                 # unit + integration (test doubles), all workspace crates
cargo test -- --ignored    # FFI smoke tests that touch the real system
cd crates/prexp-desktop && cargo test              # GUI behavioral tests (standalone)
cd crates/prexp-desktop && cargo test -- --ignored # headless wgpu view snapshots
```

- Integration tests drive the code against a `ProcessSource` **test double** with
  canned data — no live system, deterministic.
- `prexp-desktop` is excluded from the workspace; test it from its own directory.
- Its view snapshots render on **wgpu** (not headless tiny-skia, which doesn't
  rasterize the canvas chart) and are `#[ignore]`d. Delete a PNG to re-baseline.

## Swift (`swift/`)

```sh
# Core — pure-logic unit tests over canned data
cd swift/Core && swift test
cd swift/Core && .build/debug/prexp-smoke json    # eyeball live output

# TUI — behavior + render tests (render a frame to an offscreen Buffer, inspect text)
cd swift/TUI && swift test
cd swift/TUI && .build/debug/prexp --snapshot 110x30 --stats   # live frame, no TTY

# App — compiles/links (behavioral logic is the shared, already-tested Core)
cd swift/App && xcodegen generate && xcodebuild -scheme Prexp build
```

- Both TUI and App drive an `AppState`/`AppModel` against a `Send`-safe
  `ProcessSource` double; the doubles override `killProcess` → no-op so tests never
  signal a real process.
- The TUI's `View.render` is a pure function, tested by rendering into a `Buffer`
  and asserting on `allText()` — the tint/immediate-mode testing style.
- `tint`'s own additions (Popup/Clear/TextInput) are tested in the `tint` repo.

## Cross-ecosystem parity

```sh
scripts/parity.py          # runs both implementations, diffs live output
```

The guardrail that the Rust and Swift data layers agree. Run it after changing
`prexp-ffi`/`prexp-core` or `PrexpCore`. Full detail in [PARITY.md](PARITY.md).

## What's tested where

| Concern | Test |
|---|---|
| JSON/TSV formatting, sorting, filtering, CPU-delta, network formatting | unit tests (both ecosystems) |
| libproc/Mach extraction + end-to-end field/JSON shape | `scripts/parity.py` |
| TUI state machine + rendering | Swift `PrexpTUITests`; Rust `prexp` app-state tests |
| GUI behavior | Rust `prexp-desktop` behavioral tests + wgpu snapshots; Swift App compiles over the tested Core |
