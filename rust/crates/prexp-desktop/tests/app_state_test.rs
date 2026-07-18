//! Behavioral tests for `prexp_desktop::App`, driven through its public message
//! API against a `Send + Sync` process-source double — mirroring prexp's TUI
//! `app_state_test.rs`. These cover the M2 surface: sorting, PID-anchored
//! selection across re-sorts/refreshes, the detail-pane resolution, and the
//! CPU%-delta behavior.

mod support;

use std::sync::Arc;
use std::time::Duration;

use prexp_core::models::{
    DiskIo, OpenResource, ProcessActivity, ProcessDetail, ProcessMemory, ProcessSnapshot,
    ProcessState, ResourceKind,
};
use prexp_core::system::{BatteryInfo, CpuTicks, MemoryInfo};
use prexp_desktop::{App, InfoTab, Message, Sample, Signal, SortField};
use support::fake_source::FakeProcessSource;

// --- fixtures -------------------------------------------------------------

fn resource(fd: i32, kind: ResourceKind, path: Option<&str>) -> OpenResource {
    OpenResource {
        descriptor: fd,
        kind,
        path: path.map(String::from),
    }
}

#[allow(clippy::too_many_arguments)]
fn snap(
    pid: i32,
    name: &str,
    rss: u64,
    phys: u64,
    threads: i32,
    cpu_time_ns: u64,
    resources: Vec<OpenResource>,
) -> ProcessSnapshot {
    ProcessSnapshot {
        pid,
        ppid: 1,
        name: name.into(),
        state: ProcessState::Running,
        accessible: true,
        memory: ProcessMemory { rss, phys },
        activity: ProcessActivity {
            cpu_time_ns,
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

/// Three processes with deliberately distinct fields so every sort order is
/// unambiguous. Resource mix: alpha 2 files+1 socket (total 3), bravo 5 files+1
/// socket (total 6), charlie 1 file (total 1).
fn sample() -> Vec<ProcessSnapshot> {
    vec![
        snap(
            100,
            "alpha",
            50,
            30,
            8,
            1_000_000_000,
            vec![
                resource(3, ResourceKind::File, Some("/a/1")),
                resource(4, ResourceKind::File, Some("/a/2")),
                resource(5, ResourceKind::Socket, None),
            ],
        ),
        snap(
            200,
            "bravo",
            90,
            60,
            4,
            1_000_000_000,
            vec![
                resource(3, ResourceKind::File, Some("/b/1")),
                resource(4, ResourceKind::File, Some("/b/2")),
                resource(5, ResourceKind::File, Some("/b/3")),
                resource(6, ResourceKind::File, Some("/b/4")),
                resource(7, ResourceKind::File, Some("/b/5")),
                resource(8, ResourceKind::Socket, None),
            ],
        ),
        snap(
            300,
            "charlie",
            10,
            5,
            12,
            1_000_000_000,
            vec![resource(3, ResourceKind::File, Some("/c/1"))],
        ),
    ]
}

/// A fresh app loaded with `sample()` via the real refresh path. The boot Task is
/// dropped (we never run an iced runtime); state is driven by messages.
fn loaded_app() -> App {
    let (mut app, _boot) = App::boot(
        Arc::new(FakeProcessSource::new(sample())),
        Duration::from_secs(2),
    );
    let _ = app.update(Message::Refreshed(Ok(Sample::of(sample()))));
    app
}

fn pids(app: &App) -> Vec<i32> {
    app.processes().iter().map(|p| p.pid).collect()
}

fn sort(app: &mut App, field: SortField) {
    let _ = app.update(Message::SortBy(field));
}

// --- sorting --------------------------------------------------------------

#[test]
fn sorts_descending_by_numeric_fields() {
    let mut app = loaded_app();

    sort(&mut app, SortField::Rss);
    assert_eq!(pids(&app), vec![200, 100, 300], "RSS desc");

    sort(&mut app, SortField::Pmem);
    assert_eq!(pids(&app), vec![200, 100, 300], "PMEM desc");

    sort(&mut app, SortField::Threads);
    assert_eq!(pids(&app), vec![300, 100, 200], "THR desc");

    sort(&mut app, SortField::Files);
    assert_eq!(pids(&app), vec![200, 100, 300], "FILES desc");

    sort(&mut app, SortField::Total);
    assert_eq!(pids(&app), vec![200, 100, 300], "TOTAL desc");
}

#[test]
fn sorts_ascending_by_textual_fields() {
    let mut app = loaded_app();

    sort(&mut app, SortField::Name);
    assert_eq!(pids(&app), vec![100, 200, 300], "name asc");

    sort(&mut app, SortField::Pid);
    assert_eq!(pids(&app), vec![100, 200, 300], "pid asc");
}

#[test]
fn reverse_flips_the_active_order() {
    let mut app = loaded_app();
    sort(&mut app, SortField::Name);
    assert_eq!(pids(&app), vec![100, 200, 300]);

    let _ = app.update(Message::ToggleReverse);
    assert_eq!(pids(&app), vec![300, 200, 100], "reversed name order");
    assert!(app.reverse());
}

// --- selection & detail pane ----------------------------------------------

#[test]
fn selection_is_pid_anchored_across_resort_and_refresh() {
    let mut app = loaded_app();
    sort(&mut app, SortField::Pid); // order [100, 200, 300]
    let _ = app.update(Message::SelectRow(1)); // pid 200
    assert_eq!(app.selected_pid(), Some(200));

    // Re-sort so 200 moves to a different row index.
    sort(&mut app, SortField::Threads); // order [300, 100, 200]
    assert_eq!(app.selected_pid(), Some(200), "anchor survives re-sort");
    assert_eq!(app.selected_process().map(|p| p.pid), Some(200));

    // A refresh with the same set keeps the selection resolved.
    let _ = app.update(Message::Refreshed(Ok(Sample::of(sample()))));
    assert_eq!(app.selected_process().map(|p| p.pid), Some(200));
}

#[test]
fn detail_resolves_the_selected_process_resources() {
    let mut app = loaded_app();
    sort(&mut app, SortField::Pid);
    let _ = app.update(Message::SelectRow(1)); // pid 200 (bravo)

    let proc = app.selected_process().expect("bravo is selected");
    assert_eq!(proc.name, "bravo");
    assert_eq!(proc.resources.len(), 6);
    assert_eq!(
        proc.count_by_kind(&ResourceKind::File),
        5,
        "bravo has 5 open files"
    );
}

#[test]
fn selection_anchor_survives_but_unresolves_when_process_exits() {
    let mut app = loaded_app();
    sort(&mut app, SortField::Pid);
    let _ = app.update(Message::SelectRow(1)); // pid 200
    assert_eq!(app.selected_process().map(|p| p.pid), Some(200));

    // Next refresh no longer contains pid 200.
    let survivors: Vec<_> = sample().into_iter().filter(|p| p.pid != 200).collect();
    let _ = app.update(Message::Refreshed(Ok(Sample::of(survivors))));

    assert_eq!(app.selected_pid(), Some(200), "anchor is retained");
    assert!(
        app.selected_process().is_none(),
        "but it no longer resolves to a live process"
    );
}

// --- CPU% deltas ----------------------------------------------------------

#[test]
fn cpu_percent_is_zero_on_the_first_refresh() {
    let app = loaded_app(); // exactly one Refreshed so far
    for pid in [100, 200, 300] {
        assert_eq!(app.cpu_percent(pid), 0.0, "no baseline yet for pid {pid}");
    }
}

#[test]
fn cpu_percent_reflects_cpu_time_deltas_between_refreshes() {
    let mut app = loaded_app(); // baseline: all at 1_000_000_000 ns

    // Second refresh: only bravo (200) burned more CPU time.
    let mut next = sample();
    for p in &mut next {
        if p.pid == 200 {
            p.activity.cpu_time_ns = 3_000_000_000; // +2s of CPU
        }
    }
    let _ = app.update(Message::Refreshed(Ok(Sample::of(next))));

    assert!(
        app.cpu_percent(200) > 0.0,
        "bravo consumed CPU, so its percent is positive"
    );
    assert_eq!(app.cpu_percent(100), 0.0, "alpha was idle");
    assert!(
        app.cpu_percent(200) > app.cpu_percent(100),
        "the busy process ranks above the idle one"
    );
}

// --- system stats header --------------------------------------------------

fn ticks(user: u64, system: u64, idle: u64) -> CpuTicks {
    CpuTicks {
        user,
        system,
        idle,
        nice: 0,
    }
}

fn sample_with(cpu_ticks: Vec<CpuTicks>) -> Sample {
    Sample {
        processes: sample(),
        cpu_ticks,
        memory: Some(MemoryInfo {
            total: 16_000_000_000,
            used: 8_000_000_000,
            free: 8_000_000_000,
            wired: 2_000_000_000,
            compressed: 1_000_000_000,
            swap_total: 0,
            swap_used: 0,
        }),
        load: Some([1.5, 1.2, 0.9]),
        battery: Some(BatteryInfo {
            percent: 82.0,
            charging: true,
            time_to_empty_min: -1,
            time_to_full_min: 25,
        }),
    }
}

fn boot_app() -> App {
    let (app, _boot) = App::boot(
        Arc::new(FakeProcessSource::new(sample())),
        Duration::from_secs(2),
    );
    app
}

#[test]
fn per_core_cpu_needs_two_tick_reads_to_diff() {
    let mut app = boot_app();

    // First read establishes the baseline — no per-core numbers yet.
    let _ = app.update(Message::Refreshed(Ok(sample_with(vec![
        ticks(100, 0, 100),
        ticks(100, 0, 100),
    ]))));
    assert!(
        app.system().per_core().is_empty(),
        "no delta available on the first sample"
    );

    // Second read: core 0 spent all delta busy (200 vs 100 = +100 user, +0 idle);
    // core 1 spent it all idle (+0 user, +100 idle).
    let _ = app.update(Message::Refreshed(Ok(sample_with(vec![
        ticks(200, 0, 100),
        ticks(100, 0, 200),
    ]))));

    let per_core = app.system().per_core();
    assert_eq!(per_core.len(), 2);
    assert!((per_core[0] - 100.0).abs() < 1e-6, "core 0 fully busy");
    assert!((per_core[1] - 0.0).abs() < 1e-6, "core 1 fully idle");
    assert!((app.system().aggregate_cpu() - 50.0).abs() < 1e-6, "mean is 50%");
}

#[test]
fn cpu_history_grows_one_point_per_diffed_sample() {
    let mut app = boot_app();
    let _ = app.update(Message::Refreshed(Ok(sample_with(vec![ticks(100, 0, 100)]))));
    assert!(app.system().cpu_history().is_empty(), "baseline pushes nothing");

    let _ = app.update(Message::Refreshed(Ok(sample_with(vec![ticks(150, 0, 150)]))));
    assert_eq!(app.system().cpu_history().len(), 1);

    let _ = app.update(Message::Refreshed(Ok(sample_with(vec![ticks(200, 0, 250)]))));
    assert_eq!(app.system().cpu_history().len(), 2);
}

#[test]
fn point_in_time_metrics_pass_through_to_the_header() {
    let mut app = boot_app();
    let _ = app.update(Message::Refreshed(Ok(sample_with(vec![ticks(100, 0, 100)]))));

    assert_eq!(app.system().memory().map(|m| m.total), Some(16_000_000_000));
    assert_eq!(app.system().load(), Some([1.5, 1.2, 0.9]));
    assert_eq!(app.system().battery().map(|b| b.charging), Some(true));
}

// --- info panel -----------------------------------------------------------

fn detail_for(pid: i32) -> ProcessDetail {
    ProcessDetail {
        pid,
        ppid: 1,
        parent_name: "launchd".into(),
        name: "bravo".into(),
        path: "/usr/bin/bravo".into(),
        cwd: "/".into(),
        user: "dev".into(),
        uid: 501,
        state: ProcessState::Running,
        nice: 0,
        started_secs: 1_700_000_000,
        thread_count: 4,
        virtual_size: 0,
        memory_rss: 0,
        memory_phys: 0,
        cpu_time_ns: 0,
        fd_files: 5,
        fd_sockets: 1,
        fd_pipes: 0,
        fd_other: 0,
        fd_total: 6,
        faults: 0,
        context_switches: 0,
        syscalls_mach: 0,
        syscalls_unix: 0,
        disk_bytes_read: 0,
        disk_bytes_written: 0,
        network: Vec::new(),
        environment: Vec::new(),
    }
}

#[test]
fn open_info_marks_loading_then_populates_on_load() {
    let mut app = loaded_app();
    sort(&mut app, SortField::Pid);
    let _ = app.update(Message::SelectRow(1)); // pid 200

    let _ = app.update(Message::OpenInfo);
    let info = app.info().expect("panel is open");
    assert_eq!(info.pid, 200);
    assert_eq!(info.tab, InfoTab::Overview, "opens on Overview");
    assert!(info.detail.is_none(), "detail not fetched yet");

    let _ = app.update(Message::InfoLoaded(Ok(detail_for(200))));
    assert_eq!(
        app.info().and_then(|i| i.detail.as_ref()).map(|d| d.pid),
        Some(200)
    );
}

#[test]
fn open_info_does_nothing_without_a_selection() {
    let mut app = loaded_app(); // nothing selected
    let _ = app.update(Message::OpenInfo);
    assert!(app.info().is_none(), "no panel opens without a selected process");
}

#[test]
fn info_tab_switch_resets_scroll_and_close_clears() {
    let mut app = loaded_app();
    let _ = app.update(Message::SelectRow(0));
    let _ = app.update(Message::OpenInfo);
    let _ = app.update(Message::InfoScrolled(120.0));

    let _ = app.update(Message::SelectInfoTab(InfoTab::Environment));
    let info = app.info().expect("still open");
    assert_eq!(info.tab, InfoTab::Environment);
    assert_eq!(info.scroll, 0.0, "scroll resets on tab change");

    let _ = app.update(Message::CloseInfo);
    assert!(app.info().is_none(), "close clears the panel");
}

#[test]
fn info_load_failure_surfaces_an_error() {
    let mut app = loaded_app();
    let _ = app.update(Message::SelectRow(0));
    let _ = app.update(Message::OpenInfo);
    let _ = app.update(Message::InfoLoaded(Err("denied".into())));

    let info = app.info().expect("panel stays open");
    assert!(info.detail.is_none());
    assert_eq!(info.error.as_deref(), Some("denied"));
}

// --- send signal ----------------------------------------------------------

#[test]
fn signal_prompt_defaults_to_sigterm_and_tracks_selection() {
    let mut app = loaded_app();
    sort(&mut app, SortField::Pid);
    let _ = app.update(Message::SelectRow(1)); // pid 200 (bravo)

    let _ = app.update(Message::OpenSignal);
    let prompt = app.signal_prompt().expect("prompt is open");
    assert_eq!(prompt.pid, 200);
    assert_eq!(prompt.name, "bravo");
    assert_eq!(prompt.signal, Signal::Term, "defaults to graceful SIGTERM");

    let _ = app.update(Message::SelectSignal(Signal::Kill));
    assert_eq!(app.signal_prompt().map(|p| p.signal), Some(Signal::Kill));
}

#[test]
fn confirming_signal_closes_prompt_and_posts_a_notice() {
    let mut app = loaded_app();
    let _ = app.update(Message::SelectRow(0));
    let _ = app.update(Message::OpenSignal);
    let _ = app.update(Message::ConfirmSignal); // fake source returns Ok

    assert!(app.signal_prompt().is_none(), "prompt closes after sending");
    assert!(
        app.notice().unwrap_or_default().contains("SIGTERM"),
        "a success notice names the signal"
    );

    let _ = app.update(Message::DismissNotice);
    assert!(app.notice().is_none());
}

#[test]
fn cancel_signal_closes_without_a_notice() {
    let mut app = loaded_app();
    let _ = app.update(Message::SelectRow(0));
    let _ = app.update(Message::OpenSignal);
    let _ = app.update(Message::CancelSignal);
    assert!(app.signal_prompt().is_none());
    assert!(app.notice().is_none());
}

// --- reverse lookup -------------------------------------------------------

#[test]
fn lookup_runs_query_and_stores_results() {
    let mut app = loaded_app();
    let _ = app.update(Message::OpenLookup);
    assert!(app.lookup().is_some());

    let _ = app.update(Message::LookupQuery("/a/1".to_string()));
    // The fetch itself is a Task (dropped here); simulate its completion.
    let hits = vec![sample()[0].clone()];
    let _ = app.update(Message::LookupLoaded(Ok(hits)));

    let lookup = app.lookup().expect("still open");
    assert!(lookup.searched);
    assert_eq!(lookup.results.len(), 1);
    assert_eq!(lookup.results[0].pid, 100);

    let _ = app.update(Message::CloseLookup);
    assert!(app.lookup().is_none());
}

#[test]
fn lookup_failure_surfaces_an_error() {
    let mut app = loaded_app();
    let _ = app.update(Message::OpenLookup);
    let _ = app.update(Message::LookupLoaded(Err("nope".to_string())));

    let lookup = app.lookup().expect("still open");
    assert_eq!(lookup.error.as_deref(), Some("nope"));
    assert!(lookup.results.is_empty());
}
