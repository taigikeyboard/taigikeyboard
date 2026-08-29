//! The panes, one module each, in the sidebar's order
//! (`SettingsPane.allCases`) plus the unlisted search page. `show` is the
//! pane→form mapping of `SettingsDetailView` (`SettingsSplitView.swift:126-149`).

// 中文: 各設定 pane;`show` 依目前選取畫出對應表單。

pub mod appearance;
pub mod custom_dictionary;
pub mod dictionary_search;
pub mod dictionary_sources;
pub mod general;
pub mod shortcuts;
pub mod sidebar;

use crate::app::SettingsApp;
use taigi_windows_core::settings::SettingsPane;

/// Space between a form's edge and the window's (`Form.formStyle(.grouped)`).
pub const FORM_INSET: f32 = 20.0;
/// Label column to control column.
pub const ROW_SPACING: [f32; 2] = [24.0, 10.0];
/// The label column: wide enough for the longest label in five languages
/// to sit beside its control.
pub const LABEL_WIDTH: f32 = 200.0;

pub fn show(ui: &mut egui::Ui, app: &mut SettingsApp, frame: &eframe::Frame) {
    egui::Frame::NONE
        .inner_margin(FORM_INSET)
        .show(ui, |ui| match app.pane() {
            SettingsPane::General => general::show(ui, app),
            SettingsPane::Appearance => appearance::show(ui, app),
            SettingsPane::Shortcuts => shortcuts::show(ui, app),
            SettingsPane::CustomDictionary => custom_dictionary::show(ui, app, frame),
            SettingsPane::DictionarySources => dictionary_sources::show(ui, app),
            SettingsPane::DictionarySearch => dictionary_search::show(ui, app),
        });
}

/// One labelled row of a form: the label in its column, the control after it.
pub fn labelled_row(ui: &mut egui::Ui, label: &str, control: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [LABEL_WIDTH, ui.spacing().interact_size.y],
            egui::Label::new(label),
        );
        control(ui);
    });
    ui.add_space(ROW_SPACING[1]);
}

/// A section break between two groups of rows (`Section` in a grouped form).
pub fn section_break(ui: &mut egui::Ui) {
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(12.0);
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
