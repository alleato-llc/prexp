use std::collections::VecDeque;

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::tui::app::{self, App, Chart};

use super::detail_rect;

pub fn draw(frame: &mut Frame, app: &App) {
    let t = app.current_theme();
    let area = frame.area();
    let overlay = detail_rect(area);
    frame.render_widget(Clear, overlay);

    let detail = match &app.info_detail {
        Some(d) => d,
        None => return,
    };

    let tab_labels = ["Overview", "Resources", "Network", "Environment"];
    let tab_bar: Vec<Span> = tab_labels
        .iter()
        .enumerate()
        .flat_map(|(i, label)| {
            let style = if i == app.info_tab {
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.muted)
            };
            vec![Span::styled(format!(" [{}] ", label), style)]
        })
        .collect();

    let title = format!(" {} (pid {}) ", detail.name, detail.pid);
    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(t.border_process));

    let inner = block.inner(overlay);
    frame.render_widget(block, overlay);

    // Tab bar takes first line.
    let tab_line = Line::from(tab_bar);
    let tab_area = Rect { height: 1, ..inner };
    frame.render_widget(Paragraph::new(tab_line), tab_area);

    // Content area below tab bar.
    let content_area = Rect {
        y: inner.y + 1,
        height: inner.height.saturating_sub(1),
        ..inner
    };

    if app.info_tab == 3 {
        draw_environment_tab(frame, app, detail, t, content_area);
    } else {
        let lines = match app.info_tab {
            0 => overview_lines(detail, t),
            1 => resources_lines(detail, app, t),
            2 => network_lines(detail, t),
            _ => Vec::new(),
        };

        let max_scroll = lines.len().saturating_sub(content_area.height as usize);
        let scroll = app.info_scroll.min(max_scroll);
        let visible: Vec<Line> = lines.into_iter().skip(scroll).collect();

        let paragraph = Paragraph::new(visible);
        frame.render_widget(paragraph, content_area);
    }
}

fn overview_lines(detail: &prexp_ffi::ProcessDetail, t: &super::super::theme::Theme) -> Vec<Line<'static>> {
    let nice_label = nice_display(detail.nice);
    let uptime = format_uptime(detail.started_secs);
    let started = format_timestamp(detail.started_secs);

    vec![
        Line::from(""),
        section_header("IDENTITY", t),
        Line::from(""),
        kv("PID", &detail.pid.to_string()),
        kv("Parent", &format!("{} (pid {})", detail.parent_name, detail.ppid)),
        kv("Path", &detail.path),
        kv("CWD", &detail.cwd),
        kv("User", &format!("{} (uid {})", detail.user, detail.uid)),
        kv("State", &format!("{}", detail.state.label())),
        kv_styled("Nice", &nice_label.0, nice_label.1),
        kv("Started", &format!("{} (uptime {})", started, uptime)),
    ]
}

fn resources_lines(detail: &prexp_ffi::ProcessDetail, app: &App, t: &super::super::theme::Theme) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(""),
        section_header("RESOURCES", t),
        Line::from(""),
        kv2("Threads", &detail.thread_count.to_string(), "Files", &detail.fd_files.to_string()),
        kv2("Virtual", &app::format_memory(detail.virtual_size), "Sockets", &detail.fd_sockets.to_string()),
        kv2("RSS", &app::format_memory(detail.memory_rss), "Pipes", &detail.fd_pipes.to_string()),
        kv2("PMEM", &app::format_memory(detail.memory_phys), "Other", &detail.fd_other.to_string()),
        kv2("", "", "Total", &detail.fd_total.to_string()),
    ];

    // Sparklines.
    if let Some(history) = app.process_history.get(&detail.pid) {
        lines.push(Line::from(""));
        lines.push(section_header("CPU % (history)", t));
        let cpu_data: Vec<f64> = history.cpu.iter().copied().collect();
        lines.push(sparkline_line(&cpu_data, t));
        if let Some(peak) = history.cpu.iter().cloned().reduce(f64::max) {
            lines.push(Line::from(Span::styled(
                format!("  peak: {:.1}%", peak),
                Style::default().fg(t.muted),
            )));
        }

        lines.push(Line::from(""));
        lines.push(section_header("Memory (history)", t));
        let mem_pcts: Vec<f64> = if let Some(&max) = history.memory.iter().max() {
            if max > 0 {
                history.memory.iter().map(|&m| (m as f64 / max as f64) * 100.0).collect()
            } else {
                vec![0.0; history.memory.len()]
            }
        } else {
            Vec::new()
        };
        lines.push(sparkline_line(&mem_pcts, t));
        if let Some(&peak) = history.memory.iter().max() {
            lines.push(Line::from(Span::styled(
                format!("  peak: {}", app::format_memory(peak)),
                Style::default().fg(t.muted),
            )));
        }

        // Configurable charts.
        let cc = &app.chart_config;

        if cc.is_enabled(Chart::ThreadCount) && !history.threads.is_empty() {
            add_chart(&mut lines, "Threads (history)", &history.threads, |v| format!("{:.0}", v), t);
        }
        if cc.is_enabled(Chart::FdCount) && !history.fd_count.is_empty() {
            add_chart(&mut lines, "Open FDs (history)", &history.fd_count, |v| format!("{:.0}", v), t);
        }
        if cc.is_enabled(Chart::PageFaults) && !history.faults_rate.is_empty() {
            add_chart(&mut lines, "Page Faults (rate)", &history.faults_rate, |v| format!("{:.0}/s", v), t);
        }
        if cc.is_enabled(Chart::ContextSwitches) && !history.csw_rate.is_empty() {
            add_chart(&mut lines, "Context Switches (rate)", &history.csw_rate, |v| format!("{:.0}/s", v), t);
        }
        if cc.is_enabled(Chart::DiskIo) && (!history.disk_read_rate.is_empty() || !history.disk_write_rate.is_empty()) {
            add_dual_chart(
                &mut lines,
                "Disk I/O (rate)",
                "R", &history.disk_read_rate,
                "W", &history.disk_write_rate,
                |v| format!("{}/s", format_rate(v)),
                t,
            );
        }
        if cc.is_enabled(Chart::SyscallRate) && !history.syscall_rate.is_empty() {
            add_chart(&mut lines, "Syscalls (rate)", &history.syscall_rate, |v| format!("{:.0}/s", v), t);
        }

        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "  [c: configure charts]",
            Style::default().fg(t.muted),
        )));
    }

    lines
}

