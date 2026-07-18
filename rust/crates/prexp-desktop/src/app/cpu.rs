use std::collections::HashMap;
use std::time::Instant;

use prexp_core::models::ProcessSnapshot;

/// Per-process CPU% computed by diffing cumulative CPU time between refreshes,
/// mirroring the TUI's `compute_cpu_percentages`: a process's `cpu_time_ns` is
/// monotonic, so `(Δcpu / Δwall) * 100` is its share of one core over the
/// interval. The first refresh has no baseline and reports 0%.
#[derive(Default)]
pub struct CpuTracker {
    /// pid -> cumulative cpu time (ns) at the last refresh.
    prev_times: HashMap<i32, u64>,
    /// Wall-clock instant of the last refresh.
    prev_at: Option<Instant>,
    /// pid -> computed percentage for the current frame.
    pct: HashMap<i32, f64>,
}

impl CpuTracker {
    /// The most recently computed percentage for `pid` (0.0 if unknown).
    pub fn percent(&self, pid: i32) -> f64 {
        self.pct.get(&pid).copied().unwrap_or(0.0)
    }

    /// Recompute every process's percentage from a fresh snapshot set, then roll
    /// the baseline forward for the next call.
    pub fn update(&mut self, snaps: &[ProcessSnapshot]) {
        let now = Instant::now();
        let elapsed_ns = self
            .prev_at
            .map(|prev| now.duration_since(prev).as_nanos() as f64)
            .unwrap_or(0.0);

        self.pct.clear();
        if elapsed_ns > 0.0 {
            for s in snaps {
                if let Some(&prev) = self.prev_times.get(&s.pid) {
                    let delta = s.activity.cpu_time_ns.saturating_sub(prev) as f64;
                    self.pct.insert(s.pid, (delta / elapsed_ns) * 100.0);
                }
            }
        }

        self.prev_times.clear();
        for s in snaps {
            self.prev_times.insert(s.pid, s.activity.cpu_time_ns);
        }
        self.prev_at = Some(now);
    }
}
