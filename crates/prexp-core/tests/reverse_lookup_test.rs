mod support;

use prexp_core::models::{OpenResource, ProcessSnapshot, ResourceKind};
use prexp_core::source::ProcessSource;
use support::fake_source::FakeProcessSource;

fn resource(fd: i32, kind: ResourceKind, path: Option<&str>) -> OpenResource {
    OpenResource {
        descriptor: fd,
        kind,
        path: path.map(String::from),
    }
}

fn create_source_with_overlapping_paths() -> FakeProcessSource {
    FakeProcessSource::new(vec![
        ProcessSnapshot {
            pid: 100,
            ppid: 1,
            name: "nginx".into(),
            thread_count: 8,
            memory_rss: 1024 * 1024 * 50, memory_phys: 1024 * 1024 * 30, cpu_time_ns: 1_000_000_000, accessible: true,
            resources: vec![
                resource(3, ResourceKind::File, Some("/var/log/nginx/access.log")),
                resource(4, ResourceKind::File, Some("/var/log/nginx/error.log")),
                resource(5, ResourceKind::Socket, None),
            ],
        },
        ProcessSnapshot {
            pid: 200,
            ppid: 1,
            name: "logrotate".into(),
            thread_count: 1,
            memory_rss: 1024 * 1024 * 50, memory_phys: 1024 * 1024 * 30, cpu_time_ns: 1_000_000_000, accessible: true,
            resources: vec![
                resource(3, ResourceKind::File, Some("/var/log/nginx/access.log")),
            ],
        },
        ProcessSnapshot {
            pid: 300,
            ppid: 1,
            name: "redis".into(),
            thread_count: 4,
            memory_rss: 1024 * 1024 * 50, memory_phys: 1024 * 1024 * 30, cpu_time_ns: 1_000_000_000, accessible: true,
            resources: vec![
                resource(3, ResourceKind::File, Some("/var/lib/redis/dump.rdb")),
                resource(4, ResourceKind::Socket, None),
            ],
        },
    ])
}

#[test]
fn find_by_path_returns_matching_processes() {
    let source = create_source_with_overlapping_paths();

    let results = source.find_by_path("/var/log/nginx/access.log").unwrap();

    assert_eq!(results.len(), 2);
    let pids: Vec<i32> = results.iter().map(|s| s.pid).collect();
    assert!(pids.contains(&100));
    assert!(pids.contains(&200));
}

#[test]
fn find_by_path_excludes_non_matching_processes() {
    let source = create_source_with_overlapping_paths();

    let results = source.find_by_path("/var/log/nginx/access.log").unwrap();

    let pids: Vec<i32> = results.iter().map(|s| s.pid).collect();
    assert!(!pids.contains(&300), "redis should not match access.log");
}

#[test]
fn find_by_path_returns_empty_for_unknown_path() {
    let source = create_source_with_overlapping_paths();

    let results = source.find_by_path("/nonexistent/path").unwrap();

    assert!(results.is_empty());
}

#[test]
fn find_by_path_does_not_match_sockets() {
    let source = create_source_with_overlapping_paths();

    // Sockets have path: None, so no path should match them
    let results = source.find_by_path("").unwrap();
    assert!(results.is_empty());
}

#[test]
fn find_by_path_with_unique_path() {
    let source = create_source_with_overlapping_paths();

    let results = source.find_by_path("/var/lib/redis/dump.rdb").unwrap();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].pid, 300);
    assert_eq!(results[0].name, "redis");
}

#[test]
fn snapshot_pid_returns_matching_process() {
    let source = create_source_with_overlapping_paths();

    let snap = source.snapshot_pid(100).unwrap();

    assert_eq!(snap.pid, 100);
    assert_eq!(snap.name, "nginx");
    assert_eq!(snap.resources.len(), 3);
}

#[test]
fn snapshot_pid_returns_error_for_unknown_pid() {
    let source = create_source_with_overlapping_paths();

    let result = source.snapshot_pid(999);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        prexp_core::error::PrexpError::ProcessNotFound { pid: 999 }
    ));
}
