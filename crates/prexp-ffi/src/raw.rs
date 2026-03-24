//! Raw FFI bindings to macOS libproc.
//!
//! All types here mirror the C definitions in:
//! - `<sys/proc_info.h>` (structs, constants)
//! - `<libproc.h>` (function signatures)

#![allow(non_camel_case_types, dead_code)]

use std::os::raw::{c_char, c_int, c_void};

// ---------------------------------------------------------------------------
// Extern function declarations
// ---------------------------------------------------------------------------

/// Mach timebase info for converting absolute time to nanoseconds.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MachTimebaseInfo {
    pub numer: u32,
    pub denom: u32,
}

extern "C" {
    pub fn mach_timebase_info(info: *mut MachTimebaseInfo) -> i32;
    pub fn mach_task_self() -> u32;
    pub fn mach_host_self() -> u32;
    pub fn task_name_for_pid(target_tport: u32, pid: c_int, tn: *mut u32) -> i32;
    pub fn task_info(
        target_task: u32,
        flavor: i32,
        task_info_out: *mut c_void,
        task_info_count: *mut u32,
    ) -> i32;
    pub fn mach_port_deallocate(task: u32, name: u32) -> i32;
}

/// Flavor for task_info: VM info including phys_footprint.
pub const TASK_VM_INFO: i32 = 22;

/// Partial `task_vm_info` — we only need fields up to phys_footprint.
/// The full struct has many more fields, but task_info returns
/// as many as fit in the provided buffer.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TaskVmInfo {
    pub virtual_size: u64,
    pub region_count: i32,
    pub page_size: i32,
    pub resident_size: u64,
    pub resident_size_peak: u64,
    pub device: u64,
    pub device_peak: u64,
    pub internal: u64,
    pub internal_peak: u64,
    pub external: u64,
    pub external_peak: u64,
    pub reusable: u64,
    pub reusable_peak: u64,
    pub purgeable_volatile_pmap: u64,
    pub purgeable_volatile_resident: u64,
    pub purgeable_volatile_virtual: u64,
    pub compressed: u64,
    pub compressed_peak: u64,
    pub compressed_lifetime: u64,
    pub phys_footprint: u64,
}

