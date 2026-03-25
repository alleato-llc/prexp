mod support;

use std::time::Duration;

use prexp_core::models::{OpenResource, ProcessSnapshot, ResourceKind};

use prexp_app::tui::app::{App, Chart, ChartConfig, Column, FileSortField, InputMode, MainView, ProcessSortField, SortDirection};
use support::fake_source::FakeProcessSource;

fn resource(fd: i32, kind: ResourceKind, path: Option<&str>) -> OpenResource {
    OpenResource {
        descriptor: fd,
        kind,
        path: path.map(String::from),
    }
}

fn sample_snapshots() -> Vec<ProcessSnapshot> {
    vec![
        ProcessSnapshot {
            pid: 100,
            ppid: 1,
            name: "nginx".into(),
            thread_count: 8,
            memory_rss: 1024 * 1024 * 50, memory_phys: 1024 * 1024 * 30, cpu_time_ns: 1_000_000_000, faults: 0, context_switches: 0, syscalls_mach: 0, syscalls_unix: 0, disk_bytes_read: 0, disk_bytes_written: 0, state: prexp_ffi::ProcessState::Running, accessible: true,
            resources: vec![
                resource(3, ResourceKind::File, Some("/var/log/access.log")),
                resource(4, ResourceKind::Socket, None),
            ],
        },
        ProcessSnapshot {
            pid: 200,
            ppid: 100,
            name: "node".into(),
            thread_count: 4,
            memory_rss: 1024 * 1024 * 50, memory_phys: 1024 * 1024 * 30, cpu_time_ns: 1_000_000_000, faults: 0, context_switches: 0, syscalls_mach: 0, syscalls_unix: 0, disk_bytes_read: 0, disk_bytes_written: 0, state: prexp_ffi::ProcessState::Running, accessible: true,
            resources: vec![resource(3, ResourceKind::File, Some("/app/server.js"))],
        },
        ProcessSnapshot {
            pid: 300,
            ppid: 1,
            name: "redis-server".into(),
            thread_count: 3,
            memory_rss: 1024 * 1024 * 50, memory_phys: 1024 * 1024 * 30, cpu_time_ns: 1_000_000_000, faults: 0, context_switches: 0, syscalls_mach: 0, syscalls_unix: 0, disk_bytes_read: 0, disk_bytes_written: 0, state: prexp_ffi::ProcessState::Running, accessible: true,
            resources: vec![
                resource(3, ResourceKind::File, Some("/var/lib/redis/dump.rdb")),
                resource(4, ResourceKind::Socket, None),
                resource(5, ResourceKind::Pipe, None),
            ],
        },
    ]
}

fn create_app_with_data() -> (App, FakeProcessSource) {
    let source = FakeProcessSource::new(sample_snapshots());
    let mut app = App::new(Duration::from_secs(2));
    app.refresh(&source);
    (app, source)
}

// -- Process list navigation --

#[test]
fn refresh_loads_snapshots() {
    let (app, _) = create_app_with_data();
    assert_eq!(app.snapshots.len(), 3);
    assert_eq!(app.filtered_indices.len(), 3);
}

#[test]
fn initial_selection_is_first_process() {
    let (app, _) = create_app_with_data();
    assert_eq!(app.selected_index, 0);
    assert_eq!(app.selected_snapshot().unwrap().pid, 100);
}

#[test]
fn move_down_advances_selection() {
    let (mut app, _) = create_app_with_data();
    app.move_down();
    assert_eq!(app.selected_index, 1);
    assert_eq!(app.selected_snapshot().unwrap().pid, 200);
}

#[test]
fn move_down_does_not_overflow() {
    let (mut app, _) = create_app_with_data();
    for _ in 0..10 {
        app.move_down();
    }
    assert_eq!(app.selected_index, 2);
}

#[test]
fn move_up_decrements_selection() {
    let (mut app, _) = create_app_with_data();
    app.move_down();
    app.move_down();
    app.move_up();
    assert_eq!(app.selected_index, 1);
}

#[test]
fn move_up_does_not_underflow() {
    let (mut app, _) = create_app_with_data();
    app.move_up();
    assert_eq!(app.selected_index, 0);
}

// -- View switching --

#[test]
fn starts_in_process_list_view() {
    let (app, _) = create_app_with_data();
    assert_eq!(app.main_view, MainView::Processes);
    assert!(!app.detail_open);
}

