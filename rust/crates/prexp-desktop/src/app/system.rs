//! System-wide metrics for the stats header: one raw [`Sample`] gathered off the
//! UI thread each refresh, and a rolling [`SystemStats`] derived from successive
//! samples (per-core CPU% needs two reads to diff, mirroring the TUI's
//! `refresh_system_stats`).

use prexp_core::models::ProcessSnapshot;
use prexp_core::system::{BatteryInfo, CpuKind, CpuTicks, MemoryInfo};

/// One refresh's worth of raw data, gathered together in the background task so
/// the whole header updates from a single message. `cpu_ticks` are cumulative
/// (diffed here); the rest are point-in-time.
#[derive(Debug, Clone)]
pub struct Sample {
    pub processes: Vec<ProcessSnapshot>,
    pub cpu_ticks: Vec<CpuTicks>,
    pub memory: Option<MemoryInfo>,
    pub load: Option<[f64; 3]>,
    pub battery: Option<BatteryInfo>,
}

impl Sample {
    /// A process-only sample (no system metrics) — for tests that exercise the
    /// table/selection paths without seeding the header.
    pub fn of(processes: Vec<ProcessSnapshot>) -> Self {
        Self {
            processes,
            cpu_ticks: Vec::new(),
            memory: None,
            load: None,
            battery: None,
        }
    }
}

/// How many aggregate-CPU% points the history keeps for the line chart.
const HISTORY_CAP: usize = 120;

/// Rolling system stats for the header. `per_core` is the instantaneous usage of
/// each logical CPU; `cpu_history` is the aggregate over time for the chart.
#[derive(Default)]
pub struct SystemStats {
    /// Previous cumulative ticks, kept to diff against the next sample.
    prev_ticks: Vec<CpuTicks>,
    /// Current per-core usage percentage, index-aligned with [`Self::perf_levels`].
    per_core: Vec<f64>,
    /// Aggregate CPU% over recent samples (oldest first), capped at [`HISTORY_CAP`].
    cpu_history: Vec<f32>,
    memory: Option<MemoryInfo>,
    load: Option<[f64; 3]>,
    battery: Option<BatteryInfo>,
    /// Each core's P/E cluster (static; set once at boot).
    perf_levels: Vec<CpuKind>,
}

impl SystemStats {
    /// Record the static per-core cluster topology (read once — it never changes).
    pub fn set_perf_levels(&mut self, levels: Vec<CpuKind>) {
        self.perf_levels = levels;
    }

    /// Fold a new sample in: diff the CPU ticks into per-core percentages, push an
    /// aggregate history point, and store the point-in-time metrics.
    pub fn update(&mut self, sample: &Sample) {
        let ticks = &sample.cpu_ticks;
        if !ticks.is_empty() && self.prev_ticks.len() == ticks.len() {
            self.per_core = ticks
                .iter()
                .zip(self.prev_ticks.iter())
                .map(|(cur, prev)| core_usage(cur, prev))
                .collect();

            // One aggregate point per sample once we have real per-core numbers.
            let agg = self.aggregate_cpu() as f32;
            self.cpu_history.push(agg);
            if self.cpu_history.len() > HISTORY_CAP {
                let excess = self.cpu_history.len() - HISTORY_CAP;
                self.cpu_history.drain(0..excess);
            }
        }
        if !ticks.is_empty() {
            self.prev_ticks = ticks.clone();
        }

        self.memory = sample.memory.clone();
        self.load = sample.load;
        self.battery = sample.battery;
    }

    /// Mean per-core usage — the headline CPU number.
    pub fn aggregate_cpu(&self) -> f64 {
        if self.per_core.is_empty() {
            0.0
        } else {
            self.per_core.iter().sum::<f64>() / self.per_core.len() as f64
        }
    }

    pub fn per_core(&self) -> &[f64] {
        &self.per_core
    }
    pub fn perf_levels(&self) -> &[CpuKind] {
        &self.perf_levels
    }
    pub fn cpu_history(&self) -> &[f32] {
        &self.cpu_history
    }
    pub fn memory(&self) -> Option<&MemoryInfo> {
        self.memory.as_ref()
    }
    pub fn load(&self) -> Option<[f64; 3]> {
        self.load
    }
    pub fn battery(&self) -> Option<BatteryInfo> {
        self.battery
    }
}

/// A single core's usage percentage from its cumulative-tick delta:
/// `(user+system+nice) / (user+system+idle+nice)`.
fn core_usage(cur: &CpuTicks, prev: &CpuTicks) -> f64 {
    let user = cur.user.wrapping_sub(prev.user) as f64;
    let system = cur.system.wrapping_sub(prev.system) as f64;
    let idle = cur.idle.wrapping_sub(prev.idle) as f64;
    let nice = cur.nice.wrapping_sub(prev.nice) as f64;
    let total = user + system + idle + nice;
    if total > 0.0 {
        ((user + system + nice) / total) * 100.0
    } else {
        0.0
    }
}
