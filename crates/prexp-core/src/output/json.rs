use std::io::Write;

use crate::error::FdtopError;
use crate::models::ProcessSnapshot;

/// Write process snapshots as grouped JSON (array of ProcessSnapshot).
pub fn format(snapshots: &[ProcessSnapshot], writer: &mut dyn Write) -> Result<(), FdtopError> {
    serde_json::to_writer_pretty(writer, snapshots)?;
    Ok(())
}
