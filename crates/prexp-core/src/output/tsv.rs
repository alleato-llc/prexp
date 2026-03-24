use std::io::Write;

use crate::error::PrexpError;
use crate::models::ProcessSnapshot;

/// Write process snapshots as TSV (tab-separated values).
pub fn format(snapshots: &[ProcessSnapshot], writer: &mut dyn Write) -> Result<(), PrexpError> {
    writeln!(writer, "PID\tPROCESS\tDESCRIPTOR\tKIND\tPATH")?;

    for proc in snapshots {
        for res in &proc.resources {
            let kind = format!("{:?}", res.kind).to_lowercase();
            let path = res.path.as_deref().unwrap_or("-");
            writeln!(
                writer,
                "{}\t{}\t{}\t{}\t{}",
                proc.pid, proc.name, res.descriptor, kind, path
            )?;
        }
    }

    Ok(())
}
