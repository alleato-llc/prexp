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
    assert!(
        info.state == prexp_ffi::ProcessState::Running || info.state == prexp_ffi::ProcessState::Sleeping,
        "test process should be running or sleeping, got {:?}",
        info.state
    );
    assert!(info.start_time > 0, "should have a start time");
}

#[test]
#[ignore]
fn get_process_path_for_self() {
    let my_pid = std::process::id() as i32;
    let path = prexp_ffi::get_process_path(my_pid).expect("failed to get process path");
    assert!(!path.is_empty(), "path should not be empty");
    // Should be an absolute path
    assert!(path.starts_with('/'), "path should be absolute: {}", path);
}

#[test]
#[ignore]
fn get_process_cwd_for_self() {
    let my_pid = std::process::id() as i32;
    let cwd = prexp_ffi::get_process_cwd(my_pid).expect("failed to get process cwd");
    assert!(!cwd.is_empty(), "cwd should not be empty");
    assert!(cwd.starts_with('/'), "cwd should be absolute: {}", cwd);
}

#[test]
#[ignore]
fn get_process_env_for_self() {
    let my_pid = std::process::id() as i32;
    let env = prexp_ffi::get_process_env(my_pid).expect("failed to get process env");
    // Should have at least PATH and HOME
    assert!(!env.is_empty(), "should have environment variables");
    assert!(
        env.iter().any(|(k, _)| k == "PATH"),
        "should have PATH env var"
    );
}

#[test]
#[ignore]
fn get_process_detail_for_self() {
    let my_pid = std::process::id() as i32;
    let detail = prexp_ffi::get_process_detail(my_pid, "test_parent")
        .expect("failed to get process detail");

    assert_eq!(detail.pid, my_pid);
    assert!(!detail.name.is_empty());
    assert!(!detail.path.is_empty());
    assert!(detail.path.starts_with('/'));
    assert!(!detail.cwd.is_empty());
    assert!(detail.thread_count >= 1);
    assert!(detail.fd_total >= 3); // stdin/stdout/stderr
    assert!(!detail.environment.is_empty());
    assert_eq!(detail.parent_name, "test_parent");
}

#[test]
#[ignore]
fn get_network_connections_for_self() {
    let my_pid = std::process::id() as i32;
    // May or may not have connections, but should not panic
    let conns = prexp_ffi::get_network_connections(my_pid);
    // Just verify it returns without error — test process may have no sockets
    let _ = conns;
}

#[test]
#[ignore]
fn syscall_counter_accuracy() {
    use std::process::{Command, Stdio};
    use std::io::{BufRead, BufReader};

    // Spawn a child that does a known number of getpid() syscalls then reports.
    let mut child = Command::new("python3")
        .arg("-c")
        .arg(r#"
import os, sys
# Do 50,000 stat() syscalls (each is a real unix syscall, not cached)
for _ in range(50000):
    os.stat(".")
sys.stdout.write("done\n")
sys.stdout.flush()
import time
time.sleep(5)
"#)
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn python3");

    let pid = child.id() as i32;

    // Wait for the child to finish its syscall burst.
    let stdout = child.stdout.take().unwrap();
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).expect("failed to read from child");
    assert_eq!(line.trim(), "done");

    // Read syscall counters.
    let info = prexp_ffi::get_process_info(pid).expect("failed to get process info");
    let total_syscalls = info.syscalls_mach as i64 + info.syscalls_unix as i64;

    // The child did at least 50,000 stat() calls plus Python startup overhead.
    // Unix syscalls should be well above 50,000.
    assert!(
        info.syscalls_unix > 50_000,
        "expected >50K unix syscalls, got {} (mach: {}, unix: {}, total: {})",
        info.syscalls_unix, info.syscalls_mach, info.syscalls_unix, total_syscalls
    );

    // Sanity: total should be positive and reasonable.
    assert!(total_syscalls > 50_000, "total syscalls should be >50K, got {}", total_syscalls);
    assert!(total_syscalls < 100_000_000, "total syscalls should be <100M, got {}", total_syscalls);

    child.kill().ok();
    child.wait().ok();
}
