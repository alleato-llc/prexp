# prexp-desktop

A native desktop GUI for prexp — the same process/file-descriptor explorer as the
`prexp` TUI, built on [iced](https://iced.rs) via the sibling
[`rime`](../../../../rime) component kit. It surfaces `prexp-core`'s
`ProcessSource` API as a windowed app: a live process table with fd counts, a
per-process resource detail pane, a system-stats header (per-core CPU, memory,
load, battery, CPU-history chart), a tabbed info panel, send-signal, and reverse
path lookup.

## Run

`prexp-desktop` is **excluded from the Rust workspace** (heavy iced/`rime` deps), so
run these from **this crate's directory** — `rust/crates/prexp-desktop/` — not the
workspace root:

```bash
cargo run                    # 2s refresh (default)
cargo run -- --interval 1    # custom refresh cadence (seconds)
```

`--interval <SECONDS>` sets how often the process table re-snapshots. It's a
float, floored at `0.25` (a full fd-enumerating snapshot can't keep up with
faster polling).

## Features & controls

The title strip holds the controls (a sort dropdown + buttons); the process
table is the main surface.

| Control | Action |
|---|---|
| **sort dropdown** | Order by CPU% / MEM / PMEM / THR / FILES / TOTAL / NAME / PID |
| **▲ / ▼** | Reverse the sort direction |
| **Info** | Open the info panel for the selected process |
| **Signal** | Open the send-signal dialog for the selected process |
| **Find** | Open the reverse path-lookup panel |
| **Stats** | Show/hide the system-stats header |
| **Theme** | Toggle the dark (Dracula) / light (GitHub) palette |
| row click | Select a process (its open resources fill the detail pane) |

- **Process table** — one row per process: PID, NAME, CPU%, RSS, PMEM, THR, and
  the fd breakdown (FILES/SOCKS/PIPES/TOTAL). CPU% is graded by load;
  inaccessible (permission-denied) processes are dimmed. Selection is anchored by
  PID, so the highlight survives re-sorts and refreshes.
- **Detail pane** — the selected process's open resources (FD / KIND / PATH),
  read straight off the loaded snapshot.
- **Stats header** — CPU/MEM/LOAD/BATTERY stat tiles, a per-core usage strip
  grouped into P/E clusters on Apple Silicon (colored by load), and a full-width
  CPU-history line chart.
- **Info panel** (modal) — four tabs over `process_detail`:
  Overview (identity), Resources (memory/CPU/IO/fd stats), Network (connections
  table), Environment (KEY/VALUE table).
- **Send signal** (modal) — a signal picker (SIGTERM/KILL/INT/HUP/QUIT/STOP/CONT,
  default SIGTERM) with a confirm; a success notice or error banner reports the
  outcome.
- **Reverse lookup** (modal) — enter a path, get the processes that have it open
  (`find_by_path`), shown in a results table.

## Architecture

Standard iced application (`iced::application(new, update, view)`), lib/bin split
per the workspace convention — `main.rs` parses the CLI and calls
`prexp_desktop::run`; all domain/UI logic is in the library.

- **Source behind a trait object.** `App` holds
  `Arc<dyn ProcessSource + Send + Sync>` (inversion of control) — swappable for a
  canned test double, and shareable into background tasks. The native backend is
  `prexp_core::backend::NativeSource`.
- **Heavy work runs off the UI thread.** `snapshot_all` walks every process's fds,
  so it — plus the cheap system metrics — is gathered in a background
  `iced::Task` (`sample_task`) and delivered as one `Message::Refreshed(Sample)`.
  `process_detail` (info panel) and `find_by_path` (lookup) run in their own
  tasks. Only `kill_process` runs inline (a fast syscall). A refresh timer
  subscription (`iced::time::every`) drives the cadence.
- **Delta-based metrics.** Per-process CPU% and per-core CPU% are computed from
  `cpu_time_ns` / tick deltas between refreshes (mirroring the TUI), with a
  rolling CPU-history buffer feeding the chart.
- **Theming via rime.** `view()` opens the rime palette once
  (`theme::enter`); every component reads it. Widgets come from `rime` (table,
  stat, line_chart, modal, dialog, select, banner, window_shell, …).

### Module layout

```
src/
  main.rs                 CLI (--interval) → run()
  lib.rs                  run(Arc<dyn ProcessSource + Send + Sync>, Duration)
  app/
    mod.rs                App state, update, subscription, theme, background tasks
    message.rs            Message enum
    sort.rs               SortField + sorting
    cpu.rs                per-process CPU% delta tracker
    system.rs             Sample + SystemStats (per-core delta, CPU history)
    info.rs               InfoState + InfoTab
    signal.rs             Signal enum + SignalPrompt
    lookup.rs             LookupState
  view/
    mod.rs                view() dispatcher: chrome, controls, overlay chaining
    process_table.rs      the process table
    detail.rs             selected-process resource pane
    header.rs             system-stats header (tiles + per-core + chart)
    info_panel.rs         tabbed info modal
    signal_dialog.rs      send-signal modal
    lookup_panel.rs       reverse-lookup modal
    fmt.rs                byte / percent / duration / UTC-timestamp formatters
  snapshot.rs             #[cfg(test)] headless view snapshots (see Testing)
tests/
  app_state_test.rs       behavioral tests through the public App API
  support/fake_source.rs  a Send+Sync ProcessSource double
```

## Testing

Two tiers, both following the workspace conventions:

**Behavioral tests** (`tests/app_state_test.rs`) drive the public `App` message
API against a canned `Send + Sync` `ProcessSource` double and assert on state —
sorting, PID-anchored selection, CPU%/system-stat deltas, and the info / signal /
lookup flows.

```bash
cargo test          # from this crate's directory (excluded from the workspace)
```

**Headless view snapshots** (`src/snapshot.rs`) render the real `view()` to a PNG
via `iced_test::Simulator` and byte-compare against a committed baseline under
`snapshots/`. Adopted from the `tty` project (its ADR 0005). They're `#[ignore]`d
so a plain `cargo test` stays green:

```bash
cargo test -- --ignored snapshot
```

> **Backend: wgpu, not tiny-skia.** The stats header's CPU chart is an iced
> `canvas`, and the headless `tiny-skia` backend does not rasterize canvas
> strokes (the line and axes come out blank). So these snapshots render on
> **wgpu** (the default — this is why there's no `ICED_TEST_BACKEND` override),
> matching `tty`, which baselines its chart views on wgpu only. wgpu baselines
> are machine-local (GPU/driver specific), which is fine here because the tests
> are `#[ignore]`d, not a CI byte-match gate. Baseline filenames carry the
> backend suffix (`…-wgpu.png`); **delete a PNG to re-baseline it.**

Both test doubles override `kill_process` to return `Ok(())` so tests and
snapshots never signal a real process (the trait's default impl calls real
`kill(2)`).

## Dependencies

- `prexp-core` (path) — the domain `ProcessSource` API and models.
- `rime` (path, `../../../../rime/rime`) — the iced component kit. iced is
  re-exported through rime; the direct `iced` dep here pins the same `0.14` line
  and enables the `tokio` feature (the async runtime behind `iced::time::every`
  and off-thread `Task`s).
- `clap` — the `--interval` CLI flag.
