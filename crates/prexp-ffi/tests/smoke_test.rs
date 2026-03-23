use prexp_ffi::raw;

#[test]
fn struct_sizes_are_reasonable() {
    // ProcFdInfo: i32 + u32 = 8 bytes
    assert_eq!(std::mem::size_of::<raw::ProcFdInfo>(), 8);

    // ProcFileInfo: u32 + u32 + i64 + i32 + u32 = 24 bytes
    assert_eq!(std::mem::size_of::<raw::ProcFileInfo>(), 24);

    // VnodeFdInfoWithPath should be large enough to hold the path buffer
    assert!(std::mem::size_of::<raw::VnodeFdInfoWithPath>() > 1024);

    // PipeFdInfo should be at least ProcFileInfo + PipeInfo
    assert!(std::mem::size_of::<raw::PipeFdInfo>() > std::mem::size_of::<raw::ProcFileInfo>());

    // ProcBsdInfo and ProcTaskInfo should be non-trivial
    assert!(std::mem::size_of::<raw::ProcBsdInfo>() > 100);
    assert!(std::mem::size_of::<raw::ProcTaskInfo>() > 50);
}

#[test]
#[ignore] // Requires running on macOS — run with: cargo test -- --ignored
fn list_all_pids_returns_processes() {
    let pids = prexp_ffi::list_all_pids().expect("failed to list pids");
    assert!(!pids.is_empty(), "should find at least one process");
    // Our own process should be in the list
    let my_pid = std::process::id() as i32;
    assert!(
        pids.contains(&my_pid),
        "our own pid {} should be in the list",
        my_pid
    );
}

#[test]
#[ignore]
fn get_process_name_for_self() {
    let my_pid = std::process::id() as i32;
    let name = prexp_ffi::get_process_name(my_pid).expect("failed to get process name");
    assert!(!name.is_empty(), "process name should not be empty");
}

#[test]
#[ignore]
fn list_fds_for_self() {
    let my_pid = std::process::id() as i32;
    let fds = prexp_ffi::list_fds(my_pid).expect("failed to list fds");
    // Every process has at least stdin/stdout/stderr (fds 0, 1, 2)
    assert!(fds.len() >= 3, "should have at least 3 fds, got {}", fds.len());
}

#[test]
#[ignore]
fn resolve_fd_for_stdout() {
    let my_pid = std::process::id() as i32;
    let fds = prexp_ffi::list_fds(my_pid).expect("failed to list fds");

    // Find fd 1 (stdout) and resolve it
    let stdout_fd = fds.iter().find(|f| f.fd == 1);
    assert!(stdout_fd.is_some(), "should have fd 1 (stdout)");

    let fd = stdout_fd.unwrap();
    let detail = prexp_ffi::resolve_fd(my_pid, fd.fd, fd.fdtype);
    assert!(detail.is_ok(), "should resolve fd 1: {:?}", detail);
}

#[test]
#[ignore]
fn get_process_info_returns_ppid_and_threads() {
    let my_pid = std::process::id() as i32;
    let info = prexp_ffi::get_process_info(my_pid).expect("failed to get process info");

    assert!(info.ppid > 0, "should have a parent pid, got {}", info.ppid);
    assert!(
        info.thread_count >= 1,
        "should have at least 1 thread, got {}",
        info.thread_count
    );
    assert!(!info.name.is_empty(), "should have a name");
}
