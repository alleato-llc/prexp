use prexp_ffi::{FdDetail, FdInfo, FfiError};

use crate::error::PrexpError;
use crate::models::{OpenResource, ProcessSnapshot, ResourceKind};
use crate::source::ProcessSource;

pub struct MacosProcessSource;

impl MacosProcessSource {
    pub fn new() -> Self {
        Self
    }
}

impl ProcessSource for MacosProcessSource {
    fn snapshot_all(&self) -> Result<Vec<ProcessSnapshot>, PrexpError> {
        let pids = prexp_ffi::list_all_pids().map_err(ffi_to_prexp)?;

        let mut snapshots = Vec::with_capacity(pids.len());
        for pid in pids {
            match self.snapshot_pid(pid) {
                Ok(snap) => snapshots.push(snap),
                // Process exited between list and query — skip entirely.
                Err(PrexpError::ProcessNotFound { .. }) => continue,
                // Permission denied or other soft failure — include partial snapshot.
                Err(PrexpError::PermissionDenied { .. } | PrexpError::Backend(_)) => {
                    snapshots.push(partial_snapshot(pid));
                }
                Err(e) => return Err(e),
            }
        }

        Ok(snapshots)
    }

    fn snapshot_pid(&self, pid: i32) -> Result<ProcessSnapshot, PrexpError> {
        // Get process metadata (ppid, name, thread count).
        let info = prexp_ffi::get_process_info(pid).map_err(ffi_to_prexp)?;

        let fds = prexp_ffi::list_fds(pid).map_err(ffi_to_prexp)?;
        let resources = resolve_all_fds(pid, &fds);

        Ok(ProcessSnapshot {
            pid,
            ppid: info.ppid,
            name: info.name,
            thread_count: info.thread_count,
            memory_rss: info.memory_rss,
            memory_phys: info.memory_phys,
            cpu_time_ns: info.cpu_time_ns,
            accessible: true,
            resources,
        })
    }

    fn find_by_path(&self, path: &str) -> Result<Vec<ProcessSnapshot>, PrexpError> {
        let pids = prexp_ffi::list_pids_by_path(path).map_err(ffi_to_prexp)?;

        let mut snapshots = Vec::new();
        for pid in pids {
            match self.snapshot_pid(pid) {
                Ok(snap) => {
                    // Only include if the process actually has this path open.
                    let has_path = snap
                        .resources
                        .iter()
                        .any(|r| r.path.as_deref() == Some(path));
                    if has_path {
                        snapshots.push(snap);
                    }
                }
                Err(_) => continue,
            }
        }

        Ok(snapshots)
    }
}

/// Create a partial snapshot for a process we couldn't fully inspect.
/// Tries to get at least the name via proc_name.
fn partial_snapshot(pid: i32) -> ProcessSnapshot {
    let name = prexp_ffi::get_process_name(pid)
        .unwrap_or_else(|_| format!("pid:{}", pid));

    // Try to get ppid from get_process_info — it may succeed even when list_fds fails.
    let (ppid, thread_count, memory_rss, memory_phys, cpu_time_ns, better_name) =
        prexp_ffi::get_process_info(pid)
            .map(|info| (info.ppid, info.thread_count, info.memory_rss, info.memory_phys, info.cpu_time_ns, info.name))
            .unwrap_or((0, 0, 0, 0, 0, name));

    ProcessSnapshot {
        pid,
        ppid,
        name: better_name,
        thread_count,
        memory_rss,
        memory_phys,
        cpu_time_ns,
        accessible: false,
        resources: Vec::new(),
    }
}

/// Resolve all FDs for a process, skipping individual failures.
fn resolve_all_fds(pid: i32, fds: &[FdInfo]) -> Vec<OpenResource> {
    let mut resources = Vec::with_capacity(fds.len());
    for fd_info in fds {
        match prexp_ffi::resolve_fd(pid, fd_info.fd, fd_info.fdtype) {
            Ok(detail) => {
                let (kind, path) = classify_fd_detail(&detail);
                resources.push(OpenResource {
                    descriptor: fd_info.fd,
                    kind,
                    path,
                });
            }
            Err(_) => {
                // FD may have closed between list and resolve — skip silently.
                resources.push(OpenResource {
                    descriptor: fd_info.fd,
                    kind: classify_fdtype(fd_info.fdtype),
                    path: None,
                });
            }
        }
    }
    resources
}

/// Map resolved FdDetail to (ResourceKind, Option<path>).
fn classify_fd_detail(detail: &FdDetail) -> (ResourceKind, Option<String>) {
    match detail {
        FdDetail::Vnode { path } => {
            let kind = if path.starts_with("/dev/") {
                ResourceKind::Device
            } else {
                ResourceKind::File
            };
            let p = if path.is_empty() {
                None
            } else {
                Some(path.clone())
            };
            (kind, p)
        }
        FdDetail::Socket { .. } => (ResourceKind::Socket, None),
        FdDetail::Pipe => (ResourceKind::Pipe, None),
        FdDetail::Kqueue => (ResourceKind::Kqueue, None),
        FdDetail::Pshm | FdDetail::Psem => (ResourceKind::Unknown, None),
        FdDetail::Unknown(_) => (ResourceKind::Unknown, None),
    }
}

/// Fallback classification from fdtype when resolve_fd fails.
fn classify_fdtype(fdtype: u32) -> ResourceKind {
    match fdtype {
        prexp_ffi::raw::PROX_FDTYPE_VNODE => ResourceKind::File,
        prexp_ffi::raw::PROX_FDTYPE_SOCKET => ResourceKind::Socket,
        prexp_ffi::raw::PROX_FDTYPE_PIPE => ResourceKind::Pipe,
        prexp_ffi::raw::PROX_FDTYPE_KQUEUE => ResourceKind::Kqueue,
        _ => ResourceKind::Unknown,
    }
}

/// Convert FFI errors to domain errors.
fn ffi_to_prexp(err: FfiError) -> PrexpError {
    match err {
        FfiError::ProcessGone(pid) => PrexpError::ProcessNotFound { pid },
        FfiError::PermissionDenied(pid) => PrexpError::PermissionDenied { pid },
        FfiError::SystemError { reason, .. } => PrexpError::Backend(reason),
    }
}
