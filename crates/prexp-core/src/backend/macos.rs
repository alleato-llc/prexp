use prexp_ffi::{FdDetail, FdInfo, FfiError};

use crate::error::PrexpError;
use crate::models::{DiskIo, NetworkConnection, OpenResource, ProcessActivity, ProcessDetail, ProcessMemory, ProcessSnapshot, ProcessState, ResourceKind};
use crate::source::ProcessSource;
use crate::system::{CpuKind, CpuTicks, DiskCounters, MemoryInfo, NetworkCounters};

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
            state: convert_state(info.state),
            accessible: true,
            memory: ProcessMemory {
                rss: info.memory_rss,
                phys: info.memory_phys,
            },
            activity: ProcessActivity {
                cpu_time_ns: info.cpu_time_ns,
                thread_count: info.thread_count,
                faults: info.faults,
                context_switches: info.context_switches,
                syscalls_mach: info.syscalls_mach,
                syscalls_unix: info.syscalls_unix,
            },
            disk_io: DiskIo {
                bytes_read: info.disk_bytes_read,
                bytes_written: info.disk_bytes_written,
            },
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

    fn cpu_ticks(&self) -> Result<Vec<CpuTicks>, PrexpError> {
        let ffi_ticks = prexp_ffi::get_cpu_ticks().map_err(ffi_to_prexp)?;
        Ok(ffi_ticks.into_iter().map(|t| CpuTicks {
            user: t.user as u64,
            system: t.system as u64,
            idle: t.idle as u64,
            nice: t.nice as u64,
        }).collect())
    }

    fn memory_info(&self) -> Result<MemoryInfo, PrexpError> {
        let m = prexp_ffi::get_memory_info().map_err(ffi_to_prexp)?;
        Ok(MemoryInfo {
            total: m.total,
            used: m.used,
            free: m.free,
            wired: m.wired,
            compressed: m.compressed,
        })
    }

    fn network_counters(&self) -> Result<NetworkCounters, PrexpError> {
        let n = prexp_ffi::get_network_counters().map_err(ffi_to_prexp)?;
        Ok(NetworkCounters {
            rx_bytes: n.rx_bytes,
            tx_bytes: n.tx_bytes,
        })
    }

    fn disk_counters(&self) -> Result<DiskCounters, PrexpError> {
        let d = prexp_ffi::get_disk_counters().map_err(ffi_to_prexp)?;
        Ok(DiskCounters {
            read_bytes: d.read_bytes,
            write_bytes: d.write_bytes,
        })
    }

    fn system_boot_time_secs(&self) -> Result<u64, PrexpError> {
        prexp_ffi::get_boot_time_secs().map_err(ffi_to_prexp)
    }

    fn system_load_average(&self) -> Result<[f64; 3], PrexpError> {
        prexp_ffi::get_load_average().map_err(ffi_to_prexp)
    }

    fn cpu_perf_levels(&self) -> Result<Vec<CpuKind>, PrexpError> {
        let levels = prexp_ffi::get_cpu_perf_levels().map_err(ffi_to_prexp)?;
        Ok(levels
            .into_iter()
            .map(|l| match l {
                prexp_ffi::CoreType::Performance => CpuKind::Performance,
                prexp_ffi::CoreType::Efficiency => CpuKind::Efficiency,
                prexp_ffi::CoreType::Unknown => CpuKind::Unknown,
            })
            .collect())
    }

    fn process_detail(&self, pid: i32, parent_name: &str) -> Result<ProcessDetail, PrexpError> {
        let d = prexp_ffi::get_process_detail(pid, parent_name).map_err(ffi_to_prexp)?;
        Ok(ProcessDetail {
            pid: d.pid,
            ppid: d.ppid,
            parent_name: d.parent_name,
            name: d.name,
            path: d.path,
            cwd: d.cwd,
            user: d.user,
            uid: d.uid,
            state: convert_state(d.state),
            nice: d.nice,
            started_secs: d.started_secs,
            thread_count: d.thread_count,
            virtual_size: d.virtual_size,
            memory_rss: d.memory_rss,
            memory_phys: d.memory_phys,
            cpu_time_ns: d.cpu_time_ns,
            fd_files: d.fd_files,
            fd_sockets: d.fd_sockets,
            fd_pipes: d.fd_pipes,
            fd_other: d.fd_other,
            fd_total: d.fd_total,
            faults: d.faults,
            context_switches: d.context_switches,
            syscalls_mach: d.syscalls_mach,
            syscalls_unix: d.syscalls_unix,
            disk_bytes_read: d.disk_bytes_read,
            disk_bytes_written: d.disk_bytes_written,
            network: d.network.into_iter().map(|n| NetworkConnection {
                proto: n.proto,
                local_addr: n.local_addr,
                remote_addr: n.remote_addr,
                state: n.state,
            }).collect(),
            environment: d.environment,
        })
    }
}

/// Create a partial snapshot for a process we couldn't fully inspect.
/// Tries to get at least the name via proc_name.
fn partial_snapshot(pid: i32) -> ProcessSnapshot {
    let name = prexp_ffi::get_process_name(pid)
        .unwrap_or_else(|_| format!("pid:{}", pid));

    // Try to get ppid from get_process_info — it may succeed even when list_fds fails.
    let info = prexp_ffi::get_process_info(pid);
    match info {
        Ok(i) => ProcessSnapshot {
            pid,
            ppid: i.ppid,
            name: i.name,
            state: convert_state(i.state),
            accessible: false,
            memory: ProcessMemory {
                rss: i.memory_rss,
                phys: i.memory_phys,
            },
            activity: ProcessActivity {
                cpu_time_ns: i.cpu_time_ns,
                thread_count: i.thread_count,
                faults: i.faults,
                context_switches: i.context_switches,
                syscalls_mach: i.syscalls_mach,
                syscalls_unix: i.syscalls_unix,
            },
            disk_io: DiskIo {
                bytes_read: i.disk_bytes_read,
                bytes_written: i.disk_bytes_written,
            },
            resources: Vec::new(),
        },
        Err(_) => ProcessSnapshot {
            pid,
            ppid: 0,
            name,
            state: ProcessState::Unknown,
            accessible: false,
            memory: ProcessMemory { rss: 0, phys: 0 },
            activity: ProcessActivity {
                cpu_time_ns: 0,
                thread_count: 0,
                faults: 0,
                context_switches: 0,
                syscalls_mach: 0,
                syscalls_unix: 0,
            },
            disk_io: DiskIo { bytes_read: 0, bytes_written: 0 },
            resources: Vec::new(),
        },
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

/// Convert FFI process state to domain process state.
fn convert_state(s: prexp_ffi::ProcessState) -> ProcessState {
    match s {
        prexp_ffi::ProcessState::Running => ProcessState::Running,
        prexp_ffi::ProcessState::Sleeping => ProcessState::Sleeping,
        prexp_ffi::ProcessState::Stopped => ProcessState::Stopped,
        prexp_ffi::ProcessState::Zombie => ProcessState::Zombie,
        prexp_ffi::ProcessState::Idle => ProcessState::Idle,
        prexp_ffi::ProcessState::Unknown => ProcessState::Unknown,
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
