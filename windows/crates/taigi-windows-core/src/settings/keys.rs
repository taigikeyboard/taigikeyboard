//! Every persisted settings key in one place, with the value a fresh install
//! reads when the key is absent. Port of `SettingsStore.Keys`
//! (`macos/Sources/TaigiInputMethodCore/Settings/SettingsStore.swift:36-301`).
//!
//! Key spellings are macOS's (which are iOS's) so `settings.json` speaks the
//! same vocabulary — with three NAMED divergences, all intentional:
//!
//! - `updateNextCheckMs` replaces macOS's `updateNextCheckDate`: JSON has no
//!   date type, so the desktop key says its unit (Unix milliseconds).
//! - `hasOfferedUpdateNotifications` is absent: a Windows toast needs no
//!   permission, so there is no one-time offer to remember.
//! - `candidateSlotModifier` and the `composingShortcut.<action>` rows arrive
//!   with the key contract (`crate::keys`), whose types they are keyed on.
//!
//! Defaults for engine-facing keys are NOT literals here: they are read from
//! `EngineSettings::default()`, the domain model's own statement of what a
//! fresh install types with, so the cross-platform alignment has one home.

// 中文: 所有持久化設定 key 與預設值;引擎相關預設值從 EngineSettings::default() 取,避免兩處漂移。

use super::choices::{
    AppearanceMode, CandidateFontChoice, CandidateLayout, CandidateTextSizeChoice,
    CandidateWindowSizeChoice, SettingsPane,
};
use super::document::SettingsKey;
use super::engine_settings::{
    CandidateDisplayMode, DictionarySourceToggles, EngineSettings, InputMode,
};
use crate::strings::DisplayLanguage;

const fn engine_defaults() -> EngineSettings {
    // `Default::default` is not const; spell the same values out once, and pin
    // them equal to the runtime default in `tests` below.
    EngineSettings {
        input_mode: InputMode::Tl,
        is_translate_swapped: false,
        is_output_both_scripts: false,
        candidate_display_mode: CandidateDisplayMode::SideBySide,
        is_literal_roman_candidate_enabled: true,
        is_frequency_recording_enabled: true,
        is_association_recording_enabled: true,
        is_custom_dict_enabled: true,
        dictionary_sources: DictionarySourceToggles {
            kautian: true,
            taigitv: true,
            itaigi: false,
            sitbut: false,
            taihoa: false,
            taijit: false,
            kungge: true,
            stti: true,
            khpoo: true,
            variant: false,
            khiin: false,
            lkk: true,
            dev: true,
            kautian_subcollections: super::engine_settings::KautianSubcollections {
                accent_lukang: true,
                accent_sansia: true,
                accent_taipak: true,
                accent_gilan: true,
                accent_tainan: true,
                accent_kaohsiung: true,
                accent_kinmen: true,
                accent_makung: true,
                accent_sintik: true,
                accent_taichung: true,
                name_appendix: true,
            },
        },
    }
}

const ENGINE_DEFAULTS: EngineSettings = engine_defaults();

pub const INPUT_MODE: SettingsKey<InputMode> =
    SettingsKey::new("inputMode", ENGINE_DEFAULTS.input_mode);
pub const IS_TRANSLATE_SWAPPED: SettingsKey<bool> =
    SettingsKey::new("isTranslateSwapped", ENGINE_DEFAULTS.is_translate_swapped);
pub const IS_OUTPUT_BOTH_SCRIPTS: SettingsKey<bool> =
    SettingsKey::new("outputBothScripts", ENGINE_DEFAULTS.is_output_both_scripts);
/// What a candidate cell shows. Same key and raw strings on every platform
/// (`SharedSettings.swift` `candidateDisplayMode`). The two toggles above
/// keep their own storage while this is `romanOnly`; the derived pair lives
/// in `SettingsDocument::engine_settings`.
pub const CANDIDATE_DISPLAY_MODE: SettingsKey<CandidateDisplayMode> = SettingsKey::new(
    "candidateDisplayMode",
    ENGINE_DEFAULTS.candidate_display_mode,
);
pub const IS_LITERAL_ROMAN_CANDIDATE_ENABLED: SettingsKey<bool> = SettingsKey::new(
    "literalRomanCandidateEnabled",
    ENGINE_DEFAULTS.is_literal_roman_candidate_enabled,
);
pub const IS_FREQUENCY_RECORDING_ENABLED: SettingsKey<bool> = SettingsKey::new(
    "frequencyRecordingEnabled",
    ENGINE_DEFAULTS.is_frequency_recording_enabled,
);
pub const IS_ASSOCIATION_RECORDING_ENABLED: SettingsKey<bool> = SettingsKey::new(
    "associationRecordingEnabled",
    ENGINE_DEFAULTS.is_association_recording_enabled,
);
pub const IS_CUSTOM_DICT_ENABLED: SettingsKey<bool> =
    SettingsKey::new("customDictEnabled", ENGINE_DEFAULTS.is_custom_dict_enabled);

