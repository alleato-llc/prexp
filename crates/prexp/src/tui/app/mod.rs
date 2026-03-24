mod search;
mod sorting;
pub mod stats;
mod tree;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use prexp_core::models::ProcessSnapshot;
use prexp_core::source::ProcessSource;

// Re-export format_memory for use by ui module.
pub use stats::format_memory;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// An entry in the tree-ordered display list.
#[derive(Debug, Clone)]
pub struct TreeEntry {
    pub snapshot_index: usize,
    pub depth: usize,
    pub prefix: String,
}

/// A unique open file path with the processes that have it open.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub path: String,
    pub openers: Vec<FileOpener>,
}

/// A process that has a particular file open.
#[derive(Debug, Clone)]
pub struct FileOpener {
    pub pid: i32,
    pub name: String,
    pub descriptor: i32,
}

/// The main view mode (toggled with 'v').
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainView {
    Processes,
    Files,
}

/// Process list sort field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessSortField {
    Unsorted,
    Pid,
    Name,
    Total,
}

/// File list sort field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileSortField {
    ProcessCount,
    Filename,
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    pub fn toggle(self) -> Self {
        match self {
            SortDirection::Asc => SortDirection::Desc,
            SortDirection::Desc => SortDirection::Asc,
        }
    }

    pub fn arrow(self) -> &'static str {
        match self {
            SortDirection::Asc => "↑",
            SortDirection::Desc => "↓",
        }
    }
}

/// A configurable process list column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Cpu,
    Mem,
    Pmem,
    Thr,
    Files,
    Socks,
    Pipes,
    Other,
    Total,
}

impl Column {
    pub const ALL: &'static [Column] = &[
        Column::Cpu, Column::Mem, Column::Pmem, Column::Thr,
        Column::Files, Column::Socks, Column::Pipes, Column::Other, Column::Total,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Column::Cpu => "CPU%",
            Column::Mem => "MEM",
            Column::Pmem => "PMEM",
            Column::Thr => "THR",
            Column::Files => "FILES",
            Column::Socks => "SOCKS",
            Column::Pipes => "PIPES",
            Column::Other => "OTHER",
            Column::Total => "TOTAL",
        }
    }
}

/// Column visibility configuration. All enabled by default.
#[derive(Debug, Clone)]
pub struct ColumnConfig {
    pub enabled: Vec<bool>,
}

impl Default for ColumnConfig {
    fn default() -> Self {
        Self {
            enabled: vec![true; Column::ALL.len()],
        }
    }
}

impl ColumnConfig {
    pub fn is_enabled(&self, col: Column) -> bool {
        let idx = Column::ALL.iter().position(|&c| c == col).unwrap();
        self.enabled[idx]
    }

    pub fn toggle(&mut self, index: usize) {
        if index < self.enabled.len() {
            self.enabled[index] = !self.enabled[index];
        }
    }
}

/// Current input mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    ReverseLookup,
}

/// System-level statistics for the summary header.
#[derive(Debug, Clone)]
pub struct SystemStats {
    pub cpu_usage: Vec<f64>,
    pub memory: Option<prexp_ffi::MemoryInfo>,
    pub total_processes: usize,
    pub total_threads: i64,
    pub total_fds: usize,
}

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

pub struct App {
    pub snapshots: Vec<ProcessSnapshot>,

    // CPU% tracking
    pub(crate) prev_cpu_times: HashMap<i32, u64>,
    pub(crate) prev_refresh: Option<Instant>,
    pub cpu_percentages: HashMap<i32, f64>,
    pub num_cpus: usize,

    // System summary
    pub show_summary: bool,
    pub system_stats: SystemStats,
    pub(crate) prev_cpu_ticks: Vec<prexp_ffi::CpuTicks>,

    // Process view state
    pub filtered_indices: Vec<usize>,
    pub tree_entries: Vec<TreeEntry>,
    pub selected_index: usize,
    pub process_anchor: Option<i32>,

    // File view state
    pub file_entries: Vec<FileEntry>,
    pub filtered_file_indices: Vec<usize>,
    pub file_selected_index: usize,
    pub file_anchor: Option<String>,

    // View state
    pub main_view: MainView,
    pub detail_open: bool,
    pub input_mode: InputMode,
    pub search_text: String,
    pub search_active: bool,
    pub reverse_lookup_text: String,
    pub reverse_results: Vec<ProcessSnapshot>,
    pub should_quit: bool,
    pub show_all: bool,
    pub refresh_interval: Duration,
    pub last_refresh: Instant,
    pub status_message: Option<String>,