#[test]
fn enter_opens_detail_overlay() {
    let (mut app, _) = create_app_with_data();
    app.open_detail();
    assert!(app.detail_open);
    assert_eq!(app.detail_selected, 0);
    assert_eq!(app.detail_h_scroll, 0);
}

#[test]
fn close_detail_returns_to_main_view() {
    let (mut app, _) = create_app_with_data();
    app.open_detail();
    app.close_detail();
    assert!(!app.detail_open);
}

// -- Detail view navigation --

#[test]
fn detail_move_down_scrolls_resources() {
    let (mut app, _) = create_app_with_data();
    app.move_down();
    app.move_down();
    app.open_detail();

    app.move_down();
    assert_eq!(app.detail_selected, 1);
    app.move_down();
    assert_eq!(app.detail_selected, 2);
    app.move_down();
    assert_eq!(app.detail_selected, 2);
}

#[test]
fn detail_move_up_scrolls_resources() {
    let (mut app, _) = create_app_with_data();
    app.open_detail();
    app.move_down();
    app.move_up();
    assert_eq!(app.detail_selected, 0);
    app.move_up();
    assert_eq!(app.detail_selected, 0);
}

#[test]
fn detail_horizontal_scroll() {
    let (mut app, _) = create_app_with_data();
    app.open_detail();
    assert_eq!(app.detail_h_scroll, 0);

    app.scroll_right();
    assert_eq!(app.detail_h_scroll, 4);
    app.scroll_left();
    assert_eq!(app.detail_h_scroll, 0);
    app.scroll_left();
    assert_eq!(app.detail_h_scroll, 0);
}

#[test]
fn horizontal_scroll_only_in_detail() {
    let (mut app, _) = create_app_with_data();
    app.scroll_right();
    assert_eq!(app.detail_h_scroll, 0);
}

// -- Search --

#[test]
fn search_filters_by_name() {
    let (mut app, _) = create_app_with_data();
    app.enter_search_mode();
    assert_eq!(app.input_mode, InputMode::Search);

    for c in "nginx".chars() {
        app.push_input_char(c);
    }

    assert_eq!(app.filtered_indices.len(), 1);
    assert_eq!(app.selected_snapshot().unwrap().pid, 100);
}

#[test]
fn search_filters_by_pid() {
    let (mut app, _) = create_app_with_data();
    app.enter_search_mode();
    for c in "200".chars() {
        app.push_input_char(c);
    }

    assert_eq!(app.filtered_indices.len(), 1);
    assert_eq!(app.selected_snapshot().unwrap().pid, 200);
}

#[test]
fn search_clear_restores_all() {
    let (mut app, _) = create_app_with_data();
    app.enter_search_mode();
    for c in "xyz".chars() {
        app.push_input_char(c);
    }
    assert_eq!(app.filtered_indices.len(), 0);

    app.pop_input_char();
    app.pop_input_char();
    app.pop_input_char();
    assert_eq!(app.filtered_indices.len(), 3);
}

#[test]
fn selection_clamps_after_filter_reduces_list() {
    let (mut app, _) = create_app_with_data();
    app.move_down();
    app.move_down();
    assert_eq!(app.selected_index, 2);

    app.enter_search_mode();
    for c in "node".chars() {
        app.push_input_char(c);
    }

    assert_eq!(app.selected_index, 0);
    assert_eq!(app.filtered_indices.len(), 1);
}

// -- Reverse lookup --

#[test]
fn reverse_lookup_finds_matching_processes() {
    let (mut app, source) = create_app_with_data();
    app.enter_reverse_lookup_mode();

    for c in "/var/log/access.log".chars() {
        app.push_input_char(c);
    }
    app.perform_reverse_lookup(&source);

    assert_eq!(app.reverse_results.len(), 1);
    assert_eq!(app.reverse_results[0].pid, 100);
    assert_eq!(app.input_mode, InputMode::Normal);
}

#[test]
fn reverse_lookup_with_no_matches() {
    let (mut app, source) = create_app_with_data();
    app.enter_reverse_lookup_mode();

    for c in "/nonexistent".chars() {
        app.push_input_char(c);
    }
    app.perform_reverse_lookup(&source);

    assert!(app.reverse_results.is_empty());
    assert!(app.status_message.is_some());
}

