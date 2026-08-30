//! 詞庫來源: which dictionaries the engine draws from, in three sections —
//! 教育部, the others, the supplements — each one card of switches, with
//! 教典's eleven subcollections indented under it and greyed while it is
//! off. Port of `DictionaryTogglesView.swift`. Every toggle is read live by
//! the engine bridge on the next fetch.

// 中文: 詞庫來源頁 — 三個區段各一張卡片的開關,教典子集縮排、教典關閉時停用;恢復預設。

use super::section_gap;
use crate::app::SettingsApp;
use crate::widgets::settings_card::{self, CardRows};
use crate::widgets::toggle_switch;
use taigi_windows_core::settings::{keys, SettingsKey};
use taigi_windows_core::strings::{StringKey, StringResolver};

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

    settings_card::section_title(
        ui,
        strings.resolve(StringKey::DictionaryMoeSectionTitle),
        None,
    );
    settings_card::group(ui, |rows| {
        let is_kautian_enabled = toggle_row(
            rows,
            app,
            &strings,
            &keys::IS_KAUTIAN_ENABLED,
            StringKey::CommonMoeDict,
            None,
        );
        // Disabled, not cleared, while 教典 is off: the choices come back with it.
        for (key, label) in KAUTIAN_SUBCOLLECTIONS {
            toggle_row(rows, app, &strings, key, label, Some(is_kautian_enabled));
        }
        for (key, label) in MOE_OTHERS {
            toggle_row(rows, app, &strings, key, label, None);
        }
    });

    for (title, sources) in [
        (StringKey::DictionaryOtherSectionTitle, &OTHERS[..]),
        (
            StringKey::DictionarySupplementSectionTitle,
            &SUPPLEMENTS[..],
        ),
    ] {
        settings_card::section_title(ui, strings.resolve(title), None);
        settings_card::group(ui, |rows| {
            for (key, label) in sources {
                toggle_row(rows, app, &strings, key, *label, None);
            }
        });
    }

    section_gap(ui);
    if settings_card::action(ui, strings.resolve(StringKey::ThemeEditorResetAll), false) {
        app.update_document(|document| document.reset_dictionary_sources());
    }
}

/// One switch bound to `key` in the card; a subcollection
/// (`parent_enabled` given) is indented and follows its parent's state.
/// Answers the value after the row.
fn toggle_row(
    rows: &mut CardRows,
    app: &mut SettingsApp,
    strings: &StringResolver,
    key: &SettingsKey<bool>,
    label: StringKey,
    parent_enabled: Option<bool>,
) -> bool {
    let mut value = app.document().bool(key);
    let header = strings.resolve(label);
    let control = |ui: &mut egui::Ui| {
        if toggle_switch::show(ui, &mut value, header).changed() {
            app.update_document(|document| document.set_bool(key, value));
        }
    };
    match parent_enabled {
        None => rows.row(header, control),
        Some(enabled) => rows.sub_row(enabled, header, control),
    }
    value
}
