use std::io::Write;

use crate::error::PrexpError;
use crate::models::ProcessSnapshot;

/// Write process snapshots as grouped JSON (array of ProcessSnapshot).
pub fn format(snapshots: &[ProcessSnapshot], writer: &mut dyn Write) -> Result<(), PrexpError> {
    serde_json::to_writer_pretty(writer, snapshots)?;
    Ok(())
}
