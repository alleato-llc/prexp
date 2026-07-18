//! State for the reverse path lookup — find every process that has a given path
//! open (via `ProcessSource::find_by_path`).

use prexp_core::models::ProcessSnapshot;

/// The open reverse-lookup panel: the query, the last results, and progress.
#[derive(Default)]
pub struct LookupState {
    pub query: String,
    pub results: Vec<ProcessSnapshot>,
    /// A search is in flight.
    pub searching: bool,
    /// Whether a search has completed at least once (so the UI can tell "no
    /// results yet" from "no matches").
    pub searched: bool,
    pub error: Option<String>,
    pub scroll: f32,
}
