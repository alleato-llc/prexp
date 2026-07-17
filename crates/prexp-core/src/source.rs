use crate::error::PrexpError;
use crate::models::{ProcessDetail, ProcessSnapshot};
use crate::system::{CpuKind, CpuTicks, DiskCounters, MemoryInfo, NetworkCounters};

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

    /// Each logical CPU's cluster type (P/E), indexed to align with
    /// [`Self::cpu_ticks`]. The topology is static, so callers read this once
    /// and cache it. The default is unsupported; only platforms with
    /// heterogeneous cores (macOS) override it.
    fn cpu_perf_levels(&self) -> Result<Vec<CpuKind>, PrexpError> {
        Err(PrexpError::Backend(
            "cpu perf levels not available on this platform".into(),
        ))
    }

    /// The Unix-epoch second at which the system booted. Subtract from the
    /// current time to get the system uptime; the value only changes across
    /// reboots, so a caller can read it once and cache it. The default is
    /// unsupported; each platform backend overrides it.
    fn system_boot_time_secs(&self) -> Result<u64, PrexpError> {
        Err(PrexpError::Backend(
            "system boot time not available on this platform".into(),
        ))
    }

    /// The system load average — the 1, 5, and 15-minute run-queue averages. The
    /// default is unsupported; each platform backend overrides it.
    fn system_load_average(&self) -> Result<[f64; 3], PrexpError> {
        Err(PrexpError::Backend(
            "load average not available on this platform".into(),
        ))
    }

    /// The primary battery's state, or an error when the machine has no battery.
    /// The default is unsupported; each platform backend overrides it.
    fn system_battery(&self) -> Result<crate::system::BatteryInfo, PrexpError> {
        Err(PrexpError::Backend(
            "battery not available on this platform".into(),
        ))
    }

    /// Get detailed process information for the info panel.
    fn process_detail(&self, pid: i32, parent_name: &str) -> Result<ProcessDetail, PrexpError>;
}
