//! The send-signal confirmation — a modal with a signal picker and a destructive
//! confirm button, over the main view.

use iced::widget::{column, row, text, Space};
use iced::{Element, Length};
use rime::widgets::{button, labeled, modal_sized, section, select};

use crate::app::{App, Message, Signal, SignalPrompt};

const PANEL_WIDTH: f32 = 460.0;

/// Wrap `base` in the send-signal modal when a prompt is pending.
pub fn signal_overlay<'a>(base: Element<'a, Message>, app: &'a App) -> Element<'a, Message> {
    match app.signal_prompt() {
        Some(prompt) => modal_sized(base, dialog(prompt), Message::CancelSignal, PANEL_WIDTH),
        None => base,
    }
}

fn dialog(prompt: &SignalPrompt) -> Element<'_, Message> {
    let picker = labeled(
        "Signal",
        select(
            Signal::ALL.to_vec(),
            Some(prompt.signal),
            Message::SelectSignal,
        ),
    );

    let actions = row![
        Space::new().width(Length::Fill),
        button::ghost("Cancel", Message::CancelSignal),
        button::danger("Send", Message::ConfirmSignal),
    ]
    .spacing(8);

    column![
        section("Send signal"),
        text(format!("{} · pid {}", prompt.name, prompt.pid)).size(13),
        picker,
        actions,
    ]
    .spacing(16)
    .into()
}
