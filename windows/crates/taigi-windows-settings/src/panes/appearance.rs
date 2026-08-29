//! The 外觀 pane: an 外觀 row of light/dark/auto thumbnails, then the
//! candidate window's own pickers — layout, the two size steps, the
//! typeface — and the reset row. Port of `AppearanceSettingsView.swift`.
//! The values are read live by the DLL's window on every show, so a
//! change here applies from the next keystroke.

// 中文: 外觀 pane — 亮暗縮圖列、版面/大小/字型 picker、恢復預設。

use super::{choice_combo, labelled_row, section_break};
use crate::app::SettingsApp;
use crate::widgets::{appearance_thumbnails, wide_action_row};
use taigi_windows_core::settings::{
    keys, AppearanceMode, CandidateFontChoice, CandidateLayout, CandidateTextSizeChoice,
    CandidateWindowSizeChoice, SettingChoice,
};
use taigi_windows_core::strings::StringKey;

pub fn show(ui: &mut egui::Ui, app: &mut SettingsApp) {
    let strings = app.strings();
    let document = app.document();
    let mut mode: AppearanceMode = document.choice(&keys::APPEARANCE_MODE);
    let mut layout: CandidateLayout = document.choice(&keys::CANDIDATE_LAYOUT);
    let mut window_size: CandidateWindowSizeChoice = document.choice(&keys::CANDIDATE_WINDOW_SIZE);
    let mut text_size: CandidateTextSizeChoice = document.choice(&keys::CANDIDATE_TEXT_SIZE);
    let mut font: CandidateFontChoice = document.choice(&keys::FONT_TYPE);

    // The System Settings shape: the mode selector leads its own group.
    labelled_row(ui, strings.resolve(StringKey::DesktopAppearanceTab), |ui| {
        if appearance_thumbnails::show(ui, &mut mode, &strings) {
            app.update_document(|document| document.set_choice(&keys::APPEARANCE_MODE, mode));
        }
    });

    section_break(ui);
    labelled_row(
        ui,
        strings.resolve(StringKey::DesktopCandidateWindowLayout),
        |ui| {
            let choices = CandidateLayout::ALL
                .iter()
                .map(|choice| (*choice, strings.resolve(choice.label_key()).to_owned()));
            if choice_combo(
                ui,
                "layout",
                strings.resolve(layout.label_key()),
                &mut layout,
                choices,
            ) {
                app.update_document(|document| {
                    document.set_choice(&keys::CANDIDATE_LAYOUT, layout)
                });
            }
        },
    );
    // The two size rows are named steps, not continuous values: pop-ups
    // rather than sliders (Apple HIG, Pop-up Buttons).
    labelled_row(
        ui,
        strings.resolve(StringKey::DesktopCandidateWindowSize),
        |ui| {
            let choices = CandidateWindowSizeChoice::ALL
                .iter()
                .map(|choice| (*choice, strings.resolve(choice.label_key()).to_owned()));
            if choice_combo(
                ui,
                "window_size",
                strings.resolve(window_size.label_key()),
                &mut window_size,
                choices,
            ) {
                app.update_document(|document| {
                    document.set_choice(&keys::CANDIDATE_WINDOW_SIZE, window_size)
                });
            }
        },
    );
    labelled_row(
        ui,
        strings.resolve(StringKey::ThemeCandidateTextSize),
        |ui| {
            let choices = CandidateTextSizeChoice::ALL
                .iter()
                .map(|choice| (*choice, strings.resolve(choice.label_key()).to_owned()));
            if choice_combo(
                ui,
                "text_size",
                strings.resolve(text_size.label_key()),
                &mut text_size,
                choices,
            ) {
                app.update_document(|document| {
                    document.set_choice(&keys::CANDIDATE_TEXT_SIZE, text_size)
                });
            }
        },
    );
    // The roster comes from the type: a list the bundle can grow.
    labelled_row(ui, strings.resolve(StringKey::ThemeCustomFont), |ui| {
        let choices = CandidateFontChoice::ALL
            .iter()
            .map(|choice| (*choice, strings.resolve(choice.label_key()).to_owned()));
        if choice_combo(
            ui,
            "font",
            strings.resolve(font.label_key()),
            &mut font,
            choices,
        ) {
            app.update_document(|document| document.set_choice(&keys::FONT_TYPE, font));
        }
    });

    // Its own section, at the end: it acts on every row above it.
    section_break(ui);
    if wide_action_row::show(ui, strings.resolve(StringKey::ThemeEditorResetAll), false) {
        app.update_document(|document| document.reset_appearance());
    }
}
