//! Headless view snapshots (dev/test only) — prexp-desktop's counterpart to the
//! tty project's `src/snapshot.rs` (see tty's ADR 0005).
//!
//! Renders the process-table chrome to a PNG under `crates/prexp-desktop/snapshots/`
//! via `iced_test::Simulator` — no GPU, window, display, or accessibility grant
//! required. The state is canned (a fake [`ProcessSource`]), so the pixels are
//! deterministic: CPU% is 0.0 on the first refresh (no prior baseline to diff),
//! and every other value is hard-coded here.
//!
//! These are `#[ignore]`d so a plain `cargo test` stays green; opt in with:
//!
//! ```sh
//! cargo test -p prexp-desktop -- --ignored snapshot
//! ```
//!
//! **Backend:** these render on **wgpu** (the default), not `tiny-skia`. The
//! stats header's CPU chart is an `iced` canvas, and the headless `tiny-skia`
//! backend doesn't rasterize canvas strokes — the line (and axes) come out
//! blank and mispositioned. This mirrors the `tty` project, which baselines its
//! chart views on wgpu only. wgpu baselines are machine-local (GPU/driver
//! specific), which is fine here since these tests are `#[ignore]`d and not a CI
//! byte-match gate. The baseline file name gets the backend suffix automatically
//! (`…-wgpu.png`); delete a PNG to re-baseline it.

use std::sync::Arc;
use std::time::Duration;

use prexp_core::error::PrexpError;
use prexp_core::models::{
    DiskIo, NetworkConnection, OpenResource, ProcessActivity, ProcessDetail, ProcessMemory,
    ProcessSnapshot, ProcessState, ResourceKind,
};
use prexp_core::source::ProcessSource;
use prexp_core::system::{
    BatteryInfo, CpuKind, CpuTicks, DiskCounters, MemoryInfo, NetworkCounters,
};

use crate::app::{App, Message, Sample};

/// A `ProcessSource` that hands back a canned process list. The GUI reads its
/// table data from state populated via [`Message::Refreshed`], so none of the
/// other trait methods are exercised by a view render — they return empty/errors.
struct FakeSource;

impl ProcessSource for FakeSource {
    fn snapshot_all(&self) -> Result<Vec<ProcessSnapshot>, PrexpError> {
        Ok(sample_processes())
    }
    fn snapshot_pid(&self, pid: i32) -> Result<ProcessSnapshot, PrexpError> {
        sample_processes()
            .into_iter()
            .find(|p| p.pid == pid)
            .ok_or_else(|| PrexpError::Backend("no such pid".into()))
    }
    fn find_by_path(&self, _path: &str) -> Result<Vec<ProcessSnapshot>, PrexpError> {
        Ok(Vec::new())
    }
    fn cpu_ticks(&self) -> Result<Vec<CpuTicks>, PrexpError> {
        Ok(Vec::new())
    }
    fn memory_info(&self) -> Result<MemoryInfo, PrexpError> {
        Ok(MemoryInfo {
            total: 0,
            used: 0,
            free: 0,
            wired: 0,
            compressed: 0,
            swap_total: 0,
            swap_used: 0,
        })
    }
    fn network_counters(&self) -> Result<NetworkCounters, PrexpError> {
        Ok(NetworkCounters {
            rx_bytes: 0,
            tx_bytes: 0,
        })
    }
    fn disk_counters(&self) -> Result<DiskCounters, PrexpError> {
        Ok(DiskCounters {
            read_bytes: 0,
            write_bytes: 0,
        })
    }
    fn process_detail(&self, _pid: i32, _parent: &str) -> Result<ProcessDetail, PrexpError> {
        Ok(sample_detail())
    }
    // Override the default (which calls real `kill(2)`) so snapshots never signal
    // a live process.
    fn kill_process(&self, _pid: i32, _sig: i32) -> Result<(), PrexpError> {
        Ok(())
    }
    fn cpu_perf_levels(&self) -> Result<Vec<CpuKind>, PrexpError> {
        // Four Performance + four Efficiency cores, so the header shows P/E chips.
        Ok(vec![
            CpuKind::Performance,
            CpuKind::Performance,
            CpuKind::Performance,
            CpuKind::Performance,
            CpuKind::Efficiency,
            CpuKind::Efficiency,
            CpuKind::Efficiency,
            CpuKind::Efficiency,
        ])
    }
}

