//! The 一般 pane: romanization system, display language, auto-space, the
//! installed version, and the attribution footer. Port of
//! `GeneralSettingsView.swift`. The update rows (檢查更新, a pending
//! update's offer) arrive with PR9 — roadmap W9.

// 中文: 一般 pane — 輸入模式、介面語言、自動空白、版本、頁尾。

use super::{choice_combo, section_gap};
use crate::app::SettingsApp;
use crate::presentation::{self, SPONSOR_URL};
use crate::updates::INSTALLED_VERSION;
use crate::widgets::external_link;
use crate::widgets::settings_card::{self, row};
use taigi_windows_core::settings::{keys, InputMode};
use taigi_windows_core::strings::{DisplayLanguage, StringKey};
use taigi_windows_update::checker;

pub fn show(ui: &mut egui::Ui, app: &mut SettingsApp) {
    let strings = app.strings();
    let document = app.document();
    let mut input_mode: InputMode = document.choice(&keys::INPUT_MODE);
    let mut language = DisplayLanguage::from_tag(&document.string(&keys::DISPLAY_LANGUAGE));
    let mut is_auto_space = document.bool(&keys::IS_AUTO_SPACE_ENABLED);

    // A pop-up like the row under it, not a radio group (System Settings'
    // shape for a small mutually-exclusive choice).
    row(ui, strings.resolve(StringKey::SettingsInputMode), |ui| {
        let label = |mode: InputMode| strings.resolve(mode.label_key()).to_owned();
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
    row(
        ui,
        strings.resolve(StringKey::SettingsDisplayLanguage),
        |ui| {
            let label = |language: DisplayLanguage| {
                presentation::display_language_label(language, &strings)
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

    if settings_card::switch_row(
        ui,
        strings.resolve(StringKey::SettingsAutoSpace),
        &mut is_auto_space,
    ) {
        app.update_document(|document| {
            document.set_bool(&keys::IS_AUTO_SPACE_ENABLED, is_auto_space)
        });
    }

    section_gap(ui);
    update_row(ui, app);

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

/// One row, never two (`GeneralSettingsView.swift:89-117`): a known update
/// replaces the version-and-check row rather than sitting under it, and its
/// trailing control is whatever the user's next move is; the note appears
/// only when something went wrong.
fn update_row(ui: &mut egui::Ui, app: &mut SettingsApp) {
    let strings = app.strings();
    let Some(manifest) = checker::pending_update(app.document(), INSTALLED_VERSION) else {
        let label = strings.format(
            StringKey::DesktopUpdateCurrentVersionLabel,
            &[&INSTALLED_VERSION],
        );
        let checking = app.updates.is_checking();
        row(ui, &label, |ui| {
            if ui
                .add_enabled(
                    !checking,
                    egui::Button::new(strings.resolve(StringKey::DesktopUpdateCheckNow)),
                )
                .clicked()
            {
                app.check_for_updates();
            }
            if checking {
                ui.spinner();
            }
        });
        return;
    };
    let offer = app.updates.installation.offer(&manifest);
    let label = strings.format(
        StringKey::DesktopUpdatePendingVersionLabel,
        &[&manifest.version],
    );
    row(ui, &label, |ui| match offer.action_key() {
        Some(key) => {
            if ui.button(strings.resolve(key)).clicked() {
                app.message = app.updates.act_on_offer(&manifest);
            }
        }
        None => {
            ui.spinner();
        }
    });
    if let Some(note) = offer.note_key() {
        ui.weak(strings.resolve(note));
    }
}