// -- Detail resets on open --

#[test]
fn opening_detail_resets_scroll_state() {
    let (mut app, _) = create_app_with_data();
    app.open_detail();
    app.move_down();
    app.scroll_right();
    app.close_detail();

    app.move_down();
    app.open_detail();
    assert_eq!(app.detail_selected, 0);
    assert_eq!(app.detail_h_scroll, 0);
}

// -- Process tree --

#[test]
fn tree_view_groups_children_under_parent() {
    let (app, _) = create_app_with_data();

    assert_eq!(app.tree_entries.len(), 3);

    assert_eq!(app.snapshots[app.tree_entries[0].snapshot_index].name, "nginx");
    assert_eq!(app.tree_entries[0].depth, 0);

    assert_eq!(app.snapshots[app.tree_entries[1].snapshot_index].name, "node");
    assert_eq!(app.tree_entries[1].depth, 1);

    assert_eq!(
        app.snapshots[app.tree_entries[2].snapshot_index].name,
        "redis-server"
    );
    assert_eq!(app.tree_entries[2].depth, 0);
}

#[test]
fn tree_entries_have_correct_prefixes() {
    let (app, _) = create_app_with_data();
    assert_eq!(app.tree_entries[0].prefix, "");
    assert!(
        app.tree_entries[1].prefix.contains("└── ")
            || app.tree_entries[1].prefix.contains("├── ")
    );
}

#[test]
fn search_mode_disables_tree_view() {
    let (mut app, _) = create_app_with_data();
    app.enter_search_mode();
    for c in "node".chars() {
        app.push_input_char(c);
    }

    assert_eq!(app.tree_entries.len(), 1);
    assert_eq!(app.tree_entries[0].depth, 0);
    assert_eq!(app.tree_entries[0].prefix, "");
}

// -- File view --

#[test]
fn toggle_view_switches_to_files() {
    let (mut app, _) = create_app_with_data();
    app.toggle_view();
    assert_eq!(app.main_view, MainView::Files);
    app.toggle_view();
    assert_eq!(app.main_view, MainView::Processes);
}

#[test]
fn file_view_shows_deduplicated_paths() {
    let (mut app, _) = create_app_with_data();
    app.toggle_view();

    // Sample data has 3 unique file paths:
    // /app/server.js, /var/lib/redis/dump.rdb, /var/log/access.log
    // (sockets and pipes have no path, so they're excluded)
    assert_eq!(app.filtered_file_indices.len(), 3);
}

#[test]
fn file_entries_default_sort_by_process_count_desc() {
    let (mut app, _) = create_app_with_data();
    app.toggle_view();

    // Default sort is process count descending.
    // All files in sample data have 1 opener each, so order is stable
    // (ties broken by insertion order from HashMap, but at least count is non-increasing).
    let counts: Vec<usize> = app
        .filtered_file_indices
        .iter()
        .map(|&i| app.file_entries[i].openers.len())
        .collect();

    for window in counts.windows(2) {
        assert!(window[0] >= window[1], "should be sorted desc by count");
    }
}

#[test]
fn file_entry_tracks_opener_count() {
    let (mut app, _) = create_app_with_data();
    app.toggle_view();

    // /var/log/access.log is opened by nginx (pid 100)
    let access_log = app
        .file_entries
        .iter()
        .find(|e| e.path == "/var/log/access.log")
        .unwrap();
    assert_eq!(access_log.openers.len(), 1);
    assert_eq!(access_log.openers[0].pid, 100);
}

#[test]
fn file_view_search_filters_paths() {
    let (mut app, _) = create_app_with_data();
    app.toggle_view();
    app.enter_search_mode();
    for c in "redis".chars() {
        app.push_input_char(c);
    }

    assert_eq!(app.filtered_file_indices.len(), 1);
    assert_eq!(
        app.file_entries[app.filtered_file_indices[0]].path,
        "/var/lib/redis/dump.rdb"
    );
}

#[test]
fn file_view_navigation() {
    let (mut app, _) = create_app_with_data();
    app.toggle_view();

    assert_eq!(app.file_selected_index, 0);
    app.move_down();
    assert_eq!(app.file_selected_index, 1);
    app.move_up();
    assert_eq!(app.file_selected_index, 0);
    app.move_up();
    assert_eq!(app.file_selected_index, 0);
}

