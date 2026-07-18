//! The process table — a `rime::table` of one row per process with fd-count
//! columns. Stateless: it reads the app's data and reports clicks/scrolls back
//! as messages; the app owns selection and scroll offset.

use iced::{Color, Font, Length};
use prexp_core::models::ResourceKind;
use rime::theme::tokens;
use rime::widgets::{table, CellAlign, TableColumn};

use super::fmt;
use crate::app::{App, Message};

// Column order — must match the `col` arms in the cell factory below.
const COL_PID: usize = 0;
const COL_NAME: usize = 1;
const COL_CPU: usize = 2;
const COL_RSS: usize = 3;
const COL_PMEM: usize = 4;
const COL_THR: usize = 5;
const COL_FILES: usize = 6;
const COL_SOCKS: usize = 7;
const COL_PIPES: usize = 8;
const COL_TOTAL: usize = 9;

pub fn process_table(app: &App) -> iced::Element<'_, Message> {
    let procs = app.processes();

    let columns = vec![
        TableColumn::fixed("PID", 64.0).align(CellAlign::Right),
        TableColumn::fill("NAME"),
        TableColumn::fixed("CPU%", 64.0).align(CellAlign::Right),
        TableColumn::fixed("RSS", 80.0).align(CellAlign::Right),
        TableColumn::fixed("PMEM", 80.0).align(CellAlign::Right),
        TableColumn::fixed("THR", 52.0).align(CellAlign::Right),
        TableColumn::fixed("FILES", 64.0).align(CellAlign::Right),
        TableColumn::fixed("SOCKS", 64.0).align(CellAlign::Right),
        TableColumn::fixed("PIPES", 64.0).align(CellAlign::Right),
        TableColumn::fixed("TOTAL", 64.0).align(CellAlign::Right),
    ];

    // Resolve the PID-anchored selection to a current row index.
    let selected = app
        .selected_pid()
        .and_then(|pid| procs.iter().position(|p| p.pid == pid));

    // Palette colors captured up front (per rime's rule) for the cell grader.
    let p = tokens();
    let muted = p.muted;
    let accent = p.accent;
    let warn = p.warn;

    let cell = move |row: usize, col: usize| -> String {
        let Some(pr) = app.processes().get(row) else {
            return String::new();
        };
        match col {
            COL_PID => pr.pid.to_string(),
            COL_NAME => pr.name.clone(),
            COL_CPU => fmt::pct(app.cpu_percent(pr.pid)),
            COL_RSS => fmt::bytes(pr.memory.rss),
            COL_PMEM => fmt::bytes(pr.memory.phys),
            COL_THR => pr.activity.thread_count.to_string(),
            COL_FILES => pr.count_by_kind(&ResourceKind::File).to_string(),
            COL_SOCKS => pr.count_by_kind(&ResourceKind::Socket).to_string(),
            COL_PIPES => pr.count_by_kind(&ResourceKind::Pipe).to_string(),
            COL_TOTAL => pr.resources.len().to_string(),
            _ => String::new(),
        }
    };

    let cell_color = move |row: usize, col: usize| -> Option<Color> {
        let pr = app.processes().get(row)?;
        // Processes we couldn't fully read (permission denied) are dimmed whole.
        if !pr.accessible {
            return Some(muted);
        }
        // Grade the CPU column so busy processes stand out.
        if col == COL_CPU {
            let c = app.cpu_percent(pr.pid);
            if c >= 50.0 {
                return Some(warn);
            } else if c >= 10.0 {
                return Some(accent);
            }
        }
        None
    };

    table(procs.len(), columns, cell)
        .font(Font::MONOSPACE)
        .offset(app.scroll())
        .selected(selected)
        .cell_color(cell_color)
        .on_select(Message::SelectRow)
        .on_scroll(Message::Scrolled)
        .height(Length::Fill)
        .into()
}