#[link(name = "proc", kind = "dylib")]
extern "C" {
    pub fn proc_listallpids(buffer: *mut c_void, buffersize: c_int) -> c_int;

    pub fn proc_pidinfo(
        pid: c_int,
        flavor: c_int,
        arg: u64,
        buffer: *mut c_void,
        buffersize: c_int,
    ) -> c_int;

    pub fn proc_pidfdinfo(
        pid: c_int,
        fd: c_int,
        flavor: c_int,
        buffer: *mut c_void,
        buffersize: c_int,
    ) -> c_int;

    pub fn proc_name(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;

    pub fn proc_pidpath(pid: c_int, buffer: *mut c_void, buffersize: u32) -> c_int;

    pub fn proc_listpidspath(
        type_: u32,
        typeinfo: u32,
        path: *const c_char,
        pathflags: u32,
        buffer: *mut c_void,
        buffersize: c_int,
    ) -> c_int;
}

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Type argument for proc_listallpids / proc_listpidspath.
pub const PROC_ALL_PIDS: u32 = 1;

/// Flavor for proc_pidinfo: list file descriptors.
pub const PROC_PIDLISTFDS: c_int = 1;

/// Flavor for proc_pidfdinfo: vnode path info.
pub const PROC_PIDFDVNODEPATHINFO: c_int = 2;

/// Flavor for proc_pidfdinfo: socket info.
pub const PROC_PIDFDSOCKETINFO: c_int = 3;

/// Flavor for proc_pidfdinfo: pipe info.
pub const PROC_PIDFDPIPEINFO: c_int = 6;

/// Flavor for proc_pidinfo: BSD info (ppid, uid, name).
pub const PROC_PIDTBSDINFO: c_int = 3;

/// Flavor for proc_pidinfo: task info (thread count, memory).
pub const PROC_PIDTASKINFO: c_int = 4;

/// Flavor for proc_pidinfo: vnode path info (CWD + root dir).
pub const PROC_PIDVNODEPATHINFO: c_int = 9;

// sysctl constants for environment variables.
pub const CTL_KERN: c_int = 1;
pub const KERN_PROCARGS2: c_int = 49;

// TCP state constants.
pub const TCPS_CLOSED: i32 = 0;
pub const TCPS_LISTEN: i32 = 1;
pub const TCPS_SYN_SENT: i32 = 2;
pub const TCPS_SYN_RECEIVED: i32 = 3;
pub const TCPS_ESTABLISHED: i32 = 4;
pub const TCPS_CLOSE_WAIT: i32 = 5;
pub const TCPS_FIN_WAIT_1: i32 = 6;
pub const TCPS_CLOSING: i32 = 7;
pub const TCPS_LAST_ACK: i32 = 8;
pub const TCPS_FIN_WAIT_2: i32 = 9;
pub const TCPS_TIME_WAIT: i32 = 10;

// IP version flags for in_sockinfo.
pub const INI_IPV4: u8 = 0x1;
pub const INI_IPV6: u8 = 0x2;

// BSD process states (pbi_status).
pub const SIDL: u32 = 1;
pub const SRUN: u32 = 2;
pub const SSLEEP: u32 = 3;
pub const SSTOP: u32 = 4;
pub const SZOMB: u32 = 5;

// File descriptor type constants (proc_fdinfo.proc_fdtype).
pub const PROX_FDTYPE_ATALK: u32 = 0;
pub const PROX_FDTYPE_VNODE: u32 = 1;
pub const PROX_FDTYPE_SOCKET: u32 = 2;
pub const PROX_FDTYPE_PSHM: u32 = 3;
pub const PROX_FDTYPE_PSEM: u32 = 4;
pub const PROX_FDTYPE_KQUEUE: u32 = 5;
pub const PROX_FDTYPE_PIPE: u32 = 6;
pub const PROX_FDTYPE_FSEVENTS: u32 = 7;
pub const PROX_FDTYPE_NETPOLICY: u32 = 9;
pub const PROX_FDTYPE_CHANNEL: u32 = 10;
pub const PROX_FDTYPE_NEXUS: u32 = 11;

/// Maximum path length (PATH_MAX / MAXPATHLEN on macOS).
pub const MAXPATHLEN: usize = 1024;

/// Maximum process name length.
pub const MAXCOMLEN: usize = 16;

// ---------------------------------------------------------------------------
// #[repr(C)] struct definitions
// ---------------------------------------------------------------------------

/// Per-fd entry returned by proc_pidinfo(PROC_PIDLISTFDS).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProcFdInfo {
    pub proc_fd: i32,
    pub proc_fdtype: u32,
}

/// File info header common to vnode/socket/pipe fd info structs.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProcFileInfo {
    pub fi_openflags: u32,
    pub fi_status: u32,
    pub fi_offset: i64,
    pub fi_type: i32,
    pub fi_guardflags: u32,
}

/// Matches `struct fsid { int32_t val[2]; }`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Fsid {
    pub val: [i32; 2],
}

/// Matches `struct vinfo_stat`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VinfoStat {
    pub vst_dev: u32,
    pub vst_mode: u16,
    pub vst_nlink: u16,
    pub vst_ino: u64,
    pub vst_uid: u32,
    pub vst_gid: u32,
    pub vst_atime: i64,
    pub vst_atimensec: i64,
    pub vst_mtime: i64,
    pub vst_mtimensec: i64,
    pub vst_ctime: i64,
    pub vst_ctimensec: i64,
    pub vst_birthtime: i64,
    pub vst_birthtimensec: i64,
    pub vst_size: i64,
    pub vst_blocks: i64,
    pub vst_blksize: i32,
    pub vst_flags: u32,
    pub vst_gen: u32,
    pub vst_rdev: u32,
    pub vst_qspare: [i64; 2],
}

/// Matches `struct vnode_info`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VnodeInfo {
    pub vi_stat: VinfoStat,
    pub vi_type: i32,
    pub vi_pad: i32,
    pub vi_fsid: Fsid,
}

