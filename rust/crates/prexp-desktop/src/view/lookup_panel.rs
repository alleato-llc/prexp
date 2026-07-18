//! The reverse path-lookup panel — a modal with a path input and a table of the
//! processes that have that path open (`find_by_path`).

use iced::widget::{column, container, row, text, Space};
use iced::{Element, Font, Length};
use rime::theme::tokens;
use rime::widgets::{button, caption, modal_sized, section, table, text_field, CellAlign, TableColumn};

use crate::app::{App, LookupState, Message};

const PANEL_WIDTH: f32 = 760.0;
const CONTENT_WIDTH: f32 = 700.0;
const CONTENT_HEIGHT: f32 = 520.0;
const PLACEHOLDER: &str = "/path/to/file";

/// Wrap `base` in the reverse-lookup modal when the panel is open.
pub fn lookup_overlay<'a>(base: Element<'a, Message>, app: &'a App) -> Element<'a, Message> {
    match app.lookup() {
        Some(lookup) => modal_sized(base, panel(lookup), Message::CloseLookup, PANEL_WIDTH),
        None => base,
    }
}

fn panel(lookup: &LookupState) -> Element<'_, Message> {
    let p = tokens();

    let header = row![
        section("Find processes by open path"),
        Space::new().width(Length::Fill),
        button::ghost("✕", Message::CloseLookup),
    ]
    .align_y(iced::Alignment::Center);

    // Enter in the field runs the search, as does the button.
    let query_row = row![
        text_field(PLACEHOLDER, &lookup.query, Message::LookupQuery)
            .on_submit(Message::RunLookup)
            .width(Length::Fill),
        button::primary("Search", Message::RunLookup),
    ]
    .spacing(8);

    let status: Element<'_, Message> = if lookup.searching {
        caption("Searching…")
    } else if let Some(e) = &lookup.error {
        text(format!("Lookup failed: {e}"))
            .size(12)
            .color(p.danger)
            .into()
    } else if lookup.searched {
        caption(&format!("{} process(es) with that path open", lookup.results.len()))
    } else {
        caption("Enter a file path and search")
    };

    let results = results_view(lookup);

    container(column![header, query_row, status, results].spacing(12).height(Length::Fill))
        .width(Length::Fixed(CONTENT_WIDTH))
        .height(Length::Fixed(CONTENT_HEIGHT))
        .into()
}

fn results_view(lookup: &LookupState) -> Element<'_, Message> {
    if lookup.results.is_empty() {
        // Blank space keeps the panel height stable whether or not there are hits.
        return Space::new().height(Length::Fill).into();
    }
    let procs = &lookup.results;
    let columns = vec![
        TableColumn::fixed("PID", 72.0).align(CellAlign::Right),
        TableColumn::fill("NAME"),
        TableColumn::fixed("FILES", 72.0).align(CellAlign::Right),
        TableColumn::fixed("TOTAL", 72.0).align(CellAlign::Right),
    ];
    let cell = move |row: usize, col: usize| -> String {
        let Some(pr) = procs.get(row) else {
            return String::new();
        };
        match col {
            0 => pr.pid.to_string(),
            1 => pr.name.clone(),
            2 => pr
                .count_by_kind(&prexp_core::models::ResourceKind::File)
                .to_string(),
            3 => pr.resources.len().to_string(),
            _ => String::new(),
        }
    };
    table(procs.len(), columns, cell)
        .font(Font::MONOSPACE)
        .offset(lookup.scroll)
        .on_scroll(Message::LookupScrolled)
        .height(Length::Fill)
        .into()
}
