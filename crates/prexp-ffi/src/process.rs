//! Safe wrappers for process-level libproc APIs.
//!
//! Provides: list_all_pids, get_process_name, get_process_info, list_fds,
//! resolve_fd, list_pids_by_path.

use std::ffi::CString;
use std::mem;
use std::os::raw::{c_int, c_void};

use crate::error::{check_errno, extract_c_string, mach_ticks_to_ns, FfiError};
use crate::raw;

/// Information about a single open file descriptor.
#[derive(Debug, Clone)]
pub struct FdInfo {
    pub fd: i32,
    pub fdtype: u32,
}

/// Resolved detail for a file descriptor.
#[derive(Debug, Clone)]
pub enum FdDetail {
    Vnode { path: String },
    Socket { family: i32, socket_type: i32, protocol: i32 },
    Pipe,
    Kqueue,
    Pshm,
    Psem,
    Unknown(u32),
}

/// Process metadata: PPID, thread count, CPU time, memory, and full name.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub ppid: i32,
    pub name: String,
    pub thread_count: i32,
    pub memory_rss: u64,
    pub memory_phys: u64,
    pub cpu_time_ns: u64,
    pub state: ProcessState,
    pub start_time: u64,
    pub faults: i32,
    pub context_switches: i32,
    pub syscalls_mach: i32,
    pub syscalls_unix: i32,
    pub disk_bytes_read: u64,
    pub disk_bytes_written: u64,
}

/// Process state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProcessState {
    Running,
    Sleeping,
    Stopped,
    Zombie,
    Idle,
    Unknown,
}

impl ProcessState {
    pub fn from_bsd_status(status: u32) -> Self {
        match status {
            raw::SRUN => ProcessState::Running,
            raw::SSLEEP => ProcessState::Sleeping,
            raw::SSTOP => ProcessState::Stopped,
            raw::SZOMB => ProcessState::Zombie,
            raw::SIDL => ProcessState::Idle,
            _ => ProcessState::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            ProcessState::Running => "RUN",
            ProcessState::Sleeping => "SLP",
            ProcessState::Stopped => "STP",
            ProcessState::Zombie => "ZMB",
            ProcessState::Idle => "IDL",
            ProcessState::Unknown => "???",
        }
    }
}

/// A network connection associated with a process.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NetworkConnection {
    pub proto: String,
    pub local_addr: String,
    pub remote_addr: Option<String>,
    pub state: Option<String>,
}

/// Detailed process information for the info panel.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProcessDetail {
    pub pid: i32,
    pub ppid: i32,
    pub parent_name: String,
    pub name: String,
    pub path: String,
    pub cwd: String,
    pub user: String,
    pub uid: u32,
    pub state: ProcessState,
    pub nice: i32,
    pub started_secs: u64,
    pub thread_count: i32,
    pub virtual_size: u64,
    pub memory_rss: u64,
    pub memory_phys: u64,
    pub cpu_time_ns: u64,
    pub fd_files: usize,
    pub fd_sockets: usize,
    pub fd_pipes: usize,
    pub fd_other: usize,
    pub fd_total: usize,
    pub faults: i32,
    pub context_switches: i32,
    pub syscalls_mach: i32,
    pub syscalls_unix: i32,
    pub disk_bytes_read: u64,
    pub disk_bytes_written: u64,
    pub network: Vec<NetworkConnection>,
    pub environment: Vec<(String, String)>,
}

/// List all PIDs on the system.
pub fn list_all_pids() -> Result<Vec<i32>, FfiError> {
    let count = unsafe { raw::proc_listallpids(std::ptr::null_mut(), 0) };
    if count <= 0 {
        return Err(FfiError::SystemError {
            function: "proc_listallpids",
            pid: 0,
            reason: "failed to get pid count".into(),
        });
    }

    let capacity = (count as usize) * 2;
    let mut buffer: Vec<i32> = vec![0; capacity];
    let buf_size = (capacity * mem::size_of::<i32>()) as c_int;

    let actual = unsafe { raw::proc_listallpids(buffer.as_mut_ptr() as *mut c_void, buf_size) };
    if actual <= 0 {
        return Err(FfiError::SystemError {
            function: "proc_listallpids",
            pid: 0,
            reason: "failed to list pids".into(),
        });
    }

    buffer.truncate(actual as usize);
    Ok(buffer)
}

