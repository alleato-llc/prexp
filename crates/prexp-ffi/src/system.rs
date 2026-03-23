//! Safe wrappers for system-level APIs (CPU cores, memory).
//!
//! Provides: get_cpu_ticks, get_memory_info.

use std::mem;
use std::os::raw::c_void;

use crate::error::FfiError;
use crate::raw;

/// Per-CPU tick counts (user, system, idle, nice).
#[derive(Debug, Clone)]
pub struct CpuTicks {
    pub user: u32,
    pub system: u32,
    pub idle: u32,
    pub nice: u32,
}

/// System memory information.
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    pub total: u64,
    pub used: u64,
    pub free: u64,
    pub wired: u64,
    pub compressed: u64,
}

/// Get per-CPU tick counts for all cores.
pub fn get_cpu_ticks() -> Result<Vec<CpuTicks>, FfiError> {
    let host = unsafe { raw::mach_host_self() };
    let mut cpu_count: u32 = 0;
    let mut info_ptr: *mut i32 = std::ptr::null_mut();
    let mut info_count: u32 = 0;

    let kr = unsafe {
        raw::host_processor_info(
            host,
            raw::PROCESSOR_CPU_LOAD_INFO,
            &mut cpu_count,
            &mut info_ptr,
            &mut info_count,
        )
    };
    if kr != 0 {
        return Err(FfiError::SystemError {
            function: "host_processor_info",
            pid: 0,
            reason: format!("kern_return_t = {}", kr),
        });
    }

    let mut ticks = Vec::with_capacity(cpu_count as usize);
    for i in 0..cpu_count as usize {
        let base = i * raw::CPU_STATE_MAX;
        unsafe {
            ticks.push(CpuTicks {
                user: *info_ptr.add(base + raw::CPU_STATE_USER) as u32,
                system: *info_ptr.add(base + raw::CPU_STATE_SYSTEM) as u32,
                idle: *info_ptr.add(base + raw::CPU_STATE_IDLE) as u32,
                nice: *info_ptr.add(base + raw::CPU_STATE_NICE) as u32,
            });
        }
    }

    unsafe {
        raw::vm_deallocate(
            raw::mach_task_self(),
            info_ptr as usize,
            (info_count as usize) * mem::size_of::<i32>(),
        );
    }

    Ok(ticks)
}

/// Get system memory information.
pub fn get_memory_info() -> Result<MemoryInfo, FfiError> {
    let total = get_total_memory()?;
    let page_size = get_page_size();

    let host = unsafe { raw::mach_host_self() };
    let mut vm_info: raw::VmStatistics64 = unsafe { mem::zeroed() };
    let mut count = (mem::size_of::<raw::VmStatistics64>() / mem::size_of::<i32>()) as u32;

    let kr = unsafe {
        raw::host_statistics64(
            host,
            raw::HOST_VM_INFO64,
            &mut vm_info as *mut _ as *mut c_void,
            &mut count,
        )
    };
    if kr != 0 {
        return Err(FfiError::SystemError {
            function: "host_statistics64",
            pid: 0,
            reason: format!("kern_return_t = {}", kr),
        });
    }

    let wired = vm_info.wire_count as u64 * page_size;
    let compressed = vm_info.compressor_page_count as u64 * page_size;
    let active = vm_info.active_count as u64 * page_size;
    let used = active + wired + compressed;

    Ok(MemoryInfo {
        total,
        used,
        free: total.saturating_sub(used),
        wired,
        compressed,
    })
}

fn get_total_memory() -> Result<u64, FfiError> {
    let name = std::ffi::CString::new("hw.memsize").unwrap();
    let mut memsize: u64 = 0;
    let mut len = mem::size_of::<u64>();

    let ret = unsafe {
        raw::sysctlbyname(name.as_ptr(), &mut memsize as *mut _ as *mut c_void, &mut len, std::ptr::null(), 0)
    };
    if ret != 0 {
        return Err(FfiError::SystemError {
            function: "sysctlbyname(hw.memsize)",
            pid: 0,
            reason: "failed".into(),
        });
    }

    Ok(memsize)
}

fn get_page_size() -> u64 {
    let name = std::ffi::CString::new("hw.pagesize").unwrap();
    let mut pagesize: u64 = 0;
    let mut len = mem::size_of::<u64>();

    let ret = unsafe {
        raw::sysctlbyname(name.as_ptr(), &mut pagesize as *mut _ as *mut c_void, &mut len, std::ptr::null(), 0)
    };
    if ret != 0 { 4096 } else { pagesize }
}
