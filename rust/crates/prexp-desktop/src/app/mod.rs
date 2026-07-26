//! Application state and the iced update/subscription/theme hooks. The view is
//! delegated to the [`crate::view`] module; this file owns *what the app knows*
//! and *how messages change it*.

mod cpu;
pub(crate) mod info;
pub(crate) mod lookup;
pub(crate) mod message;
pub(crate) mod signal;
pub(crate) mod sort;
pub(crate) mod system;

pub use info::{InfoState, InfoTab};
pub use lookup::LookupState;
pub use message::Message;
pub use signal::{Signal, SignalPrompt};
pub use sort::SortField;
pub use system::Sample;

use std::sync::Arc;
use std::time::Duration;

use iced::{Subscription, Task};
use prexp_core::models::ProcessSnapshot;
use prexp_core::source::ProcessSource;
use rime::theme::ThemeChoice;

use cpu::CpuTracker;
use sort::sort_processes;
use system::SystemStats;

pub struct App {
    source: Arc<dyn ProcessSource + Send + Sync>,
    interval: Duration,
    theme: ThemeChoice,

    /// The latest completed snapshot, kept sorted by the active field.
    processes: Vec<ProcessSnapshot>,
    cpu: CpuTracker,
    sort_field: SortField,
    reverse: bool,
    /// Selection is anchored by PID so the highlight survives re-sorts and
    /// refreshes even as row indices shift.
    selected_pid: Option<i32>,
    scroll: f32,
    /// Vertical scroll of the detail pane's resource table (its own offset,
    /// reset whenever the selection changes).
    detail_scroll: f32,

    /// Rolling system-wide metrics for the header (per-core CPU, memory, load,
    /// battery) and whether that header is shown.
    system: SystemStats,
    show_stats: bool,

    /// The open info panel, if any (a modal overlay over the main view).
    info: Option<InfoState>,
    /// The pending send-signal confirmation, if any.
    signal_prompt: Option<SignalPrompt>,
    /// The open reverse path-lookup panel, if any.
    lookup: Option<LookupState>,
    /// A transient success notice (e.g. "Sent SIGTERM to …").
    notice: Option<String>,

    /// A snapshot is in flight — guards against stacking refreshes when one run
    /// outlasts the timer interval.
    refreshing: bool,
    error: Option<String>,
}

