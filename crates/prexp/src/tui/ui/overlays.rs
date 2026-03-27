use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState};
use ratatui::Frame;

use prexp_core::models::ResourceKind;

use crate::tui::app::{self, App, Chart, Column, FileKindFilter, KillState, SIGNALS};
use crate::tui::theme::{Theme, THEMES};

use super::detail_rect;

// ---------------------------------------------------------------------------
// System summary header
// ---------------------------------------------------------------------------

fn cpu_layout(cpu_count: usize, width: u16) -> (usize, usize, usize) {
    if cpu_count == 0 {
        return (1, 8, 1);
    }
    let max_rows = 4usize;
    let fixed_per_cpu = 11usize;
    let min_bar = 4usize;
    let usable = width as usize;
    let cpus_per_row = ((cpu_count + max_rows - 1) / max_rows).max(1);
    let entry_width = if cpus_per_row > 0 { usable / cpus_per_row } else { usable };
    let bar_width = entry_width.saturating_sub(fixed_per_cpu).max(min_bar);
    let cpu_rows = (cpu_count + cpus_per_row - 1) / cpus_per_row;
    (cpus_per_row, bar_width, cpu_rows)
}

pub fn summary_lines_for_width(app: &App, width: u16) -> usize {
    let cpu_count = app.system_stats.cpu_usage.len();
    let (_, _, cpu_rows) = cpu_layout(cpu_count, width);
    cpu_rows + 2
}

pub fn draw_summary(frame: &mut Frame, app: &App, area: Rect) {
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
                        Span::styled(format!(" {:>2} ", core_idx), Style::default().fg(t.muted)),
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

    if let Some(mem) = &stats.memory {
        let pct = if mem.total > 0 { (mem.used as f64 / mem.total as f64) * 100.0 } else { 0.0 };
        let mem_bar_width = (inner.width as usize).saturating_sub(30).max(8);
        let bar = make_bar(pct, mem_bar_width);
        lines.push(Line::from(vec![
            Span::styled(" MEM ", Style::default().fg(t.muted)),
            Span::styled(bar, Style::default().fg(t.accent)),
            Span::raw(format!(" {} / {} ({:.0}%)", app::format_memory(mem.used), app::format_memory(mem.total), pct)),
        ]));
    }

    lines.push(Line::from(vec![Span::styled(
        format!(" {} processes   {} threads   {} open fds", stats.total_processes, stats.total_threads, stats.total_fds),
        Style::default().fg(t.header),
    )]));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}

fn make_bar(pct: f64, width: usize) -> String {
    let filled = ((pct / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}

// ---------------------------------------------------------------------------
// Process detail overlay
// ---------------------------------------------------------------------------

pub fn draw_process_detail(frame: &mut Frame, app: &App) {
    use crate::tui::app::FileKindFilter;

    let t = app.current_theme();
    let area = frame.area();
    let overlay = detail_rect(area);
    frame.render_widget(Clear, overlay);

    let snap = match app.selected_snapshot() {
        Some(s) => s,
        None => return,
    };

    let filter_label = if app.detail_kind_filter != FileKindFilter::All {
        format!(" [{}]", app.detail_kind_filter.label())
    } else {
        String::new()
    };

    let search_label = if app.detail_searching {
        format!(" [/{}]", app.detail_search)
    } else if !app.detail_search.is_empty() {
        format!(" [/{}] {} matches", app.detail_search, app.detail_filtered_indices.len())
    } else {
        String::new()
    };

    let title = format!(
        " {} (pid {}) — {} fds{}{}  [/: search, f: filter, y: copy] ",
        snap.name, snap.pid, app.detail_filtered_indices.len(),
        filter_label, search_label
    );

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border_process));

    let header = Row::new(vec![Cell::from("FD"), Cell::from("KIND"), Cell::from("PATH")])
        .style(Style::default().fg(t.header).add_modifier(Modifier::BOLD))
        .height(1);

    let rows: Vec<Row> = app.detail_filtered_indices
        .iter()
        .map(|&i| {
            let r = &snap.resources[i];
            let kind = format!("{:?}", r.kind).to_lowercase();
            let full_path = r.path.as_deref().unwrap_or("-");
            let displayed_path = if app.detail_h_scroll < full_path.len() {
                &full_path[app.detail_h_scroll..]
            } else if full_path == "-" { "-" } else { "" };

            let path_style = fd_kind_style(r.kind.clone(), t);

            Row::new(vec![
                Cell::from(r.descriptor.to_string()),
                Cell::from(kind),
                Cell::from(Span::styled(displayed_path.to_string(), path_style)),
            ])
        })
        .collect();

    let widths = [Constraint::Length(6), Constraint::Length(8), Constraint::Min(30)];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block)
        .row_highlight_style(Style::default().bg(t.highlight_bg).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    let mut state = TableState::default();
    state.select(Some(app.detail_selected));
    frame.render_stateful_widget(table, overlay, &mut state);
}

