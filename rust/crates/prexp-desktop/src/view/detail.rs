//! The detail pane — a fixed-height band below the process table showing the
//! selected process's open resources (fd, kind, path). Reads the resources
//! straight off the already-loaded [`ProcessSnapshot`]; no extra fetch needed.

use iced::widget::{column, container};
use iced::{Border, Font, Length};
use prexp_core::models::{OpenResource, ResourceKind};
use rime::theme::tokens;
use rime::widgets::{caption, table, CellAlign, TableColumn};

use crate::app::{App, Message};

/// Height of the detail band. Tall enough for several resource rows; the table
/// scrolls within it.
const PANEL_HEIGHT: f32 = 240.0;

const COL_FD: usize = 0;
const COL_KIND: usize = 1;
const COL_PATH: usize = 2;

pub fn detail_pane(app: &App) -> iced::Element<'_, Message> {
    let p = tokens();
    let body = match app.selected_process() {
        Some(proc) => populated(app, &proc.name, proc.pid, &proc.resources),
        None => placeholder(app),
    };
    container(body)
        .width(Length::Fill)
        .height(Length::Fixed(PANEL_HEIGHT))
        .padding([8, 12])
        .style(move |_| container::Style {
            background: Some(p.bg.into()),
            border: Border {
                color: p.hairline,
                width: 1.0,
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}

fn populated<'a>(
    app: &'a App,
    name: &str,
    pid: i32,
    resources: &'a [OpenResource],
) -> iced::Element<'a, Message> {
    let header = caption(&format!(
        "{name} · pid {pid} — {} open resource{}",
        resources.len(),
        if resources.len() == 1 { "" } else { "s" },
    ));

    let columns = vec![
        TableColumn::fixed("FD", 56.0).align(CellAlign::Right),
        TableColumn::fixed("KIND", 88.0),
        TableColumn::fill("PATH"),
    ];

    let cell = move |row: usize, col: usize| -> String {
        let Some(r) = resources.get(row) else {
            return String::new();
        };
        match col {
            COL_FD => r.descriptor.to_string(),
            COL_KIND => kind_label(&r.kind).to_string(),
            COL_PATH => resource_path(r),
            _ => String::new(),
        }
    };

    column![
        header,
        table(resources.len(), columns, cell)
            .font(Font::MONOSPACE)
            .offset(app.detail_scroll())
            .on_scroll(Message::DetailScrolled)
            .height(Length::Fill),
    ]
    .spacing(6)
    .height(Length::Fill)
    .into()
}

fn placeholder(app: &App) -> iced::Element<'_, Message> {
    // Distinguish "nothing picked yet" from "the process I had picked exited".
    let msg = if app.selected_pid().is_some() {
        "Selected process is no longer running"
    } else {
        "Select a process to see its open resources"
    };
    container(caption(msg))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .into()
}

fn kind_label(kind: &ResourceKind) -> &'static str {
    match kind {
        ResourceKind::File => "File",
        ResourceKind::Socket => "Socket",
        ResourceKind::Pipe => "Pipe",
        ResourceKind::Device => "Device",
        ResourceKind::Kqueue => "Kqueue",
        ResourceKind::Unknown => "Unknown",
    }
}

/// The path column: the real path when there is one, otherwise a kind-based
/// placeholder (sockets and pipes are pathless).
fn resource_path(r: &OpenResource) -> String {
    match &r.path {
        Some(path) => path.clone(),
        None => match r.kind {
            ResourceKind::Socket => "<socket>".to_string(),
            ResourceKind::Pipe => "<pipe>".to_string(),
            _ => "—".to_string(),
        },
    }
}
