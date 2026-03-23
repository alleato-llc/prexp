use std::time::Instant;

use prexp_core::models::ProcessSnapshot;

use super::{App, Column};

impl App {
    pub fn compute_cpu_percentages(&mut self, new_snapshots: &[ProcessSnapshot]) {
        if !self.column_config.is_enabled(Column::Cpu) {
            self.cpu_percentages.clear();
            self.prev_cpu_times.clear();
            self.prev_refresh = None;
            return;
        }

        let now = Instant::now();
        let elapsed_ns = self
            .prev_refresh
            .map(|prev| now.duration_since(prev).as_nanos() as f64)
            .unwrap_or(0.0);

        self.cpu_percentages.clear();

        if elapsed_ns > 0.0 {
            for snap in new_snapshots {
                if let Some(&prev_cpu) = self.prev_cpu_times.get(&snap.pid) {
                    let delta_cpu = snap.cpu_time_ns.saturating_sub(prev_cpu) as f64;
                    let pct = (delta_cpu / elapsed_ns) * 100.0;
                    self.cpu_percentages.insert(snap.pid, pct);
                }
            }
        }

        self.prev_cpu_times.clear();
        for snap in new_snapshots {
            self.prev_cpu_times.insert(snap.pid, snap.cpu_time_ns);
        }
        self.prev_refresh = Some(now);
    }

    pub fn refresh_system_stats(&mut self) {
        // Per-core CPU usage (delta-based).
        if let Ok(new_ticks) = prexp_ffi::get_cpu_ticks() {
            if self.prev_cpu_ticks.len() == new_ticks.len() {
                self.system_stats.cpu_usage = new_ticks
                    .iter()
                    .zip(self.prev_cpu_ticks.iter())
                    .map(|(cur, prev)| {
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
                    })
                    .collect();
            }
            self.prev_cpu_ticks = new_ticks;
        }

        // Memory.
        self.system_stats.memory = prexp_ffi::get_memory_info().ok();

        // Aggregate stats from snapshots.
        self.system_stats.total_processes = self.snapshots.len();
        self.system_stats.total_threads = self
            .snapshots
            .iter()
            .map(|s| s.thread_count as i64)
            .sum();
        self.system_stats.total_fds = self.snapshots.iter().map(|s| s.resources.len()).sum();
    }

    pub fn toggle_summary(&mut self) {
        self.show_summary = !self.show_summary;
        if self.show_summary {
            self.refresh_system_stats();
        }
    }
}

/// Format bytes as human-readable (e.g., "12.3M", "1.2G").
pub fn format_memory(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = 1024.0 * 1024.0;
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;

    let b = bytes as f64;
    if b >= GB {
        format!("{:.1}G", b / GB)
    } else if b >= MB {
        format!("{:.1}M", b / MB)
    } else if b >= KB {
        format!("{:.0}K", b / KB)
    } else {
        format!("{}B", bytes)
    }
}
