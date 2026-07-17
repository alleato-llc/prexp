/// Per-CPU tick counts (user, system, idle, nice).
#[derive(Debug, Clone)]
pub struct CpuTicks {
    pub user: u64,
    pub system: u64,
    pub idle: u64,
    pub nice: u64,
}

/// System memory information (bytes). `swap_*` are the swap file's totals
/// (`0` when there is no swap).
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub wired: u64,
    pub compressed: u64,
    pub swap_total: u64,
    pub swap_used: u64,
}

/// Cumulative system-wide network byte counters (monotonic; diff two reads for
/// a rate). Summed across all non-loopback interfaces.
#[derive(Debug, Clone, Copy)]
pub struct NetworkCounters {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

/// Cumulative system-wide disk byte counters (monotonic; diff two reads for a
/// rate). Summed across all block-storage drivers.
#[derive(Debug, Clone, Copy)]
pub struct DiskCounters {
    pub read_bytes: u64,
    pub write_bytes: u64,
}

/// A logical CPU's cluster type, for grouping per-core stats. Apple Silicon
/// splits cores into Performance and Efficiency clusters; uniform hardware
/// (Intel Macs) reports `Unknown`. Static for the machine's lifetime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuKind {
    Performance,
    Efficiency,
    Unknown,
}

/// The primary battery's state. Absent (an `Err` from `system_battery`) on a
/// machine with no battery.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BatteryInfo {
    /// Charge as a 0..=100 percentage.
    pub percent: f64,
    /// Whether the battery is currently charging.
    pub charging: bool,
    /// Estimated minutes to empty while discharging, or `-1` when charging /
    /// still calculating.
    pub time_to_empty_min: i32,
    /// Estimated minutes to a full charge while charging, or `-1` otherwise.
    pub time_to_full_min: i32,
}