/// Cumulative per-core CPU ticks across a handful of frames. Each core's usage in
/// frame `f` is exactly its `pattern` percentage (busy ticks / quantum), so the
/// per-core chips, aggregate, and history curve are all deterministic. P cores
/// (0–3) run hotter than E cores (4–7).
fn cpu_frames() -> Vec<Vec<CpuTicks>> {
    const QUANTUM: u64 = 10_000;
    let pattern: [[u64; 6]; 8] = [
        [30, 45, 60, 72, 65, 58],
        [20, 35, 55, 68, 60, 50],
        [40, 50, 62, 70, 66, 60],
        [15, 28, 44, 58, 52, 47],
        [8, 12, 20, 26, 22, 18],
        [5, 9, 14, 19, 16, 12],
        [10, 14, 22, 30, 25, 20],
        [6, 10, 16, 21, 18, 14],
    ];

    let mut cumulative = vec![
        CpuTicks {
            user: 0,
            system: 0,
            idle: 0,
            nice: 0,
        };
        pattern.len()
    ];
    let mut frames = Vec::new();
    for f in 0..pattern[0].len() {
        for (c, core) in pattern.iter().enumerate() {
            let busy = QUANTUM * core[f] / 100;
            cumulative[c].user += busy;
            cumulative[c].idle += QUANTUM - busy;
        }
        frames.push(cumulative.clone());
    }
    frames
}

/// Build one canned snapshot with `files`/`socks`/`pipes` open resources of each
/// kind, so the FILES/SOCKS/PIPES/TOTAL columns show real counts.
#[allow(clippy::too_many_arguments)]
fn snap(
    pid: i32,
    ppid: i32,
    name: &str,
    accessible: bool,
    rss: u64,
    phys: u64,
    threads: i32,
    files: usize,
    socks: usize,
    pipes: usize,
) -> ProcessSnapshot {
    let mut resources = Vec::new();
    let mut fd = 3;
    for i in 0..files {
        resources.push(OpenResource {
            descriptor: fd,
            kind: ResourceKind::File,
            path: Some(format!("/usr/lib/lib{name}{i}.dylib")),
        });
        fd += 1;
    }
    for _ in 0..socks {
        resources.push(OpenResource {
            descriptor: fd,
            kind: ResourceKind::Socket,
            path: None,
        });
        fd += 1;
    }
    for _ in 0..pipes {
        resources.push(OpenResource {
            descriptor: fd,
            kind: ResourceKind::Pipe,
            path: None,
        });
        fd += 1;
    }
    ProcessSnapshot {
        pid,
        ppid,
        name: name.into(),
        state: ProcessState::Running,
        accessible,
        memory: ProcessMemory { rss, phys },
        activity: ProcessActivity {
            cpu_time_ns: 0,
            thread_count: threads,
            faults: 0,
            context_switches: 0,
            syscalls_mach: 0,
            syscalls_unix: 0,
        },
        disk_io: DiskIo {
            bytes_read: 0,
            bytes_written: 0,
        },
        resources,
    }
}

/// A fixed, varied process list: a big browser, a couple of dev tools, a daemon,
/// and one inaccessible (permission-denied) process to show the dimmed row.
fn sample_processes() -> Vec<ProcessSnapshot> {
    vec![
        snap(512, 1, "Safari", true, 1_288_490_188, 1_073_741_824, 48, 96, 24, 6),
        snap(1187, 512, "com.apple.WebKit.WebContent", true, 734_003_200, 524_288_000, 22, 40, 12, 4),
        snap(88, 1, "node", true, 220_200_960, 178_257_920, 12, 34, 18, 8),
        snap(342, 1, "rustc", true, 512_000_000, 402_653_184, 8, 120, 2, 3),
        snap(76, 1, "WindowServer", false, 314_572_800, 268_435_456, 16, 0, 0, 0),
        snap(1, 0, "launchd", true, 12_582_912, 8_388_608, 4, 8, 6, 2),
        snap(923, 88, "esbuild", true, 96_468_992, 67_108_864, 6, 12, 4, 2),
        snap(450, 1, "Terminal", true, 141_557_760, 104_857_600, 10, 22, 8, 5),
    ]
}

