use prexp_core::models::{ProcessDetail, ProcessSnapshot};

use super::info::InfoTab;
use super::signal::Signal;
use super::sort::SortField;
use super::system::Sample;

/// Everything that can change the app's state. iced clones messages, hence the
/// `Clone` derive; the [`Sample`] carries the full snapshot set plus system
/// metrics by value from the background task.
#[derive(Debug, Clone)]
pub enum Message {
    /// The refresh timer fired — kick off a background sample (unless one is
    /// already in flight).
    Tick,
    /// A background sample landed (or failed).
    Refreshed(Result<Sample, String>),
    /// A table row was clicked — select the process it maps to.
    SelectRow(usize),
    /// The sort field changed via the dropdown.
    SortBy(SortField),
    /// Flip the sort direction.
    ToggleReverse,
    /// Switch between the light and dark palettes.
    ToggleTheme,
    /// Show/hide the system-stats header.
    ToggleStats,
    /// The process table was scrolled to a new pixel offset.
    Scrolled(f32),
    /// The detail pane's resource table was scrolled to a new pixel offset.
    DetailScrolled(f32),
    /// Dismiss the error banner.
    DismissError,
    /// Open the info panel for the selected process (kicks off a detail fetch).
    OpenInfo,
    /// Close the info panel.
    CloseInfo,
    /// Switch the info panel's active tab.
    SelectInfoTab(InfoTab),
    /// The background `process_detail` fetch landed (or failed).
    InfoLoaded(Result<ProcessDetail, String>),
    /// The info panel's active tab table was scrolled.
    InfoScrolled(f32),
    /// Open the send-signal prompt for the selected process.
    OpenSignal,
    /// Pick which signal to send.
    SelectSignal(Signal),
    /// Send the chosen signal to the process.
    ConfirmSignal,
    /// Dismiss the send-signal prompt without sending.
    CancelSignal,
    /// Dismiss the transient outcome notice.
    DismissNotice,
    /// Open the reverse path-lookup panel.
    OpenLookup,
    /// Close the reverse path-lookup panel.
    CloseLookup,
    /// The lookup query text changed.
    LookupQuery(String),
    /// Run the reverse lookup for the current query.
    RunLookup,
    /// The background `find_by_path` search landed (or failed).
    LookupLoaded(Result<Vec<ProcessSnapshot>, String>),
    /// The lookup results table was scrolled.
    LookupScrolled(f32),
}