    // Sort state
    pub process_sort: ProcessSortField,
    pub process_sort_dir: SortDirection,
    pub file_sort: FileSortField,
    pub file_sort_dir: SortDirection,

    // Column configuration
    pub column_config: ColumnConfig,
    pub config_open: bool,
    pub config_selected: usize,

    // Theme
    pub theme_index: usize,
    pub theme_open: bool,

    // Help
    pub help_open: bool,
    pub help_scroll: usize,

    // Detail overlay state
    pub detail_selected: usize,
    pub detail_h_scroll: usize,
}

impl App {
    pub fn new(refresh_interval: Duration) -> Self {
        Self {
            snapshots: Vec::new(),
            prev_cpu_times: HashMap::new(),
            prev_refresh: None,
            cpu_percentages: HashMap::new(),
            num_cpus: std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1),
            show_summary: false,
            system_stats: SystemStats {
                cpu_usage: Vec::new(),
                memory: None,
                total_processes: 0,
                total_threads: 0,
                total_fds: 0,
            },
            prev_cpu_ticks: Vec::new(),
            filtered_indices: Vec::new(),
            tree_entries: Vec::new(),
            selected_index: 0,
            process_anchor: None,
            file_entries: Vec::new(),
            filtered_file_indices: Vec::new(),
            file_selected_index: 0,
            file_anchor: None,
            main_view: MainView::Processes,
            detail_open: false,
            input_mode: InputMode::Normal,
            search_text: String::new(),
            search_active: false,
            reverse_lookup_text: String::new(),
            reverse_results: Vec::new(),
            should_quit: false,
            show_all: false,
            refresh_interval,
            last_refresh: Instant::now(),
            status_message: None,
            process_sort: ProcessSortField::Unsorted,
            process_sort_dir: SortDirection::Asc,
            file_sort: FileSortField::ProcessCount,
            file_sort_dir: SortDirection::Desc,
            column_config: ColumnConfig::default(),
            config_open: false,
            config_selected: 0,
            theme_index: 0,
            theme_open: false,
            help_open: false,
            help_scroll: 0,
            detail_selected: 0,
            detail_h_scroll: 0,
        }
    }

    // -- Refresh --

    pub fn refresh(&mut self, source: &dyn ProcessSource) {
        match source.snapshot_all() {
            Ok(snapshots) => {
                self.compute_cpu_percentages(&snapshots);
                self.snapshots = snapshots;
                self.rebuild_all();
                if self.show_summary {
                    self.refresh_system_stats();
                }
                self.last_refresh = Instant::now();
                self.status_message = None;
            }
            Err(e) => {
                self.status_message = Some(format!("Refresh failed: {}", e));
            }
        }
    }

    pub fn needs_refresh(&self) -> bool {
        self.last_refresh.elapsed() >= self.refresh_interval
    }

    // -- Rebuild --

    fn rebuild_all(&mut self) {
        self.rebuild_process_list();
        self.rebuild_file_list();
    }

    fn rebuild_process_list(&mut self) {
        let visible: Vec<usize> = self
            .snapshots
            .iter()
            .enumerate()
            .filter(|(_, s)| self.show_all || s.accessible)
            .map(|(i, _)| i)
            .collect();

        if !self.search_text.is_empty() && self.main_view == MainView::Processes {
            let query = self.search_text.to_lowercase();
            self.filtered_indices = visible
                .into_iter()
                .filter(|&i| {
                    let s = &self.snapshots[i];
                    s.name.to_lowercase().contains(&query)
                        || s.pid.to_string().contains(&query)
                })
                .collect();
        } else {
            let visible_snapshots: Vec<ProcessSnapshot> =
                visible.iter().map(|&i| self.snapshots[i].clone()).collect();
            let tree_entries = tree::build_process_tree_sorted(
                &visible_snapshots,
                self.process_sort,
                self.process_sort_dir,
            );

            self.tree_entries = tree_entries
                .into_iter()
                .map(|mut e| {
                    e.snapshot_index = visible[e.snapshot_index];
                    e
                })
                .collect();
            self.filtered_indices = self.tree_entries.iter().map(|e| e.snapshot_index).collect();

            self.restore_process_anchor();
            return;
        }

        // Search mode: apply sort and build flat entries.
        self.sort_process_indices();
        self.tree_entries = self
            .filtered_indices
            .iter()
            .map(|&i| TreeEntry {
                snapshot_index: i,
                depth: 0,
                prefix: String::new(),
            })
            .collect();

        self.restore_process_anchor();
    }

    fn restore_process_anchor(&mut self) {
        if let Some(anchor_pid) = self.process_anchor {
            if let Some(pos) = self
                .filtered_indices
                .iter()
                .position(|&i| self.snapshots[i].pid == anchor_pid)
            {
                self.selected_index = pos;
            } else {
                self.selected_index = self
                    .selected_index
                    .min(self.filtered_indices.len().saturating_sub(1));
            }
        } else {
            self.selected_index = self
                .selected_index
                .min(self.filtered_indices.len().saturating_sub(1));
        }
        self.update_process_anchor();
    }

    fn rebuild_file_list(&mut self) {
        let mut path_map: HashMap<String, Vec<FileOpener>> = HashMap::new();

        for snap in &self.snapshots {
            if !self.show_all && !snap.accessible {
                continue;
            }
            for res in &snap.resources {
                if let Some(path) = &res.path {
                    path_map
                        .entry(path.clone())
                        .or_default()
                        .push(FileOpener {
                            pid: snap.pid,
                            name: snap.name.clone(),
                            descriptor: res.descriptor,
                        });
                }
            }
        }

        let mut entries: Vec<FileEntry> = path_map
            .into_iter()
            .map(|(path, openers)| FileEntry { path, openers })
            .collect();

        let dir = self.file_sort_dir;
        match self.file_sort {
            FileSortField::ProcessCount => {
                entries.sort_by(|a, b| {
                    let cmp = a.openers.len().cmp(&b.openers.len());
                    match dir {
                        SortDirection::Asc => cmp,
                        SortDirection::Desc => cmp.reverse(),
                    }
                });
            }
            FileSortField::Filename => {
                entries.sort_by(|a, b| {
                    let fa = filename_from_path(&a.path);
                    let fb = filename_from_path(&b.path);
                    let cmp = fa.cmp(fb).then_with(|| a.path.cmp(&b.path));
                    match dir {
                        SortDirection::Asc => cmp,
                        SortDirection::Desc => cmp.reverse(),
                    }
                });
            }
        }
        self.file_entries = entries;

        if !self.search_text.is_empty() && self.main_view == MainView::Files {
            let query = self.search_text.to_lowercase();
            self.filtered_file_indices = self
                .file_entries
                .iter()
                .enumerate()
                .filter(|(_, e)| e.path.to_lowercase().contains(&query))
                .map(|(i, _)| i)
                .collect();
        } else {
            self.filtered_file_indices = (0..self.file_entries.len()).collect();
        }

        if let Some(ref anchor_path) = self.file_anchor {
            if let Some(pos) = self
                .filtered_file_indices
                .iter()
                .position(|&i| self.file_entries[i].path == *anchor_path)
            {
                self.file_selected_index = pos;
            } else {
                self.file_selected_index = self
                    .file_selected_index
                    .min(self.filtered_file_indices.len().saturating_sub(1));
            }
        } else {
            self.file_selected_index = self
                .file_selected_index
                .min(self.filtered_file_indices.len().saturating_sub(1));
        }

        self.update_file_anchor();
    }

    pub(crate) fn update_process_anchor(&mut self) {
        self.process_anchor = self.selected_snapshot().map(|s| s.pid);
    }

    fn update_file_anchor(&mut self) {
        self.file_anchor = self.selected_file_entry().map(|e| e.path.clone());
    }

    // -- Show all --

    pub fn toggle_show_all(&mut self) {
        self.show_all = !self.show_all;
        self.rebuild_all();
        let mode = if self.show_all { "on" } else { "off" };
        self.status_message = Some(format!("Show all: {}", mode));
    }

    pub fn apply_filter(&mut self) {
        match self.main_view {
            MainView::Processes => self.rebuild_process_list(),
            MainView::Files => self.rebuild_file_list(),
        }
    }

    // -- Selection --

    pub fn selected_snapshot(&self) -> Option<&ProcessSnapshot> {
        self.filtered_indices
            .get(self.selected_index)
            .and_then(|&i| self.snapshots.get(i))
    }

    pub fn selected_file_entry(&self) -> Option<&FileEntry> {
        self.filtered_file_indices
            .get(self.file_selected_index)
            .and_then(|&i| self.file_entries.get(i))
    }

    pub fn toggle_view(&mut self) {
        self.main_view = match self.main_view {
            MainView::Processes => MainView::Files,
            MainView::Files => MainView::Processes,
        };
        self.search_text.clear();
        self.search_active = false;
        self.status_message = None;
    }

    // -- Navigation --

    pub fn move_up(&mut self) {
        if self.detail_open {
            if self.detail_selected > 0 {
                self.detail_selected -= 1;
            }
            return;
        }
        match self.main_view {
            MainView::Processes => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                    self.update_process_anchor();
                }
            }
            MainView::Files => {
                if self.file_selected_index > 0 {
                    self.file_selected_index -= 1;
                    self.update_file_anchor();
                }
            }
        }
    }

    pub fn move_down(&mut self) {
        if self.detail_open {
            let max = match self.main_view {
                MainView::Processes => self
                    .selected_snapshot()
                    .map(|s| s.resources.len())
                    .unwrap_or(0),
                MainView::Files => self
                    .selected_file_entry()
                    .map(|e| e.openers.len())
                    .unwrap_or(0),
            };
            if self.detail_selected < max.saturating_sub(1) {
                self.detail_selected += 1;
            }
            return;
        }
        match self.main_view {
            MainView::Processes => {
                if !self.filtered_indices.is_empty()
                    && self.selected_index < self.filtered_indices.len() - 1
                {
                    self.selected_index += 1;
                    self.update_process_anchor();
                }
            }
            MainView::Files => {
                if !self.filtered_file_indices.is_empty()
                    && self.file_selected_index < self.filtered_file_indices.len() - 1
                {
                    self.file_selected_index += 1;
                    self.update_file_anchor();
                }
            }
        }
    }

    // -- Detail overlay --

    pub fn scroll_left(&mut self) {
        if self.detail_open && self.detail_h_scroll > 0 {
            self.detail_h_scroll = self.detail_h_scroll.saturating_sub(4);
        }
    }

    pub fn scroll_right(&mut self) {
        if self.detail_open {
            self.detail_h_scroll += 4;
        }
    }

    pub fn open_detail(&mut self) {
        let has_content = match self.main_view {
            MainView::Processes => self.selected_snapshot().is_some(),
            MainView::Files => self.selected_file_entry().is_some(),
        };
        if has_content {
            self.detail_open = true;
            self.detail_selected = 0;
            self.detail_h_scroll = 0;
        }
    }

    pub fn close_detail(&mut self) {
        self.detail_open = false;
        self.detail_selected = 0;
        self.detail_h_scroll = 0;
    }

    // -- Theme picker --

    pub fn open_theme_picker(&mut self) {
        self.theme_open = true;
    }

    pub fn close_theme_picker(&mut self) {
        self.theme_open = false;
        let name = super::theme::THEMES[self.theme_index].name;
        self.status_message = Some(format!("Theme: {}", name));
    }

    pub fn theme_move_up(&mut self) {
        if self.theme_index > 0 {
            self.theme_index -= 1;
        }
    }

    pub fn theme_move_down(&mut self) {
        use super::theme::THEMES;
        if self.theme_index < THEMES.len() - 1 {
            self.theme_index += 1;
        }
    }

    pub fn current_theme(&self) -> &super::theme::Theme {
        &super::theme::THEMES[self.theme_index]
    }

    // -- Help --

    pub fn open_help(&mut self) {
        self.help_open = true;
        self.help_scroll = 0;
    }

    pub fn close_help(&mut self) {
        self.help_open = false;
    }

    pub fn help_scroll_up(&mut self) {
        if self.help_scroll > 0 {
            self.help_scroll -= 1;
        }
    }

    pub fn help_scroll_down(&mut self) {
        self.help_scroll += 1;
    }

    // -- Column config --

    pub fn open_config(&mut self) {
        self.config_open = true;
        self.config_selected = 0;
    }

    pub fn close_config(&mut self) {
        self.config_open = false;
    }

    pub fn config_move_up(&mut self) {
        if self.config_selected > 0 {
            self.config_selected -= 1;
        }
    }

    pub fn config_move_down(&mut self) {
        if self.config_selected < Column::ALL.len() - 1 {
            self.config_selected += 1;
        }
    }

    pub fn config_toggle_selected(&mut self) {
        self.column_config.toggle(self.config_selected);
    }
}

fn filename_from_path(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}