fn fd_kind_style(kind: ResourceKind, t: &Theme) -> Style {
    match kind {
        ResourceKind::Device | ResourceKind::Kqueue => Style::default().fg(t.muted),
        ResourceKind::Socket => Style::default().fg(t.fd_socket),
        ResourceKind::Pipe => Style::default().fg(t.fd_pipe),
        _ => Style::default(),
    }
}

// ---------------------------------------------------------------------------
// Help overlay
// ---------------------------------------------------------------------------

pub fn draw_help(frame: &mut Frame, app: &App) {
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
        "  f                   Filter files by kind (file view)",
        "  a                   Toggle show-all processes",
        "  R                   Force refresh",
        "",
        "  DETAIL OVERLAY",
        "  --------------",
        "  /                     Search resources by path or kind",
        "  f                     Filter by kind (File/Socket/Pipe/Device)",
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
        "  i                   Process info panel (Tab/Shift+Tab, y/Y copy env)",
        "                      c in Resources tab: configure charts",
        "  K (shift-k)         Send signal to process",
        "  g                   Toggle system summary",
        "  ?                   Show this help",
        "",
        "",
        "  Dedicated to Comet.",
        "  My fourth cat baby with the heart of a lion",
        "  and the complex of Napoleon.",
        "",
    ];

    let max_scroll = help_lines.len().saturating_sub(overlay.height.saturating_sub(2) as usize);
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
                Line::from(Span::styled(line, Style::default().fg(t.header).add_modifier(Modifier::BOLD)))
            } else if line.starts_with("  Dedicated")
                || line.starts_with("  My fourth")
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

// ---------------------------------------------------------------------------
// Theme picker
// ---------------------------------------------------------------------------

pub fn draw_theme_picker(frame: &mut Frame, app: &App) {
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
                Style::default().bg(t.highlight_bg).add_modifier(Modifier::BOLD)
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

// ---------------------------------------------------------------------------
// Column config
// ---------------------------------------------------------------------------

pub fn draw_config_overlay(frame: &mut Frame, app: &App) {
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
                Style::default().bg(t.highlight_bg).add_modifier(Modifier::BOLD)
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
// Chart config
// ---------------------------------------------------------------------------

pub fn draw_chart_config_overlay(frame: &mut Frame, app: &App) {
    let t = app.current_theme();
    let area = frame.area();
    let width = 35u16.min(area.width - 4);
    let height = (Chart::ALL.len() as u16 + 4).min(area.height - 2);
    let x = area.x + (area.width - width) / 2;
    let y = area.y + (area.height - height) / 2;
    let overlay = Rect::new(x, y, width, height);

    frame.render_widget(Clear, overlay);

    let block = Block::default()
        .title(" Charts [Enter: toggle, q: close] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.config_border));

    let rows: Vec<Row> = Chart::ALL
        .iter()
        .enumerate()
        .map(|(i, chart)| {
            let enabled = app.chart_config.enabled[i];
            let marker = if enabled { "[x]" } else { "[ ]" };
            let style = if i == app.chart_config_selected {
                Style::default().bg(t.highlight_bg).add_modifier(Modifier::BOLD)
            } else if !enabled {
                Style::default().fg(t.muted)
            } else {
                Style::default()
            };
            Row::new(vec![Cell::from(marker), Cell::from(chart.label())]).style(style)
        })
        .collect();

    let widths = [Constraint::Length(4), Constraint::Min(15)];
    let table = Table::new(rows, widths).block(block);
    frame.render_widget(table, overlay);
}

// ---------------------------------------------------------------------------
// Kill signal overlay
// ---------------------------------------------------------------------------

pub fn draw_kill_overlay(frame: &mut Frame, app: &App) {
    let t = app.current_theme();
    let area = frame.area();
    let name = app.kill_target_name.as_deref().unwrap_or("?");
    let pid = app.kill_target_pid.unwrap_or(0);

    match &app.kill_state {
        Some(KillState::Picking { selected }) => {
            let width = 50u16.min(area.width - 4);
            let height = (SIGNALS.len() as u16 + 5).min(area.height - 2);
            let x = area.x + (area.width - width) / 2;
            let y = area.y + (area.height - height) / 2;
            let overlay = Rect::new(x, y, width, height);

            frame.render_widget(Clear, overlay);

            let title = format!(" Send signal to {} (pid {}) ", name, pid);
            let block = Block::default()
                .title(title)
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ratatui::style::Color::Red));

            let mut rows: Vec<Row> = SIGNALS
                .iter()
                .enumerate()
                .map(|(i, sig)| {
                    let style = if i == *selected {
                        Style::default().bg(t.highlight_bg).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    Row::new(vec![
                        Cell::from(format!("{:>2}", sig.number)),
                        Cell::from(sig.name),
                        Cell::from(sig.description),
                    ])
                    .style(style)
                })
                .collect();

            // Custom option
            let custom_style = if *selected == SIGNALS.len() {
                Style::default().bg(t.highlight_bg).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            rows.push(
                Row::new(vec![
                    Cell::from(" ?"),
                    Cell::from("Custom"),
                    Cell::from("Enter a signal number"),
                ])
                .style(custom_style),
            );

            let widths = [
                Constraint::Length(4),
                Constraint::Length(10),
                Constraint::Min(20),
            ];
            let table = Table::new(rows, widths).block(block);
            frame.render_widget(table, overlay);
        }
        Some(KillState::CustomInput { input }) => {
            let width = 45u16.min(area.width - 4);
            let height = 5u16.min(area.height - 2);
            let x = area.x + (area.width - width) / 2;
            let y = area.y + (area.height - height) / 2;
            let overlay = Rect::new(x, y, width, height);

            frame.render_widget(Clear, overlay);

            let block = Block::default()
                .title(format!(" Custom signal for {} (pid {}) ", name, pid))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ratatui::style::Color::Red));

            let text = Line::from(vec![
                Span::raw("  Signal number: "),
                Span::styled(input.as_str(), Style::default().fg(t.accent)),
                Span::styled("█", Style::default().fg(t.accent)),
                Span::styled("  (Enter to confirm, Esc to cancel)", Style::default().fg(t.muted)),
            ]);

            let paragraph = Paragraph::new(text).block(block);
            frame.render_widget(paragraph, overlay);
        }
        Some(KillState::Confirming { signal, signal_name }) => {
            let width = 55u16.min(area.width - 4);
            let height = 5u16.min(area.height - 2);
            let x = area.x + (area.width - width) / 2;
            let y = area.y + (area.height - height) / 2;
            let overlay = Rect::new(x, y, width, height);

            frame.render_widget(Clear, overlay);

            let block = Block::default()
                .title(" Confirm ")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(ratatui::style::Color::Red));

            let text = Line::from(vec![
                Span::raw(format!(
                    "  Send {} ({}) to {} (pid {})? ",
                    signal_name, signal, name, pid
                )),
                Span::styled(
                    "[y/n]",
                    Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
                ),
            ]);

            let paragraph = Paragraph::new(text).block(block);
            frame.render_widget(paragraph, overlay);
        }
        None => {}
    }
}

