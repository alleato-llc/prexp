use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Table, TableState};
use ratatui::Frame;

use prexp_core::models::ResourceKind;

use crate::tui::app::{self, App, Column, InputMode, ProcessSortField};

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.current_theme();
    let cfg = &app.column_config;
    let sort_label = match app.process_sort {
        ProcessSortField::Unsorted => String::new(),
        ProcessSortField::Pid => format!(" [pid {}]", app.process_sort_dir.arrow()),
        ProcessSortField::Name => format!(" [name {}]", app.process_sort_dir.arrow()),
        ProcessSortField::Total => format!(" [total {}]", app.process_sort_dir.arrow()),
    };

    let title = if app.input_mode == InputMode::Search {
        format!(" Processes [/{}] ", app.search_text)
    } else if app.search_active {
        format!(
            " Processes [/{}] — {} matches{} ",
            app.search_text,
            app.filtered_indices.len(),
            sort_label
        )
    } else {
        format!(
            " prexp — {} processes{} ",
            app.filtered_indices.len(),
            sort_label
        )
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border_process));

    let arrow = app.process_sort_dir.arrow();
    let pid_h = if app.process_sort == ProcessSortField::Pid {
        format!("PID{}", arrow)
    } else {
        "PID".into()
    };
    let name_h = if app.process_sort == ProcessSortField::Name {
        format!("NAME{}", arrow)
    } else {
        "NAME".into()
    };
    let total_h = if app.process_sort == ProcessSortField::Total {
        format!("TOTAL{}", arrow)
    } else {
        "TOTAL".into()
    };

    let mut header_cells = vec![Cell::from(pid_h), Cell::from(name_h)];
    let mut widths: Vec<Constraint> = vec![Constraint::Length(8), Constraint::Min(25)];

    if cfg.is_enabled(Column::Cpu) { header_cells.push(Cell::from("CPU%")); widths.push(Constraint::Length(6)); }
    if cfg.is_enabled(Column::Mem) { header_cells.push(Cell::from("MEM")); widths.push(Constraint::Length(7)); }
    if cfg.is_enabled(Column::Pmem) { header_cells.push(Cell::from("PMEM")); widths.push(Constraint::Length(7)); }
    if cfg.is_enabled(Column::Thr) { header_cells.push(Cell::from("THR")); widths.push(Constraint::Length(4)); }
    if cfg.is_enabled(Column::Files) { header_cells.push(Cell::from("FILES")); widths.push(Constraint::Length(6)); }
    if cfg.is_enabled(Column::Socks) { header_cells.push(Cell::from("SOCKS")); widths.push(Constraint::Length(6)); }
    if cfg.is_enabled(Column::Pipes) { header_cells.push(Cell::from("PIPES")); widths.push(Constraint::Length(6)); }
    if cfg.is_enabled(Column::Other) { header_cells.push(Cell::from("OTHER")); widths.push(Constraint::Length(6)); }
    if cfg.is_enabled(Column::Total) { header_cells.push(Cell::from(total_h)); widths.push(Constraint::Length(6)); }

    let header = Row::new(header_cells)
        .style(Style::default().fg(t.header).add_modifier(Modifier::BOLD))
        .height(1);

    let rows: Vec<Row> = app
        .tree_entries
        .iter()
        .map(|entry| {
            let snap = &app.snapshots[entry.snapshot_index];
            let display_name = format!("{}{}", entry.prefix, snap.name);

            let row_style = if !snap.accessible {
                Style::default().fg(t.muted)
            } else {
                Style::default()
            };

            let mut cells = vec![Cell::from(snap.pid.to_string()), Cell::from(display_name)];

            if cfg.is_enabled(Column::Cpu) {
                let pct = app.cpu_percentages.get(&snap.pid).copied().unwrap_or(0.0);
                cells.push(Cell::from(format!("{:.1}", pct)));
            }
            if cfg.is_enabled(Column::Mem) {
                cells.push(Cell::from(app::format_memory(snap.memory_rss)));
            }
            if cfg.is_enabled(Column::Pmem) {
                cells.push(Cell::from(if snap.memory_phys > 0 {
                    app::format_memory(snap.memory_phys)
                } else {
                    "-".into()
                }));
            }
            if cfg.is_enabled(Column::Thr) {
                cells.push(Cell::from(snap.thread_count.to_string()));
            }
            if cfg.is_enabled(Column::Files) {
                let n = snap.count_by_kind(&ResourceKind::File)
                    + snap.count_by_kind(&ResourceKind::Device);
                cells.push(Cell::from(n.to_string()));
            }
            if cfg.is_enabled(Column::Socks) {
                cells.push(Cell::from(snap.count_by_kind(&ResourceKind::Socket).to_string()));
            }
            if cfg.is_enabled(Column::Pipes) {
                cells.push(Cell::from(snap.count_by_kind(&ResourceKind::Pipe).to_string()));
            }
            if cfg.is_enabled(Column::Other) {
                let n = snap.count_by_kind(&ResourceKind::Kqueue)
                    + snap.count_by_kind(&ResourceKind::Unknown);
                cells.push(Cell::from(n.to_string()));
            }
            if cfg.is_enabled(Column::Total) {
                cells.push(Cell::from(snap.resources.len().to_string()));
            }

            Row::new(cells).style(row_style)
        })
        .collect();

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().bg(t.highlight_bg).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    let mut state = TableState::default();
    if !app.filtered_indices.is_empty() {
        state.select(Some(app.selected_index));
    }

    frame.render_stateful_widget(table, area, &mut state);
}
