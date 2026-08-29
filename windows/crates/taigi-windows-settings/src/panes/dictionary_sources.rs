//! 詞庫來源: which dictionaries the engine draws from, in three sections —
//! 教育部, the others, the supplements — with 教典's eleven subcollections
//! indented under it and greyed while it is off. Port of
//! `DictionaryTogglesView.swift`. Every toggle is read live by the engine
//! bridge on the next fetch.

// 中文: 詞庫來源頁 — 三個區段的開關,教典子集縮排、教典關閉時停用;恢復預設。

use super::section_break;
use crate::app::SettingsApp;
use crate::widgets::wide_action_row;
use taigi_windows_core::settings::{keys, SettingsKey};
use taigi_windows_core::strings::{StringKey, StringResolver};

/// `Metrics.subcollectionIndent`.
const SUBCOLLECTION_INDENT: f32 = 16.0;

const KAUTIAN_SUBCOLLECTIONS: [(&SettingsKey<bool>, StringKey); 11] = [
    (
        &keys::IS_KAUTIAN_ACCENT_LUKANG_ENABLED,
        StringKey::DictionaryKautianAccentLukang,
    ),
    (
        &keys::IS_KAUTIAN_ACCENT_SANSIA_ENABLED,
        StringKey::DictionaryKautianAccentSansia,
    ),
    (
        &keys::IS_KAUTIAN_ACCENT_TAIPAK_ENABLED,
        StringKey::DictionaryKautianAccentTaipak,
    ),
    (
        &keys::IS_KAUTIAN_ACCENT_GILAN_ENABLED,
        StringKey::DictionaryKautianAccentGilan,
    ),
    (
        &keys::IS_KAUTIAN_ACCENT_TAINAN_ENABLED,
        StringKey::DictionaryKautianAccentTainan,
    ),
    (
        &keys::IS_KAUTIAN_ACCENT_KAOHSIUNG_ENABLED,
        StringKey::DictionaryKautianAccentKaohsiung,
    ),
    (
        &keys::IS_KAUTIAN_ACCENT_KINMEN_ENABLED,
        StringKey::DictionaryKautianAccentKinmen,
    ),
    (
        &keys::IS_KAUTIAN_ACCENT_MAKUNG_ENABLED,
        StringKey::DictionaryKautianAccentMakung,
    ),
    (
        &keys::IS_KAUTIAN_ACCENT_SINTIK_ENABLED,
        StringKey::DictionaryKautianAccentSintik,
    ),
    (
        &keys::IS_KAUTIAN_ACCENT_TAICHUNG_ENABLED,
        StringKey::DictionaryKautianAccentTaichung,
    ),
    (
        &keys::IS_KAUTIAN_NAME_APPENDIX_ENABLED,
        StringKey::DictionaryKautianNameAppendix,
    ),
];

const MOE_OTHERS: [(&SettingsKey<bool>, StringKey); 3] = [
    (&keys::IS_TAIGITV_ENABLED, StringKey::CommonNewwordDict),
    (&keys::IS_KUNGGE_ENABLED, StringKey::CommonKunggeDict),
    (&keys::IS_STTI_ENABLED, StringKey::CommonSttiDict),
];

const OTHERS: [(&SettingsKey<bool>, StringKey); 4] = [
    (&keys::IS_ITAIGI_ENABLED, StringKey::CommonITaigiDict),
    (&keys::IS_TAIJIT_ENABLED, StringKey::CommonTaiwanJapanDict),
    (&keys::IS_TAIHOA_ENABLED, StringKey::CommonTaiHuaDict),
    (&keys::IS_SITBUT_ENABLED, StringKey::CommonTaiwanPlantDict),
];

const SUPPLEMENTS: [(&SettingsKey<bool>, StringKey); 5] = [
    (
        &keys::IS_VARIANT_ENABLED,
        StringKey::DictionaryVariantDictionary,
    ),
    (&keys::IS_KHIIN_ENABLED, StringKey::DictionaryKhiin),
    (&keys::IS_KHPOO_ENABLED, StringKey::CommonAccentDict),
    (&keys::IS_LKK_ENABLED, StringKey::DictionaryLkkDict),
    (
        &keys::IS_DEV_ENABLED,
        StringKey::DictionaryDevSupplementDict,
    ),
];

pub fn show(ui: &mut egui::Ui, app: &mut SettingsApp) {
    let strings = app.strings();

    ui.strong(strings.resolve(StringKey::DictionaryMoeSectionTitle));
    ui.add_space(6.0);
    let is_kautian_enabled = toggle(
        ui,
        app,
        &strings,
        &keys::IS_KAUTIAN_ENABLED,
        StringKey::CommonMoeDict,
    );
    // Disabled, not cleared, while 教典 is off: the choices come back with it.
    ui.add_enabled_ui(is_kautian_enabled, |ui| {
        ui.indent("kautian_subcollections", |ui| {
            ui.spacing_mut().indent = SUBCOLLECTION_INDENT;
            for (key, label) in KAUTIAN_SUBCOLLECTIONS {
                toggle(ui, app, &strings, key, label);
            }
        });
    });
    for (key, label) in MOE_OTHERS {
        toggle(ui, app, &strings, key, label);
    }

    section_break(ui);
    ui.strong(strings.resolve(StringKey::DictionaryOtherSectionTitle));
    ui.add_space(6.0);
    for (key, label) in OTHERS {
        toggle(ui, app, &strings, key, label);
    }

    section_break(ui);
    ui.strong(strings.resolve(StringKey::DictionarySupplementSectionTitle));
    ui.add_space(6.0);
    for (key, label) in SUPPLEMENTS {
        toggle(ui, app, &strings, key, label);
    }

    section_break(ui);
    if wide_action_row::show(ui, strings.resolve(StringKey::ThemeEditorResetAll), false) {
        app.update_document(|document| document.reset_dictionary_sources());
    }
}

/// One toggle bound to `key`; answers its value after the row.
fn toggle(
    ui: &mut egui::Ui,
    app: &mut SettingsApp,
    strings: &StringResolver,
    key: &SettingsKey<bool>,
    label: StringKey,
) -> bool {
    let mut value = app.document().bool(key);
    if ui.checkbox(&mut value, strings.resolve(label)).changed() {
        app.update_document(|document| document.set_bool(key, value));
    }
    ui.add_space(2.0);
    value
}
