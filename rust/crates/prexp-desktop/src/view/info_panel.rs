//! The process info panel — a modal overlay with four tabs over a
//! [`ProcessDetail`]. Overview/Resources are labelled key/value lists;
//! Network/Environment are virtualized rime tables (they can be long).

use iced::widget::{column, container, row, scrollable, text, Row, Space};
use iced::{Element, Length};
use prexp_core::models::ProcessDetail;
use rime::theme::{tokens, Palette};
use rime::widgets::{button, caption, modal_sized, section, table, TableColumn};

use super::fmt;
use crate::app::{App, InfoState, InfoTab, Message};

const PANEL_WIDTH: f32 = 900.0;
const CONTENT_WIDTH: f32 = 840.0;
const CONTENT_HEIGHT: f32 = 500.0;
const KEY_WIDTH: f32 = 130.0;

/// Wrap `base` in the info-panel modal when the panel is open; otherwise pass it
/// through unchanged.
pub fn info_overlay<'a>(base: Element<'a, Message>, app: &'a App) -> Element<'a, Message> {
    match app.info() {
        Some(info) => modal_sized(base, panel(info), Message::CloseInfo, PANEL_WIDTH),
        None => base,
    }
}

fn panel(info: &InfoState) -> Element<'_, Message> {
    let p = tokens();

    let name = info
        .detail
        .as_ref()
        .map(|d| d.name.clone())
        .unwrap_or_else(|| "process".to_string());
    let header = row![
        section(&format!("{name} · pid {}", info.pid)),
        Space::new().width(Length::Fill),
        button::ghost("✕", Message::CloseInfo),
    ]
    .align_y(iced::Alignment::Center);

    // Tab strip: the active tab is a primary button, the rest ghost.
    let mut tab_bar = Row::new().spacing(6);
    for tab in InfoTab::ALL {
        tab_bar = tab_bar.push(if tab == info.tab {
            button::primary(tab.label(), Message::SelectInfoTab(tab))
        } else {
            button::ghost(tab.label(), Message::SelectInfoTab(tab))
        });
    }

    let body: Element<'_, Message> = if let Some(e) = &info.error {
        text(format!("Failed to load process detail: {e}"))
            .size(13)
            .color(p.danger)
            .into()
    } else if let Some(detail) = &info.detail {
        tab_content(info, detail, &p)
    } else {
        caption("Loading…")
    };

    container(column![header, tab_bar, body].spacing(12).height(Length::Fill))
        .width(Length::Fixed(CONTENT_WIDTH))
        .height(Length::Fixed(CONTENT_HEIGHT))
        .into()
}

fn tab_content<'a>(
    info: &InfoState,
    detail: &'a ProcessDetail,
    p: &Palette,
) -> Element<'a, Message> {
    match info.tab {
        InfoTab::Overview => overview(detail, p),
        InfoTab::Resources => resources(detail, p),
        InfoTab::Network => network(info, detail),
        InfoTab::Environment => environment(info, detail),
    }
}

fn overview<'a>(d: &ProcessDetail, p: &Palette) -> Element<'a, Message> {
    let body = column![
        caption("IDENTITY"),
        kv("PID", d.pid.to_string(), p),
        kv("Parent", format!("{} (pid {})", d.parent_name, d.ppid), p),
        kv("Path", d.path.clone(), p),
        kv("CWD", d.cwd.clone(), p),
        kv("User", format!("{} (uid {})", d.user, d.uid), p),
        kv("State", d.state.label().to_string(), p),
        kv("Nice", d.nice.to_string(), p),
        kv("Started", fmt::unix_utc(d.started_secs), p),
    ]
    .spacing(6);
    scrollable(body).height(Length::Fill).into()
}

fn resources<'a>(d: &ProcessDetail, p: &Palette) -> Element<'a, Message> {
    let body = column![
        caption("MEMORY"),
        kv("RSS", fmt::bytes(d.memory_rss), p),
        kv("Physical", fmt::bytes(d.memory_phys), p),
        kv("Virtual", fmt::bytes(d.virtual_size), p),
        caption("CPU"),
        kv("CPU time", fmt::duration_ns(d.cpu_time_ns), p),
        kv("Threads", d.thread_count.to_string(), p),
        kv("Faults", d.faults.to_string(), p),
        kv("Ctx switches", d.context_switches.to_string(), p),
        kv(
            "Syscalls",
            format!("{} mach · {} unix", d.syscalls_mach, d.syscalls_unix),
            p,
        ),
        caption("DISK I/O"),
        kv("Read", fmt::bytes(d.disk_bytes_read), p),
        kv("Written", fmt::bytes(d.disk_bytes_written), p),
        caption("FILE DESCRIPTORS"),
        kv("Files", d.fd_files.to_string(), p),
        kv("Sockets", d.fd_sockets.to_string(), p),
        kv("Pipes", d.fd_pipes.to_string(), p),
        kv("Other", d.fd_other.to_string(), p),
        kv("Total", d.fd_total.to_string(), p),
    ]
    .spacing(6);
    scrollable(body).height(Length::Fill).into()
}

fn network<'a>(info: &InfoState, d: &'a ProcessDetail) -> Element<'a, Message> {
    if d.network.is_empty() {
        return caption("No network connections");
    }
    let conns = &d.network;
    let columns = vec![
        TableColumn::fixed("PROTO", 64.0),
        TableColumn::fill("LOCAL"),
        TableColumn::fill("REMOTE"),
        TableColumn::fixed("STATE", 128.0),
    ];
    let cell = move |row: usize, col: usize| -> String {
        let Some(c) = conns.get(row) else {
            return String::new();
        };
        match col {
            0 => c.proto.clone(),
            1 => c.local_addr.clone(),
            2 => c.remote_addr.clone().unwrap_or_default(),
            3 => c.state.clone().unwrap_or_default(),
            _ => String::new(),
        }
    };
    table(conns.len(), columns, cell)
        .font(iced::Font::MONOSPACE)
        .offset(info.scroll)
        .on_scroll(Message::InfoScrolled)
        .height(Length::Fill)
        .into()
}

fn environment<'a>(info: &InfoState, d: &'a ProcessDetail) -> Element<'a, Message> {
    if d.environment.is_empty() {
        return caption("No environment captured");
    }
    let env = &d.environment;
    let columns = vec![TableColumn::fixed("KEY", 240.0), TableColumn::fill("VALUE")];
    let cell = move |row: usize, col: usize| -> String {
        let Some((k, v)) = env.get(row) else {
            return String::new();
        };
        match col {
            0 => k.clone(),
            1 => v.clone(),
            _ => String::new(),
        }
    };
    table(env.len(), columns, cell)
        .font(iced::Font::MONOSPACE)
        .offset(info.scroll)
        .on_scroll(Message::InfoScrolled)
        .height(Length::Fill)
        .into()
}

/// A labelled key/value row: muted fixed-width key, inked value.
fn kv<'a>(key: &str, value: String, p: &Palette) -> Element<'a, Message> {
    row![
        text(key.to_string())
            .size(13)
            .color(p.muted)
            .width(Length::Fixed(KEY_WIDTH)),
        text(value).size(13).color(p.ink),
    ]
    .spacing(10)
    .into()
}