#[test]
fn file_view_detail_shows_openers() {
    let (mut app, _) = create_app_with_data();
    app.toggle_view();
    app.open_detail();

    assert!(app.detail_open);
    assert_eq!(app.detail_selected, 0);
}

// -- Anchor-based tracking --

#[test]
fn process_anchor_survives_refresh() {
    let (mut app, source) = create_app_with_data();
    // Select redis-server (pid 300)
    app.move_down();
    app.move_down();
    assert_eq!(app.selected_snapshot().unwrap().pid, 300);

    // Refresh — selection should stay on pid 300
    app.refresh(&source);
    assert_eq!(app.selected_snapshot().unwrap().pid, 300);
}

#[test]
fn file_anchor_survives_refresh() {
    let (mut app, source) = create_app_with_data();
    app.toggle_view();

    // Move to a specific file
    app.move_down();
    let anchored_path = app.selected_file_entry().unwrap().path.clone();

    // Refresh — selection should stay on same path
    app.refresh(&source);
    assert_eq!(
        app.selected_file_entry().unwrap().path,
        anchored_path
    );
}

// -- Sorting --

#[test]
fn process_sort_cycles_through_fields() {
    let (mut app, _) = create_app_with_data();
    assert_eq!(app.process_sort, ProcessSortField::Unsorted);

    app.cycle_sort();
    assert_eq!(app.process_sort, ProcessSortField::Pid);
    assert_eq!(app.process_sort_dir, SortDirection::Asc);

    app.cycle_sort();
    assert_eq!(app.process_sort, ProcessSortField::Name);
    assert_eq!(app.process_sort_dir, SortDirection::Asc);

    app.cycle_sort();
    assert_eq!(app.process_sort, ProcessSortField::Total);
    assert_eq!(app.process_sort_dir, SortDirection::Desc);

    app.cycle_sort();
    assert_eq!(app.process_sort, ProcessSortField::Unsorted);
}

#[test]
fn process_sort_by_pid_ascending() {
    let (mut app, _) = create_app_with_data();
    app.cycle_sort(); // -> Pid asc

    let pids: Vec<i32> = app
        .filtered_indices
        .iter()
        .map(|&i| app.snapshots[i].pid)
        .collect();

    for window in pids.windows(2) {
        assert!(window[0] <= window[1], "should be sorted by pid asc");
    }
}

#[test]
fn process_sort_by_name_ascending() {
    let (mut app, _) = create_app_with_data();
    app.cycle_sort(); // Pid
    app.cycle_sort(); // Name asc

    let names: Vec<String> = app
        .filtered_indices
        .iter()
        .map(|&i| app.snapshots[i].name.to_lowercase())
        .collect();

    for window in names.windows(2) {
        assert!(window[0] <= window[1], "should be sorted by name asc");
    }
}

#[test]
fn process_sort_by_total_descending() {
    let (mut app, _) = create_app_with_data();
    app.cycle_sort(); // Pid
    app.cycle_sort(); // Name
    app.cycle_sort(); // Total desc

    let totals: Vec<usize> = app
        .filtered_indices
        .iter()
        .map(|&i| app.snapshots[i].resources.len())
        .collect();

    for window in totals.windows(2) {
        assert!(window[0] >= window[1], "should be sorted by total desc");
    }
}

#[test]
fn reverse_sort_toggles_direction() {
    let (mut app, _) = create_app_with_data();
    app.cycle_sort(); // Pid asc
    assert_eq!(app.process_sort_dir, SortDirection::Asc);

    app.reverse_sort();
    assert_eq!(app.process_sort_dir, SortDirection::Desc);

    app.reverse_sort();
    assert_eq!(app.process_sort_dir, SortDirection::Asc);
}

#[test]
fn reverse_sort_noop_when_unsorted() {
    let (mut app, _) = create_app_with_data();
    assert_eq!(app.process_sort, ProcessSortField::Unsorted);

    app.reverse_sort(); // should do nothing
    assert_eq!(app.process_sort, ProcessSortField::Unsorted);
}

