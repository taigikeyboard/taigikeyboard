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
pub mod learning_records;
pub mod shortcuts;

use crate::winui::cards;
use crate::winui::window::{Message, ResetScope, SettingsWindow};
use taigi_desktop_core::strings::{StringKey, StringResolver};
use taigi_windows_platform::DESKTOP_PLATFORM;
use windows_reactor::*;

/// The resolved header and optional inline caption for one setting row.
#[derive(Clone, Copy)]
pub struct RowLabel {
    pub header: &'static str,
    pub description: Option<&'static str>,
}

impl RowLabel {
    /// Resolves a row's header and its Windows inline caption from `title_key`.
    pub fn of(strings: &StringResolver, title_key: StringKey) -> Self {
        Self::with_option(strings, title_key, None)
    }

    /// Resolves a picker row's header and its Windows inline caption, letting
    /// the currently selected option's caption take precedence over the row's.
    pub fn with_option(
        strings: &StringResolver,
        title_key: StringKey,
        selected_option_key: Option<StringKey>,
    ) -> Self {
        Self {
            header: strings.resolve(title_key),
            description: row_caption(strings, title_key, selected_option_key),
        }
    }

    /// Resolves a row's header with no inline caption (for non-setting pickers
    /// such as Learning Records kind and sort order).
    pub fn plain(strings: &StringResolver, title_key: StringKey) -> Self {
        Self {
            header: strings.resolve(title_key),
            description: None,
        }
    }
}

/// Resolves the active inline caption for a setting row on Windows, letting
/// the currently selected picker option's own caption take precedence over the
/// row's caption.
pub fn row_caption(
    strings: &StringResolver,
    title_key: StringKey,
    selected_option_key: Option<StringKey>,
) -> Option<&'static str> {
    selected_option_key
        .and_then(|key| strings.caption(key, DESKTOP_PLATFORM))
        .or_else(|| strings.caption(title_key, DESKTOP_PLATFORM))
}

/// One on/off setting row whose header and inline caption resolve from
/// `title_key`.
pub fn switch_row(
    strings: &StringResolver,
    title_key: StringKey,
    is_on: bool,
    is_enabled: bool,
    on_toggled: Callback<bool>,
) -> View {
    let label = RowLabel::of(strings, title_key);
    cards::switch_row(
        label.header,
        label.description,
        is_on,
        is_enabled,
        on_toggled,
    )
}

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

/// A pop-up over a roster: `title` carries the resolved header and optional
/// inline caption, the labels follow the roster's order, the stored value is
/// selected, and the chosen entry is handed to `to_message` — `None` when the
/// pop-up cleared its selection. The row says what it MEANS, so a picker that
/// owes more than one write (the display language writes a tag, not a
/// `SettingChoice`) is not forced through a single-key write.
/// `is_enabled` as on `cards::switch_row`: false greys both header and caption
/// without clearing the stored selection.
pub fn choice_row<T: Copy + PartialEq + 'static>(
    title: RowLabel,
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
        title.header,
        title.description,
        labels,
        selected,
        is_enabled,
        context.callback(move |index: Option<usize>| {
            to_message(index.and_then(|index| roster.get(index)).copied())
        }),
    )
}
