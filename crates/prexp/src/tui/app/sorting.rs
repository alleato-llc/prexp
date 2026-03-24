use super::{App, FileSortField, MainView, ProcessSortField, SortDirection};

impl App {
    pub fn cycle_sort(&mut self) {
        match self.main_view {
            MainView::Processes => {
                let (next_field, default_dir) = match self.process_sort {
                    ProcessSortField::Unsorted => (ProcessSortField::Pid, SortDirection::Asc),
                    ProcessSortField::Pid => (ProcessSortField::Name, SortDirection::Asc),
                    ProcessSortField::Name => (ProcessSortField::Total, SortDirection::Desc),
                    ProcessSortField::Total => (ProcessSortField::Unsorted, SortDirection::Asc),
                };
                self.process_sort = next_field;
                self.process_sort_dir = default_dir;
                self.rebuild_process_list();
                self.status_message = Some(self.sort_status_text());
            }
            MainView::Files => {
                let (next_field, default_dir) = match self.file_sort {
                    FileSortField::ProcessCount => (FileSortField::Filename, SortDirection::Asc),
                    FileSortField::Filename => (FileSortField::ProcessCount, SortDirection::Desc),
                };
                self.file_sort = next_field;
                self.file_sort_dir = default_dir;
                self.rebuild_file_list();
                self.status_message = Some(self.sort_status_text());
            }
        }
    }

    pub fn reverse_sort(&mut self) {
        match self.main_view {
            MainView::Processes => {
                if self.process_sort != ProcessSortField::Unsorted {
                    self.process_sort_dir = self.process_sort_dir.toggle();
                    self.rebuild_process_list();
                    self.status_message = Some(self.sort_status_text());
                }
            }
            MainView::Files => {
                self.file_sort_dir = self.file_sort_dir.toggle();
                self.rebuild_file_list();
                self.status_message = Some(self.sort_status_text());
            }
        }
    }

    pub(super) fn sort_status_text(&self) -> String {
        match self.main_view {
            MainView::Processes => match self.process_sort {
                ProcessSortField::Unsorted => "Sort: tree (unsorted)".into(),
                ProcessSortField::Pid => format!("Sort: PID {}", self.process_sort_dir.arrow()),
                ProcessSortField::Name => format!("Sort: name {}", self.process_sort_dir.arrow()),
                ProcessSortField::Total => format!("Sort: total {}", self.process_sort_dir.arrow()),
            },
            MainView::Files => match self.file_sort {
                FileSortField::ProcessCount => format!("Sort: procs {}", self.file_sort_dir.arrow()),
                FileSortField::Filename => format!("Sort: filename {}", self.file_sort_dir.arrow()),
            },
        }
    }

    pub(super) fn sort_process_indices(&mut self) {
        if self.process_sort == ProcessSortField::Unsorted {
            return;
        }
        let snapshots = &self.snapshots;
        let field = self.process_sort;
        let dir = self.process_sort_dir;

        self.filtered_indices.sort_by(|&a, &b| {
            let sa = &snapshots[a];
            let sb = &snapshots[b];
            let cmp = match field {
                ProcessSortField::Pid => sa.pid.cmp(&sb.pid),
                ProcessSortField::Name => sa.name.to_lowercase().cmp(&sb.name.to_lowercase()),
                ProcessSortField::Total => sa.resources.len().cmp(&sb.resources.len()),
                ProcessSortField::Unsorted => unreachable!(),
            };
            match dir {
                SortDirection::Asc => cmp,
                SortDirection::Desc => cmp.reverse(),
            }
        });
    }
}