/// Matches `struct vnode_info_path`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VnodeInfoPath {
    pub vip_vi: VnodeInfo,
    pub vip_path: [u8; MAXPATHLEN],
}

/// Matches `struct vnode_fdinfowithpath`.
/// Returned by proc_pidfdinfo(PROC_PIDFDVNODEPATHINFO).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VnodeFdInfoWithPath {
    pub pfi: ProcFileInfo,
    pub pvip: VnodeInfoPath,
}

/// Matches `struct sockbuf_info`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SockbufInfo {
    pub sbi_cc: u32,
    pub sbi_hiwat: u32,
    pub sbi_mbcnt: u32,
    pub sbi_mbmax: u32,
    pub sbi_lowat: u32,
    pub sbi_flags: i16,
    pub sbi_timeo: i16,
}

/// Matches `struct socket_info`.
///
/// The `soi_proto` union at the end is complex (contains tcp_sockinfo,
/// un_sockinfo, etc.). We represent it as an opaque byte array since we
/// only need the fields before it (soi_family, soi_type, soi_protocol).
/// The union's largest variant is un_sockinfo (~528 bytes with alignment).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SocketInfo {
    pub soi_stat: VinfoStat,
    pub soi_so: u64,
    pub soi_pcb: u64,
    pub soi_type: i32,
    pub soi_protocol: i32,
    pub soi_family: i32,
    pub soi_options: i16,
    pub soi_linger: i16,
    pub soi_state: i16,
    pub soi_qlen: i16,
    pub soi_incqlen: i16,
    pub soi_qlimit: i16,
    pub soi_timeo: i16,
    pub soi_error: u16,
    pub soi_oobmark: u32,
    pub soi_rcv: SockbufInfo,
    pub soi_snd: SockbufInfo,
    pub soi_kind: i32,
    pub soi_rfu_1: u32,
    /// Opaque union: in_sockinfo | tcp_sockinfo | un_sockinfo | ...
    pub soi_proto: [u8; 528],
}

/// Matches `struct socket_fdinfo`.
/// Returned by proc_pidfdinfo(PROC_PIDFDSOCKETINFO).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SocketFdInfo {
    pub pfi: ProcFileInfo,
    pub psi: SocketInfo,
}

/// Matches `struct pipe_info`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PipeInfo {
    pub pipe_stat: VinfoStat,
    pub pipe_handle: u64,
    pub pipe_peerhandle: u64,
    pub pipe_status: i32,
    pub rfu_1: i32,
}

/// Matches `struct pipe_fdinfo`.
/// Returned by proc_pidfdinfo(PROC_PIDFDPIPEINFO).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PipeFdInfo {
    pub pfi: ProcFileInfo,
    pub pipeinfo: PipeInfo,
}

/// Matches `struct proc_bsdinfo`.
/// Returned by proc_pidinfo(PROC_PIDTBSDINFO).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProcBsdInfo {
    pub pbi_flags: u32,
    pub pbi_status: u32,
    pub pbi_xstatus: u32,
    pub pbi_pid: u32,
    pub pbi_ppid: u32,
    pub pbi_uid: u32,
    pub pbi_gid: u32,
    pub pbi_ruid: u32,
    pub pbi_rgid: u32,
    pub pbi_svuid: u32,
    pub pbi_svgid: u32,
    pub rfu_1: u32,
    pub pbi_comm: [u8; MAXCOMLEN],
    pub pbi_name: [u8; 2 * MAXCOMLEN],
    pub pbi_nfiles: u32,
    pub pbi_pgid: u32,
    pub pbi_pjobc: u32,
    pub e_tdev: u32,
    pub e_tpgid: u32,
    pub pbi_nice: i32,
    pub pbi_start_tvsec: u64,
    pub pbi_start_tvusec: u64,
}

/// Matches `struct proc_taskinfo`.
/// Returned by proc_pidinfo(PROC_PIDTASKINFO).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProcTaskInfo {
    pub pti_virtual_size: u64,
    pub pti_resident_size: u64,
    pub pti_total_user: u64,
    pub pti_total_system: u64,
    pub pti_threads_user: u64,
    pub pti_threads_system: u64,
    pub pti_policy: i32,
    pub pti_faults: i32,
    pub pti_pageins: i32,
    pub pti_cow_faults: i32,
    pub pti_messages_sent: i32,
    pub pti_messages_received: i32,
    pub pti_syscalls_mach: i32,
    pub pti_syscalls_unix: i32,
    pub pti_csw: i32,
    pub pti_threadnum: i32,
    pub pti_numrunning: i32,
    pub pti_priority: i32,
}

