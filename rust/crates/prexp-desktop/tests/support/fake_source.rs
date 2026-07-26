//! A `ProcessSource` test double for the GUI. Unlike prexp's TUI double it holds
//! an immutable `Vec` (no `RefCell`), so it is `Send + Sync` — required because
//! `prexp_desktop::App` shares the source into a background `Task` behind an
//! `Arc<dyn ProcessSource + Send + Sync>`.

use prexp_core::error::PrexpError;
use prexp_core::models::{ProcessDetail, ProcessSnapshot};
use prexp_core::source::ProcessSource;
use prexp_core::system::{CpuTicks, DiskCounters, MemoryInfo, NetworkCounters};

pub struct FakeProcessSource {
    snapshots: Vec<ProcessSnapshot>,
}

impl FakeProcessSource {
    pub fn new(snapshots: Vec<ProcessSnapshot>) -> Self {
        Self { snapshots }
    }
}

impl ProcessSource for FakeProcessSource {
    fn snapshot_all(&self) -> Result<Vec<ProcessSnapshot>, PrexpError> {
        Ok(self.snapshots.clone())
    }

    fn snapshot_pid(&self, pid: i32) -> Result<ProcessSnapshot, PrexpError> {
        self.snapshots
            .iter()
            .find(|s| s.pid == pid)
            .cloned()
            .ok_or(PrexpError::ProcessNotFound { pid })
    }

    fn find_by_path(&self, path: &str) -> Result<Vec<ProcessSnapshot>, PrexpError> {
        Ok(self
            .snapshots
            .iter()
            .filter(|s| s.resources.iter().any(|r| r.path.as_deref() == Some(path)))
            .cloned()
            .collect())
    }

    fn cpu_ticks(&self) -> Result<Vec<CpuTicks>, PrexpError> {
        Ok(Vec::new())
    }

    fn memory_info(&self) -> Result<MemoryInfo, PrexpError> {
        Ok(MemoryInfo {
            total: 0,
            used: 0,
            free: 0,
            wired: 0,
            compressed: 0,
            swap_total: 0,
            swap_used: 0,
        })
    }

    fn network_counters(&self) -> Result<NetworkCounters, PrexpError> {
        Ok(NetworkCounters {
            rx_bytes: 0,
            tx_bytes: 0,
        })
    }

    fn disk_counters(&self) -> Result<DiskCounters, PrexpError> {
        Ok(DiskCounters {
            read_bytes: 0,
            write_bytes: 0,
        })
    }

    fn process_detail(&self, pid: i32, _parent: &str) -> Result<ProcessDetail, PrexpError> {
        Err(PrexpError::ProcessNotFound { pid })
    }

    // Override the default (which calls real `kill(2)`) so tests never signal a
    // live process — just record success.
    fn kill_process(&self, _pid: i32, _sig: i32) -> Result<(), PrexpError> {
        Ok(())
    }
}
