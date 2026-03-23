use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState};
use ratatui::Frame;

use prexp_core::models::ResourceKind;

use super::app::{self, App, Column, FileSortField, InputMode, MainView, ProcessSortField};
use super::theme::Theme;

pub fn draw(frame: &mut Frame, app: &App) {
    let summary_height = if app.show_summary {
        summary_lines_for_width(app, frame.area().width.saturating_sub(2)) as u16 + 2
    } else {
        0
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(summary_height),
            Constraint::Min(5),
            Constraint::Length(1),
        ])
        .split(frame.area());

    if app.show_summary {
        draw_summary(frame, app, chunks[0]);
    }

    match app.main_view {
        MainView::Processes => draw_process_list(frame, app, chunks[1]),
        MainView::Files => draw_file_list(frame, app, chunks[1]),
    }

    draw_status_bar(frame, app, chunks[2]);

    if app.help_open {
        draw_help(frame, app);
    } else if app.theme_open {
        draw_theme_picker(frame, app);
    } else if app.config_open {
        draw_config_overlay(frame, app);
    } else if app.detail_open {
        match app.main_view {
            MainView::Processes => draw_process_detail(frame, app),
            MainView::Files => draw_file_detail(frame, app),
        }
    }
}

// ---------------------------------------------------------------------------
// System summary header
// ---------------------------------------------------------------------------

/// Calculate CPU layout: (cpus_per_row, bar_width, cpu_rows).
fn cpu_layout(cpu_count: usize, width: u16) -> (usize, usize, usize) {
    if cpu_count == 0 {
        return (1, 8, 1);
    }
    let max_rows = 4usize;
    // Each CPU entry: " XX " (4) + bar + " XXX.X%" (7) = 11 + bar_width
    let fixed_per_cpu = 11usize;
    let min_bar = 4usize;
    let usable = width as usize;

    // Try to fit as many per row as possible, capped so we don't exceed max_rows.
    let cpus_per_row = ((cpu_count + max_rows - 1) / max_rows).max(1);

    // How wide is the bar given cpus_per_row?
    let entry_width = if cpus_per_row > 0 {
        usable / cpus_per_row
    } else {
        usable
    };
    let bar_width = entry_width.saturating_sub(fixed_per_cpu).max(min_bar);

    let cpu_rows = (cpu_count + cpus_per_row - 1) / cpus_per_row;
    (cpus_per_row, bar_width, cpu_rows)
}

/// Calculate how many content lines the summary needs for a given width.
fn summary_lines_for_width(app: &App, width: u16) -> usize {
    let cpu_count = app.system_stats.cpu_usage.len();
    let (_, _, cpu_rows) = cpu_layout(cpu_count, width);
    cpu_rows + 2 // CPU rows + memory row + stats row
}