/// A canned `ProcessDetail` for the info-panel snapshots — node (pid 88) with
/// network connections and environment variables so every tab has content.
fn sample_detail() -> ProcessDetail {
    ProcessDetail {
        pid: 88,
        ppid: 1,
        parent_name: "launchd".to_string(),
        name: "node".to_string(),
        path: "/usr/local/bin/node".to_string(),
        cwd: "/Users/dev/projects/prexp-desktop".to_string(),
        user: "dev".to_string(),
        uid: 501,
        state: ProcessState::Running,
        nice: 0,
        started_secs: 1_700_000_000, // 2023-11-14 22:13:20 UTC (deterministic)
        thread_count: 12,
        virtual_size: 420_000_000_000,
        memory_rss: 220_200_960,
        memory_phys: 178_257_920,
        cpu_time_ns: 94_500_000_000,
        fd_files: 34,
        fd_sockets: 18,
        fd_pipes: 8,
        fd_other: 0,
        fd_total: 60,
        faults: 48213,
        context_switches: 90124,
        syscalls_mach: 12045,
        syscalls_unix: 88765,
        disk_bytes_read: 134_217_728,
        disk_bytes_written: 67_108_864,
        network: vec![
            NetworkConnection {
                proto: "tcp4".to_string(),
                local_addr: "127.0.0.1:5173".to_string(),
                remote_addr: None,
                state: Some("LISTEN".to_string()),
            },
            NetworkConnection {
                proto: "tcp4".to_string(),
                local_addr: "192.168.1.42:52233".to_string(),
                remote_addr: Some("140.82.113.25:443".to_string()),
                state: Some("ESTABLISHED".to_string()),
            },
            NetworkConnection {
                proto: "tcp6".to_string(),
                local_addr: "[::1]:6006".to_string(),
                remote_addr: None,
                state: Some("LISTEN".to_string()),
            },
        ],
        environment: vec![
            ("PATH".to_string(), "/usr/local/bin:/usr/bin:/bin".to_string()),
            ("HOME".to_string(), "/Users/dev".to_string()),
            ("NODE_ENV".to_string(), "development".to_string()),
            ("SHELL".to_string(), "/bin/zsh".to_string()),
            ("TERM".to_string(), "xterm-256color".to_string()),
            ("LANG".to_string(), "en_US.UTF-8".to_string()),
            ("PORT".to_string(), "5173".to_string()),
        ],
    }
}

/// An `App` pre-loaded with the canned processes and a seeded stats header (via
/// the real update path — several tick frames so per-core %, the aggregate, and
/// the history curve all populate).
fn sample_app() -> App {
    let (mut app, _boot_task) = App::boot(Arc::new(FakeSource), Duration::from_secs(2));
    let memory = MemoryInfo {
        total: 17_179_869_184,       // 16 GiB
        used: 11_274_289_152,        // ~10.5 GiB
        free: 5_905_580_032,
        wired: 3_221_225_472,
        compressed: 1_610_612_736,
        swap_total: 2_147_483_648,
        swap_used: 536_870_912,
    };
    // Drop the boot Task unpolled; drive state through real messages.
    for cpu_ticks in cpu_frames() {
        let _ = app.update(Message::Refreshed(Ok(Sample {
            processes: sample_processes(),
            cpu_ticks,
            memory: Some(memory.clone()),
            load: Some([2.34, 1.98, 1.62]),
            battery: Some(BatteryInfo {
                percent: 82.0,
                charging: true,
                time_to_empty_min: -1,
                time_to_full_min: 25,
            }),
        })));
    }
    app
}

#[test]
#[ignore = "renders pixels on wgpu; run with: cargo test -p prexp-desktop -- --ignored snapshot"]
fn snapshot_processes_dark() {
    let mut app = sample_app();
    // Select a row so the highlight is exercised in the baseline.
    let _ = app.update(Message::SelectRow(2));

    std::fs::create_dir_all("snapshots").expect("create snapshots dir");
    let mut sim = iced_test::Simulator::with_size(
        iced::Settings::default(),
        iced::Size::new(1040.0, 720.0),
        crate::view::view(&app),
    );
    let snapshot = sim.snapshot(&app.theme()).expect("render snapshot");
    let matches = snapshot
        .matches_image("snapshots/prexp-desktop-processes-dark.png")
        .expect("write/compare snapshot");
    assert!(
        matches,
        "snapshot `processes-dark` changed — delete its PNG to re-baseline"
    );
}

#[test]
#[ignore = "renders pixels on wgpu; run with: cargo test -p prexp-desktop -- --ignored snapshot"]
fn snapshot_processes_light() {
    let mut app = sample_app();
    let _ = app.update(Message::ToggleTheme);

    std::fs::create_dir_all("snapshots").expect("create snapshots dir");
    let mut sim = iced_test::Simulator::with_size(
        iced::Settings::default(),
        iced::Size::new(1040.0, 720.0),
        crate::view::view(&app),
    );
    let snapshot = sim.snapshot(&app.theme()).expect("render snapshot");
    let matches = snapshot
        .matches_image("snapshots/prexp-desktop-processes-light.png")
        .expect("write/compare snapshot");
    assert!(
        matches,
        "snapshot `processes-light` changed — delete its PNG to re-baseline"
    );
}

