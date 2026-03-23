//! Safe Rust wrappers around raw libproc FFI calls.
//!
//! All `unsafe` blocks are contained in this module. Downstream crates
//! call only these safe functions.

use std::ffi::CString;
use std::mem;
use std::os::raw::{c_int, c_void};

use crate::raw;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Information about a single open file descriptor.
#[derive(Debug, Clone)]
pub struct FdInfo {
    pub fd: i32,
    pub fdtype: u32,
}

/// Resolved detail for a file descriptor.
#[derive(Debug, Clone)]
pub enum FdDetail {
    /// A vnode (file, directory, device, etc.) with its path.
    Vnode { path: String },
    /// A socket with its family (AF_INET, AF_UNIX, etc.) and type (SOCK_STREAM, etc.).
    Socket { family: i32, socket_type: i32, protocol: i32 },
    /// A pipe.
    Pipe,
    /// A kqueue descriptor.
    Kqueue,
    /// POSIX shared memory.
    Pshm,
    /// POSIX semaphore.
    Psem,
    /// Unrecognized fd type.
    Unknown(u32),
}

/// Process metadata: PPID, thread count, CPU time, memory, and full name.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub ppid: i32,
    pub name: String,
    pub thread_count: i32,
    /// Resident set size in bytes.
    pub memory_rss: u64,
    /// Physical footprint (private memory) in bytes. 0 if unavailable.
    pub memory_phys: u64,
    /// Cumulative CPU time (user + system) in nanoseconds.
    pub cpu_time_ns: u64,
}

/// Errors from FFI calls.
#[derive(Debug, thiserror::Error)]
pub enum FfiError {
    #[error("process {0} has exited or does not exist")]
    ProcessGone(i32),

    #[error("permission denied for process {0}")]
    PermissionDenied(i32),

    #[error("FFI call to {function} failed for pid {pid}: {reason}")]
    SystemError {
        function: &'static str,
        pid: i32,
        reason: String,
    },
}

// ---------------------------------------------------------------------------
// Helper: check errno after a failed FFI call
// ---------------------------------------------------------------------------

fn check_errno(function: &'static str, pid: i32) -> FfiError {
    let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(0);
    match errno {
        libc_constants::ESRCH => FfiError::ProcessGone(pid),
        libc_constants::EPERM | libc_constants::EACCES => FfiError::PermissionDenied(pid),
        _ => FfiError::SystemError {
            function,
            pid,
            reason: std::io::Error::from_raw_os_error(errno).to_string(),
        },
    }
}

/// BSD errno constants we care about (avoiding a libc dependency).
mod libc_constants {
    pub const EPERM: i32 = 1;
    pub const ESRCH: i32 = 3;
    pub const EACCES: i32 = 13;
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// List all PIDs on the system.
pub fn list_all_pids() -> Result<Vec<i32>, FfiError> {
    // First call: get the number of PIDs.
    let count = unsafe { raw::proc_listallpids(std::ptr::null_mut(), 0) };
    if count <= 0 {
        return Err(FfiError::SystemError {
            function: "proc_listallpids",
            pid: 0,
            reason: "failed to get pid count".into(),
        });
    }

    // Allocate with some headroom for new processes.
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
///
/// Note: `proc_name` truncates to MAXCOMLEN (16 chars).
pub fn get_process_name(pid: i32) -> Result<String, FfiError> {
    let mut buffer = [0u8; 256];
    let ret = unsafe {
        raw::proc_name(
            pid as c_int,
            buffer.as_mut_ptr() as *mut c_void,
            buffer.len() as u32,
        )
    };
    if ret <= 0 {
        return Err(check_errno("proc_name", pid));
    }

    let name = std::str::from_utf8(&buffer[..ret as usize])
        .unwrap_or("?")
        .to_string();
    Ok(name)
}

/// List all open file descriptors for a process.
pub fn list_fds(pid: i32) -> Result<Vec<FdInfo>, FfiError> {
    let fd_size = mem::size_of::<raw::ProcFdInfo>() as c_int;

    // First call: get the buffer size needed.
    let buf_needed = unsafe { raw::proc_pidinfo(pid as c_int, raw::PROC_PIDLISTFDS, 0, std::ptr::null_mut(), 0) };
    if buf_needed <= 0 {
        return Err(check_errno("proc_pidinfo(PROC_PIDLISTFDS)", pid));
    }

    // Allocate with headroom (fds can appear between calls).
    let alloc_size = buf_needed * 2;
    let count = alloc_size / fd_size;
    let mut buffer: Vec<raw::ProcFdInfo> = vec![
        raw::ProcFdInfo {
            proc_fd: 0,
            proc_fdtype: 0,
        };
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
        .map(|info| FdInfo {
            fd: info.proc_fd,
            fdtype: info.proc_fdtype,
        })
        .collect())
}

/// Get process metadata: PPID, thread count, and full name.
///
/// Combines `proc_pidinfo(PROC_PIDTBSDINFO)` for PPID/name and
/// `proc_pidinfo(PROC_PIDTASKINFO)` for thread count.
pub fn get_process_info(pid: i32) -> Result<ProcessInfo, FfiError> {
    // Get BSD info (ppid, name).
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

    // Use pbi_name (32 chars) if available, fall back to pbi_comm (16 chars).
    let name = extract_c_string(&bsd_info.pbi_name);
    let name = if name.is_empty() {
        extract_c_string(&bsd_info.pbi_comm)
    } else {
        name
    };

    // Get task info (thread count, memory, CPU time).
    let task_info = get_task_info(pid);
    let (thread_count, memory_rss, cpu_time_ns) = match task_info {
        Ok(ti) => {
            let raw_ticks = ti.pti_total_user + ti.pti_total_system;
            let ns = mach_ticks_to_ns(raw_ticks);
            (ti.pti_threadnum, ti.pti_resident_size, ns)
        }
        Err(_) => (0, 0, 0),
    };

    // Get physical footprint (private memory) via Mach task_info.
    let memory_phys = get_phys_footprint(pid).unwrap_or(0);

    Ok(ProcessInfo {
        ppid: bsd_info.pbi_ppid as i32,
        name,
        thread_count,
        memory_rss,
        memory_phys,
        cpu_time_ns,
    })
}

/// Get physical footprint via task_name_for_pid + task_info(TASK_VM_INFO).
/// Returns 0 if the process is inaccessible (no root required for own processes
/// and many others via task_name_for_pid).
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
        raw::task_info(
            task_port,
            raw::TASK_VM_INFO,
            &mut vm_info as *mut _ as *mut c_void,
            &mut count,
        )
    };

