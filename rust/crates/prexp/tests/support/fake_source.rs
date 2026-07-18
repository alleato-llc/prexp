use std::cell::RefCell;

use prexp_core::error::PrexpError;
use prexp_core::models::{ProcessDetail, ProcessSnapshot};
use prexp_core::source::ProcessSource;
use prexp_core::system::{CpuTicks, DiskCounters, MemoryInfo, NetworkCounters};

pub struct FakeProcessSource {
    snapshots: RefCell<Vec<ProcessSnapshot>>,
}

impl FakeProcessSource {
    pub fn new(snapshots: Vec<ProcessSnapshot>) -> Self {
        Self {
            snapshots: RefCell::new(snapshots),
        }
    }
}

impl ProcessSource for FakeProcessSource {
    fn snapshot_all(&self) -> Result<Vec<ProcessSnapshot>, PrexpError> {
        Ok(self.snapshots.borrow().clone())
    }

    fn snapshot_pid(&self, pid: i32) -> Result<ProcessSnapshot, PrexpError> {
        self.snapshots
            .borrow()
            .iter()
            .find(|s| s.pid == pid)
            .cloned()
            .ok_or(PrexpError::ProcessNotFound { pid })
    }

    fn find_by_path(&self, path: &str) -> Result<Vec<ProcessSnapshot>, PrexpError> {
        Ok(self
            .snapshots
            .borrow()
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

    fn process_detail(&self, _pid: i32, _parent_name: &str) -> Result<ProcessDetail, PrexpError> {
        Err(PrexpError::Backend("not implemented in fake source".into()))
    }
}
