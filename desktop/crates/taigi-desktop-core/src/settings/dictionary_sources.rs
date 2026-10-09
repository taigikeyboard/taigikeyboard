//! The Dictionary Sources pane's roster, shared by both settings windows:
//! which toggle each row writes and what it is called, in the order the pane
//! lists them — MOE's twelve subcollections under its own switch, the other
//! MOE dictionaries, the other sources, the supplements. macOS keeps a Swift
//! twin: `DictionaryTogglesView.swift`.

use super::{keys, SettingsKey};
use crate::strings::StringKey;

/// The MOE dictionary subcollections, in the pane's order.
pub const KAUTIAN_SUBCOLLECTIONS: [(SettingsKey<bool>, StringKey); 12] = [
    (
        keys::IS_KAUTIAN_ALT_READING_ENABLED,
        StringKey::DictionaryKautianAltReading,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_LUKANG_ENABLED,
        StringKey::DictionaryKautianAccentLukang,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_SANSIA_ENABLED,
        StringKey::DictionaryKautianAccentSansia,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_TAIPAK_ENABLED,
        StringKey::DictionaryKautianAccentTaipak,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_GILAN_ENABLED,
        StringKey::DictionaryKautianAccentGilan,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_TAINAN_ENABLED,
        StringKey::DictionaryKautianAccentTainan,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_KAOHSIUNG_ENABLED,
        StringKey::DictionaryKautianAccentKaohsiung,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_KINMEN_ENABLED,
        StringKey::DictionaryKautianAccentKinmen,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_MAKUNG_ENABLED,
        StringKey::DictionaryKautianAccentMakung,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_SINTIK_ENABLED,
        StringKey::DictionaryKautianAccentSintik,
    ),
    (
        keys::IS_KAUTIAN_ACCENT_TAICHUNG_ENABLED,
        StringKey::DictionaryKautianAccentTaichung,
    ),
    (
        keys::IS_KAUTIAN_NAME_APPENDIX_ENABLED,
        StringKey::DictionaryKautianNameAppendix,
    ),
];

/// The other MOE dictionaries, under the MOE group.
pub const MOE_OTHERS: [(SettingsKey<bool>, StringKey); 3] = [
    (keys::IS_TAIGITV_ENABLED, StringKey::CommonNewwordDict),
    (keys::IS_KUNGGE_ENABLED, StringKey::CommonKunggeDict),
    (keys::IS_STTI_ENABLED, StringKey::CommonSttiDict),
];

/// The non-MOE sources.
pub const OTHERS: [(SettingsKey<bool>, StringKey); 4] = [
    (keys::IS_ITAIGI_ENABLED, StringKey::CommonITaigiDict),
    (keys::IS_TAIJIT_ENABLED, StringKey::CommonTaiwanJapanDict),
    (keys::IS_TAIHOA_ENABLED, StringKey::CommonTaiHuaDict),
    (keys::IS_SITBUT_ENABLED, StringKey::CommonTaiwanPlantDict),
];

/// The supplements, last.
pub const SUPPLEMENTS: [(SettingsKey<bool>, StringKey); 5] = [
    (
        keys::IS_VARIANT_ENABLED,
        StringKey::DictionaryVariantDictionary,
    ),
    (keys::IS_KHIIN_ENABLED, StringKey::DictionaryKhiin),
    (keys::IS_KHPOO_ENABLED, StringKey::CommonAccentDict),
    (keys::IS_LKK_ENABLED, StringKey::DictionaryLkkDict),
    (keys::IS_DEV_ENABLED, StringKey::DictionaryDevSupplementDict),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::SettingsDocument;

    #[test]
    fn every_source_row_is_a_distinct_toggle_that_starts_as_the_document_says() {
        let rows = KAUTIAN_SUBCOLLECTIONS
            .iter()
            .chain(&MOE_OTHERS)
            .chain(&OTHERS)
            .chain(&SUPPLEMENTS);
        let mut names = std::collections::HashSet::new();
        let document = SettingsDocument::default();
        for (key, _) in rows {
            assert!(names.insert(key.name), "{} listed twice", key.name);
            assert_eq!(document.bool(key), key.default);
        }
    }
}