fn draw_summary(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.current_theme();
    let stats = &app.system_stats;

    let block = Block::default()
        .title(" System ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border_process));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let mut lines: Vec<Line> = Vec::new();

    if !stats.cpu_usage.is_empty() {
        let (cpus_per_row, bar_width, _) = cpu_layout(stats.cpu_usage.len(), inner.width);
        for (chunk_idx, chunk) in stats.cpu_usage.chunks(cpus_per_row).enumerate() {
            let spans: Vec<Span> = chunk
                .iter()
                .enumerate()
                .flat_map(|(i, &pct)| {
                    let core_idx = chunk_idx * cpus_per_row + i;
                    let bar = make_bar(pct, bar_width);
                    vec![
                        Span::styled(
                            format!(" {:>2} ", core_idx),
                            Style::default().fg(t.muted),
                        ),
                        Span::styled(bar, Style::default().fg(t.accent)),
                        Span::raw(format!(" {:>5.1}%", pct)),
                    ]
                })
                .collect();
            lines.push(Line::from(spans));
        }
    } else {
        lines.push(Line::from(Span::styled(
            " CPU: no data (waiting for next refresh)",
            Style::default().fg(t.muted),
        )));
    }

    // Memory.
    if let Some(mem) = &stats.memory {
        let pct = if mem.total > 0 {
            (mem.used as f64 / mem.total as f64) * 100.0
        } else {
            0.0
        };
        // " MEM " (5) + bar + " XX.XG / XX.XG (XX%)" (~25) = 30 + bar
        let mem_bar_width = (inner.width as usize).saturating_sub(30).max(8);
        let bar = make_bar(pct, mem_bar_width);
        lines.push(Line::from(vec![
            Span::styled(" MEM ", Style::default().fg(t.muted)),
            Span::styled(bar, Style::default().fg(t.accent)),
            Span::raw(format!(
                " {} / {} ({:.0}%)",
                format_mem(mem.used),
                format_mem(mem.total),
                pct
            )),
        ]));
    }

    // Process/thread/fd totals.
    lines.push(Line::from(vec![
        Span::styled(
            format!(
                " {} processes   {} threads   {} open fds",
                stats.total_processes, stats.total_threads, stats.total_fds
            ),
            Style::default().fg(t.header),
        ),
    ]));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

/// Render a progress bar using Unicode block characters.
fn make_bar(pct: f64, width: usize) -> String {
    let filled = ((pct / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

fn format_mem(bytes: u64) -> String {
    super::app::format_memory(bytes)
}

// ---------------------------------------------------------------------------
// Process list view
// ---------------------------------------------------------------------------

fn draw_process_list(frame: &mut Frame, app: &App, area: Rect) {
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

    if cfg.is_enabled(Column::Cpu) {
        header_cells.push(Cell::from("CPU%"));
        widths.push(Constraint::Length(6));
    }
    if cfg.is_enabled(Column::Mem) {
        header_cells.push(Cell::from("MEM"));
        widths.push(Constraint::Length(7));
    }
    if cfg.is_enabled(Column::Pmem) {
        header_cells.push(Cell::from("PMEM"));
        widths.push(Constraint::Length(7));
    }
    if cfg.is_enabled(Column::Thr) {
        header_cells.push(Cell::from("THR"));
        widths.push(Constraint::Length(4));
    }
    if cfg.is_enabled(Column::Files) {
        header_cells.push(Cell::from("FILES"));
        widths.push(Constraint::Length(6));
    }
    if cfg.is_enabled(Column::Socks) {
        header_cells.push(Cell::from("SOCKS"));
        widths.push(Constraint::Length(6));
    }
    if cfg.is_enabled(Column::Pipes) {
        header_cells.push(Cell::from("PIPES"));
        widths.push(Constraint::Length(6));
    }
    if cfg.is_enabled(Column::Other) {
        header_cells.push(Cell::from("OTHER"));
        widths.push(Constraint::Length(6));
    }
    if cfg.is_enabled(Column::Total) {
        header_cells.push(Cell::from(total_h));
        widths.push(Constraint::Length(6));
    }

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
                cells.push(Cell::from(
                    snap.count_by_kind(&ResourceKind::Socket).to_string(),
                ));
            }
            if cfg.is_enabled(Column::Pipes) {
                cells.push(Cell::from(
                    snap.count_by_kind(&ResourceKind::Pipe).to_string(),
                ));
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
        .row_highlight_style(
            Style::default()
                .bg(t.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = TableState::default();
    if !app.filtered_indices.is_empty() {
        state.select(Some(app.selected_index));
    }

    frame.render_stateful_widget(table, area, &mut state);
}

// ---------------------------------------------------------------------------
// File list view
// ---------------------------------------------------------------------------

fn draw_file_list(frame: &mut Frame, app: &App, area: Rect) {
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
        format!(
            " prexp — {} open files{} ",
            app.filtered_file_indices.len(),
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
        .row_highlight_style(
            Style::default()
                .bg(t.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = TableState::default();
    if !app.filtered_file_indices.is_empty() {
        state.select(Some(app.file_selected_index));
    }

    frame.render_stateful_widget(table, area, &mut state);
}

// ---------------------------------------------------------------------------
// Process detail overlay
// ---------------------------------------------------------------------------

fn draw_process_detail(frame: &mut Frame, app: &App) {
    let t = app.current_theme();
    let area = frame.area();
    let overlay = detail_rect(area);
    frame.render_widget(Clear, overlay);

    let snap = match app.selected_snapshot() {
        Some(s) => s,
        None => return,
    };

    let title = format!(
        " {} (pid {}) — {} fds  [h/l: scroll, y: copy, q/Esc: back] ",
        snap.name,
        snap.pid,
        snap.resources.len()
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border_process));

    let header = Row::new(vec![
        Cell::from("FD"),
        Cell::from("KIND"),
        Cell::from("PATH"),
    ])
    .style(Style::default().fg(t.header).add_modifier(Modifier::BOLD))
    .height(1);

    let rows: Vec<Row> = snap
        .resources
        .iter()
        .map(|r| {
            let kind = format!("{:?}", r.kind).to_lowercase();
            let full_path = r.path.as_deref().unwrap_or("-");
            let displayed_path = if app.detail_h_scroll < full_path.len() {
                &full_path[app.detail_h_scroll..]
            } else if full_path == "-" {
                "-"
            } else {
                ""
            };

            let path_style = fd_kind_style(r.kind.clone(), t);

            Row::new(vec![
                Cell::from(r.descriptor.to_string()),
                Cell::from(kind),
                Cell::from(Span::styled(displayed_path.to_string(), path_style)),
            ])
        })
        .collect();

    let widths = [
        Constraint::Length(6),
        Constraint::Length(8),
        Constraint::Min(30),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .bg(t.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = TableState::default();
    state.select(Some(app.detail_selected));
    frame.render_stateful_widget(table, overlay, &mut state);
}

// ---------------------------------------------------------------------------
// File detail overlay
// ---------------------------------------------------------------------------

fn draw_file_detail(frame: &mut Frame, app: &App) {
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

    let widths = [
        Constraint::Length(8),
        Constraint::Min(25),
        Constraint::Length(6),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .bg(t.highlight_bg)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = TableState::default();
    state.select(Some(app.detail_selected));
    frame.render_stateful_widget(table, overlay, &mut state);
}

// ---------------------------------------------------------------------------
// Column config overlay
// ---------------------------------------------------------------------------

fn draw_help(frame: &mut Frame, app: &App) {
    let t = app.current_theme();
    let area = frame.area();
    let overlay = detail_rect(area);
    frame.render_widget(Clear, overlay);

    let block = Block::default()
        .title(" prexp — Help [q/Esc/?: close, j/k: scroll] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent));

    let help_lines = vec![
        "",
        "  NAVIGATION",
        "  ----------",
        "  j / k / Up / Down   Navigate list",
        "  Enter               Open detail (or clear search)",
        "  q                   Quit (closes overlay first)",
        "  Esc                 Close overlay / clear search",
        "",
        "  SEARCH",
        "  ------",
        "  /                   Start search",
        "  Enter               Confirm search (keep filter)",
        "  n                   Jump to next match",
        "  Enter / Esc         Clear search",
        "",
        "  VIEWS",
        "  -----",
        "  v                   Toggle process / file view",
        "  r                   Reverse lookup by file path",
        "  a                   Toggle show-all processes",
        "  R                   Force refresh",
        "",
        "  DETAIL OVERLAY",
        "  --------------",
        "  h / l / Left / Right  Horizontal scroll",
        "  y                     Copy selected path to clipboard",
        "",
        "  SORTING",
        "  -------",
        "  s                   Cycle sort field",
        "                      Process: Unsorted > PID > Name > Total",
        "                      Files:   Procs > Filename",
        "  S                   Reverse sort direction",
        "",
        "  CONFIGURATION",
        "  -------------",
        "  c                   Configure visible columns",
        "  t                   Choose color theme",
        "  g                   Toggle system summary",
        "  ?                   Show this help",
        "",
        "",
        "  Dedicated to Comet.",
        "  My fourth cat baby with the heart of a lion",
        "  and the complex of Napoleon.",
        "",
    ];

    let max_scroll = help_lines.len().saturating_sub(
        overlay.height.saturating_sub(2) as usize, // account for border
    );
    let scroll = app.help_scroll.min(max_scroll);

    let text: Vec<Line> = help_lines
        .iter()
        .skip(scroll)
        .map(|&line| {
            if line.starts_with("  ---")
                || line.starts_with("  NAV")
                || line.starts_with("  SEA")
                || line.starts_with("  VIEW")
                || line.starts_with("  DET")
                || line.starts_with("  SORT")
                || line.starts_with("  CONF")
            {
                Line::from(Span::styled(
                    line,
                    Style::default().fg(t.header).add_modifier(Modifier::BOLD),
                ))
            } else if line.starts_with("  Dedicated")
                || line.starts_with("  My 4th")
                || line.starts_with("  and the")
            {
                Line::from(Span::styled(line, Style::default().fg(t.muted)))
            } else {
                Line::from(line)
            }
        })
        .collect();

    let paragraph = Paragraph::new(text).block(block);
    frame.render_widget(paragraph, overlay);
}

fn draw_theme_picker(frame: &mut Frame, app: &App) {
    use super::theme::THEMES;
    let t = app.current_theme();
    let area = frame.area();
    let width = 32u16.min(area.width - 4);
    let height = (THEMES.len() as u16 + 4).min(area.height - 2);
    let x = area.x + (area.width - width) / 2;
    let y = area.y + (area.height - height) / 2;
    let overlay = Rect::new(x, y, width, height);

    frame.render_widget(Clear, overlay);

    let block = Block::default()
        .title(" Theme [j/k: preview, Enter: apply] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.accent));

    let rows: Vec<Row> = THEMES
        .iter()
        .enumerate()
        .map(|(i, theme)| {
            let marker = if i == app.theme_index { "▶" } else { " " };
            let style = if i == app.theme_index {
                Style::default()
                    .bg(t.highlight_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(vec![Cell::from(marker), Cell::from(theme.name)]).style(style)
        })
        .collect();

    let widths = [Constraint::Length(2), Constraint::Min(15)];
    let table = Table::new(rows, widths).block(block);
    frame.render_widget(table, overlay);
}

fn draw_config_overlay(frame: &mut Frame, app: &App) {
    let t = app.current_theme();
    let area = frame.area();
    let width = 30u16.min(area.width - 4);
    let height = (Column::ALL.len() as u16 + 4).min(area.height - 2);
    let x = area.x + (area.width - width) / 2;
    let y = area.y + (area.height - height) / 2;
    let overlay = Rect::new(x, y, width, height);

    frame.render_widget(Clear, overlay);

    let block = Block::default()
        .title(" Columns [Enter: toggle, q: close] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.config_border));

    let rows: Vec<Row> = Column::ALL
        .iter()
        .enumerate()
        .map(|(i, col)| {
            let enabled = app.column_config.enabled[i];
            let marker = if enabled { "[x]" } else { "[ ]" };
            let style = if i == app.config_selected {
                Style::default()
                    .bg(t.highlight_bg)
                    .add_modifier(Modifier::BOLD)
            } else if !enabled {
                Style::default().fg(t.muted)
            } else {
                Style::default()
            };

            Row::new(vec![Cell::from(marker), Cell::from(col.label())]).style(style)
        })
        .collect();

    let widths = [Constraint::Length(4), Constraint::Min(10)];
    let table = Table::new(rows, widths).block(block);
    frame.render_widget(table, overlay);
}

// ---------------------------------------------------------------------------
// Status bar
// ---------------------------------------------------------------------------

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.current_theme();
    let key_style = Style::default()
        .fg(t.status_key)
        .add_modifier(Modifier::BOLD);

    let content = match app.input_mode {
        InputMode::Search => {
            let label = match app.main_view {
                MainView::Processes => " / ",
                MainView::Files => " /path: ",
            };
            Line::from(vec![
                Span::styled(label, Style::default().fg(t.accent)),
                Span::raw(&app.search_text),
                Span::styled("█", Style::default().fg(t.accent)),
                Span::styled(
                    "  (Enter to confirm, Esc to cancel)",
                    Style::default().fg(t.muted),
                ),
            ])
        }
        InputMode::ReverseLookup => Line::from(vec![
            Span::styled(" Path: ", Style::default().fg(t.accent)),
            Span::raw(&app.reverse_lookup_text),
            Span::styled("█", Style::default().fg(t.accent)),
            Span::styled(
                "  (Enter to search, Esc to cancel)",
                Style::default().fg(t.muted),
            ),
        ]),
        InputMode::Normal => {
            if let Some(msg) = &app.status_message {
                Line::from(Span::styled(
                    format!(" {}", msg),
                    Style::default().fg(t.accent),
                ))
            } else if app.detail_open {
                Line::from(vec![
                    Span::styled(" q/Esc", key_style),
                    Span::raw(" Back  "),
                    Span::styled("h/l", key_style),
                    Span::raw(" Scroll  "),
                    Span::styled("y", key_style),
                    Span::raw(" Copy path"),
                ])
            } else if app.search_active {
                Line::from(vec![
                    Span::styled(" n", key_style),
                    Span::raw(" Next  "),
                    Span::styled("Enter", key_style),
                    Span::raw(" Clear search  "),
                    Span::styled("Esc", key_style),
                    Span::raw(" Clear search"),
                ])
            } else {
                let mut spans = vec![
                    Span::styled(" q", key_style),
                    Span::raw(" Quit  "),
                    Span::styled("Enter", key_style),
                    Span::raw(" Detail  "),
                    Span::styled("/", key_style),
                    Span::raw(" Search  "),
                    Span::styled("s", key_style),
                    Span::raw(" Sort  "),
                    Span::styled("c", key_style),
                    Span::raw(" Columns  "),
                ];

                if app.main_view == MainView::Files {
                    spans.push(Span::styled("y", key_style));
                    spans.push(Span::raw(" Copy  "));
                }

                spans.push(Span::styled("?", key_style));
                spans.push(Span::raw(" Help"));

                Line::from(spans)
            }
        }
    };

    let paragraph = Paragraph::new(content).style(Style::default().bg(t.status_bg));
    frame.render_widget(paragraph, area);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn fd_kind_style(kind: ResourceKind, t: &Theme) -> Style {
    match kind {
        ResourceKind::Device | ResourceKind::Kqueue => Style::default().fg(t.muted),
        ResourceKind::Socket => Style::default().fg(t.fd_socket),
        ResourceKind::Pipe => Style::default().fg(t.fd_pipe),
        _ => Style::default(),
    }
}

fn detail_rect(area: Rect) -> Rect {
    Rect {
        x: area.x + (area.width * 4 / 100),
        y: area.y + 1,
        width: area.width * 92 / 100,
        height: area.height.saturating_sub(2),
    }
}
