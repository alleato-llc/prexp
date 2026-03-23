//! prexp-ffi: Raw FFI bindings and safe wrappers for macOS libproc.
//!
//! This crate is only compiled on macOS. All unsafe code is contained here;
//! downstream crates call only safe Rust APIs.

#[cfg(target_os = "macos")]
pub mod raw;
#[cfg(target_os = "macos")]
mod safe;

#[cfg(target_os = "macos")]
pub use safe::*;
