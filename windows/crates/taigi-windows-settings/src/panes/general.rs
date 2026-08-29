//! The 一般 pane: romanization system, display language, auto-space, the
//! installed version, and the attribution footer. Port of
//! `GeneralSettingsView.swift`. The update rows (檢查更新, a pending
//! update's offer) arrive with PR9 — roadmap W9.

// 中文: 一般 pane — 輸入模式、介面語言、自動空白、版本、頁尾。

use super::{choice_combo, labelled_row, section_break};
use crate::app::SettingsApp;
use crate::widgets::external_link;
use taigi_windows_core::settings::{keys, InputMode};
use taigi_windows_core::strings::{DisplayLanguage, StringKey};

/// `GeneralSettingsView.sponsorURL`.
const SPONSOR_URL: &str = "https://p.ecpay.com.tw/AA663DE";
/// The version the settings window belongs to — the workspace's, which
/// `make version` writes (`AppVersion.installed`).
const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn show(ui: &mut egui::Ui, app: &mut SettingsApp) {
    let strings = app.strings();
    let document = app.document();
    let mut input_mode: InputMode = document.choice(&keys::INPUT_MODE);
    let mut language = DisplayLanguage::from_tag(&document.string(&keys::DISPLAY_LANGUAGE));
    let mut is_auto_space = document.bool(&keys::IS_AUTO_SPACE_ENABLED);

    // A pop-up like the row under it, not a radio group (System Settings'
    // shape for a small mutually-exclusive choice).
    labelled_row(ui, strings.resolve(StringKey::SettingsInputMode), |ui| {
        let label = |mode: InputMode| match mode {
            InputMode::Tl => strings.resolve(StringKey::SettingsTlMode).to_owned(),
            InputMode::Poj => strings.resolve(StringKey::SettingsPojMode).to_owned(),
        };
        let choices = [InputMode::Tl, InputMode::Poj].map(|mode| (mode, label(mode)));
        if choice_combo(
            ui,
            "input_mode",
            &label(input_mode),
            &mut input_mode,
            choices,
        ) {
            app.update_document(|document| document.set_choice(&keys::INPUT_MODE, input_mode));
        }
    });

    // Endonyms for the authored languages, so a user can find their own
    // language whatever the UI currently reads in; `System` is the one
    // translated row.
    labelled_row(
        ui,
        strings.resolve(StringKey::SettingsDisplayLanguage),
        |ui| {
            let label = |language: DisplayLanguage| {
                language.endonym().map_or_else(
                    || {
                        strings
                            .resolve(StringKey::SettingsDisplayLanguageAutomatic)
                            .to_owned()
                    },
                    str::to_owned,
                )
            };
            let choices = DisplayLanguage::PICKER.map(|language| (language, label(language)));
            if choice_combo(
                ui,
                "display_language",
                &label(language),
                &mut language,
                choices,
            ) {
                app.update_document(|document| {
                    document.set_string(&keys::DISPLAY_LANGUAGE, language.tag())
                });
            }
        },
    );

    labelled_row(ui, strings.resolve(StringKey::SettingsAutoSpace), |ui| {
        if ui.checkbox(&mut is_auto_space, "").changed() {
            app.update_document(|document| {
                document.set_bool(&keys::IS_AUTO_SPACE_ENABLED, is_auto_space)
            });
        }
    });

    section_break(ui);
    // One row, never two (`GeneralSettingsView.swift:89-108`): the version,
    // with the on-demand check beside it once PR9 lands.
    labelled_row(
        ui,
        &strings.format(StringKey::DesktopUpdateCurrentVersionLabel, &[&VERSION]),
        |_ui| {},
    );

    // Centred at the foot of the pane rather than inside the form: it is
    // neither a setting nor a note about one (`sponsorFooter`).
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let total = ui.available_width();
            let width = footer_width(ui, &strings);
            ui.add_space(((total - width) / 2.0).max(0.0));
            ui.weak(strings.resolve(StringKey::DesktopCopyrightLine));
            ui.weak("\u{00B7}");
            external_link::footer(
                ui,
                app,
                strings.resolve(StringKey::DesktopSponsorLink),
                SPONSOR_URL,
            );
        });
    });
}

/// The footer's width, measured so the three pieces centre as one line.
fn footer_width(ui: &egui::Ui, strings: &taigi_windows_core::strings::StringResolver) -> f32 {
    let font = egui::TextStyle::Body.resolve(ui.style());
    let measure = |text: &str| {
        ui.fonts(|fonts| fonts.layout_no_wrap(text.to_owned(), font.clone(), egui::Color32::WHITE))
            .size()
            .x
    };
    measure(strings.resolve(StringKey::DesktopCopyrightLine))
        + measure("\u{00B7}")
        + measure(strings.resolve(StringKey::DesktopSponsorLink))
        + 2.0 * 4.0
}
