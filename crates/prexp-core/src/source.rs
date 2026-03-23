use crate::error::FdtopError;
use crate::models::ProcessSnapshot;

/// Platform-agnostic trait for querying process file descriptors.
///
/// Implementations exist for macOS (via libproc FFI) and Linux (via procfs).
/// Test doubles implement this trait with canned data.
pub trait ProcessSource {
    /// Snapshot all visible processes and their open file descriptors.
    fn snapshot_all(&self) -> Result<Vec<ProcessSnapshot>, FdtopError>;

    /// Snapshot a single process by PID.
    fn snapshot_pid(&self, pid: i32) -> Result<ProcessSnapshot, FdtopError>;

    /// Reverse lookup: find all processes that have the given path open.
    fn find_by_path(&self, path: &str) -> Result<Vec<ProcessSnapshot>, FdtopError>;
}
