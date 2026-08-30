//! The pages, one module per pane, in the sidebar's order. Each is a
//! function over the window's state — presentation only, no state of its
//! own (the guide's rule for a stateless subtree).

// 中文: 各 pane 的頁面 — 純呈現函式,狀態全在 SettingsWindow。

pub mod appearance;
pub mod general;

use crate::winui::cards;
use crate::winui::window::{Message, SettingsWindow, SettingsWrite};
use windows_reactor::*;

/// A pop-up over a roster: the labels in the roster's order, the stored
/// value selected, and the chosen entry sent back as the write it stands
/// for — so the window applies it without knowing which picker it was.
pub fn choice_row<T: Copy + PartialEq + 'static>(
    header: &str,
    roster: &'static [T],
    current: T,
    label: impl Fn(T) -> String,
    write: impl Fn(T) -> SettingsWrite + 'static,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    let labels = roster.iter().map(|choice| label(*choice)).collect();
    let selected = roster.iter().position(|choice| *choice == current);
    cards::choice_row(
        header,
        labels,
        selected,
        context.callback(move |index: Option<usize>| {
            Message::SetChoice(
                index
                    .and_then(|index| roster.get(index))
                    .map(|choice| write(*choice)),
            )
        }),
    )
}
