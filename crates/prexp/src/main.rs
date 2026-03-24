use std::io::{self, Write};

use anyhow::{Context, Result};
use clap::Parser;

use prexp_core::backend::NativeSource;
use prexp_core::output::{self, OutputFormat};
use prexp_core::source::ProcessSource;

use prexp_app::cli::{Cli, CliOutputFormat, InfoTab};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let source = NativeSource::new();

    // --info mode: detailed process info as JSON.
    if let Some(ref info_tab) = cli.info {
        return run_info_mode(&source, &cli, info_tab.as_ref());
    }

    match cli.format {
        Some(ref fmt) => run_output_mode(&source, &cli, fmt),
        None => run_tui_mode(&source, &cli),
    }
}

fn run_info_mode(source: &dyn ProcessSource, cli: &Cli, tab: Option<&InfoTab>) -> Result<()> {
    let pid = cli.pid.context("--info requires --pid")?;

    // Get parent name from snapshot.
    let all = source.snapshot_all().unwrap_or_default();
    let snap = all.iter().find(|s| s.pid == pid);
    let parent_name = snap
        .and_then(|s| all.iter().find(|p| p.pid == s.ppid))
        .map(|p| p.name.as_str())
        .unwrap_or("?");

    let detail = prexp_ffi::get_process_detail(pid, parent_name)
        .context(format!("failed to get info for pid {}", pid))?;

    let mut stdout = io::stdout().lock();

    match tab {
        Some(InfoTab::Overview) => {
            let overview = serde_json::json!({
                "pid": detail.pid,
                "ppid": detail.ppid,
                "parent_name": detail.parent_name,
                "name": detail.name,
                "path": detail.path,
                "cwd": detail.cwd,
                "user": detail.user,
                "uid": detail.uid,
                "state": detail.state,
                "nice": detail.nice,
                "started_secs": detail.started_secs,
            });
            serde_json::to_writer_pretty(&mut stdout, &overview)?;
        }
        Some(InfoTab::Resources) => {
            let resources = serde_json::json!({
                "thread_count": detail.thread_count,
                "virtual_size": detail.virtual_size,
                "memory_rss": detail.memory_rss,
                "memory_phys": detail.memory_phys,
                "cpu_time_ns": detail.cpu_time_ns,
                "fds": {
                    "files": detail.fd_files,
                    "sockets": detail.fd_sockets,
                    "pipes": detail.fd_pipes,
                    "other": detail.fd_other,
                    "total": detail.fd_total,
                }
            });
            serde_json::to_writer_pretty(&mut stdout, &resources)?;
        }
        Some(InfoTab::Network) => {
            serde_json::to_writer_pretty(&mut stdout, &detail.network)?;
        }
        Some(InfoTab::Env) => {
            let env_map: serde_json::Map<String, serde_json::Value> = detail
                .environment
                .into_iter()
                .map(|(k, v)| (k, serde_json::Value::String(v)))
                .collect();
            serde_json::to_writer_pretty(&mut stdout, &env_map)?;
        }
        None => {
            // All tabs.
            serde_json::to_writer_pretty(&mut stdout, &detail)?;
        }
    }

    writeln!(stdout)?;
    Ok(())
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
