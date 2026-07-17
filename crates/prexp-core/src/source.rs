use crate::error::PrexpError;
use crate::models::{ProcessDetail, ProcessSnapshot};
use crate::system::{CpuTicks, DiskCounters, MemoryInfo, NetworkCounters};

/// Platform-agnostic trait for querying processes and system metrics.
///
/// Implementations exist for macOS (via libproc FFI) and Linux (via procfs).
/// Test doubles implement this trait with canned data.
pub trait ProcessSource {
    /// Snapshot all visible processes and their open file descriptors.
    fn snapshot_all(&self) -> Result<Vec<ProcessSnapshot>, PrexpError>;

    /// Snapshot a single process by PID.
    fn snapshot_pid(&self, pid: i32) -> Result<ProcessSnapshot, PrexpError>;

    /// Reverse lookup: find all processes that have the given path open.
    fn find_by_path(&self, path: &str) -> Result<Vec<ProcessSnapshot>, PrexpError>;

    /// Get per-CPU tick counts for all cores.
    fn cpu_ticks(&self) -> Result<Vec<CpuTicks>, PrexpError>;

    /// Get system memory information.
    fn memory_info(&self) -> Result<MemoryInfo, PrexpError>;

    /// Get cumulative system-wide network byte counters (rx/tx), summed across
    /// non-loopback interfaces. Diff two reads to get a rate.
    fn network_counters(&self) -> Result<NetworkCounters, PrexpError>;

    /// Get cumulative system-wide disk byte counters (read/written). Diff two
    /// reads to get a rate.
    fn disk_counters(&self) -> Result<DiskCounters, PrexpError>;

    /// Get detailed process information for the info panel.
    fn process_detail(&self, pid: i32, parent_name: &str) -> Result<ProcessDetail, PrexpError>;
}
