use std::ffi::CStr;
use std::mem;
use std::os::raw::{c_char, c_int};

/// Resolve a UID to a username. Falls back to the numeric UID string.
pub fn get_username(uid: u32) -> String {
    #[cfg(target_os = "macos")]
    {
        #[repr(C)]
        struct Passwd {
            pw_name: *mut c_char,
            pw_passwd: *mut c_char,
            pw_uid: u32,
            pw_gid: u32,
            pw_change: i64,
            pw_class: *mut c_char,
            pw_gecos: *mut c_char,
            pw_dir: *mut c_char,
            pw_shell: *mut c_char,
            pw_expire: i64,
            pw_fields: c_int,
        }

        extern "C" {
            fn getpwuid_r(
                uid: u32,
                pwd: *mut Passwd,
                buf: *mut c_char,
                buflen: usize,
                result: *mut *mut Passwd,
            ) -> c_int;
        }

        let mut pwd: Passwd = unsafe { mem::zeroed() };
        let mut result: *mut Passwd = std::ptr::null_mut();
        let mut buf = vec![0u8; 1024];

        let ret = unsafe {
            getpwuid_r(
                uid,
                &mut pwd,
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
                &mut result,
            )
        };

        if ret == 0 && !result.is_null() {
            let name = unsafe { CStr::from_ptr(pwd.pw_name) };
            name.to_string_lossy().into_owned()
        } else {
            format!("{}", uid)
        }
    }

    #[cfg(target_os = "linux")]
    {
        #[repr(C)]
        struct Passwd {
            pw_name: *mut c_char,
            pw_passwd: *mut c_char,
            pw_uid: u32,
            pw_gid: u32,
            pw_gecos: *mut c_char,
            pw_dir: *mut c_char,
            pw_shell: *mut c_char,
        }

        extern "C" {
            fn getpwuid_r(
                uid: u32,
                pwd: *mut Passwd,
                buf: *mut c_char,
                buflen: usize,
                result: *mut *mut Passwd,
            ) -> c_int;
        }

        let mut pwd: Passwd = unsafe { mem::zeroed() };
        let mut result: *mut Passwd = std::ptr::null_mut();
        let mut buf = vec![0u8; 1024];

        let ret = unsafe {
            getpwuid_r(
                uid,
                &mut pwd,
                buf.as_mut_ptr() as *mut c_char,
                buf.len(),
                &mut result,
            )
        };

        if ret == 0 && !result.is_null() {
            let name = unsafe { CStr::from_ptr(pwd.pw_name) };
            name.to_string_lossy().into_owned()
        } else {
            format!("{}", uid)
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        format!("{}", uid)
    }
}
