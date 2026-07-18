//! prexp-desktop — a GUI over prexp-core, built on `iced` via the `rime`
//! component kit.
//!
//! The library exposes one entry point, [`run`], taking a boxed
//! [`ProcessSource`](prexp_core::source::ProcessSource) so the GUI is decoupled
//! from the platform backend (and testable against a canned-data double, per the
//! project's inversion-of-control convention). The heavy `snapshot_all` FFI runs
//! off the UI thread via an iced `Task`; a timer subscription drives periodic
//! refreshes.

mod app;
mod view;

#[cfg(test)]
mod snapshot;

use std::sync::Arc;
use std::time::Duration;

use prexp_core::source::ProcessSource;

pub use app::{App, InfoState, InfoTab, LookupState, Message, Sample, Signal, SignalPrompt, SortField};

/// Launch the GUI against `source`, re-snapshotting every `interval`.
///
/// `source` is shared into the background refresh task, so it must be `Send +
/// Sync`; the native backend is a zero-sized unit struct and satisfies both.
pub fn run(
    source: Arc<dyn ProcessSource + Send + Sync>,
    interval: Duration,
) -> iced::Result {
    iced::application(
        move || App::boot(source.clone(), interval),
        App::update,
        App::view,
    )
    .title("prexp-desktop")
    .theme(App::theme)
    .subscription(App::subscription)
    .font(rime::icons::FONT_BYTES)
    .window_size(iced::Size::new(1040.0, 720.0))
    .run()
}
