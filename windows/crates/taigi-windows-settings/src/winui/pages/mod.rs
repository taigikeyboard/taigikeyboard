//! The pages, one module per pane, in the sidebar's order, then the two the
//! sidebar does not list. Each is a function over the window's state —
//! presentation only, no state of its own (the guide's rule for a stateless
//! subtree).

pub mod about;
pub mod appearance;
pub mod custom_dictionary;
pub mod dictionary_search;
pub mod dictionary_sources;
pub mod font_management;
pub mod general;
pub mod shortcuts;

use crate::winui::cards;
use crate::winui::window::{Message, ResetScope, SettingsWindow};
use taigi_desktop_core::strings::{StringKey, StringResolver};
use windows_reactor::*;

/// A pane's Reset to Defaults card, after a gap: its own section at the end,
/// acting on every row above it. One shape for every pane that has one.
pub fn reset_row(
    strings: &StringResolver,
    scope: ResetScope,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    View::fragment((
        cards::section_gap(),
        cards::action_row(
            strings.resolve(StringKey::ThemeEditorResetAll),
            strings.resolve(StringKey::SettingsReset),
            false,
            true,
            context.callback(move |()| Message::Reset(scope)),
        ),
    ))
}

/// A pop-up over a roster: the labels in the roster's order, the stored
/// value selected, and the chosen entry handed to `to_message` — `None`
/// when the pop-up cleared its selection. The row says what it MEANS, so a
/// picker that owes more than one write (the display language writes a tag,
/// not a `SettingChoice`) is not forced through a single-key write.
/// `is_enabled` as on `cards::switch_row`: false greys the row without
/// clearing it.
pub fn choice_row<T: Copy + PartialEq + 'static>(
    header: &str,
    roster: &'static [T],
    current: T,
    is_enabled: bool,
    label: impl Fn(T) -> String,
    to_message: impl Fn(Option<T>) -> Message + 'static,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    let labels = roster.iter().map(|choice| label(*choice)).collect();
    let selected = roster.iter().position(|choice| *choice == current);
    cards::choice_row(
        header,
        labels,
        selected,
        is_enabled,
        context.callback(move |index: Option<usize>| {
            to_message(index.and_then(|index| roster.get(index)).copied())
        }),
    )
}
