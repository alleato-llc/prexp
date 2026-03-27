use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;
use ratatui::widgets::{Block, Borders, Cell, Clear, Row, Table, TableState};
use ratatui::Frame;

use crate::tui::app::{App, FileKindFilter, FileSortField, InputMode};

use super::detail_rect;

pub fn draw(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.current_theme();
    let sort_label = match app.file_sort {
        FileSortField::ProcessCount => format!(" [procs {}]", app.file_sort_dir.arrow()),
        FileSortField::Filename => format!(" [filename {}]", app.file_sort_dir.arrow()),
    };

    let title = if app.input_mode == InputMode::Search {
        format!(" Files [/{}] ", app.search_text)
    } else if app.search_active {
        format!(
            " Files [/{}] — {} matches{} ",
            app.search_text,
            app.filtered_file_indices.len(),
            sort_label
        )
    } else {
        let kind_label = if app.file_kind_filter != FileKindFilter::All {
            format!(" [{}]", app.file_kind_filter.label())
        } else {
            String::new()
        };
        format!(
            " prexp — {} open files{}{} ",
            app.filtered_file_indices.len(),
            kind_label,
            sort_label
        )
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border_file));

    let arrow = app.file_sort_dir.arrow();
    let path_h = if app.file_sort == FileSortField::Filename {
        format!("PATH{}", arrow)
    } else {
        "PATH".into()
    };
    let procs_h = if app.file_sort == FileSortField::ProcessCount {
        format!("PROCS{}", arrow)
    } else {
        "PROCS".into()
    };

    let header = Row::new(vec![Cell::from(path_h), Cell::from(procs_h)])
        .style(Style::default().fg(t.header).add_modifier(Modifier::BOLD))
        .height(1);

    let rows: Vec<Row> = app
        .filtered_file_indices
        .iter()
        .map(|&i| {
            let entry = &app.file_entries[i];
            let path_style = if entry.path.starts_with("/dev/") {
                Style::default().fg(t.muted)
            } else {
                Style::default()
            };

            Row::new(vec![
                Cell::from(Span::styled(entry.path.clone(), path_style)),
                Cell::from(entry.openers.len().to_string()),
            ])
        })
        .collect();

    let widths = [Constraint::Min(40), Constraint::Length(6)];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().bg(t.highlight_bg).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    let mut state = TableState::default();
    if !app.filtered_file_indices.is_empty() {
        state.select(Some(app.file_selected_index));
    }

    frame.render_stateful_widget(table, area, &mut state);
}

pub fn draw_detail(frame: &mut Frame, app: &App) {
    let t = app.current_theme();
    let area = frame.area();
    let overlay = detail_rect(area);
    frame.render_widget(Clear, overlay);

    let entry = match app.selected_file_entry() {
        Some(e) => e,
        None => return,
    };

    let title = format!(
        " {} — {} process(es)  [y: copy, q/Esc: back] ",
        entry.path,
        entry.openers.len()
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border_file));

    let header = Row::new(vec![
        Cell::from("PID"),
        Cell::from("PROCESS"),
        Cell::from("FD"),
    ])
    .style(Style::default().fg(t.header).add_modifier(Modifier::BOLD))
    .height(1);

    let rows: Vec<Row> = entry
        .openers
        .iter()
        .map(|opener| {
            Row::new(vec![
                Cell::from(opener.pid.to_string()),
                Cell::from(opener.name.clone()),
                Cell::from(opener.descriptor.to_string()),
            ])
        })
        .collect();

    let widths = [Constraint::Length(8), Constraint::Min(25), Constraint::Length(6)];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().bg(t.highlight_bg).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    let mut state = TableState::default();
    state.select(Some(app.detail_selected));
    frame.render_stateful_widget(table, overlay, &mut state);
}