impl App {
    /// Build the initial state and fire the first snapshot immediately, so the
    /// table isn't empty until the first timer tick.
    pub fn boot(
        source: Arc<dyn ProcessSource + Send + Sync>,
        interval: Duration,
    ) -> (Self, Task<Message>) {
        // The CPU cluster topology is static; read it once here.
        let mut system = SystemStats::default();
        system.set_perf_levels(source.cpu_perf_levels().unwrap_or_default());

        let app = App {
            source: source.clone(),
            interval,
            theme: ThemeChoice::Dark,
            processes: Vec::new(),
            cpu: CpuTracker::default(),
            sort_field: SortField::Cpu,
            reverse: false,
            selected_pid: None,
            scroll: 0.0,
            detail_scroll: 0.0,
            system,
            show_stats: true,
            info: None,
            signal_prompt: None,
            lookup: None,
            notice: None,
            refreshing: true,
            error: None,
        };
        (app, sample_task(source))
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                if !self.refreshing {
                    self.refreshing = true;
                    return sample_task(self.source.clone());
                }
            }
            Message::Refreshed(Ok(sample)) => {
                self.refreshing = false;
                self.error = None;
                self.cpu.update(&sample.processes);
                self.system.update(&sample);
                self.processes = sample.processes;
                self.resort();
            }
            Message::Refreshed(Err(e)) => {
                self.refreshing = false;
                self.error = Some(e);
            }
            Message::SelectRow(row) => {
                self.selected_pid = self.processes.get(row).map(|p| p.pid);
                self.detail_scroll = 0.0;
            }
            Message::DetailScrolled(offset) => self.detail_scroll = offset,
            Message::SortBy(field) => {
                self.sort_field = field;
                self.resort();
            }
            Message::ToggleReverse => {
                self.reverse = !self.reverse;
                self.resort();
            }
            Message::ToggleTheme => self.theme = self.theme.toggled(),
            Message::ToggleStats => self.show_stats = !self.show_stats,
            Message::Scrolled(offset) => self.scroll = offset,
            Message::DismissError => self.error = None,
            Message::OpenInfo => {
                if let Some(proc) = self.selected_process() {
                    let pid = proc.pid;
                    let parent = self.parent_name(proc.ppid);
                    self.info = Some(InfoState::loading(pid));
                    return info_task(self.source.clone(), pid, parent);
                }
            }
            Message::CloseInfo => self.info = None,
            Message::SelectInfoTab(tab) => {
                if let Some(info) = &mut self.info {
                    info.tab = tab;
                    info.scroll = 0.0;
                }
            }
            Message::InfoLoaded(result) => {
                if let Some(info) = &mut self.info {
                    match result {
                        Ok(detail) => {
                            info.detail = Some(detail);
                            info.error = None;
                        }
                        Err(e) => info.error = Some(e),
                    }
                }
            }
            Message::InfoScrolled(offset) => {
                if let Some(info) = &mut self.info {
                    info.scroll = offset;
                }
            }
            Message::OpenSignal => {
                if let Some(proc) = self.selected_process() {
                    self.signal_prompt = Some(SignalPrompt::new(proc.pid, proc.name.clone()));
                }
            }
            Message::SelectSignal(sig) => {
                if let Some(prompt) = &mut self.signal_prompt {
                    prompt.signal = sig;
                }
            }
            Message::ConfirmSignal => {
                if let Some(prompt) = self.signal_prompt.take() {
                    // `kill(2)` is a fast syscall, so it's fine to run inline.
                    match self.source.kill_process(prompt.pid, prompt.signal.number()) {
                        Ok(()) => {
                            self.notice = Some(format!(
                                "Sent {} to {} (pid {})",
                                prompt.signal.name(),
                                prompt.name,
                                prompt.pid
                            ));
                        }
                        Err(e) => self.error = Some(e.to_string()),
                    }
                }
            }
            Message::CancelSignal => self.signal_prompt = None,
            Message::DismissNotice => self.notice = None,
            Message::OpenLookup => self.lookup = Some(LookupState::default()),
            Message::CloseLookup => self.lookup = None,
            Message::LookupQuery(q) => {
                if let Some(lookup) = &mut self.lookup {
                    lookup.query = q;
                }
            }
            Message::RunLookup => {
                if let Some(lookup) = &mut self.lookup {
                    let query = lookup.query.trim().to_string();
                    if !query.is_empty() {
                        lookup.searching = true;
                        lookup.error = None;
                        return lookup_task(self.source.clone(), query);
                    }
                }
            }
            Message::LookupLoaded(result) => {
                if let Some(lookup) = &mut self.lookup {
                    lookup.searching = false;
                    lookup.searched = true;
                    match result {
                        Ok(results) => {
                            lookup.results = results;
                            lookup.error = None;
                        }
                        Err(e) => lookup.error = Some(e),
                    }
                }
            }
            Message::LookupScrolled(offset) => {
                if let Some(lookup) = &mut self.lookup {
                    lookup.scroll = offset;
                }
            }
        }
        Task::none()
    }

    fn resort(&mut self) {
        sort_processes(&mut self.processes, self.sort_field, &self.cpu, self.reverse);
    }

    /// The name of the process with `ppid` in the latest snapshot (empty if it's
    /// not visible — e.g. pid 0 / kernel_task).
    fn parent_name(&self, ppid: i32) -> String {
        self.processes
            .iter()
            .find(|p| p.pid == ppid)
            .map(|p| p.name.clone())
            .unwrap_or_default()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        iced::time::every(self.interval).map(|_| Message::Tick)
    }

    pub fn theme(&self) -> iced::Theme {
        self.theme.theme()
    }

    pub fn view(&self) -> iced::Element<'_, Message> {
        crate::view::view(self)
    }

    // --- read-only accessors (used by the view module and integration tests) ---

    pub fn processes(&self) -> &[ProcessSnapshot] {
        &self.processes
    }
    /// The currently selected process, resolved from the PID anchor, if it is
    /// still present in the latest snapshot.
    pub fn selected_process(&self) -> Option<&ProcessSnapshot> {
        self.selected_pid
            .and_then(|pid| self.processes.iter().find(|p| p.pid == pid))
    }
    pub fn cpu_percent(&self, pid: i32) -> f64 {
        self.cpu.percent(pid)
    }
    pub(crate) fn theme_choice(&self) -> ThemeChoice {
        self.theme
    }
    pub(crate) fn show_stats(&self) -> bool {
        self.show_stats
    }
    /// The rolling system metrics for the header.
    pub fn system(&self) -> &SystemStats {
        &self.system
    }
    /// The open info panel, if any.
    pub fn info(&self) -> Option<&InfoState> {
        self.info.as_ref()
    }
    /// The pending send-signal prompt, if any.
    pub fn signal_prompt(&self) -> Option<&SignalPrompt> {
        self.signal_prompt.as_ref()
    }
    /// The open reverse-lookup panel, if any.
    pub fn lookup(&self) -> Option<&LookupState> {
        self.lookup.as_ref()
    }
    /// A transient success notice, if any.
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }
    pub fn sort_field(&self) -> SortField {
        self.sort_field
    }
    pub fn reverse(&self) -> bool {
        self.reverse
    }
    pub fn selected_pid(&self) -> Option<i32> {
        self.selected_pid
    }
    pub(crate) fn scroll(&self) -> f32 {
        self.scroll
    }
    pub(crate) fn detail_scroll(&self) -> f32 {
        self.detail_scroll
    }
    pub(crate) fn is_refreshing(&self) -> bool {
        self.refreshing
    }
    pub(crate) fn interval(&self) -> Duration {
        self.interval
    }
    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

