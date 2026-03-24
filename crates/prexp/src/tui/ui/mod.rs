mod file_list;
mod info_panel;
mod overlays;
mod process_list;

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::app::{App, InputMode, MainView};

pub fn draw(frame: &mut Frame, app: &App) {
    let summary_height = if app.show_summary {
        overlays::summary_lines_for_width(app, frame.area().width.saturating_sub(2)) as u16 + 2
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
        overlays::draw_summary(frame, app, chunks[0]);
    }

    match app.main_view {
        MainView::Processes => process_list::draw(frame, app, chunks[1]),
        MainView::Files => file_list::draw(frame, app, chunks[1]),
    }

    draw_status_bar(frame, app, chunks[2]);

    if app.info_open {
        info_panel::draw(frame, app);
    } else if app.help_open {
        overlays::draw_help(frame, app);
    } else if app.theme_open {
        overlays::draw_theme_picker(frame, app);
    } else if app.config_open {
        overlays::draw_config_overlay(frame, app);
    } else if app.detail_open {
        match app.main_view {
            MainView::Processes => overlays::draw_process_detail(frame, app),
            MainView::Files => file_list::draw_detail(frame, app),
        }
    }
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let t = app.current_theme();
    let key_style = Style::default().fg(t.status_key).add_modifier(Modifier::BOLD);

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
                Span::styled("  (Enter to confirm, Esc to cancel)", Style::default().fg(t.muted)),
            ])
        }
        InputMode::ReverseLookup => Line::from(vec![
            Span::styled(" Path: ", Style::default().fg(t.accent)),
            Span::raw(&app.reverse_lookup_text),
            Span::styled("█", Style::default().fg(t.accent)),
            Span::styled("  (Enter to search, Esc to cancel)", Style::default().fg(t.muted)),
        ]),
        InputMode::Normal => {
            if let Some(msg) = &app.status_message {
                Line::from(Span::styled(format!(" {}", msg), Style::default().fg(t.accent)))
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

/// Compute the detail overlay rect.
pub(self) fn detail_rect(area: Rect) -> Rect {
    Rect {
        x: area.x + (area.width * 4 / 100),
        y: area.y + 1,
        width: area.width * 92 / 100,
        height: area.height.saturating_sub(2),
    }
}
