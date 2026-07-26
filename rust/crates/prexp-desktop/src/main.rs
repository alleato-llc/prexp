//! `prexp-desktop` entry point: parse the CLI, build the native process source,
//! and hand off to the GUI runtime. Application-level wiring only — anyhow-free
//! since iced owns the return type; all domain work lives in the library crate.

use std::sync::Arc;
use std::time::Duration;

use clap::Parser;
use prexp_core::backend::NativeSource;

/// Process explorer — desktop GUI for inspecting open file descriptors per process.
#[derive(Parser)]
#[command(name = "prexp-desktop", version)]
struct Cli {
    /// Refresh interval in seconds — how often the process table re-snapshots.
    #[arg(long, default_value_t = 2.0)]
    interval: f64,
}

fn main() -> iced::Result {
    let cli = Cli::parse();
    // Clamp to a sane floor: a full fd-enumerating snapshot can't keep up with
    // sub-quarter-second polling, and 0 would busy-loop.
    let interval = Duration::from_secs_f64(cli.interval.max(0.25));
    let source = Arc::new(NativeSource);
    prexp_desktop::run(source, interval)
}
