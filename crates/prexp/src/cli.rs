use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(name = "prexp", about = "Process explorer — inspect open file descriptors per process")]
pub struct Cli {
    /// Output format. Defaults to TUI when omitted.
    #[arg(short = 'o', long = "output")]
    pub format: Option<CliOutputFormat>,

    /// Filter to a specific process by PID.
    #[arg(short, long)]
    pub pid: Option<i32>,

    /// Reverse lookup: find processes with this file path open.
    #[arg(short = 'P', long = "path")]
    pub file_path: Option<String>,

    /// Show detailed process info (requires --pid). Optional tab: overview, resources, network, env.
    #[arg(long)]
    pub info: Option<Option<InfoTab>>,

    /// Refresh interval in seconds (TUI mode only).
    #[arg(short, long, default_value = "2")]
    pub interval: u64,
}

#[derive(Clone, ValueEnum)]
pub enum InfoTab {
    Overview,
    Resources,
    Network,
    Env,
}

#[derive(Clone, ValueEnum)]
pub enum CliOutputFormat {
    Json,
    Tsv,
}
