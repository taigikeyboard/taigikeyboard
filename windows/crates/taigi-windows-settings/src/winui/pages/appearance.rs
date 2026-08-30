//! The 外觀 pane: the window's light / dark mode, then the candidate
//! window's own pickers — layout, the two size steps, the typeface — and
//! the reset card. Port of `AppearanceSettingsView.swift`. The values are
//! read live by the DLL's window on every show, so a change here applies
//! from the next keystroke.
//!
//! Named divergence: the Mac's three drawn light / dark / auto thumbnails
//! are a native pop-up here, the shape Windows 11 Settings itself uses for
//! "Choose your mode".

// 中文: 外觀 pane — 亮暗模式、候選窗版面/大小/字型 picker、恢復預設。

use super::choice_row;
use crate::winui::cards;
use crate::winui::window::{Message, SettingsWindow, SettingsWrite};
use taigi_windows_core::settings::{
    keys, AppearanceMode, CandidateFontChoice, CandidateLayout, CandidateTextSizeChoice,
    CandidateWindowSizeChoice, SettingChoice,
};
use taigi_windows_core::strings::{StringKey, StringResolver};
use windows_reactor::*;

pub fn view(
    window: &SettingsWindow,
    strings: &StringResolver,
    context: &mut ViewContext<SettingsWindow>,
) -> View {
    let document = window.document();
    View::fragment((
        // The System Settings shape: the mode selector leads its own group.
        choice_row(
            strings.resolve(StringKey::DesktopAppearanceTab),
            AppearanceMode::ALL,
            document.choice(&keys::APPEARANCE_MODE),
            |mode: AppearanceMode| strings.resolve(mode.label_key()).to_owned(),
            |mode| SettingsWrite::choice(&keys::APPEARANCE_MODE, mode),
            context,
        ),
        cards::section_gap(),
        choice_row(
            strings.resolve(StringKey::DesktopCandidateWindowLayout),
            CandidateLayout::ALL,
            document.choice(&keys::CANDIDATE_LAYOUT),
            |choice: CandidateLayout| strings.resolve(choice.label_key()).to_owned(),
            |choice| SettingsWrite::choice(&keys::CANDIDATE_LAYOUT, choice),
            context,
        ),
        // The two size rows are named steps, not continuous values: pop-ups
        // rather than sliders (Apple HIG, Pop-up Buttons).
        choice_row(
            strings.resolve(StringKey::DesktopCandidateWindowSize),
            CandidateWindowSizeChoice::ALL,
            document.choice(&keys::CANDIDATE_WINDOW_SIZE),
            |choice: CandidateWindowSizeChoice| strings.resolve(choice.label_key()).to_owned(),
            |choice| SettingsWrite::choice(&keys::CANDIDATE_WINDOW_SIZE, choice),
            context,
        ),
        choice_row(
            strings.resolve(StringKey::ThemeCandidateTextSize),
            CandidateTextSizeChoice::ALL,
            document.choice(&keys::CANDIDATE_TEXT_SIZE),
            |choice: CandidateTextSizeChoice| strings.resolve(choice.label_key()).to_owned(),
            |choice| SettingsWrite::choice(&keys::CANDIDATE_TEXT_SIZE, choice),
            context,
        ),
        // The roster comes from the type: a list the bundle can grow.
        choice_row(
            strings.resolve(StringKey::ThemeCustomFont),
            CandidateFontChoice::ALL,
            document.choice(&keys::FONT_TYPE),
            |choice: CandidateFontChoice| strings.resolve(choice.label_key()).to_owned(),
            |choice| SettingsWrite::choice(&keys::FONT_TYPE, choice),
            context,
        ),
        // Its own section, at the end: it acts on every row above it.
        cards::section_gap(),
        cards::action(
            strings.resolve(StringKey::ThemeEditorResetAll),
            false,
            context.callback(|()| Message::ResetAppearance),
        ),
    ))
}
