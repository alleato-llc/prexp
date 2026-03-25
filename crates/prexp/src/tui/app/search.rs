use std::io::Write;
use std::process::Command;

use prexp_core::error::PrexpError;
use prexp_core::source::ProcessSource;

use super::{App, InputMode, MainView};

impl App {
    pub fn enter_search_mode(&mut self) {
        self.input_mode = InputMode::Search;
        self.search_text.clear();
        self.search_active = false;
    }

    pub fn confirm_search(&mut self) {
        self.input_mode = InputMode::Normal;
        self.search_active = !self.search_text.is_empty();
    }

    pub fn clear_search(&mut self) {
        self.search_text.clear();
        self.search_active = false;
        self.apply_filter();
    }

    pub fn next_search_match(&mut self) {
        if !self.search_active {
            return;
        }
        match self.main_view {
            MainView::Processes => {
                if !self.filtered_indices.is_empty() {
                    self.selected_index =
                        (self.selected_index + 1) % self.filtered_indices.len();
                    self.update_process_anchor();
                }
            }
            MainView::Files => {
                if !self.filtered_file_indices.is_empty() {
                    self.file_selected_index =
                        (self.file_selected_index + 1) % self.filtered_file_indices.len();
                    self.update_file_anchor();
                }
            }
        }
    }

    pub fn enter_reverse_lookup_mode(&mut self) {
        self.input_mode = InputMode::ReverseLookup;
        self.reverse_lookup_text.clear();
        self.reverse_results.clear();
    }

    pub fn exit_input_mode(&mut self) {
        self.input_mode = InputMode::Normal;
    }

    pub fn perform_reverse_lookup(&mut self, source: &dyn ProcessSource) {
        match source.find_by_path(&self.reverse_lookup_text) {
            Ok(results) => {
                let count = results.len();
                self.reverse_results = results;
                self.status_message = Some(format!(
                    "{} process(es) have '{}' open",
                    count, self.reverse_lookup_text
                ));
            }
            Err(PrexpError::ProcessNotFound { .. }) => {
                self.reverse_results.clear();
                self.status_message = Some("No processes found".into());
            }
            Err(e) => {
                self.reverse_results.clear();
                self.status_message = Some(format!("Lookup failed: {}", e));
            }
        }
        self.input_mode = InputMode::Normal;
    }

    pub fn push_input_char(&mut self, c: char) {
        match self.input_mode {
            InputMode::Search => {
                self.search_text.push(c);
                self.apply_filter();
            }
            InputMode::ReverseLookup => {
                self.reverse_lookup_text.push(c);
            }
            InputMode::Normal => {}
        }
    }

    pub fn pop_input_char(&mut self) {
        match self.input_mode {
            InputMode::Search => {
                self.search_text.pop();
                self.apply_filter();
            }
            InputMode::ReverseLookup => {
                self.reverse_lookup_text.pop();
            }
            InputMode::Normal => {}
        }
    }

    pub fn yank_selected_path(&self) -> String {
        let path = if self.detail_open {
            match self.main_view {
                MainView::Processes => self
                    .selected_snapshot()
                    .and_then(|snap| snap.resources.get(self.detail_selected))
                    .and_then(|r| r.path.as_deref()),
                MainView::Files => self.selected_file_entry().map(|e| e.path.as_str()),
            }
        } else if self.main_view == MainView::Files {
            self.selected_file_entry().map(|e| e.path.as_str())
        } else {
            None
        };

        match path {
            Some(p) => match copy_to_clipboard(p) {
                Ok(()) => format!("Copied to clipboard: {}", p),
                Err(e) => format!("Copy failed: {}", e),
            },
            None => "No path to copy".into(),
        }
    }
}

pub fn copy_to_clipboard_pub(text: &str) -> Result<(), String> {
    copy_to_clipboard(text)
}

fn copy_to_clipboard(text: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let mut child = Command::new("pbcopy")
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn pbcopy: {}", e))?;

    #[cfg(target_os = "linux")]
    let mut child = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn xclip: {}", e))?;

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    return Err("clipboard not supported on this platform".into());

    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(text.as_bytes())
                .map_err(|e| format!("failed to write to clipboard: {}", e))?;
        }
        child
            .wait()
            .map_err(|e| format!("clipboard command failed: {}", e))?;
        Ok(())
    }
}
