use std::cell::RefCell;

use prexp_core::error::FdtopError;
use prexp_core::models::ProcessSnapshot;
use prexp_core::source::ProcessSource;

/// Test double for ProcessSource.
///
/// Each test creates its own FakeProcessSource with canned data.
/// Uses RefCell for interior mutability through &self trait methods.
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
    fn snapshot_all(&self) -> Result<Vec<ProcessSnapshot>, FdtopError> {
        Ok(self.snapshots.borrow().clone())
    }

    fn snapshot_pid(&self, pid: i32) -> Result<ProcessSnapshot, FdtopError> {
        self.snapshots
            .borrow()
            .iter()
            .find(|s| s.pid == pid)
            .cloned()
            .ok_or(FdtopError::ProcessNotFound { pid })
    }

    fn find_by_path(&self, path: &str) -> Result<Vec<ProcessSnapshot>, FdtopError> {
        Ok(self
            .snapshots
            .borrow()
            .iter()
            .filter(|s| s.resources.iter().any(|r| r.path.as_deref() == Some(path)))
            .cloned()
            .collect())
    }
}