#[test]
fn sort_preserves_tree_structure() {
    let (mut app, _) = create_app_with_data();
    // Tree view has depth > 0 entries (node is child of nginx)
    assert!(app.tree_entries.iter().any(|e| e.depth > 0));

    app.cycle_sort(); // Pid — should still have tree structure
    assert!(
        app.tree_entries.iter().any(|e| e.depth > 0),
        "sorted view should preserve parent-child grouping"
    );

    // node (pid 200) should still be a child entry
    let node_entry = app
        .tree_entries
        .iter()
        .find(|e| app.snapshots[e.snapshot_index].name == "node")
        .unwrap();
    assert_eq!(node_entry.depth, 1, "node should remain a child");
}

#[test]
fn sort_reorders_only_roots() {
    let (mut app, _) = create_app_with_data();
    // Default tree order: nginx (100), node (200, child), redis (300)
    // Sort by name asc: nginx, redis — node stays under nginx
    app.cycle_sort(); // Pid
    app.cycle_sort(); // Name asc

    let root_names: Vec<&str> = app
        .tree_entries
        .iter()
        .filter(|e| e.depth == 0)
        .map(|e| app.snapshots[e.snapshot_index].name.as_str())
        .collect();

    assert_eq!(root_names, vec!["nginx", "redis-server"]);

    // node should immediately follow nginx as its child
    let nginx_pos = app
        .tree_entries
        .iter()
        .position(|e| app.snapshots[e.snapshot_index].name == "nginx")
        .unwrap();
    let node_entry = &app.tree_entries[nginx_pos + 1];
    assert_eq!(app.snapshots[node_entry.snapshot_index].name, "node");
    assert_eq!(node_entry.depth, 1);
}

#[test]
fn file_sort_cycles_through_fields() {
    let (mut app, _) = create_app_with_data();
    app.toggle_view();

    assert_eq!(app.file_sort, FileSortField::ProcessCount);
    assert_eq!(app.file_sort_dir, SortDirection::Desc);

    app.cycle_sort();
    assert_eq!(app.file_sort, FileSortField::Filename);
    assert_eq!(app.file_sort_dir, SortDirection::Asc);

    app.cycle_sort();
    assert_eq!(app.file_sort, FileSortField::ProcessCount);
    assert_eq!(app.file_sort_dir, SortDirection::Desc);
}

#[test]
fn file_sort_by_filename() {
    let (mut app, _) = create_app_with_data();
    app.toggle_view();
    app.cycle_sort(); // Filename asc

    let filenames: Vec<&str> = app
        .filtered_file_indices
        .iter()
        .map(|&i| {
            app.file_entries[i]
                .path
                .rsplit('/')
                .next()
                .unwrap_or(&app.file_entries[i].path)
        })
        .collect();

    for window in filenames.windows(2) {
        assert!(window[0] <= window[1], "should be sorted by filename asc: {:?}", filenames);
    }
}

#[test]
fn sort_persists_across_refresh() {
    let (mut app, source) = create_app_with_data();
    app.cycle_sort(); // Pid asc
    app.reverse_sort(); // Pid desc

    let pids_before: Vec<i32> = app
        .filtered_indices
        .iter()
        .map(|&i| app.snapshots[i].pid)
        .collect();

    app.refresh(&source);

    let pids_after: Vec<i32> = app
        .filtered_indices
        .iter()
        .map(|&i| app.snapshots[i].pid)
        .collect();

    assert_eq!(pids_before, pids_after);
    assert_eq!(app.process_sort, ProcessSortField::Pid);
    assert_eq!(app.process_sort_dir, SortDirection::Desc);
}

// -- Column configuration --

#[test]
fn default_columns_enabled() {
    let (app, _) = create_app_with_data();
    // All columns enabled by default except State
    assert!(app.column_config.is_enabled(Column::Cpu));
    assert!(app.column_config.is_enabled(Column::Mem));
    assert!(app.column_config.is_enabled(Column::Total));
    assert!(!app.column_config.is_enabled(Column::State)); // off by default
}

#[test]
fn config_toggle_disables_and_enables_column() {
    let (mut app, _) = create_app_with_data();
    app.open_config();
    assert!(app.config_open);

    // Toggle first column (CPU%) off
    app.config_toggle_selected();
    assert!(!app.column_config.is_enabled(Column::Cpu));

    // Toggle it back on
    app.config_toggle_selected();
    assert!(app.column_config.is_enabled(Column::Cpu));
}

