//! State for the process info panel — a modal overlay with four tabs
//! (Overview / Resources / Network / Environment) over a process's
//! [`ProcessDetail`], fetched off-thread when the panel opens.

use prexp_core::models::ProcessDetail;

/// The info panel's four tabs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoTab {
    Overview,
    Resources,
    Network,
    Environment,
}

impl InfoTab {
    pub const ALL: [InfoTab; 4] = [
        InfoTab::Overview,
        InfoTab::Resources,
        InfoTab::Network,
        InfoTab::Environment,
    ];

    pub fn label(self) -> &'static str {
        match self {
            InfoTab::Overview => "Overview",
            InfoTab::Resources => "Resources",
            InfoTab::Network => "Network",
            InfoTab::Environment => "Environment",
        }
    }
}

/// The open info panel: which process it's for, the active tab, and the fetched
/// detail (`None` while the background fetch is in flight).
pub struct InfoState {
    pub pid: i32,
    pub tab: InfoTab,
    pub detail: Option<ProcessDetail>,
    pub error: Option<String>,
    /// Scroll offset of the active tab's table (Network / Environment); reset when
    /// the tab changes.
    pub scroll: f32,
}

impl InfoState {
    /// A freshly-opened panel for `pid`, on the Overview tab, awaiting its detail.
    pub fn loading(pid: i32) -> Self {
        Self {
            pid,
            tab: InfoTab::Overview,
            detail: None,
            error: None,
            scroll: 0.0,
        }
    }
}