// ---------------------------------------------------------------------------
// System stats (host-level APIs)
// ---------------------------------------------------------------------------

extern "C" {
    pub fn host_processor_info(
        host: u32,
        flavor: i32,
        out_processor_count: *mut u32,
        out_processor_info: *mut *mut i32,
        out_processor_info_cnt: *mut u32,
    ) -> i32;

    pub fn host_statistics64(
        host_priv: u32,
        flavor: i32,
        host_info_out: *mut c_void,
        host_info_count: *mut u32,
    ) -> i32;

    pub fn vm_deallocate(target_task: u32, address: usize, size: usize) -> i32;

    pub fn sysctlbyname(
        name: *const c_char,
        oldp: *mut c_void,
        oldlenp: *mut usize,
        newp: *const c_void,
        newlen: usize,
    ) -> c_int;
}

/// Flavor for host_processor_info.
pub const PROCESSOR_CPU_LOAD_INFO: i32 = 2;

/// Flavor for host_statistics64.
pub const HOST_VM_INFO64: i32 = 4;

/// CPU state indices into cpu_ticks array.
pub const CPU_STATE_USER: usize = 0;
pub const CPU_STATE_SYSTEM: usize = 1;
pub const CPU_STATE_IDLE: usize = 2;
pub const CPU_STATE_NICE: usize = 3;
pub const CPU_STATE_MAX: usize = 4;

/// Matches `struct vm_statistics64` (partial — fields we need).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct VmStatistics64 {
    pub free_count: u32,
    pub active_count: u32,
    pub inactive_count: u32,
    pub wire_count: u32,
    pub zero_fill_count: u64,
    pub reactivations: u64,
    pub pageins: u64,
    pub pageouts: u64,
    pub faults: u64,
    pub cow_faults: u64,
    pub lookups: u64,
    pub hits: u64,
    pub purges: u64,
    pub purgeable_count: u32,
    pub speculative_count: u32,
    pub decompressions: u64,
    pub compressions: u64,
    pub swapins: u64,
    pub swapouts: u64,
    pub compressor_page_count: u32,
    pub throttled_count: u32,
    pub external_page_count: u32,
    pub internal_page_count: u32,
    pub total_uncompressed_pages_in_compressor: u64,
    pub swapped_count: u64,
}

/// Matches `struct proc_vnodepathinfo`.
/// Returned by proc_pidinfo(PROC_PIDVNODEPATHINFO).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProcVnodePathInfo {
    pub pvi_cdir: VnodeInfoPath,
    pub pvi_rdir: VnodeInfoPath,
}

/// Matches `struct in4in6_addr`.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct In4In6Addr {
    pub i46a_pad32: [u32; 3],
    pub i46a_addr4: [u8; 4], // struct in_addr = u32, but we read as bytes
}

/// Matches `struct in_sockinfo` (partial — fields we need for IP:port).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InSockInfo {
    pub insi_fport: i32,
    pub insi_lport: i32,
    pub insi_gencnt: u64,
    pub insi_flags: u32,
    pub insi_flow: u32,
    pub insi_vflag: u8,
    pub insi_ip_ttl: u8,
    pub rfu_1: u16, // padding
    pub _rfu_pad: u16, // more padding to align to u32
    pub insi_faddr: In4In6Addr,
    pub insi_laddr: In4In6Addr,
}

/// Matches `struct tcp_sockinfo` (partial).
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TcpSockInfo {
    pub tcpsi_ini: InSockInfo,
    pub tcpsi_state: i32,
}

extern "C" {
    pub fn sysctl(
        name: *const c_int,
        namelen: u32,
        oldp: *mut c_void,
        oldlenp: *mut usize,
        newp: *const c_void,
        newlen: usize,
    ) -> c_int;
}
