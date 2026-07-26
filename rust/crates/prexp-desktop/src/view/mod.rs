//! The view: opens the rime palette once, then composes the process table with
//! the window chrome (title strip controls + status bar). All rendering lives
//! under this module; each submodule builds one region.

mod detail;
mod fmt;
mod header;
mod info_panel;
mod lookup_panel;
mod process_table;
mod signal_dialog;

use iced::widget::Column;
use iced::{Element, Length};
use rime::theme;
use rime::widgets::{banner, button, select, window_shell};

use crate::app::sort::SortField;
use crate::app::{App, Message};
use detail::detail_pane;
use header::header;
use info_panel::info_overlay;
use lookup_panel::lookup_overlay;
use process_table::process_table;
use signal_dialog::signal_overlay;

pub fn view(app: &App) -> Element<'_, Message> {
    // Open the palette for this whole render pass; components read it themselves.
    let _scope = theme::enter(app.theme_choice().palette());

    let controls: Vec<Element<'_, Message>> = vec![
        select(
            SortField::ALL.to_vec(),
            Some(app.sort_field()),
            Message::SortBy,
        )
        .into(),
        button::ghost(direction_glyph(app.reverse()), Message::ToggleReverse).into(),
        button::ghost("Info", Message::OpenInfo).into(),
        button::ghost("Signal", Message::OpenSignal).into(),
        button::ghost("Find", Message::OpenLookup).into(),
        button::ghost("Stats", Message::ToggleStats).into(),
        button::secondary("Theme", Message::ToggleTheme).into(),
    ];

    // Stack the body top-to-bottom: optional error banner, optional stats header,
    // the process table (fills), then the fixed-height detail band.
    let mut sections: Vec<Element<'_, Message>> = Vec::new();
    if let Some(e) = app.error() {
        sections.push(banner(e, Message::DismissError));
    } else if let Some(n) = app.notice() {
        sections.push(banner(n, Message::DismissNotice));
    }
    if app.show_stats() {
        sections.push(header(app));
    }
    sections.push(process_table(app));
    sections.push(detail_pane(app));
    let body = Column::with_children(sections)
        .spacing(8)
        .height(Length::Fill);

    let count = app.processes().len();
    let status_left = if app.is_refreshing() {
        format!("{count} processes · refreshing…")
    } else {
        format!("{count} processes")
    };
    let status_right = format!(
        "sort {} {} · every {:.1}s",
        app.sort_field(),
        direction_glyph(app.reverse()),
        app.interval().as_secs_f64(),
    );

    let shell = window_shell(
        "prexp-desktop",
        controls,
        body,
        &status_left,
        &status_right,
    );

    // Overlays float over the whole window as modals (at most one is open).
    let with_info = info_overlay(shell, app);
    let with_signal = signal_overlay(with_info, app);
    lookup_overlay(with_signal, app)
}

fn direction_glyph(reverse: bool) -> &'static str {
    if reverse {
        "▲"
    } else {
        "▼"
    }
}