/// A `Task` that gathers one [`Sample`] on a background thread — the fd-enumerating
/// `snapshot_all` plus the cheap system metrics — and reports it as
/// [`Message::Refreshed`]. Keeping it off the update thread is what keeps the UI
/// responsive while the FFI walks every process. Only `snapshot_all` can fail the
/// whole refresh; the system metrics degrade to `None` when unavailable.
fn sample_task(source: Arc<dyn ProcessSource + Send + Sync>) -> Task<Message> {
    Task::perform(
        async move {
            let processes = source.snapshot_all().map_err(|e| e.to_string())?;
            Ok(Sample {
                processes,
                cpu_ticks: source.cpu_ticks().unwrap_or_default(),
                memory: source.memory_info().ok(),
                load: source.system_load_average().ok(),
                battery: source.system_battery().ok(),
            })
        },
        Message::Refreshed,
    )
}

/// A `Task` that fetches a process's [`ProcessDetail`](prexp_core::models::ProcessDetail)
/// off-thread for the info panel and reports it as [`Message::InfoLoaded`].
fn info_task(
    source: Arc<dyn ProcessSource + Send + Sync>,
    pid: i32,
    parent_name: String,
) -> Task<Message> {
    Task::perform(
        async move {
            source
                .process_detail(pid, &parent_name)
                .map_err(|e| e.to_string())
        },
        Message::InfoLoaded,
    )
}

/// A `Task` that runs the reverse path lookup (`find_by_path`) off-thread and
/// reports it as [`Message::LookupLoaded`].
fn lookup_task(source: Arc<dyn ProcessSource + Send + Sync>, query: String) -> Task<Message> {
    Task::perform(
        async move { source.find_by_path(&query).map_err(|e| e.to_string()) },
        Message::LookupLoaded,
    )
}
