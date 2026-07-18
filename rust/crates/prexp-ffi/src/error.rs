//! Error types and helpers for FFI calls.

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

/// Check errno after a failed FFI call and return an appropriate error.
pub(crate) fn check_errno(function: &'static str, pid: i32) -> FfiError {
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

/// BSD errno constants (avoiding a libc dependency).
mod libc_constants {
    pub const EPERM: i32 = 1;
    pub const ESRCH: i32 = 3;
    pub const EACCES: i32 = 13;
}

/// Convert Mach absolute time ticks to nanoseconds.
///
/// On Intel, the ratio is 1:1. On Apple Silicon, the ratio is typically 125:3
/// (~41.67x). We query `mach_timebase_info()` once and cache the result.
pub(crate) fn mach_ticks_to_ns(ticks: u64) -> u64 {
    use std::sync::OnceLock;
    static TIMEBASE: OnceLock<(u32, u32)> = OnceLock::new();

    let &(numer, denom) = TIMEBASE.get_or_init(|| {
        let mut info = crate::raw::MachTimebaseInfo { numer: 0, denom: 0 };
        unsafe { crate::raw::mach_timebase_info(&mut info) };
        (info.numer, info.denom)
    });

    ((ticks as u128 * numer as u128) / denom as u128) as u64
}

/// Extract a null-terminated C string from a byte slice.
pub(crate) fn extract_c_string(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}