/// Auto-insert a trailing space after committing a word. Platform-side on
/// every platform — the engine never reads it. The key spelling is iOS's, and
/// so is the default: OFF on every platform (USER 2026-09-02; the desktop ran
/// ON from 2026-08-23), so a settings transfer must carry only explicitly
/// stored values.
pub const IS_AUTO_SPACE_ENABLED: SettingsKey<bool> = SettingsKey::new("autoSpaceEnabled", false);
// Pinned at compile time so a silent flip back to ON is loud.
const _: () = assert!(!IS_AUTO_SPACE_ENABLED.default);

// RETIRED 2026-09-05 (USER): `shiftTogglesEnglishEnabled`. The Shift tap now
// switches 中/英 unconditionally — the 一般 pane is a 1:1 mirror of the Mac's,
// which has no such row, and the tray letter plus the mode flash already say
// which mode is on. The spelling is permanently reserved: a stored `false`
// still sits in existing `settings.json` files (Windows has no retired-key
// sweep), so a future configurable feature must use a NEW key rather than
// inherit those values.

// Dictionary sources. Spellings are iOS's verbatim (`SharedSettings.swift:53-66`)
// — including `khiin`, the one key with no `Enabled` suffix. The constant name
// is the engine's vocabulary, the string the settings vocabulary.
const SOURCES: DictionarySourceToggles = ENGINE_DEFAULTS.dictionary_sources;
pub const IS_KAUTIAN_ENABLED: SettingsKey<bool> =
    SettingsKey::new("moeDictEnabled", SOURCES.kautian);
pub const IS_TAIGITV_ENABLED: SettingsKey<bool> =
    SettingsKey::new("newwordDictEnabled", SOURCES.taigitv);
pub const IS_ITAIGI_ENABLED: SettingsKey<bool> =
    SettingsKey::new("iTaigiDictEnabled", SOURCES.itaigi);
pub const IS_SITBUT_ENABLED: SettingsKey<bool> =
    SettingsKey::new("taiwanPlantDictEnabled", SOURCES.sitbut);
pub const IS_TAIHOA_ENABLED: SettingsKey<bool> =
    SettingsKey::new("taiHuaDictEnabled", SOURCES.taihoa);
pub const IS_TAIJIT_ENABLED: SettingsKey<bool> =
    SettingsKey::new("taiwanJapanDictEnabled", SOURCES.taijit);
pub const IS_KUNGGE_ENABLED: SettingsKey<bool> =
    SettingsKey::new("kunggeDictEnabled", SOURCES.kungge);
pub const IS_STTI_ENABLED: SettingsKey<bool> = SettingsKey::new("sttiDictEnabled", SOURCES.stti);
pub const IS_KHPOO_ENABLED: SettingsKey<bool> = SettingsKey::new("khpooDictEnabled", SOURCES.khpoo);
pub const IS_VARIANT_ENABLED: SettingsKey<bool> =
    SettingsKey::new("variantEnabled", SOURCES.variant);
pub const IS_KHIIN_ENABLED: SettingsKey<bool> = SettingsKey::new("khiin", SOURCES.khiin);
pub const IS_LKK_ENABLED: SettingsKey<bool> = SettingsKey::new("lkkDictEnabled", SOURCES.lkk);
pub const IS_DEV_ENABLED: SettingsKey<bool> = SettingsKey::new("devDictEnabled", SOURCES.dev);

/// Kautian subcollections (`SharedSettings.swift:74-84`), all default on.
const SUBCOLL: super::engine_settings::KautianSubcollections = SOURCES.kautian_subcollections;
pub const IS_KAUTIAN_ACCENT_LUKANG_ENABLED: SettingsKey<bool> =
    SettingsKey::new("kautianAccentLukangEnabled", SUBCOLL.accent_lukang);
pub const IS_KAUTIAN_ACCENT_SANSIA_ENABLED: SettingsKey<bool> =
    SettingsKey::new("kautianAccentSansiaEnabled", SUBCOLL.accent_sansia);