/// Get the name of a process by PID.
pub fn get_process_name(pid: i32) -> Result<String, FfiError> {
    let mut buffer = [0u8; 256];
    let ret = unsafe {
        raw::proc_name(pid as c_int, buffer.as_mut_ptr() as *mut c_void, buffer.len() as u32)
    };
    if ret <= 0 {
        return Err(check_errno("proc_name", pid));
    }

    Ok(std::str::from_utf8(&buffer[..ret as usize])
        .unwrap_or("?")
        .to_string())
}

/// List all open file descriptors for a process.
pub fn list_fds(pid: i32) -> Result<Vec<FdInfo>, FfiError> {
    let fd_size = mem::size_of::<raw::ProcFdInfo>() as c_int;

    let buf_needed = unsafe {
        raw::proc_pidinfo(pid as c_int, raw::PROC_PIDLISTFDS, 0, std::ptr::null_mut(), 0)
    };
    if buf_needed <= 0 {
        return Err(check_errno("proc_pidinfo(PROC_PIDLISTFDS)", pid));
    }

    let alloc_size = buf_needed * 2;
    let count = alloc_size / fd_size;
    let mut buffer: Vec<raw::ProcFdInfo> = vec![
        raw::ProcFdInfo { proc_fd: 0, proc_fdtype: 0 };
        count as usize
    ];

    let actual = unsafe {
        raw::proc_pidinfo(
            pid as c_int,
            raw::PROC_PIDLISTFDS,
            0,
            buffer.as_mut_ptr() as *mut c_void,
            alloc_size,
        )
    };
    if actual <= 0 {
        return Err(check_errno("proc_pidinfo(PROC_PIDLISTFDS)", pid));
    }

    let fd_count = actual / fd_size;
    buffer.truncate(fd_count as usize);

    Ok(buffer
        .iter()
        .map(|info| FdInfo { fd: info.proc_fd, fdtype: info.proc_fdtype })
        .collect())
}

/// Get process metadata: PPID, thread count, CPU time, memory, and full name.
pub fn get_process_info(pid: i32) -> Result<ProcessInfo, FfiError> {
    let mut bsd_info: raw::ProcBsdInfo = unsafe { mem::zeroed() };
    let size = mem::size_of::<raw::ProcBsdInfo>() as c_int;

    let ret = unsafe {
        raw::proc_pidinfo(
            pid as c_int,
            raw::PROC_PIDTBSDINFO,
            0,
            &mut bsd_info as *mut _ as *mut c_void,
            size,
        )
    };
    if ret <= 0 {
        return Err(check_errno("proc_pidinfo(PROC_PIDTBSDINFO)", pid));
    }

    let name = extract_c_string(&bsd_info.pbi_name);
    let name = if name.is_empty() {
        extract_c_string(&bsd_info.pbi_comm)
    } else {
        name
    };

    let task_info = get_task_info(pid);
    let (thread_count, memory_rss, cpu_time_ns, faults, csw, sys_mach, sys_unix) = match task_info {
        Ok(ti) => {
            let raw_ticks = ti.pti_total_user + ti.pti_total_system;
            let ns = mach_ticks_to_ns(raw_ticks);
            (ti.pti_threadnum, ti.pti_resident_size, ns,
             ti.pti_faults, ti.pti_csw, ti.pti_syscalls_mach, ti.pti_syscalls_unix)
        }
        Err(_) => (0, 0, 0, 0, 0, 0, 0),
    };

    let memory_phys = get_phys_footprint(pid).unwrap_or(0);
    let (disk_read, disk_write) = get_disk_io(pid).unwrap_or((0, 0));

    Ok(ProcessInfo {
        ppid: bsd_info.pbi_ppid as i32,
        name,
        thread_count,
        memory_rss,
        memory_phys,
        cpu_time_ns,
        state: ProcessState::from_bsd_status(bsd_info.pbi_status),
        start_time: bsd_info.pbi_start_tvsec,
        faults,
        context_switches: csw,
        syscalls_mach: sys_mach,
        syscalls_unix: sys_unix,
        disk_bytes_read: disk_read,
        disk_bytes_written: disk_write,
    })
}

/// Resolve the detail of a single file descriptor.
pub fn resolve_fd(pid: i32, fd: i32, fdtype: u32) -> Result<FdDetail, FfiError> {
    match fdtype {
        raw::PROX_FDTYPE_VNODE => resolve_vnode(pid, fd),
        raw::PROX_FDTYPE_SOCKET => resolve_socket(pid, fd),
        raw::PROX_FDTYPE_PIPE => resolve_pipe(pid, fd),
        raw::PROX_FDTYPE_KQUEUE => Ok(FdDetail::Kqueue),
        raw::PROX_FDTYPE_PSHM => Ok(FdDetail::Pshm),
        raw::PROX_FDTYPE_PSEM => Ok(FdDetail::Psem),
        other => Ok(FdDetail::Unknown(other)),
    }
}

