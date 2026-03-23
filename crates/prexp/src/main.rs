use std::io::{self, Write};

use anyhow::{Context, Result};
use clap::Parser;

use prexp_core::backend::NativeSource;
use prexp_core::output::{self, OutputFormat};
use prexp_core::source::ProcessSource;

use prexp_app::cli::{Cli, CliOutputFormat};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let source = NativeSource::new();

    match cli.format {
        Some(ref fmt) => run_output_mode(&source, &cli, fmt),
        None => run_tui_mode(&source, &cli),
    }
}

fn run_output_mode(source: &dyn ProcessSource, cli: &Cli, fmt: &CliOutputFormat) -> Result<()> {
    let format = match fmt {
        CliOutputFormat::Json => OutputFormat::Json,
        CliOutputFormat::Tsv => OutputFormat::Tsv,
    };

    let snapshots = if let Some(path) = &cli.file_path {
        source
            .find_by_path(path)
            .context("failed to perform reverse lookup")?
    } else if let Some(pid) = cli.pid {
        let snap = source
            .snapshot_pid(pid)
            .context(format!("failed to snapshot pid {}", pid))?;
        vec![snap]
    } else {
        source
            .snapshot_all()
            .context("failed to snapshot all processes")?
    };

    let mut stdout = io::stdout().lock();
    output::format_snapshots(&snapshots, format, &mut stdout)
        .context("failed to write output")?;
    writeln!(stdout)?;

    Ok(())
}

fn run_tui_mode(source: &dyn ProcessSource, cli: &Cli) -> Result<()> {
    let interval = std::time::Duration::from_secs(cli.interval);
    prexp_app::tui::run_tui(source, interval).context("TUI error")
}
