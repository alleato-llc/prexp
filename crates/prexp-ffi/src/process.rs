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
    let (thread_count, memory_rss, cpu_time_ns) = match task_info {
        Ok(ti) => {
            let raw_ticks = ti.pti_total_user + ti.pti_total_system;
            let ns = mach_ticks_to_ns(raw_ticks);
            (ti.pti_threadnum, ti.pti_resident_size, ns)
        }
        Err(_) => (0, 0, 0),
    };

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

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

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