/// Find PIDs that have a given file path open.
pub fn list_pids_by_path(path: &str) -> Result<Vec<i32>, FfiError> {
    let c_path = CString::new(path).map_err(|e| FfiError::SystemError {
        function: "proc_listpidspath",
        pid: 0,
        reason: format!("invalid path: {}", e),
    })?;

    let count = unsafe {
        raw::proc_listpidspath(raw::PROC_ALL_PIDS, 0, c_path.as_ptr(), 0, std::ptr::null_mut(), 0)
    };
    if count <= 0 {
        return Ok(Vec::new());
    }

    let capacity = (count as usize) * 2;
    let mut buffer: Vec<i32> = vec![0; capacity];
    let buf_size = (capacity * mem::size_of::<i32>()) as c_int;

    let actual = unsafe {
        raw::proc_listpidspath(
            raw::PROC_ALL_PIDS, 0, c_path.as_ptr(), 0,
            buffer.as_mut_ptr() as *mut c_void, buf_size,
        )
    };
    if actual <= 0 {
        return Ok(Vec::new());
    }

    buffer.truncate(actual as usize);
    buffer.retain(|&pid| pid > 0);
    Ok(buffer)
}

/// Get the full path of a process's executable.
pub fn get_process_path(pid: i32) -> Result<String, FfiError> {
    let mut buffer = [0u8; raw::MAXPATHLEN];
    let ret = unsafe {
        raw::proc_pidpath(pid as c_int, buffer.as_mut_ptr() as *mut c_void, buffer.len() as u32)
    };
    if ret <= 0 {
        return Err(check_errno("proc_pidpath", pid));
    }
    let end = buffer.iter().position(|&b| b == 0).unwrap_or(ret as usize);
    Ok(String::from_utf8_lossy(&buffer[..end]).into_owned())
}

/// Get the current working directory of a process.
pub fn get_process_cwd(pid: i32) -> Result<String, FfiError> {
    let mut info: raw::ProcVnodePathInfo = unsafe { mem::zeroed() };
    let size = mem::size_of::<raw::ProcVnodePathInfo>() as c_int;

    let ret = unsafe {
        raw::proc_pidinfo(
            pid as c_int, raw::PROC_PIDVNODEPATHINFO, 0,
            &mut info as *mut _ as *mut c_void, size,
        )
    };
    if ret <= 0 {
        return Err(check_errno("proc_pidinfo(PROC_PIDVNODEPATHINFO)", pid));
    }

    let end = info.pvi_cdir.vip_path.iter().position(|&b| b == 0).unwrap_or(info.pvi_cdir.vip_path.len());
    Ok(String::from_utf8_lossy(&info.pvi_cdir.vip_path[..end]).into_owned())
}

/// Get environment variables of a process via sysctl KERN_PROCARGS2.
pub fn get_process_env(pid: i32) -> Result<Vec<(String, String)>, FfiError> {
    let mut mib = [raw::CTL_KERN, raw::KERN_PROCARGS2, pid as c_int];
    let mut size: usize = 0;

    // First call: get buffer size.
    let ret = unsafe {
        raw::sysctl(mib.as_mut_ptr(), 3, std::ptr::null_mut(), &mut size, std::ptr::null(), 0)
    };
    if ret != 0 || size == 0 {
        return Ok(Vec::new()); // permission denied or unavailable
    }

    let mut buffer = vec![0u8; size];
    let ret = unsafe {
        raw::sysctl(mib.as_mut_ptr(), 3, buffer.as_mut_ptr() as *mut c_void, &mut size, std::ptr::null(), 0)
    };
    if ret != 0 {
        return Ok(Vec::new());
    }
    buffer.truncate(size);

    // Parse KERN_PROCARGS2 format:
    // [argc: i32] [exec_path\0] [argv[0]\0 ... argv[argc-1]\0] [env[0]\0 env[1]\0 ... \0]
    if buffer.len() < 4 {
        return Ok(Vec::new());
    }

    let argc = i32::from_ne_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]) as usize;
    let mut pos = 4;

    // Skip exec path.
    while pos < buffer.len() && buffer[pos] != 0 { pos += 1; }
    pos += 1; // skip null

    // Skip padding nulls.
    while pos < buffer.len() && buffer[pos] == 0 { pos += 1; }

    // Skip argv.
    for _ in 0..argc {
        while pos < buffer.len() && buffer[pos] != 0 { pos += 1; }
        pos += 1;
    }

    // Remaining null-terminated strings are environment variables.
    let mut env = Vec::new();
    while pos < buffer.len() {
        let start = pos;
        while pos < buffer.len() && buffer[pos] != 0 { pos += 1; }
        if pos == start { break; } // empty string = end

        if let Ok(s) = std::str::from_utf8(&buffer[start..pos]) {
            if let Some((key, val)) = s.split_once('=') {
                env.push((key.to_string(), val.to_string()));
            }
        }
        pos += 1;
    }

    Ok(env)
}