pub const IS_KAUTIAN_ACCENT_TAIPAK_ENABLED: SettingsKey<bool> =
    SettingsKey::new("kautianAccentTaipakEnabled", SUBCOLL.accent_taipak);
pub const IS_KAUTIAN_ACCENT_GILAN_ENABLED: SettingsKey<bool> =
    SettingsKey::new("kautianAccentGilanEnabled", SUBCOLL.accent_gilan);
pub const IS_KAUTIAN_ACCENT_TAINAN_ENABLED: SettingsKey<bool> =
    SettingsKey::new("kautianAccentTainanEnabled", SUBCOLL.accent_tainan);
pub const IS_KAUTIAN_ACCENT_KAOHSIUNG_ENABLED: SettingsKey<bool> =
    SettingsKey::new("kautianAccentKaohsiungEnabled", SUBCOLL.accent_kaohsiung);
pub const IS_KAUTIAN_ACCENT_KINMEN_ENABLED: SettingsKey<bool> =
    SettingsKey::new("kautianAccentKinmenEnabled", SUBCOLL.accent_kinmen);
pub const IS_KAUTIAN_ACCENT_MAKUNG_ENABLED: SettingsKey<bool> =
    SettingsKey::new("kautianAccentMakungEnabled", SUBCOLL.accent_makung);
pub const IS_KAUTIAN_ACCENT_SINTIK_ENABLED: SettingsKey<bool> =
    SettingsKey::new("kautianAccentSintikEnabled", SUBCOLL.accent_sintik);
pub const IS_KAUTIAN_ACCENT_TAICHUNG_ENABLED: SettingsKey<bool> =
    SettingsKey::new("kautianAccentTaichungEnabled", SUBCOLL.accent_taichung);
pub const IS_KAUTIAN_NAME_APPENDIX_ENABLED: SettingsKey<bool> =
    SettingsKey::new("kautianNameAppendixEnabled", SUBCOLL.name_appendix);

/// The app UI display language, as a `DisplayLanguage` tag. The roster owns
/// the default: the composing engine never reads which language the UI is in.
pub const DISPLAY_LANGUAGE: SettingsKey<&'static str> =
    SettingsKey::new("displayLanguage", DisplayLanguage::DEFAULT_TAG);

/// Update-check bookkeeping. NAMED DIVERGENCE from macOS's
/// `updateNextCheckDate` (`SettingsStore.swift:201`): Unix milliseconds, because
/// JSON has no date type and the key should say its unit. Absent = distant
/// past = check now.
pub const UPDATE_NEXT_CHECK_MS: SettingsKey<i64> = SettingsKey::new("updateNextCheckMs", 0);
/// Which version was already announced — announced once, never repeated.
pub const UPDATE_LAST_NOTIFIED_VERSION: SettingsKey<&'static str> =
    SettingsKey::new("updateLastNotifiedVersion", "");
/// The pending update, stored as the manifest's own wire JSON: version and
/// download page are one fact, and two keys could be read torn.
pub const UPDATE_PENDING_MANIFEST: SettingsKey<&'static str> =
    SettingsKey::new("updatePendingManifest", "");

/// The settings-window pane the sidebar reopens on.
pub const SELECTED_SETTINGS_PANE: SettingsKey<SettingsPane> =
    SettingsKey::new("selectedSettingsPane", SettingsPane::General);

/// Presentation choices the candidate window and settings window read
/// (`SettingsStore.swift:236-276`). Spellings are iOS's where a matching
/// iOS setting exists (`fontType`), macOS's otherwise.
pub const CANDIDATE_LAYOUT: SettingsKey<CandidateLayout> =
    SettingsKey::new("candidateLayout", CandidateLayout::Expandable);
pub const APPEARANCE_MODE: SettingsKey<AppearanceMode> =
    SettingsKey::new("candidateAppearanceMode", AppearanceMode::Auto);
pub const CANDIDATE_TEXT_SIZE: SettingsKey<CandidateTextSizeChoice> =
    SettingsKey::new("candidateTextSize", CandidateTextSizeChoice::Medium);
pub const CANDIDATE_WINDOW_SIZE: SettingsKey<CandidateWindowSizeChoice> =
    SettingsKey::new("candidateWindowSize", CandidateWindowSizeChoice::Medium);
pub const FONT_TYPE: SettingsKey<CandidateFontChoice> =
    SettingsKey::new("fontType", CandidateFontChoice::System);

