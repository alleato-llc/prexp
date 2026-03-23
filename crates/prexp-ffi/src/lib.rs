//! prexp-ffi: Raw FFI bindings and safe wrappers for macOS libproc.
//!
//! Modules:
//! - `raw` — #[repr(C)] structs, extern functions, constants
//! - `error` — FfiError type and helpers
//! - `process` — process-level APIs (list_all_pids, get_process_info, list_fds, resolve_fd)
//! - `system` — system-level APIs (get_cpu_ticks, get_memory_info)

#[cfg(target_os = "macos")]
pub mod raw;
#[cfg(target_os = "macos")]
mod error;
#[cfg(target_os = "macos")]
mod process;
#[cfg(target_os = "macos")]
mod system;

#[cfg(target_os = "macos")]
pub use error::FfiError;
#[cfg(target_os = "macos")]
pub use process::*;
#[cfg(target_os = "macos")]
pub use system::*;