/// Resolve network connections for a process by parsing its socket fds.
pub fn get_network_connections(pid: i32) -> Vec<NetworkConnection> {
    let fds = match list_fds(pid) {
        Ok(fds) => fds,
        Err(_) => return Vec::new(),
    };

    let mut connections = Vec::new();
    for fd_info in &fds {
        if fd_info.fdtype != raw::PROX_FDTYPE_SOCKET {
            continue;
        }
        if let Ok(conn) = resolve_socket_detail(pid, fd_info.fd) {
            connections.push(conn);
        }
    }
    connections
}

fn resolve_socket_detail(pid: i32, fd: i32) -> Result<NetworkConnection, FfiError> {
    let mut info: raw::SocketFdInfo = unsafe { mem::zeroed() };
    let size = mem::size_of::<raw::SocketFdInfo>() as c_int;

    let ret = unsafe {
        raw::proc_pidfdinfo(
            pid as c_int, fd as c_int, raw::PROC_PIDFDSOCKETINFO,
            &mut info as *mut _ as *mut c_void, size,
        )
    };
    if ret <= 0 {
        return Err(check_errno("proc_pidfdinfo(SOCKET)", pid));
    }

    let family = info.psi.soi_family;
    let sock_type = info.psi.soi_type;

    // Only parse AF_INET (2) and AF_INET6 (30) TCP/UDP sockets.
    if family != 2 && family != 30 {
        return Ok(NetworkConnection {
            proto: format!("sock({})", family),
            local_addr: "*".into(),
            remote_addr: None,
            state: None,
        });
    }

    // Read in_sockinfo from the opaque soi_proto union.
    let in_info: raw::InSockInfo = unsafe {
        std::ptr::read(info.psi.soi_proto.as_ptr() as *const raw::InSockInfo)
    };

    let proto = if sock_type == 1 { "tcp" } else { "udp" };
    let local_addr = format_sock_addr(&in_info.insi_laddr, in_info.insi_lport, in_info.insi_vflag);
    let remote_addr = format_sock_addr(&in_info.insi_faddr, in_info.insi_fport, in_info.insi_vflag);

    let state = if sock_type == 1 {
        // TCP — read tcp_sockinfo for state.
        let tcp_info: raw::TcpSockInfo = unsafe {
            std::ptr::read(info.psi.soi_proto.as_ptr() as *const raw::TcpSockInfo)
        };
        Some(tcp_state_name(tcp_info.tcpsi_state))
    } else {
        None
    };

    let remote = if in_info.insi_fport != 0 {
        Some(remote_addr)
    } else {
        None
    };

    Ok(NetworkConnection {
        proto: proto.to_string(),
        local_addr,
        remote_addr: remote,
        state,
    })
}

fn format_sock_addr(addr: &raw::In4In6Addr, port: i32, vflag: u8) -> String {
    let ip = if vflag & raw::INI_IPV4 != 0 {
        let b = addr.i46a_addr4;
        if b == [0, 0, 0, 0] {
            "*".to_string()
        } else {
            format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3])
        }
    } else {
        "*".to_string() // IPv6 display simplified for now
    };

    if port == 0 {
        format!("{}:*", ip)
    } else {
        format!("{}:{}", ip, port)
    }
}