/// Which keys the candidate slots take (`CandidateSlotKeySet`). The stored
/// name predates the letter set, from when the choice was only which
/// modifier held the digits; kept because the values already stored under it
/// still mean what they did (`SettingsStore.swift:277-292`).
pub const CANDIDATE_SLOT_MODIFIER: SettingsKey<crate::keys::CandidateSlotKeySet> = SettingsKey::new(
    "candidateSlotModifier",
    crate::keys::CandidateSlotKeySet::BareKeys,
);

/// What a user-cleared composing chord row stores. An absent key means "never
/// touched" and reads as the action's default; the empty string means the
/// user cleared the row, which is why the two cannot be collapsed.
pub const CLEARED_COMPOSING_CHORD: &str = "";

/// The keys the 外觀 pane's reset removes (`SettingsStore.swift:457-465`).
pub const APPEARANCE_KEYS: [&str; 6] = [
    APPEARANCE_MODE.name,
    CANDIDATE_LAYOUT.name,
    CANDIDATE_DISPLAY_MODE.name,
    CANDIDATE_WINDOW_SIZE.name,
    CANDIDATE_TEXT_SIZE.name,
    FONT_TYPE.name,
];

/// The 13 source toggles + 11 subcollection toggles the 詞庫來源 pane's reset
/// removes (`SettingsStore.swift:476-503`).
pub const DICTIONARY_SOURCE_KEYS: [&str; 24] = [
    IS_KAUTIAN_ENABLED.name,
    IS_TAIGITV_ENABLED.name,
    IS_ITAIGI_ENABLED.name,
    IS_SITBUT_ENABLED.name,
    IS_TAIHOA_ENABLED.name,
    IS_TAIJIT_ENABLED.name,
    IS_KUNGGE_ENABLED.name,
    IS_STTI_ENABLED.name,
    IS_KHPOO_ENABLED.name,
    IS_VARIANT_ENABLED.name,
    IS_KHIIN_ENABLED.name,
    IS_LKK_ENABLED.name,
    IS_DEV_ENABLED.name,
    IS_KAUTIAN_ACCENT_LUKANG_ENABLED.name,
    IS_KAUTIAN_ACCENT_SANSIA_ENABLED.name,
    IS_KAUTIAN_ACCENT_TAIPAK_ENABLED.name,
    IS_KAUTIAN_ACCENT_GILAN_ENABLED.name,
    IS_KAUTIAN_ACCENT_TAINAN_ENABLED.name,
    IS_KAUTIAN_ACCENT_KAOHSIUNG_ENABLED.name,
    IS_KAUTIAN_ACCENT_KINMEN_ENABLED.name,
    IS_KAUTIAN_ACCENT_MAKUNG_ENABLED.name,
    IS_KAUTIAN_ACCENT_SINTIK_ENABLED.name,
    IS_KAUTIAN_ACCENT_TAICHUNG_ENABLED.name,
    IS_KAUTIAN_NAME_APPENDIX_ENABLED.name,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn const_defaults_equal_the_runtime_default() {
        // The const copy exists only because `Default::default` is not const;
        // this is what keeps the two from drifting.
        assert_eq!(ENGINE_DEFAULTS, EngineSettings::default());
    }

    #[test]
    fn key_names_are_the_ios_spellings() {
        // trace: SettingsStore.swift:42-137 — `khiin` has no `Enabled` suffix,
        // `moeDictEnabled` is the kautian toggle.
        assert_eq!(IS_KHIIN_ENABLED.name, "khiin");
        assert_eq!(IS_KAUTIAN_ENABLED.name, "moeDictEnabled");
        assert_eq!(IS_TRANSLATE_SWAPPED.name, "isTranslateSwapped");
        assert_eq!(IS_OUTPUT_BOTH_SCRIPTS.name, "outputBothScripts");
        assert_eq!(CANDIDATE_DISPLAY_MODE.name, "candidateDisplayMode");
        assert_eq!(
            CANDIDATE_DISPLAY_MODE.default,
            CandidateDisplayMode::SideBySide
        );
    }

    #[test]
    fn reset_rosters_have_no_duplicates() {
        let mut appearance = APPEARANCE_KEYS.to_vec();
        appearance.sort_unstable();
        appearance.dedup();
        assert_eq!(appearance.len(), APPEARANCE_KEYS.len());
        let mut sources = DICTIONARY_SOURCE_KEYS.to_vec();
        sources.sort_unstable();
        sources.dedup();
        assert_eq!(sources.len(), DICTIONARY_SOURCE_KEYS.len());
    }
}
