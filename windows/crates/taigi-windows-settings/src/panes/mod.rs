//! The panes, one module each, in the sidebar's order
//! (`SettingsPane.allCases`) plus the unlisted search page. `show` is the
//! pane→form mapping of `SettingsDetailView` (`SettingsSplitView.swift:126-149`),
//! under the page's title as the Settings app puts it.

// 中文: 各設定 pane;`show` 畫頁標題,再依目前選取畫出對應表單。

pub mod appearance;
pub mod custom_dictionary;
pub mod dictionary_search;
pub mod dictionary_sources;
pub mod general;
pub mod shortcuts;
pub mod sidebar;

use crate::app::SettingsApp;
use crate::theme;
use taigi_windows_core::settings::SettingsPane;

/// Space between a pane's content and the window's edge — the only inset
/// (the central panel's own frame is bare).
pub const FORM_INSET: f32 = 24.0;
/// Under the page title, before the first card.
const TITLE_SPACING: f32 = 24.0;
/// Between two groups of cards; Settings draws no rule between them.
const SECTION_GAP: f32 = 16.0;

pub fn show(ui: &mut egui::Ui, app: &mut SettingsApp, frame: &eframe::Frame) {
    egui::Frame::NONE.inner_margin(FORM_INSET).show(ui, |ui| {
        ui.label(egui::RichText::new(app.title()).text_style(theme::title_text_style()));
        ui.add_space(TITLE_SPACING);
        match app.pane() {
            SettingsPane::General => general::show(ui, app),
            SettingsPane::Appearance => appearance::show(ui, app),
            SettingsPane::Shortcuts => shortcuts::show(ui, app),
            SettingsPane::CustomDictionary => custom_dictionary::show(ui, app, frame),
            SettingsPane::DictionarySources => dictionary_sources::show(ui, app),
            SettingsPane::DictionarySearch => dictionary_search::show(ui, app),
        }
    });
}

/// The gap between two groups of cards.
pub fn section_gap(ui: &mut egui::Ui) {
    ui.add_space(SECTION_GAP);
}

/// A pop-up of named choices bound to `selected`; answers whether it
/// changed. The current label comes first so a caller can spell it from
/// the value it is about to lend mutably.
pub fn choice_combo<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    id_salt: &str,
    selected_label: &str,
    selected: &mut T,
    choices: impl IntoIterator<Item = (T, String)>,
) -> bool {
    let before = *selected;
    egui::ComboBox::from_id_salt(id_salt)
        .width(220.0)
        .selected_text(selected_label)
        .show_ui(ui, |ui| {
            for (choice, label) in choices {
                ui.selectable_value(selected, choice, label);
            }
        });
    *selected != before
}
