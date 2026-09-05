//! The pages, one module per pane, in the sidebar's order. Each is a
//! function over the window's state — presentation only, no state of its
//! own (the guide's rule for a stateless subtree).

pub mod appearance;
pub mod custom_dictionary;
pub mod dictionary_search;
pub mod dictionary_sources;
pub mod general;
pub mod shortcuts;

use crate::winui::cards;
use crate::winui::window::{Message, SettingsWindow};
use windows_reactor::*;

/// A pop-up over a roster: the labels in the roster's order, the stored
/// value selected, and the chosen entry handed to `to_message` — `None`
/// when the pop-up cleared its selection. The row says what it MEANS, so a
/// picker that owes more than one write (the slot-key set owes its
/// conflict pass) is not forced through a single-key write.
pub fn choice_row<T: Copy + PartialEq + 'static>(
    header: &str,
    roster: &'static [T],
    current: T,
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
        context.callback(move |index: Option<usize>| {
            to_message(index.and_then(|index| roster.get(index)).copied())
        }),
    )
}
