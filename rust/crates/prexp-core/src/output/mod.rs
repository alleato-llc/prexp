pub mod json;
pub mod tsv;

use std::io::Write;

use crate::error::PrexpError;
use crate::models::ProcessSnapshot;

/// Supported output formats for non-TUI mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Tsv,
}

/// Format process snapshots and write to the given writer.
pub fn format_snapshots(
    snapshots: &[ProcessSnapshot],
    format: OutputFormat,
    writer: &mut dyn Write,
) -> Result<(), PrexpError> {
    match format {
        OutputFormat::Json => json::format(snapshots, writer),
        OutputFormat::Tsv => tsv::format(snapshots, writer),
    }
}
