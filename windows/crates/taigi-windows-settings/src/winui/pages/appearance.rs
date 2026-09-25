//! The Appearance pane: the window's light / dark mode, then the candidate
//! window's own pickers — layout, size, what each cell shows — and the
//! reset card.
//! The TYPEFACE is not here: it moved to Manage Typefaces (`font_management`), where
//! the bundled roster and the user's own typefaces are one list. Port of `AppearanceSettingsView.swift`. The values are
//! read live by the DLL's window on every show, so a change here applies
//! from the next keystroke.
//!
//! The light / dark / auto row is a native pop-up, the shape Windows 11
//! Settings itself uses for "Choose your mode" — and, since 2026-09-02, what
//! the Mac draws too.

use super::{choice_row, reset_row};
use crate::winui::window::{Message, ResetScope, SettingsWindow};
use taigi_desktop_core::settings::{
    keys, AppearanceMode, CandidateDisplayMode, CandidateLayout, CandidateSizeChoice, SettingChoice,
};
use taigi_desktop_core::strings::{StringKey, StringResolver};
use windows_reactor::*;

pub fn view(
    window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    let document = window.document();
    View::fragment((
        // The mode selector leads the pane's one group of rows.
        choice_row(
            strings.resolve(StringKey::DesktopAppearanceTab),
            AppearanceMode::ALL,
            document.choice(&keys::APPEARANCE_MODE),
            true,
            |mode: AppearanceMode| strings.resolve(mode.label_key()).to_owned(),
            |mode| Message::set_choice(mode, &keys::APPEARANCE_MODE),
            context,
        ),
        choice_row(
            strings.resolve(StringKey::DesktopCandidateWindowLayout),
            CandidateLayout::ALL,
            document.choice(&keys::CANDIDATE_LAYOUT),
            true,
            |choice: CandidateLayout| strings.resolve(choice.label_key()).to_owned(),
            |choice| Message::set_choice(choice, &keys::CANDIDATE_LAYOUT),
            context,
        ),
        // One pop-up sizes the whole window — text and air together — over
        // five named steps, beside the window layout it sizes (USER
        // 2026-09-23, `AppearanceSettingsView.swift`).
        choice_row(
            strings.resolve(StringKey::DesktopCandidateWindowSize),
            CandidateSizeChoice::ALL,
            document.choice(&keys::CANDIDATE_SIZE),
            true,
            |step: CandidateSizeChoice| strings.resolve(step.label_key()).to_owned(),
            |step| Message::set_choice(step, &keys::CANDIDATE_SIZE),
            context,
        ),
        // What each cell shows, after the window rows: both scripts, or the
        // romanization alone.
        choice_row(
            strings.resolve(StringKey::SettingsCandidateDisplayMode),
            CandidateDisplayMode::ALL,
            document.choice(&keys::CANDIDATE_DISPLAY_MODE),
            true,
            |choice: CandidateDisplayMode| strings.resolve(choice.label_key()).to_owned(),
            |choice| Message::set_choice(choice, &keys::CANDIDATE_DISPLAY_MODE),
            context,
        ),
        reset_row(strings, ResetScope::Appearance, context),
    ))
}