fn tcp_state_name(state: i32) -> String {
    match state {
        raw::TCPS_CLOSED => "CLOSED",
        raw::TCPS_LISTEN => "LISTEN",
        raw::TCPS_SYN_SENT => "SYN_SENT",
        raw::TCPS_SYN_RECEIVED => "SYN_RCVD",
        raw::TCPS_ESTABLISHED => "ESTABLISHED",
        raw::TCPS_CLOSE_WAIT => "CLOSE_WAIT",
        raw::TCPS_FIN_WAIT_1 => "FIN_WAIT_1",
        raw::TCPS_CLOSING => "CLOSING",
        raw::TCPS_LAST_ACK => "LAST_ACK",
        raw::TCPS_FIN_WAIT_2 => "FIN_WAIT_2",
        raw::TCPS_TIME_WAIT => "TIME_WAIT",
        _ => "UNKNOWN",
    }
    .to_string()
}

/// Get username for a UID. Falls back to the numeric UID.
pub fn get_username(uid: u32) -> String {
    // Simple approach: try /etc/passwd or just return uid.
    // On macOS, most users are in Directory Services, not /etc/passwd.
    // We'll just return the numeric uid for now.
    format!("uid:{}", uid)
}

/// Build a full ProcessDetail for the info panel.
pub fn get_process_detail(pid: i32, parent_name: &str) -> Result<ProcessDetail, FfiError> {
    // Get BSD info directly for uid, nice, state, etc.
    let mut bsd_info: raw::ProcBsdInfo = unsafe { mem::zeroed() };
    let bsd_size = mem::size_of::<raw::ProcBsdInfo>() as c_int;
    let ret = unsafe {
        raw::proc_pidinfo(
            pid as c_int, raw::PROC_PIDTBSDINFO, 0,
            &mut bsd_info as *mut _ as *mut c_void, bsd_size,
        )
    };
    if ret <= 0 {
        return Err(check_errno("proc_pidinfo(PROC_PIDTBSDINFO)", pid));
    }

    let name = extract_c_string(&bsd_info.pbi_name);
    let name = if name.is_empty() { extract_c_string(&bsd_info.pbi_comm) } else { name };

    let task_info = get_task_info(pid);
    let (thread_count, memory_rss, cpu_time_ns, virtual_size, faults, csw, sys_mach, sys_unix) =
        match &task_info {
            Ok(ti) => {
                let ns = mach_ticks_to_ns(ti.pti_total_user + ti.pti_total_system);
                (ti.pti_threadnum, ti.pti_resident_size, ns, ti.pti_virtual_size,
                 ti.pti_faults, ti.pti_csw, ti.pti_syscalls_mach, ti.pti_syscalls_unix)
            }
            Err(_) => (0, 0, 0, 0, 0, 0, 0, 0),
        };
    let memory_phys = get_phys_footprint(pid).unwrap_or(0);
    let (disk_read, disk_write) = get_disk_io(pid).unwrap_or((0, 0));

    let path = get_process_path(pid).unwrap_or_default();
    let cwd = get_process_cwd(pid).unwrap_or_default();
    let env = get_process_env(pid).unwrap_or_default();
    let network = get_network_connections(pid);

    let fds = list_fds(pid).unwrap_or_default();
    let mut fd_files = 0usize;
    let mut fd_sockets = 0usize;
    let mut fd_pipes = 0usize;
    let mut fd_other = 0usize;
    for fd in &fds {
        match fd.fdtype {
            raw::PROX_FDTYPE_VNODE => fd_files += 1,
            raw::PROX_FDTYPE_SOCKET => fd_sockets += 1,
            raw::PROX_FDTYPE_PIPE => fd_pipes += 1,
            _ => fd_other += 1,
        }
    }

    Ok(ProcessDetail {
        pid,
        ppid: bsd_info.pbi_ppid as i32,
        parent_name: parent_name.to_string(),
        name,
        path,
        cwd,
        user: get_username(bsd_info.pbi_uid),
        uid: bsd_info.pbi_uid,
        state: ProcessState::from_bsd_status(bsd_info.pbi_status),
        nice: bsd_info.pbi_nice,
        started_secs: bsd_info.pbi_start_tvsec,
        thread_count,
        virtual_size,
        memory_rss,
        memory_phys,
        cpu_time_ns,
        fd_files,
        fd_sockets,
        fd_pipes,
        fd_other,
        fd_total: fds.len(),
        faults,
        context_switches: csw,
        syscalls_mach: sys_mach,
        syscalls_unix: sys_unix,
        disk_bytes_read: disk_read,
        disk_bytes_written: disk_write,
        network,
        environment: env,
    })
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn get_disk_io(pid: i32) -> Result<(u64, u64), FfiError> {
    let mut ri: raw::RusageInfoV4 = unsafe { mem::zeroed() };
    let ret = unsafe {
        raw::proc_pid_rusage(
            pid as c_int,
            raw::RUSAGE_INFO_V4,
            &mut ri as *mut _ as *mut c_void,
        )
    };
    if ret != 0 {
        return Err(check_errno("proc_pid_rusage", pid));
    }
    Ok((ri.ri_diskio_bytesread, ri.ri_diskio_byteswritten))
}

fn get_phys_footprint(pid: i32) -> Result<u64, FfiError> {
    let self_task = unsafe { raw::mach_task_self() };
    let mut task_port: u32 = 0;

    let kr = unsafe { raw::task_name_for_pid(self_task, pid as c_int, &mut task_port) };
    if kr != 0 {
        return Err(FfiError::PermissionDenied(pid));
    }

    let mut vm_info: raw::TaskVmInfo = unsafe { mem::zeroed() };
    let mut count = (mem::size_of::<raw::TaskVmInfo>() / mem::size_of::<u32>()) as u32;

    let kr = unsafe {
        raw::task_info(task_port, raw::TASK_VM_INFO, &mut vm_info as *mut _ as *mut c_void, &mut count)
    };

    unsafe { raw::mach_port_deallocate(self_task, task_port) };

    if kr != 0 {
        return Err(FfiError::SystemError {
            function: "task_info(TASK_VM_INFO)",
            pid,
            reason: format!("kern_return_t = {}", kr),
        });
    }

    Ok(vm_info.phys_footprint)
}

fn get_task_info(pid: i32) -> Result<raw::ProcTaskInfo, FfiError> {
    let mut task_info: raw::ProcTaskInfo = unsafe { mem::zeroed() };
    let size = mem::size_of::<raw::ProcTaskInfo>() as c_int;

    let ret = unsafe {
        raw::proc_pidinfo(
            pid as c_int, raw::PROC_PIDTASKINFO, 0,
            &mut task_info as *mut _ as *mut c_void, size,
        )
    };
    if ret <= 0 {
        return Err(check_errno("proc_pidinfo(PROC_PIDTASKINFO)", pid));
    }

    Ok(task_info)
}

fn resolve_vnode(pid: i32, fd: i32) -> Result<FdDetail, FfiError> {
    let mut info: raw::VnodeFdInfoWithPath = unsafe { mem::zeroed() };
    let size = mem::size_of::<raw::VnodeFdInfoWithPath>() as c_int;

    let ret = unsafe {
        raw::proc_pidfdinfo(pid as c_int, fd as c_int, raw::PROC_PIDFDVNODEPATHINFO, &mut info as *mut _ as *mut c_void, size)
    };
    if ret <= 0 {
        return Err(check_errno("proc_pidfdinfo(VNODE)", pid));
    }

    let end = info.pvip.vip_path.iter().position(|&b| b == 0).unwrap_or(info.pvip.vip_path.len());
    let path = String::from_utf8_lossy(&info.pvip.vip_path[..end]).into_owned();
    Ok(FdDetail::Vnode { path })
}

fn resolve_socket(pid: i32, fd: i32) -> Result<FdDetail, FfiError> {
    let mut info: raw::SocketFdInfo = unsafe { mem::zeroed() };
    let size = mem::size_of::<raw::SocketFdInfo>() as c_int;

    let ret = unsafe {
        raw::proc_pidfdinfo(pid as c_int, fd as c_int, raw::PROC_PIDFDSOCKETINFO, &mut info as *mut _ as *mut c_void, size)
    };
    if ret <= 0 {
        return Err(check_errno("proc_pidfdinfo(SOCKET)", pid));
    }

    Ok(FdDetail::Socket {
        family: info.psi.soi_family,
        socket_type: info.psi.soi_type,
        protocol: info.psi.soi_protocol,
    })
}

fn resolve_pipe(pid: i32, fd: i32) -> Result<FdDetail, FfiError> {
    let mut info: raw::PipeFdInfo = unsafe { mem::zeroed() };
    let size = mem::size_of::<raw::PipeFdInfo>() as c_int;

    let ret = unsafe {
        raw::proc_pidfdinfo(pid as c_int, fd as c_int, raw::PROC_PIDFDPIPEINFO, &mut info as *mut _ as *mut c_void, size)
    };
    if ret <= 0 {
        return Err(check_errno("proc_pidfdinfo(PIPE)", pid));
    }

    Ok(FdDetail::Pipe)
}
