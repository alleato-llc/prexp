use prexp_core::models::{OpenResource, ProcessSnapshot, ResourceKind};
use prexp_core::output;
use prexp_core::output::OutputFormat;

fn sample_snapshots() -> Vec<ProcessSnapshot> {
    vec![
        ProcessSnapshot {
            pid: 1234,
            ppid: 1,
            name: "node".into(),
            thread_count: 4,
            memory_rss: 1024 * 1024 * 50, memory_phys: 1024 * 1024 * 30, cpu_time_ns: 1_000_000_000, state: prexp_ffi::ProcessState::Running, accessible: true,
            resources: vec![
                OpenResource {
                    descriptor: 3,
                    kind: ResourceKind::File,
                    path: Some("/Users/javier/app/server.js".into()),
                },
                OpenResource {
                    descriptor: 5,
                    kind: ResourceKind::Socket,
                    path: None,
                },
                OpenResource {
                    descriptor: 6,
                    kind: ResourceKind::Pipe,
                    path: None,
                },
            ],
        },
        ProcessSnapshot {
            pid: 5678,
            ppid: 1,
            name: "cargo".into(),
            thread_count: 2,
            memory_rss: 1024 * 1024 * 50, memory_phys: 1024 * 1024 * 30, cpu_time_ns: 1_000_000_000, state: prexp_ffi::ProcessState::Running, accessible: true,
            resources: vec![OpenResource {
                descriptor: 4,
                kind: ResourceKind::File,
                path: Some("/Users/javier/project/Cargo.toml".into()),
            }],
        },
    ]
}

#[test]
fn json_grouped_format_contains_all_processes() {
    let snapshots = sample_snapshots();
    let mut buf = Vec::new();

    output::format_snapshots(&snapshots, OutputFormat::Json, &mut buf).unwrap();

    let output = String::from_utf8(buf).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let arr = parsed.as_array().unwrap();

    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["pid"], 1234);
    assert_eq!(arr[0]["name"], "node");
    assert_eq!(arr[0]["resources"].as_array().unwrap().len(), 3);
    assert_eq!(arr[1]["pid"], 5678);
    assert_eq!(arr[1]["name"], "cargo");
}

#[test]
fn json_grouped_format_null_path_for_sockets() {
    let snapshots = sample_snapshots();
    let mut buf = Vec::new();

    output::format_snapshots(&snapshots, OutputFormat::Json, &mut buf).unwrap();

    let output = String::from_utf8(buf).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let socket_resource = &parsed[0]["resources"][1];

    assert_eq!(socket_resource["kind"], "socket");
    assert!(socket_resource["path"].is_null());
}

#[test]
fn tsv_format_has_header_and_rows() {
    let snapshots = sample_snapshots();
    let mut buf = Vec::new();

    output::format_snapshots(&snapshots, OutputFormat::Tsv, &mut buf).unwrap();

    let output = String::from_utf8(buf).unwrap();
    let lines: Vec<&str> = output.lines().collect();

    // Header + 4 data rows
    let expected_rows: usize = snapshots.iter().map(|s| s.resources.len()).sum();
    assert_eq!(lines.len(), 1 + expected_rows);

    assert_eq!(lines[0], "PID\tPROCESS\tDESCRIPTOR\tKIND\tPATH");
}

#[test]
fn tsv_uses_dash_for_null_paths() {
    let snapshots = sample_snapshots();
    let mut buf = Vec::new();

    output::format_snapshots(&snapshots, OutputFormat::Tsv, &mut buf).unwrap();

    let output = String::from_utf8(buf).unwrap();
    // The socket line (fd 5) should have "-" for path
    let socket_line = output.lines().find(|l| l.contains("\tsocket\t")).unwrap();
    assert!(socket_line.ends_with("\t-"), "socket line should end with tab-dash: {}", socket_line);
}

#[test]
fn tsv_columns_are_tab_separated() {
    let snapshots = sample_snapshots();
    let mut buf = Vec::new();

    output::format_snapshots(&snapshots, OutputFormat::Tsv, &mut buf).unwrap();

    let output = String::from_utf8(buf).unwrap();
    for line in output.lines() {
        let columns: Vec<&str> = line.split('\t').collect();
        assert_eq!(columns.len(), 5, "each line should have 5 tab-separated columns: {}", line);
    }
}

#[test]
fn json_grouped_empty_input() {
    let mut buf = Vec::new();
    output::format_snapshots(&[], OutputFormat::Json, &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    assert_eq!(output.trim(), "[]");
}

#[test]
fn tsv_empty_input_has_only_header() {
    let mut buf = Vec::new();
    output::format_snapshots(&[], OutputFormat::Tsv, &mut buf).unwrap();
    let output = String::from_utf8(buf).unwrap();
    let lines: Vec<&str> = output.lines().collect();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], "PID\tPROCESS\tDESCRIPTOR\tKIND\tPATH");
}