fn add_chart(
    lines: &mut Vec<Line<'static>>,
    title: &str,
    data: &VecDeque<f64>,
    fmt_peak: impl Fn(f64) -> String,
    t: &super::super::theme::Theme,
) {
    let data_vec: Vec<f64> = data.iter().copied().collect();
    lines.push(Line::from(""));
    lines.push(section_header(title, t));
    lines.push(sparkline_line(&data_vec, t));
    if let Some(peak) = data.iter().cloned().reduce(f64::max) {
        lines.push(Line::from(Span::styled(
            format!("  peak: {}", fmt_peak(peak)),
            Style::default().fg(t.muted),
        )));
    }
}

fn add_dual_chart(
    lines: &mut Vec<Line<'static>>,
    title: &str,
    label1: &str,
    data1: &VecDeque<f64>,
    label2: &str,
    data2: &VecDeque<f64>,
    fmt_peak: impl Fn(f64) -> String,
    t: &super::super::theme::Theme,
) {
    lines.push(Line::from(""));
    lines.push(section_header(title, t));

    // Line 1: reads
    let d1: Vec<f64> = data1.iter().copied().collect();
    let peak1 = data1.iter().cloned().reduce(f64::max).unwrap_or(0.0);
    lines.push(Line::from(vec![
        Span::styled(format!("  {} ", label1), Style::default().fg(t.header)),
        sparkline_span(&d1, t),
        Span::styled(format!("  peak: {}", fmt_peak(peak1)), Style::default().fg(t.muted)),
    ]));

    // Line 2: writes
    let d2: Vec<f64> = data2.iter().copied().collect();
    let peak2 = data2.iter().cloned().reduce(f64::max).unwrap_or(0.0);
    lines.push(Line::from(vec![
        Span::styled(format!("  {} ", label2), Style::default().fg(t.header)),
        sparkline_span(&d2, t),
        Span::styled(format!("  peak: {}", fmt_peak(peak2)), Style::default().fg(t.muted)),
    ]));
}

fn sparkline_span(data: &[f64], t: &super::super::theme::Theme) -> Span<'static> {
    const BLOCKS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if data.is_empty() {
        return Span::styled("(no data)", Style::default().fg(t.muted));
    }
    let max = data.iter().cloned().reduce(f64::max).unwrap_or(1.0).max(1.0);
    let chars: String = data.iter().map(|&v| {
        let idx = ((v / max) * 7.0).round() as usize;
        BLOCKS[idx.min(7)]
    }).collect();
    Span::styled(chars, Style::default().fg(t.accent))
}

fn format_rate(bytes: f64) -> String {
    app::format_memory(bytes as u64)
}

fn network_lines(detail: &prexp_ffi::ProcessDetail, t: &super::super::theme::Theme) -> Vec<Line<'static>> {
    let active = detail.network.iter().filter(|c| {
        matches!(c.state.as_deref(), Some("ESTABLISHED") | Some("LISTEN"))
    }).count();
    let total = detail.network.len();
    let header = format!("NETWORK CONNECTIONS ({} active, {} total)", active, total);

    let mut lines = vec![
        Line::from(""),
        section_header(&header, t),
        Line::from(""),
    ];

    if detail.network.is_empty() {
        lines.push(Line::from(Span::styled("  No network connections", Style::default().fg(t.muted))));
    } else {
        for conn in &detail.network {
            let state_str = conn.state.as_deref().unwrap_or("");
            let remote = conn.remote_addr.as_deref().unwrap_or("");
            let line = if remote.is_empty() {
                format!("  {:<5} {:<25} {}", conn.proto, conn.local_addr, state_str)
            } else {
                format!("  {:<5} {:<25} → {:<25} {}", conn.proto, conn.local_addr, remote, state_str)
            };
            lines.push(Line::from(line));
        }
    }

    lines
}

