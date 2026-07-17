/// Per-CPU tick counts (user, system, idle, nice).
#[derive(Debug, Clone)]
pub struct CpuTicks {
    pub user: u64,
    pub system: u64,
    pub idle: u64,
    pub nice: u64,
}

/// System memory information.
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub wired: u64,
    pub compressed: u64,
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