    // Always deallocate the port.
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
            pid as c_int,
            raw::PROC_PIDTASKINFO,
            0,
            &mut task_info as *mut _ as *mut c_void,
            size,
        )
    };
    if ret <= 0 {
        return Err(check_errno("proc_pidinfo(PROC_PIDTASKINFO)", pid));
    }

    Ok(task_info)
}

/// Convert Mach absolute time ticks to nanoseconds.
///
/// On Intel, the ratio is 1:1. On Apple Silicon, the ratio is typically 125:3
/// (~41.67x). We query `mach_timebase_info()` once and cache the result.
fn mach_ticks_to_ns(ticks: u64) -> u64 {
    use std::sync::OnceLock;
    static TIMEBASE: OnceLock<(u32, u32)> = OnceLock::new();

    let &(numer, denom) = TIMEBASE.get_or_init(|| {
        let mut info = raw::MachTimebaseInfo { numer: 0, denom: 0 };
        unsafe { raw::mach_timebase_info(&mut info) };
        (info.numer, info.denom)
    });

    // ticks * numer / denom, avoiding overflow with u128.
    ((ticks as u128 * numer as u128) / denom as u128) as u64
}

/// Extract a null-terminated C string from a byte slice.
fn extract_c_string(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

/// Resolve the detail of a single file descriptor.
///
/// The `fdtype` comes from `FdInfo.fdtype` and determines which
/// `proc_pidfdinfo` flavor to use.
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

    // First call: get the number of matching PIDs.
    let count = unsafe {
        raw::proc_listpidspath(
            raw::PROC_ALL_PIDS,
            0,
            c_path.as_ptr(),
            0,
            std::ptr::null_mut(),
            0,
        )
    };
    if count <= 0 {
        // No matches or error — return empty rather than error.
        return Ok(Vec::new());
    }

    let capacity = (count as usize) * 2;
    let mut buffer: Vec<i32> = vec![0; capacity];
    let buf_size = (capacity * mem::size_of::<i32>()) as c_int;

    let actual = unsafe {
        raw::proc_listpidspath(
            raw::PROC_ALL_PIDS,
            0,
            c_path.as_ptr(),
            0,
            buffer.as_mut_ptr() as *mut c_void,
            buf_size,
        )
    };
    if actual <= 0 {
        return Ok(Vec::new());
    }

    buffer.truncate(actual as usize);
    // Filter out zero entries.
    buffer.retain(|&pid| pid > 0);
    Ok(buffer)
}

// ---------------------------------------------------------------------------
// Internal resolve helpers
// ---------------------------------------------------------------------------

fn resolve_vnode(pid: i32, fd: i32) -> Result<FdDetail, FfiError> {
    let mut info: raw::VnodeFdInfoWithPath = unsafe { mem::zeroed() };
    let size = mem::size_of::<raw::VnodeFdInfoWithPath>() as c_int;

    let ret = unsafe {
        raw::proc_pidfdinfo(
            pid as c_int,
            fd as c_int,
            raw::PROC_PIDFDVNODEPATHINFO,
            &mut info as *mut _ as *mut c_void,
            size,
        )
    };
    if ret <= 0 {
        return Err(check_errno("proc_pidfdinfo(VNODE)", pid));
    }

    let path = extract_path(&info.pvip.vip_path);
    Ok(FdDetail::Vnode { path })
}

fn resolve_socket(pid: i32, fd: i32) -> Result<FdDetail, FfiError> {
    let mut info: raw::SocketFdInfo = unsafe { mem::zeroed() };
    let size = mem::size_of::<raw::SocketFdInfo>() as c_int;

    let ret = unsafe {
        raw::proc_pidfdinfo(
            pid as c_int,
            fd as c_int,
            raw::PROC_PIDFDSOCKETINFO,
            &mut info as *mut _ as *mut c_void,
            size,
        )
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
        raw::proc_pidfdinfo(
            pid as c_int,
            fd as c_int,
            raw::PROC_PIDFDPIPEINFO,
            &mut info as *mut _ as *mut c_void,
            size,
        )
    };
    if ret <= 0 {
        return Err(check_errno("proc_pidfdinfo(PIPE)", pid));
    }

    Ok(FdDetail::Pipe)
}

/// Extract a null-terminated C string from a fixed-size byte buffer.
fn extract_path(buf: &[u8; raw::MAXPATHLEN]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}