fn draw_environment_tab(
    frame: &mut Frame,
    app: &App,
    detail: &prexp_ffi::ProcessDetail,
    t: &super::super::theme::Theme,
    area: Rect,
) {
    use ratatui::widgets::{Row, Table, TableState, Cell};

    let title_line = Line::from(Span::styled(
        format!("  ENVIRONMENT ({} vars)  [y: copy]", detail.environment.len()),
        Style::default().fg(t.header).add_modifier(Modifier::BOLD),
    ));
    let title_area = Rect { height: 1, ..area };
    frame.render_widget(Paragraph::new(title_line), title_area);

    let table_area = Rect {
        y: area.y + 1,
        height: area.height.saturating_sub(1),
        ..area
    };

    if detail.environment.is_empty() {
        let msg = Paragraph::new(Span::styled("  No environment variables available", Style::default().fg(t.muted)));
        frame.render_widget(msg, table_area);
        return;
    }

    let rows: Vec<Row> = detail
        .environment
        .iter()
        .map(|(key, val)| {
            Row::new(vec![
                Cell::from(key.clone()).style(Style::default().fg(t.header)),
                Cell::from(val.clone()),
            ])
        })
        .collect();

    let widths = [ratatui::layout::Constraint::Length(25), ratatui::layout::Constraint::Min(30)];

    let table = Table::new(rows, widths)
        .row_highlight_style(Style::default().bg(t.highlight_bg).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    let mut state = TableState::default();
    state.select(Some(app.info_env_selected));
    frame.render_stateful_widget(table, table_area, &mut state);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn section_header(text: &str, t: &super::super::theme::Theme) -> Line<'static> {
    Line::from(Span::styled(
        format!("  {}", text),
        Style::default().fg(t.header).add_modifier(Modifier::BOLD),
    ))
}

fn kv(key: &str, val: &str) -> Line<'static> {
    Line::from(format!("  {:<12}{}", key, val))
}

fn kv2(key1: &str, val1: &str, key2: &str, val2: &str) -> Line<'static> {
    Line::from(format!("  {:<12}{:<14}{:<12}{}", key1, val1, key2, val2))
}

fn kv_styled(key: &str, val: &str, color: Color) -> Line<'static> {
    Line::from(vec![
        Span::raw(format!("  {:<12}", key)),
        Span::styled(val.to_string(), Style::default().fg(color)),
    ])
}

fn nice_display(nice: i32) -> (String, Color) {
    let (label, color) = match nice {
        -20..=-11 => ("CRIT", Color::Red),
        -10..=-1 => ("HIGH", Color::Yellow),
        0 => ("NORM", Color::White),
        1..=10 => ("LOW", Color::Cyan),
        11..=20 => ("IDLE", Color::DarkGray),
        _ => ("???", Color::White),
    };
    (format!("{} {}", nice, label), color)
}

fn sparkline_line(data: &[f64], t: &super::super::theme::Theme) -> Line<'static> {
    const BLOCKS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    if data.is_empty() {
        return Line::from(Span::styled("  (no data)", Style::default().fg(t.muted)));
    }
    let max = data.iter().cloned().reduce(f64::max).unwrap_or(1.0).max(1.0);
    let chars: String = data.iter().map(|&v| {
        let idx = ((v / max) * 7.0).round() as usize;
        BLOCKS[idx.min(7)]
    }).collect();
    Line::from(Span::styled(format!("  {}", chars), Style::default().fg(t.accent)))
}

fn format_uptime(start_secs: u64) -> String {
    if start_secs == 0 {
        return "unknown".into();
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let elapsed = now.saturating_sub(start_secs);

    let days = elapsed / 86400;
    let hours = (elapsed % 86400) / 3600;
    let mins = (elapsed % 3600) / 60;

    if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

fn format_timestamp(secs: u64) -> String {
    if secs == 0 {
        return "unknown".into();
    }
    // Simple UTC timestamp formatting.
    let s = secs;
    let days_since_epoch = s / 86400;
    let time_of_day = s % 86400;
    let hours = time_of_day / 3600;
    let mins = (time_of_day % 3600) / 60;
    let secs_rem = time_of_day % 60;

    // Approximate date (good enough for display).
    // This is a simplified calculation — not accounting for leap seconds.
    let mut y = 1970i64;
    let mut remaining = days_since_epoch as i64;
    loop {
        let days_in_year = if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        y += 1;
    }
    let months = [31, if y % 4 == 0 { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut m = 1;
    for &dm in &months {
        if remaining < dm { break; }
        remaining -= dm;
        m += 1;
    }
    let d = remaining + 1;

    format!("{}-{:02}-{:02} {:02}:{:02}:{:02}", y, m, d, hours, mins, secs_rem)
}