/// An `App` with the info panel open on `tab`, loaded with [`sample_detail`].
fn info_app(tab: crate::app::InfoTab) -> App {
    let mut app = sample_app();
    let _ = app.update(Message::SelectRow(2)); // node
    let _ = app.update(Message::OpenInfo); // fires a (dropped) fetch task
    let _ = app.update(Message::InfoLoaded(Ok(sample_detail())));
    let _ = app.update(Message::SelectInfoTab(tab));
    app
}

#[test]
#[ignore = "renders pixels on wgpu; run with: cargo test -p prexp-desktop -- --ignored snapshot"]
fn snapshot_info_overview() {
    let app = info_app(crate::app::InfoTab::Overview);
    std::fs::create_dir_all("snapshots").expect("create snapshots dir");
    let mut sim = iced_test::Simulator::with_size(
        iced::Settings::default(),
        iced::Size::new(1040.0, 720.0),
        crate::view::view(&app),
    );
    let snapshot = sim.snapshot(&app.theme()).expect("render snapshot");
    let matches = snapshot
        .matches_image("snapshots/prexp-desktop-info-overview.png")
        .expect("write/compare snapshot");
    assert!(
        matches,
        "snapshot `info-overview` changed — delete its PNG to re-baseline"
    );
}

#[test]
#[ignore = "renders pixels on wgpu; run with: cargo test -p prexp-desktop -- --ignored snapshot"]
fn snapshot_info_environment() {
    let app = info_app(crate::app::InfoTab::Environment);
    std::fs::create_dir_all("snapshots").expect("create snapshots dir");
    let mut sim = iced_test::Simulator::with_size(
        iced::Settings::default(),
        iced::Size::new(1040.0, 720.0),
        crate::view::view(&app),
    );
    let snapshot = sim.snapshot(&app.theme()).expect("render snapshot");
    let matches = snapshot
        .matches_image("snapshots/prexp-desktop-info-environment.png")
        .expect("write/compare snapshot");
    assert!(
        matches,
        "snapshot `info-environment` changed — delete its PNG to re-baseline"
    );
}

#[test]
#[ignore = "renders pixels on wgpu; run with: cargo test -p prexp-desktop -- --ignored snapshot"]
fn snapshot_signal_dialog() {
    let mut app = sample_app();
    let _ = app.update(Message::SelectRow(2)); // node
    let _ = app.update(Message::OpenSignal);
    let _ = app.update(Message::SelectSignal(crate::app::Signal::Kill));

    std::fs::create_dir_all("snapshots").expect("create snapshots dir");
    let mut sim = iced_test::Simulator::with_size(
        iced::Settings::default(),
        iced::Size::new(1040.0, 720.0),
        crate::view::view(&app),
    );
    let snapshot = sim.snapshot(&app.theme()).expect("render snapshot");
    let matches = snapshot
        .matches_image("snapshots/prexp-desktop-signal-dialog.png")
        .expect("write/compare snapshot");
    assert!(
        matches,
        "snapshot `signal-dialog` changed — delete its PNG to re-baseline"
    );
}

#[test]
#[ignore = "renders pixels on wgpu; run with: cargo test -p prexp-desktop -- --ignored snapshot"]
fn snapshot_lookup_panel() {
    let mut app = sample_app();
    let _ = app.update(Message::OpenLookup);
    let _ = app.update(Message::LookupQuery("/usr/lib/libnode0.dylib".to_string()));
    let hits: Vec<_> = sample_processes().into_iter().take(3).collect();
    let _ = app.update(Message::LookupLoaded(Ok(hits)));

    std::fs::create_dir_all("snapshots").expect("create snapshots dir");
    let mut sim = iced_test::Simulator::with_size(
        iced::Settings::default(),
        iced::Size::new(1040.0, 720.0),
        crate::view::view(&app),
    );
    let snapshot = sim.snapshot(&app.theme()).expect("render snapshot");
    let matches = snapshot
        .matches_image("snapshots/prexp-desktop-lookup-panel.png")
        .expect("write/compare snapshot");
    assert!(
        matches,
        "snapshot `lookup-panel` changed — delete its PNG to re-baseline"
    );
}
