use std::fs;

use procfs::process::{FDTarget, Process};
use procfs::{Current, CurrentSI};

use crate::error::PrexpError;
use crate::models::{
    DiskIo, NetworkConnection, OpenResource, ProcessActivity, ProcessDetail, ProcessMemory,
    ProcessSnapshot, ProcessState, ResourceKind,
};
use crate::source::ProcessSource;
use crate::system::{CpuTicks, DiskCounters, MemoryInfo, NetworkCounters};
use crate::username::get_username;

pub struct LinuxProcessSource;

impl LinuxProcessSource {
    pub fn new() -> Self {
        Self
    }
}

impl ProcessSource for LinuxProcessSource {
    fn snapshot_all(&self) -> Result<Vec<ProcessSnapshot>, PrexpError> {
        let all = procfs::process::all_processes()
            .map_err(|e| PrexpError::Backend(format!("failed to list processes: {}", e)))?;

        let mut snapshots = Vec::new();
        for proc_result in all {
            let proc = match proc_result {
                Ok(p) => p,
                Err(_) => continue,
            };
            match snapshot_process(&proc) {
                Ok(snap) => snapshots.push(snap),
                Err(PrexpError::ProcessNotFound { .. }) => continue,
                Err(PrexpError::PermissionDenied { .. } | PrexpError::Backend(_)) => {
                    snapshots.push(partial_snapshot(&proc));
                }
                Err(e) => return Err(e),
            }
        }

        Ok(snapshots)
    }

    fn snapshot_pid(&self, pid: i32) -> Result<ProcessSnapshot, PrexpError> {
        let proc = Process::new(pid)
            .map_err(|e| proc_err_to_prexp(e, pid))?;
        snapshot_process(&proc)
    }

    fn find_by_path(&self, path: &str) -> Result<Vec<ProcessSnapshot>, PrexpError> {
        let all = procfs::process::all_processes()
            .map_err(|e| PrexpError::Backend(format!("failed to list processes: {}", e)))?;

        let mut snapshots = Vec::new();
        for proc_result in all {
            let proc = match proc_result {
                Ok(p) => p,
                Err(_) => continue,
            };
            if let Ok(snap) = snapshot_process(&proc) {
                let has_path = snap
                    .resources
                    .iter()
                    .any(|r| r.path.as_deref() == Some(path));
                if has_path {
                    snapshots.push(snap);
                }
            }
        }

        Ok(snapshots)
    }

    fn cpu_ticks(&self) -> Result<Vec<CpuTicks>, PrexpError> {
        let kstat = procfs::KernelStats::current()
            .map_err(|e| PrexpError::Backend(format!("failed to read /proc/stat: {}", e)))?;

        Ok(kstat
            .cpu_time
            .iter()
            .map(|cpu| CpuTicks {
                user: cpu.user as u64,
                system: cpu.system as u64,
                idle: cpu.idle as u64,
                nice: cpu.nice as u64,
            })
            .collect())
    }

    fn memory_info(&self) -> Result<MemoryInfo, PrexpError> {
        let mem = procfs::Meminfo::current()
            .map_err(|e| PrexpError::Backend(format!("failed to read /proc/meminfo: {}", e)))?;

        let total = mem.mem_total;
        let available = mem.mem_available.unwrap_or(mem.mem_free);
        let used = total.saturating_sub(available);

        Ok(MemoryInfo {
            total,
            used,
            free: available,
            wired: 0,
            compressed: 0,
        })
    }

    // Network/disk throughput counters are not yet implemented on Linux. The
    // macOS backend reads them; the Linux path (`/proc/net/dev` +
    // `/proc/diskstats`) is a tracked follow-up (see status-bar-metrics phase 3).
    // Returning an error keeps callers degrading gracefully rather than
    // reporting a fabricated zero rate.
    fn network_counters(&self) -> Result<NetworkCounters, PrexpError> {
        Err(PrexpError::Backend(
            "network counters not yet implemented on Linux".into(),
        ))
    }

    fn disk_counters(&self) -> Result<DiskCounters, PrexpError> {
        Err(PrexpError::Backend(
            "disk counters not yet implemented on Linux".into(),
        ))
    }

    fn system_boot_time_secs(&self) -> Result<u64, PrexpError> {
        procfs::boot_time_secs()
            .map(|s| s as u64)
            .map_err(|e| PrexpError::Backend(format!("boot time read failed: {e}")))
    }