#[test]
fn config_navigation() {
    let (mut app, _) = create_app_with_data();
    app.open_config();
    assert_eq!(app.config_selected, 0);

    app.config_move_down();
    assert_eq!(app.config_selected, 1);

    app.config_move_up();
    assert_eq!(app.config_selected, 0);

    // Should not underflow
    app.config_move_up();
    assert_eq!(app.config_selected, 0);
}

#[test]
fn disabling_cpu_column_skips_cpu_computation() {
    let (mut app, source) = create_app_with_data();

    // Disable CPU column
    app.open_config();
    app.config_toggle_selected(); // CPU is first
    app.close_config();

    // Refresh twice to give delta computation a chance
    app.refresh(&source);
    app.refresh(&source);

    // CPU percentages should be empty since column is disabled
    assert!(app.cpu_percentages.is_empty());
}

#[test]
fn close_config_returns_to_main() {
    let (mut app, _) = create_app_with_data();
    app.open_config();
    assert!(app.config_open);
    app.close_config();
    assert!(!app.config_open);
}

// -- Themes --

#[test]
fn theme_picker_opens_and_closes() {
    let (mut app, _) = create_app_with_data();
    assert!(!app.theme_open);

    app.open_theme_picker();
    assert!(app.theme_open);

    app.close_theme_picker();
    assert!(!app.theme_open);
}

#[test]
fn theme_picker_navigates_and_previews() {
    use prexp_app::tui::theme::THEMES;
    let (mut app, _) = create_app_with_data();
    app.open_theme_picker();
    assert_eq!(app.theme_index, 0);

    app.theme_move_down();
    assert_eq!(app.theme_index, 1);
    assert_eq!(app.current_theme().name, THEMES[1].name);

    app.theme_move_down();
    assert_eq!(app.theme_index, 2);

    app.theme_move_up();
    assert_eq!(app.theme_index, 1);

    // Does not underflow
    app.theme_move_up();
    app.theme_move_up();
    assert_eq!(app.theme_index, 0);

    // Does not overflow
    for _ in 0..THEMES.len() + 5 {
        app.theme_move_down();
    }
    assert_eq!(app.theme_index, THEMES.len() - 1);
}

#[test]
fn theme_name_shown_on_close() {
    use prexp_app::tui::theme::THEMES;
    let (mut app, _) = create_app_with_data();
    app.open_theme_picker();
    app.theme_move_down();
    app.close_theme_picker();

    let msg = app.status_message.as_ref().unwrap();
    assert!(msg.contains(THEMES[1].name));
}

// -- Process state --

fn sample_with_zombie() -> Vec<ProcessSnapshot> {
    let mut snaps = sample_snapshots();
    snaps.push(ProcessSnapshot {
        pid: 400,
        ppid: 1,
        name: "defunct".into(),
        thread_count: 0,
        memory_rss: 0,
        memory_phys: 0,
        cpu_time_ns: 0,
        faults: 0, context_switches: 0, syscalls_mach: 0, syscalls_unix: 0, disk_bytes_read: 0, disk_bytes_written: 0,
        state: prexp_ffi::ProcessState::Zombie,
        accessible: true,
        resources: Vec::new(),
    });
    snaps
}

#[test]
fn zombie_process_state() {
    let source = FakeProcessSource::new(sample_with_zombie());
    let mut app = App::new(Duration::from_secs(2));
    app.refresh(&source);

    let zombie = app.snapshots.iter().find(|s| s.pid == 400).unwrap();
    assert_eq!(zombie.state, prexp_ffi::ProcessState::Zombie);
    assert_eq!(zombie.state.label(), "ZMB");
}

#[test]
fn state_column_off_by_default() {
    let (app, _) = create_app_with_data();
    assert!(!app.column_config.is_enabled(Column::State));
}

// -- Info panel --

#[test]
fn info_panel_opens_and_closes() {
    let (mut app, _) = create_app_with_data();
    assert!(!app.info_open);

    app.open_info();
    assert!(app.info_open);
    assert!(app.info_detail.is_none()); // FakeProcessSource can't call FFI
    // But the method sets info_open regardless of detail success

    app.close_info();
    assert!(!app.info_open);
    assert!(app.info_detail.is_none());
}

#[test]
fn info_tab_navigation_with_set() {
    let (mut app, _) = create_app_with_data();
    assert_eq!(app.info_tab, 0);

    app.info_set_tab(1);
    assert_eq!(app.info_tab, 1);

    app.info_set_tab(3);
    assert_eq!(app.info_tab, 3);

    // Out of bounds ignored
    app.info_set_tab(5);
    assert_eq!(app.info_tab, 3);
}

