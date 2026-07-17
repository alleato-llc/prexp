//! Safe wrappers for system-level APIs (CPU cores, memory, network, disk).
//!
//! Provides: get_cpu_ticks, get_memory_info, get_network_counters,
//! get_disk_counters, get_cpu_perf_levels.

use std::ffi::CString;
use std::mem;
use std::os::raw::{c_char, c_void};

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

/// Cumulative system-wide network byte counters (monotonic; diff two reads for
/// a rate). Summed across all non-loopback interfaces.
#[derive(Debug, Clone, Copy)]
pub struct NetworkCounters {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

/// Cumulative system-wide disk byte counters (monotonic; diff two reads for a
/// rate). Summed across all block-storage drivers.
#[derive(Debug, Clone, Copy)]
pub struct DiskCounters {
    pub read_bytes: u64,
    pub write_bytes: u64,
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

/// Get cumulative system-wide network byte counters, summed across all
/// non-loopback interfaces. Reads the kernel's per-interface counter list via
/// `sysctl(NET_RT_IFLIST2)` and walks the returned `if_msghdr2` records.
pub fn get_network_counters() -> Result<NetworkCounters, FfiError> {
    let mut mib: [i32; 6] = [
        raw::CTL_NET,
        raw::PF_ROUTE,
        0,
        0,
        raw::NET_RT_IFLIST2,
        0,
    ];

    // First call with a null buffer asks the kernel for the byte length.
    let mut len: usize = 0;
    let ret = unsafe {
        raw::sysctl(mib.as_mut_ptr(), 6, std::ptr::null_mut(), &mut len, std::ptr::null(), 0)
    };
    if ret != 0 || len == 0 {
        return Err(FfiError::SystemError {
            function: "sysctl(NET_RT_IFLIST2) sizing",
            pid: 0,
            reason: "failed to size interface list".into(),
        });
    }

    let mut buf = vec![0u8; len];
    let ret = unsafe {
        raw::sysctl(
            mib.as_mut_ptr(),
            6,
            buf.as_mut_ptr() as *mut c_void,
            &mut len,
            std::ptr::null(),
            0,
        )
    };
    if ret != 0 {
        return Err(FfiError::SystemError {
            function: "sysctl(NET_RT_IFLIST2)",
            pid: 0,
            reason: "failed to read interface list".into(),
        });
    }

    let mut rx: u64 = 0;
    let mut tx: u64 = 0;
    let hdr_size = mem::size_of::<raw::IfMsghdr2>();
    let mut off = 0usize;
    // Each record starts with `ifm_msglen` (u16) giving its own length; walk by
    // that. Only RTM_IFINFO2 records carry the `if_data64` byte counters.
    while off + 4 <= len {
        let msglen = u16::from_ne_bytes([buf[off], buf[off + 1]]) as usize;
        if msglen == 0 {
            break;
        }
        let mtype = buf[off + 3];
        if mtype == raw::RTM_IFINFO2 && off + hdr_size <= len {
            // The buffer is byte-aligned, so read the header unaligned.
            let hdr = unsafe {
                std::ptr::read_unaligned(buf.as_ptr().add(off) as *const raw::IfMsghdr2)
            };
            if hdr.ifm_flags & raw::IFF_LOOPBACK == 0 {
                rx = rx.wrapping_add(hdr.ifm_data.ifi_ibytes);
                tx = tx.wrapping_add(hdr.ifm_data.ifi_obytes);
            }
        }
        off += msglen;
    }

    Ok(NetworkCounters { rx_bytes: rx, tx_bytes: tx })
}

/// Get cumulative system-wide disk byte counters, summed across every
/// `IOBlockStorageDriver` in the IORegistry (its `Statistics` dictionary's
/// `Bytes (Read)` / `Bytes (Write)` entries).
pub fn get_disk_counters() -> Result<DiskCounters, FfiError> {
    let class = CString::new("IOBlockStorageDriver").unwrap();
    let matching = unsafe { raw::IOServiceMatching(class.as_ptr()) };
    if matching.is_null() {
        return Err(FfiError::SystemError {
            function: "IOServiceMatching",
            pid: 0,
            reason: "returned null".into(),
        });
    }

    // IOServiceGetMatchingServices consumes the `matching` dictionary reference.
    let mut iter: u32 = 0;
    let kr = unsafe {
        raw::IOServiceGetMatchingServices(raw::KIO_MASTER_PORT_DEFAULT, matching, &mut iter)
    };
    if kr != 0 {
        return Err(FfiError::SystemError {
            function: "IOServiceGetMatchingServices",
            pid: 0,
            reason: format!("kern_return_t = {}", kr),
        });
    }

    let stats_key = cfstr("Statistics");
    let read_key = cfstr("Bytes (Read)");
    let write_key = cfstr("Bytes (Write)");

    let mut total_read: u64 = 0;
    let mut total_write: u64 = 0;
    loop {
        let drive = unsafe { raw::IOIteratorNext(iter) };
        if drive == 0 {
            break;
        }
        let props = unsafe {
            raw::IORegistryEntryCreateCFProperty(drive, stats_key, std::ptr::null(), 0)
        };
        if !props.is_null() {
            total_read = total_read.wrapping_add(cf_dict_u64(props, read_key));
            total_write = total_write.wrapping_add(cf_dict_u64(props, write_key));
            unsafe { raw::CFRelease(props) };
        }
        unsafe { raw::IOObjectRelease(drive) };
    }

    unsafe {
        raw::IOObjectRelease(iter);
        cf_release(stats_key);
        cf_release(read_key);
        cf_release(write_key);
    }

    Ok(DiskCounters { read_bytes: total_read, write_bytes: total_write })
}

/// Create a CoreFoundation string from a UTF-8 `&str` (null if creation fails).
fn cfstr(s: &str) -> raw::CfStringRef {
    let Ok(c) = CString::new(s) else {
        return std::ptr::null();
    };
    unsafe {
        raw::CFStringCreateWithCString(
            std::ptr::null(),
            c.as_ptr() as *const c_char,
            raw::KCF_STRING_ENCODING_UTF8,
        )
    }
}

/// Release a CF ref, guarding against the null `cfstr` may return.
fn cf_release(cf: raw::CfTypeRef) {
    if !cf.is_null() {
        unsafe { raw::CFRelease(cf) };
    }
}

/// Read a `u64` out of a CF dictionary by key, or `0` when absent/non-numeric.
fn cf_dict_u64(dict: raw::CfTypeRef, key: raw::CfStringRef) -> u64 {
    if key.is_null() {
        return 0;
    }
    let val = unsafe { raw::CFDictionaryGetValue(dict, key as *const c_void) };
    if val.is_null() {
        return 0;
    }
    let mut out: i64 = 0;
    let ok = unsafe {
        raw::CFNumberGetValue(
            val,
            raw::KCF_NUMBER_SINT64_TYPE,
            &mut out as *mut i64 as *mut c_void,
        )
    };
    if ok != 0 && out > 0 {
        out as u64
    } else {
        0
    }
}

/// A logical CPU's cluster type, for grouping per-core stats. On Apple Silicon
/// each core is a Performance (P) or Efficiency (E) core; hardware without
/// heterogeneous clusters (Intel Macs) reports `Unknown`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreType {
    Performance,
    Efficiency,
    Unknown,
}

/// Read each logical CPU's cluster type once from the IORegistry device tree
/// (`IODeviceTree:/cpus/*`: the `cluster-type` CFData is `"P"`/`"E"`, keyed by
/// the `logical-cpu-id` CFNumber that indexes `get_cpu_ticks`). The topology is
/// static for the machine's lifetime, so callers read this once and cache it.
/// The returned vector is indexed by logical CPU id; a core whose node lacks the
/// property (e.g. Intel Macs, which have no P/E split) reads `Unknown`.
pub fn get_cpu_perf_levels() -> Result<Vec<CoreType>, FfiError> {
    let path = CString::new("IODeviceTree:/cpus").unwrap();
    let cpus = unsafe { raw::IORegistryEntryFromPath(raw::KIO_MASTER_PORT_DEFAULT, path.as_ptr()) };
    if cpus == 0 {
        return Err(FfiError::SystemError {
            function: "IORegistryEntryFromPath(IODeviceTree:/cpus)",
            pid: 0,
            reason: "cpus node not found".into(),
        });
    }

    let plane = CString::new("IODeviceTree").unwrap();
    let mut iter: u32 = 0;
    let kr = unsafe { raw::IORegistryEntryGetChildIterator(cpus, plane.as_ptr(), &mut iter) };
    unsafe { raw::IOObjectRelease(cpus) };
    if kr != 0 {
        return Err(FfiError::SystemError {
            function: "IORegistryEntryGetChildIterator",
            pid: 0,
            reason: format!("kern_return_t = {}", kr),
        });
    }

    let type_key = cfstr("cluster-type");
    let id_key = cfstr("logical-cpu-id");

    // Children can arrive out of id order, so collect (id, type) then index.
    let mut pairs: Vec<(usize, CoreType)> = Vec::new();
    loop {
        let child = unsafe { raw::IOIteratorNext(iter) };
        if child == 0 {
            break;
        }
        let id = cf_prop_u64(child, id_key).map(|v| v as usize);
        let kind = cf_prop_cluster_type(child, type_key);
        unsafe { raw::IOObjectRelease(child) };
        if let Some(id) = id {
            pairs.push((id, kind));
        }
    }

    unsafe {
        raw::IOObjectRelease(iter);
        cf_release(type_key);
        cf_release(id_key);
    }

    if pairs.is_empty() {
        return Err(FfiError::SystemError {
            function: "IODeviceTree:/cpus",
            pid: 0,
            reason: "no cpu nodes carried logical-cpu-id".into(),
        });
    }
    let n = pairs.iter().map(|(i, _)| *i).max().unwrap() + 1;
    let mut out = vec![CoreType::Unknown; n];
    for (i, k) in pairs {
        if i < out.len() {
            out[i] = k;
        }
    }
    Ok(out)
}

/// Read a non-negative integer property off an IORegistry entry, or `None`.
fn cf_prop_u64(entry: u32, key: raw::CfStringRef) -> Option<u64> {
    if key.is_null() {
        return None;
    }
    let val = unsafe { raw::IORegistryEntryCreateCFProperty(entry, key, std::ptr::null(), 0) };
    if val.is_null() {
        return None;
    }
    let mut out: i64 = 0;
    let ok = unsafe {
        raw::CFNumberGetValue(
            val,
            raw::KCF_NUMBER_SINT64_TYPE,
            &mut out as *mut i64 as *mut c_void,
        )
    };
    unsafe { raw::CFRelease(val) };
    (ok != 0 && out >= 0).then_some(out as u64)
}

/// Read an IORegistry entry's `cluster-type` (a `CFData` of `"P"`/`"E"`), or
/// `Unknown` when absent/unrecognized.
fn cf_prop_cluster_type(entry: u32, key: raw::CfStringRef) -> CoreType {
    if key.is_null() {
        return CoreType::Unknown;
    }
    let val = unsafe { raw::IORegistryEntryCreateCFProperty(entry, key, std::ptr::null(), 0) };
    if val.is_null() {
        return CoreType::Unknown;
    }
    let len = unsafe { raw::CFDataGetLength(val) };
    let ptr = unsafe { raw::CFDataGetBytePtr(val) };
    let kind = if len >= 1 && !ptr.is_null() {
        match unsafe { *ptr } {
            b'P' => CoreType::Performance,
            b'E' => CoreType::Efficiency,
            _ => CoreType::Unknown,
        }
    } else {
        CoreType::Unknown
    };
    unsafe { raw::CFRelease(val) };
    kind
}

/// The Unix-epoch second at which the system booted, from
/// `sysctl(KERN_BOOTTIME)` (a `timeval`). Subtract from the current time to get
/// the system uptime. Static-ish (only changes across reboots), so a caller can
/// read it once.
pub fn get_boot_time_secs() -> Result<u64, FfiError> {
    let mib = [raw::CTL_KERN, raw::KERN_BOOTTIME];
    let mut tv = raw::Timeval {
        tv_sec: 0,
        tv_usec: 0,
    };
    let mut len = mem::size_of::<raw::Timeval>();
    let ret = unsafe {
        raw::sysctl(
            mib.as_ptr(),
            2,
            &mut tv as *mut _ as *mut c_void,
            &mut len,
            std::ptr::null(),
            0,
        )
    };
    if ret != 0 || tv.tv_sec <= 0 {
        return Err(FfiError::SystemError {
            function: "sysctl(KERN_BOOTTIME)",
            pid: 0,
            reason: "failed to read boot time".into(),
        });
    }
    Ok(tv.tv_sec as u64)
}

#[cfg(test)]
mod perf_level_tests {
    use super::*;

    #[test]
    fn boot_time_is_in_the_past() {
        let boot = get_boot_time_secs().expect("read boot time");
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after epoch")
            .as_secs();
        assert!(boot > 0 && boot <= now, "boot {boot} should be <= now {now}");
    }

    #[test]
    fn perf_levels_cover_every_core_and_sum_to_the_logical_count() {
        let levels = get_cpu_perf_levels().expect("read cpu perf levels");
        let ticks = get_cpu_ticks().expect("read cpu ticks");
        // One entry per logical CPU, aligned with the tick source's indexing.
        assert_eq!(
            levels.len(),
            ticks.len(),
            "perf-level count must match the logical CPU count"
        );
        // On Apple Silicon every core is P or E (no Unknown); on an Intel Mac
        // they would all be Unknown. Either way the vector is fully populated.
        let classified = levels
            .iter()
            .filter(|l| matches!(l, CoreType::Performance | CoreType::Efficiency))
            .count();
        assert!(
            classified == levels.len() || classified == 0,
            "cores should be uniformly classified or uniformly unknown, got {levels:?}"
        );
    }
}