// ---------------------------------------------------------------------------
// File kind filter picker
// ---------------------------------------------------------------------------

pub fn draw_kind_picker(frame: &mut Frame, app: &App) {
    let t = app.current_theme();
    let area = frame.area();
    let width = 30u16.min(area.width - 4);
    let height = (FileKindFilter::OPTIONS.len() as u16 + 4).min(area.height - 2);
    let x = area.x + (area.width - width) / 2;
    let y = area.y + (area.height - height) / 2;
    let overlay = Rect::new(x, y, width, height);

    frame.render_widget(Clear, overlay);

    let block = Block::default()
        .title(" Filter by kind [Enter: select] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border_file));

    let rows: Vec<Row> = FileKindFilter::OPTIONS
        .iter()
        .enumerate()
        .map(|(i, kind)| {
            let marker = if *kind == app.file_kind_filter { "▶" } else { " " };
            let style = if i == app.file_kind_picker_selected {
                Style::default().bg(t.highlight_bg).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![Cell::from(marker), Cell::from(kind.label())]).style(style)
        })
        .collect();

    let widths = [Constraint::Length(2), Constraint::Min(15)];
    let table = Table::new(rows, widths).block(block);
    frame.render_widget(table, overlay);
}

pub fn draw_detail_kind_picker(frame: &mut Frame, app: &App) {
    let t = app.current_theme();
    let area = frame.area();
    let width = 30u16.min(area.width - 4);
    let height = (FileKindFilter::OPTIONS.len() as u16 + 4).min(area.height - 2);
    let x = area.x + (area.width - width) / 2;
    let y = area.y + (area.height - height) / 2;
    let overlay = Rect::new(x, y, width, height);

    frame.render_widget(Clear, overlay);

    let block = Block::default()
        .title(" Filter by kind [Enter: select] ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border_file));

    let rows: Vec<Row> = FileKindFilter::OPTIONS
        .iter()
        .enumerate()
        .map(|(i, kind)| {
            let marker = if *kind == app.detail_kind_filter { "▶" } else { " " };
            let style = if i == app.detail_kind_picker_selected {
                Style::default().bg(t.highlight_bg).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![Cell::from(marker), Cell::from(kind.label())]).style(style)
        })
        .collect();

    let widths = [Constraint::Length(2), Constraint::Min(15)];
    let table = Table::new(rows, widths).block(block);
    frame.render_widget(table, overlay);
}