    fn process_detail(&self, pid: i32, parent_name: &str) -> Result<ProcessDetail, PrexpError> {
        let proc = Process::new(pid)
            .map_err(|e| proc_err_to_prexp(e, pid))?;
        let stat = proc.stat()
            .map_err(|e| proc_err_to_prexp(e, pid))?;

        let status = proc.status().ok();

        let uid = status.as_ref()
            .map(|s| s.ruid as u32)
            .unwrap_or(0);

        let name = stat.comm.clone();
        let ppid = stat.ppid;
        let state = linux_state(stat.state);
        let nice = stat.nice as i32;
        let thread_count = stat.num_threads as i32;
        let virtual_size = stat.vsize;

        let ticks_per_sec = procfs::ticks_per_second() as u64;
        let cpu_time_ns = if ticks_per_sec > 0 {
            ((stat.utime + stat.stime) as u64 * 1_000_000_000) / ticks_per_sec
        } else {
            0
        };

        let memory_rss = stat.rss as u64 * page_size();
        let memory_phys = memory_rss;

        // Started time (approximate).
        let boot_time = procfs::boot_time_secs().unwrap_or(0) as u64;
        let started_secs = if ticks_per_sec > 0 {
            boot_time + (stat.starttime / ticks_per_sec)
        } else {
            0
        };

        let path = proc.exe().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        let cwd = proc.cwd().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default();
        let user = get_username(uid);

        // FD counts.
        let (fd_files, fd_sockets, fd_pipes, fd_other, fd_total) = count_fds(&proc);

        // Activity counters.
        let faults = (stat.minflt + stat.majflt) as i32;
        let context_switches = status.as_ref()
            .map(|s| (s.voluntary_ctxt_switches.unwrap_or(0) + s.nonvoluntary_ctxt_switches.unwrap_or(0)) as i32)
            .unwrap_or(0);

        let (disk_bytes_read, disk_bytes_written) = proc.io()
            .map(|io| (io.read_bytes, io.write_bytes))
            .unwrap_or((0, 0));

        // Environment.
        let environment: Vec<(String, String)> = proc.environ().ok()
            .map(|env| {
                env.into_iter()
                    .map(|(k, v)| {
                        (
                            k.to_string_lossy().into_owned(),
                            v.to_string_lossy().into_owned(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Network connections.
        let network = read_network_connections(pid);

        Ok(ProcessDetail {
            pid,
            ppid,
            parent_name: parent_name.to_string(),
            name,
            path,
            cwd,
            user,
            uid,
            state,
            nice,
            started_secs,
            thread_count,
            virtual_size,
            memory_rss,
            memory_phys,
            cpu_time_ns,
            fd_files,
            fd_sockets,
            fd_pipes,
            fd_other,
            fd_total,
            faults,
            context_switches,
            syscalls_mach: 0,
            syscalls_unix: 0,
            disk_bytes_read,
            disk_bytes_written,
            network,
            environment,
        })
    }
}

fn snapshot_process(proc: &Process) -> Result<ProcessSnapshot, PrexpError> {
    let pid = proc.pid();
    let stat = proc.stat()
        .map_err(|e| proc_err_to_prexp(e, pid))?;

    let ticks_per_sec = procfs::ticks_per_second() as u64;
    let cpu_time_ns = if ticks_per_sec > 0 {
        ((stat.utime + stat.stime) as u64 * 1_000_000_000) / ticks_per_sec
    } else {
        0
    };

    let rss_bytes = stat.rss as u64 * page_size();

    let status = proc.status().ok();
    let context_switches = status.as_ref()
        .map(|s| (s.voluntary_ctxt_switches.unwrap_or(0) + s.nonvoluntary_ctxt_switches.unwrap_or(0)) as i32)
        .unwrap_or(0);

    let (disk_read, disk_written) = proc.io()
        .map(|io| (io.read_bytes, io.write_bytes))
        .unwrap_or((0, 0));

    let resources = resolve_fds(proc);

    Ok(ProcessSnapshot {
        pid,
        ppid: stat.ppid,
        name: stat.comm.clone(),
        state: linux_state(stat.state),
        accessible: true,
        memory: ProcessMemory {
            rss: rss_bytes,
            phys: rss_bytes,
        },
        activity: ProcessActivity {
            cpu_time_ns,
            thread_count: stat.num_threads as i32,
            faults: (stat.minflt + stat.majflt) as i32,
            context_switches,
            syscalls_mach: 0,
            syscalls_unix: 0,
        },
        disk_io: DiskIo {
            bytes_read: disk_read,
            bytes_written: disk_written,
        },
        resources,
    })
}

fn partial_snapshot(proc: &Process) -> ProcessSnapshot {
    let pid = proc.pid();
    let (name, ppid, state) = proc.stat()
        .map(|s| (s.comm.clone(), s.ppid, linux_state(s.state)))
        .unwrap_or_else(|_| (format!("pid:{}", pid), 0, ProcessState::Unknown));

    ProcessSnapshot {
        pid,
        ppid,
        name,
        state,
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
    }
}

fn resolve_fds(proc: &Process) -> Vec<OpenResource> {
    let fds = match proc.fd() {
        Ok(fds) => fds,
        Err(_) => return Vec::new(),
    };

    let mut resources = Vec::new();
    for fd_result in fds {
        let fd_info = match fd_result {
            Ok(f) => f,
            Err(_) => continue,
        };

        let descriptor = fd_info.fd as i32;
        let (kind, path) = match fd_info.target {
            FDTarget::Path(ref p) => {
                let path_str = p.to_string_lossy().into_owned();
                let kind = if path_str.starts_with("/dev/") {
                    ResourceKind::Device
                } else {
                    ResourceKind::File
                };
                (kind, Some(path_str))
            }
            FDTarget::Socket(_) => (ResourceKind::Socket, None),
            FDTarget::Pipe(_) => (ResourceKind::Pipe, None),
            FDTarget::AnonInode(ref s) if s.contains("eventpoll") => (ResourceKind::Unknown, None),
            _ => (ResourceKind::Unknown, None),
        };

        resources.push(OpenResource { descriptor, kind, path });
    }
    resources
}

fn count_fds(proc: &Process) -> (usize, usize, usize, usize, usize) {
    let fds = match proc.fd() {
        Ok(fds) => fds,
        Err(_) => return (0, 0, 0, 0, 0),
    };

    let mut files = 0;
    let mut sockets = 0;
    let mut pipes = 0;
    let mut other = 0;
    let mut total = 0;

    for fd_result in fds {
        let fd_info = match fd_result {
            Ok(f) => f,
            Err(_) => continue,
        };
        total += 1;
        match fd_info.target {
            FDTarget::Path(_) => files += 1,
            FDTarget::Socket(_) => sockets += 1,
            FDTarget::Pipe(_) => pipes += 1,
            _ => other += 1,
        }
    }

    (files, sockets, pipes, other, total)
}

fn linux_state(c: char) -> ProcessState {
    match c {
        'R' => ProcessState::Running,
        'S' | 'D' => ProcessState::Sleeping,
        'T' | 't' => ProcessState::Stopped,
        'Z' | 'X' => ProcessState::Zombie,
        'I' => ProcessState::Idle,
        _ => ProcessState::Unknown,
    }
}

fn read_network_connections(pid: i32) -> Vec<NetworkConnection> {
    let mut connections = Vec::new();

    // Read TCP connections from /proc/[pid]/net/tcp and tcp6.
    for (path, proto) in &[
        (format!("/proc/{}/net/tcp", pid), "tcp"),
        (format!("/proc/{}/net/tcp6", pid), "tcp6"),
        (format!("/proc/{}/net/udp", pid), "udp"),
        (format!("/proc/{}/net/udp6", pid), "udp6"),
    ] {
        if let Ok(content) = fs::read_to_string(path) {
            for line in content.lines().skip(1) {
                if let Some(conn) = parse_net_line(line, proto) {
                    connections.push(conn);
                }
            }
        }
    }

    connections
}

fn parse_net_line(line: &str, proto: &str) -> Option<NetworkConnection> {
    let fields: Vec<&str> = line.split_whitespace().collect();
    if fields.len() < 4 {
        return None;
    }

    let local = parse_addr(fields[1])?;
    let remote = parse_addr(fields[2])?;
    let state_hex = u32::from_str_radix(fields[3], 16).ok()?;

    let state = if proto.starts_with("tcp") {
        Some(tcp_state(state_hex).to_string())
    } else {
        None
    };

    let remote_addr = if remote == "0.0.0.0:0" || remote == "[::]:0" {
        None
    } else {
        Some(remote)
    };

    Some(NetworkConnection {
        proto: proto.to_string(),
        local_addr: local,
        remote_addr,
        state,
    })
}

fn parse_addr(hex_addr: &str) -> Option<String> {
    let parts: Vec<&str> = hex_addr.split(':').collect();
    if parts.len() != 2 {
        return None;
    }

    let port = u16::from_str_radix(parts[1], 16).ok()?;
    let ip_hex = parts[0];

    if ip_hex.len() == 8 {
        // IPv4
        let bytes = u32::from_str_radix(ip_hex, 16).ok()?;
        let ip = format!(
            "{}.{}.{}.{}",
            bytes & 0xff,
            (bytes >> 8) & 0xff,
            (bytes >> 16) & 0xff,
            (bytes >> 24) & 0xff
        );
        Some(format!("{}:{}", ip, port))
    } else if ip_hex.len() == 32 {
        // IPv6 (simplified)
        Some(format!("[::ipv6]:{}", port))
    } else {
        None
    }
}

fn tcp_state(state: u32) -> &'static str {
    match state {
        0x01 => "ESTABLISHED",
        0x02 => "SYN_SENT",
        0x03 => "SYN_RECV",
        0x04 => "FIN_WAIT1",
        0x05 => "FIN_WAIT2",
        0x06 => "TIME_WAIT",
        0x07 => "CLOSE",
        0x08 => "CLOSE_WAIT",
        0x09 => "LAST_ACK",
        0x0A => "LISTEN",
        0x0B => "CLOSING",
        _ => "UNKNOWN",
    }
}

fn page_size() -> u64 {
    // Safe: sysconf is always available on Linux.
    let ps = unsafe { libc_sysconf(30) }; // _SC_PAGESIZE = 30 on Linux
    if ps > 0 { ps as u64 } else { 4096 }
}

extern "C" {
    #[link_name = "sysconf"]
    fn libc_sysconf(name: i32) -> i64;
}

fn proc_err_to_prexp(err: procfs::ProcError, pid: i32) -> PrexpError {
    match err {
        procfs::ProcError::NotFound(_) => PrexpError::ProcessNotFound { pid },
        procfs::ProcError::PermissionDenied(_) => PrexpError::PermissionDenied { pid },
        other => PrexpError::Backend(format!("{}", other)),
    }
}