#[test]
fn info_tab_cycles_forward() {
    let (mut app, _) = create_app_with_data();
    assert_eq!(app.info_tab, 0);

    app.info_next_tab();
    assert_eq!(app.info_tab, 1);

    app.info_next_tab();
    assert_eq!(app.info_tab, 2);

    app.info_next_tab();
    assert_eq!(app.info_tab, 3);

    // Wraps to 0
    app.info_next_tab();
    assert_eq!(app.info_tab, 0);
}

#[test]
fn info_tab_cycles_backward() {
    let (mut app, _) = create_app_with_data();
    assert_eq!(app.info_tab, 0);

    // Wraps to 3
    app.info_prev_tab();
    assert_eq!(app.info_tab, 3);

    app.info_prev_tab();
    assert_eq!(app.info_tab, 2);
}

#[test]
fn info_set_tab_resets_scroll() {
    let (mut app, _) = create_app_with_data();
    app.info_scroll = 5;
    app.info_env_selected = 3;

    app.info_set_tab(2);
    assert_eq!(app.info_scroll, 0);
    assert_eq!(app.info_env_selected, 0);
}

#[test]
fn info_scroll_on_non_env_tab() {
    let (mut app, _) = create_app_with_data();
    app.info_set_tab(0); // Overview

    app.info_scroll_down();
    assert_eq!(app.info_scroll, 1);
    app.info_scroll_down();
    assert_eq!(app.info_scroll, 2);
    app.info_scroll_up();
    assert_eq!(app.info_scroll, 1);
    app.info_scroll_up();
    assert_eq!(app.info_scroll, 0);
    // Does not underflow
    app.info_scroll_up();
    assert_eq!(app.info_scroll, 0);
}

#[test]
fn info_env_tab_moves_selection() {
    let (mut app, _) = create_app_with_data();
    app.info_set_tab(3); // Environment

    // Without detail, selection stays at 0 (no env to navigate)
    app.info_scroll_down();
    assert_eq!(app.info_env_selected, 0); // no detail loaded

    // With a mock detail
    app.info_detail = Some(prexp_ffi::ProcessDetail {
        pid: 100,
        ppid: 1,
        parent_name: "init".into(),
        name: "test".into(),
        path: "/bin/test".into(),
        cwd: "/".into(),
        user: "root".into(),
        uid: 0,
        state: prexp_ffi::ProcessState::Running,
        nice: 0,
        started_secs: 0,
        thread_count: 1,
        virtual_size: 0,
        memory_rss: 0,
        memory_phys: 0,
        cpu_time_ns: 0,
        fd_files: 0,
        fd_sockets: 0,
        fd_pipes: 0,
        fd_other: 0,
        fd_total: 0,
        faults: 0,
        context_switches: 0,
        syscalls_mach: 0,
        syscalls_unix: 0,
        disk_bytes_read: 0,
        disk_bytes_written: 0,
        network: Vec::new(),
        environment: vec![
            ("HOME".into(), "/root".into()),
            ("PATH".into(), "/usr/bin".into()),
            ("SHELL".into(), "/bin/bash".into()),
        ],
    });

    app.info_scroll_down();
    assert_eq!(app.info_env_selected, 1);
    app.info_scroll_down();
    assert_eq!(app.info_env_selected, 2);
    // Does not overflow
    app.info_scroll_down();
    assert_eq!(app.info_env_selected, 2);
    app.info_scroll_up();
    assert_eq!(app.info_env_selected, 1);
}

// -- History tracking --

#[test]
fn history_tracks_cpu_and_memory() {
    use prexp_app::tui::app::ProcessHistory;

    let mut history = ProcessHistory::new();
    history.push(10.0, 1000);
    history.push(20.0, 2000);
    history.push(30.0, 3000);

    assert_eq!(history.cpu.len(), 3);
    assert_eq!(history.memory.len(), 3);
    assert_eq!(*history.cpu.back().unwrap(), 30.0);
    assert_eq!(*history.memory.back().unwrap(), 3000);
}

#[test]
fn history_caps_at_max_samples() {
    use prexp_app::tui::app::ProcessHistory;

    let mut history = ProcessHistory::new();
    for i in 0..100 {
        history.push(i as f64, i * 1000);
    }

    assert_eq!(history.cpu.len(), 60); // MAX_SAMPLES
    assert_eq!(history.memory.len(), 60);
    // Oldest values were dropped
    assert_eq!(*history.cpu.front().unwrap(), 40.0);
}

#[test]
fn history_updated_on_refresh() {
    let (mut app, source) = create_app_with_data();

    // First refresh populates history
    app.refresh(&source);
    assert!(!app.process_history.is_empty());

    // Each process should have a history entry
    for snap in &app.snapshots {
        assert!(app.process_history.contains_key(&snap.pid));
    }
}

#[test]
fn history_removes_dead_processes() {
    let source1 = FakeProcessSource::new(sample_snapshots());
    let mut app = App::new(Duration::from_secs(2));
    app.refresh(&source1);
    assert!(app.process_history.contains_key(&100));

    // Second refresh with fewer processes
    let source2 = FakeProcessSource::new(vec![ProcessSnapshot {
        pid: 100,
        ppid: 1,
        name: "nginx".into(),
        thread_count: 8,
        memory_rss: 1024 * 1024 * 50,
        memory_phys: 1024 * 1024 * 30,
        cpu_time_ns: 1_000_000_000,
        faults: 0, context_switches: 0, syscalls_mach: 0, syscalls_unix: 0, disk_bytes_read: 0, disk_bytes_written: 0,
        state: prexp_ffi::ProcessState::Running,
        accessible: true,
        resources: Vec::new(),
    }]);
    app.refresh(&source2);

    // pid 200 and 300 should be removed from history
    assert!(app.process_history.contains_key(&100));
    assert!(!app.process_history.contains_key(&200));
    assert!(!app.process_history.contains_key(&300));
}

// -- Chart config --

#[test]
fn chart_config_all_disabled_by_default() {
    let config = ChartConfig::default();
    for chart in Chart::ALL {
        assert!(!config.is_enabled(*chart), "{:?} should be off by default", chart);
    }
}

#[test]
fn chart_config_toggle() {
    let (mut app, _) = create_app_with_data();
    app.open_chart_config();
    assert!(app.chart_config_open);

    // Default is off, toggle turns it on
    app.chart_config_toggle_selected();
    assert!(app.chart_config.is_enabled(Chart::ThreadCount));

    // Toggle again turns it off
    app.chart_config_toggle_selected();
    assert!(!app.chart_config.is_enabled(Chart::ThreadCount));
}

#[test]
fn chart_config_navigation() {
    let (mut app, _) = create_app_with_data();
    app.open_chart_config();
    assert_eq!(app.chart_config_selected, 0);

    app.chart_config_move_down();
    assert_eq!(app.chart_config_selected, 1);

    app.chart_config_move_up();
    assert_eq!(app.chart_config_selected, 0);

    app.chart_config_move_up();
    assert_eq!(app.chart_config_selected, 0);
}

#[test]
fn chart_config_close() {
    let (mut app, _) = create_app_with_data();
    app.open_chart_config();
    assert!(app.chart_config_open);
    app.close_chart_config();
    assert!(!app.chart_config_open);
}

#[test]
fn disabled_chart_skips_new_data() {
    let source = FakeProcessSource::new(sample_snapshots());
    let mut app = App::new(Duration::from_secs(2));

    // ThreadCount is off by default
    assert!(!app.chart_config.is_enabled(Chart::ThreadCount));

    app.refresh(&source);
    app.refresh(&source);

    // Thread history should be empty since chart was never enabled
    for history in app.process_history.values() {
        assert!(history.threads.is_empty(), "disabled chart should not collect data");
    }
}

#[test]
fn enabled_chart_collects_history() {
    let source = FakeProcessSource::new(sample_snapshots());
    let mut app = App::new(Duration::from_secs(2));

    // Enable FD count chart manually (off by default)
    let fd_idx = Chart::ALL.iter().position(|&c| c == Chart::FdCount).unwrap();
    app.chart_config.toggle(fd_idx);
    assert!(app.chart_config.is_enabled(Chart::FdCount));

    app.refresh(&source);

    // FD count history should have data
    for history in app.process_history.values() {
        assert!(!history.fd_count.is_empty(), "enabled chart should collect data");
    }
}
