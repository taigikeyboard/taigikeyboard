//! Every persisted settings key in one place, with the value a fresh install
//! reads when the key is absent. macOS keeps a Swift twin: `SettingsStore.Keys`
//! (`macos/Sources/TaigiInputMethodCore/Settings/SettingsStore.swift`).
//!
//! Key spellings are macOS's (which are iOS's) so `settings.json` speaks the
//! same vocabulary — with three NAMED divergences, all intentional:
//!
//! - `updateNextCheckMs` replaces macOS's `updateNextCheckDate`: JSON has no
//!   date type, so the desktop key says its unit (Unix milliseconds).
//! - `hasOfferedUpdateNotifications` is absent: a Windows toast needs no
//!   permission, so there is no one-time offer to remember.
//! - `toneInputScheme` and the `composingShortcut.<action>` rows arrive
//!   with the key contract (`crate::keys`), whose types they are keyed on.
//!
//! Defaults for engine-facing keys are NOT literals here: they are read from
//! `EngineSettings::DEFAULT`, the domain model's own statement of what a
//! fresh install types with, so the cross-platform alignment has one home.

use super::choices::{
    AppearanceMode, CandidateColorTheme, CandidateColors, CandidateFontChoice, CandidateLayout,
    CandidateSizeChoice, CandidateSwatch, KeyboardLayout, SettingChoice, SettingsPane,
};
use super::document::{SettingsDocument, SettingsKey};
use super::engine_settings::{
    CandidateDisplayMode, DictionarySourceToggles, EngineSettings, InputMode, Romanization,
    SyllableSeparator,
};
use crate::engine::user_data::{LearningRecordKind, LearningRecordOrder};
use crate::strings::DisplayLanguage;

const ENGINE_DEFAULTS: EngineSettings = EngineSettings::DEFAULT;

pub const INPUT_MODE: SettingsKey<InputMode> =
    SettingsKey::new("inputMode", ENGINE_DEFAULTS.input_mode);
/// The romanization TPS was entered from — where Switch TPS returns and
/// what Switch Romanization leaves TPS away from (desktop TPS roadmap D5).
/// Written only by `SettingsDocument::switch_input_mode`, only when a
/// romanization is left for TPS. Desktop-only: mobile has no TPS chord.
pub const LAST_ROMANIZATION_MODE: SettingsKey<Romanization> =
    SettingsKey::new("lastRomanizationMode", Romanization::Tl);
/// Whether the on-screen TPS key panel is up while TPS is typed (desktop TPS
/// roadmap D6): flipped by Show TPS Keyboard, kept across mode switches so
/// the panel comes back with TPS. Read by Windows, across host processes;
/// macOS keeps a Swift twin (`SettingsStore.swift`); the Linux panel is a
/// window of the settings app and keeps no key.
pub const TPS_KEYBOARD_SHOWN: SettingsKey<bool> = SettingsKey::new("tpsKeyboardShown", false);
/// Whether the Telex guide is up while a romanization is typed (#684):
/// flipped by the guide's shortcut, kept across mode switches so the guide
/// comes back when TPS is left. macOS keeps a Swift twin
/// (`SettingsStore.swift`).
pub const TELEX_GUIDE_SHOWN: SettingsKey<bool> = SettingsKey::new("telexGuideShown", false);
pub const IS_HANJI_FIRST: SettingsKey<bool> =
    SettingsKey::new("isTranslateSwapped", ENGINE_DEFAULTS.is_hanji_first);
// RETIRED 2026-09-30: `outputBothScripts` (Annotate in Brackets). No desktop
// UI ever wrote it; mobile keeps its own key and UI. A hand-edited `true` in an
// existing `settings.json` is now inert, so the spelling stays reserved.
/// What a candidate cell shows. Same key and raw strings on every platform
/// (`SharedSettings.swift` `candidateDisplayMode`). The swap toggle above
/// keeps its own storage while this is `romanOnly`; the derived value lives
/// in `SettingsDocument::engine_settings`.
pub const CANDIDATE_DISPLAY_MODE: SettingsKey<CandidateDisplayMode> = SettingsKey::new(
    "candidateDisplayMode",
    ENGINE_DEFAULTS.candidate_display_mode,
);
pub const IS_LITERAL_ROMAN_CANDIDATE_ENABLED: SettingsKey<bool> = SettingsKey::new(
    "literalRomanCandidateEnabled",
    ENGINE_DEFAULTS.is_literal_roman_candidate_enabled,
);
pub const SYLLABLE_SEPARATOR: SettingsKey<SyllableSeparator> =
    SettingsKey::new("syllableSeparator", ENGINE_DEFAULTS.syllable_separator);
/// RETIRED 2026-10-06: the No Hyphens switch, replaced by `SYLLABLE_SEPARATOR`.
/// `SettingsDocument::from_json` carries a stored `true` over as `none`, then
/// drops the key. macOS twin: `RetiredSettingsCleanup.swift`.
pub const RETIRED_HYPHENLESS_ROMAN_ENABLED: &str = "hyphenlessRomanEnabled";
pub const IS_NASAL_MARKER_UPPERCASE_ENABLED: SettingsKey<bool> = SettingsKey::new(
    "nasalMarkerUppercaseEnabled",
    ENGINE_DEFAULTS.is_nasal_marker_uppercase_enabled,
);
// RETIRED 2026-09-30: `frequencyRecordingEnabled`. No platform has a UI for it
// and the engine counts every pick now (`RecordUsage` tag 4 reserved). A
// hand-edited `false` in an existing `settings.json` is inert; spelling reserved.
//
// RETIRED 2026-09-25: `associationRecordingEnabled`. The engine dropped
// `AppConfig.is_association_recording_enabled` — association recording is
// always on. No desktop UI ever wrote the key, but a hand-edited `false` may
// still sit in an existing `settings.json` (kept as an unknown key, now
// inert), so the spelling is permanently reserved: a future feature must use a
// NEW key rather than inherit that value.
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
// switches Chinese/English unconditionally — the General pane is a 1:1 mirror of the Mac's,
// which has no such row, and the tray icon plus the mode flash already say
// which mode is on. The spelling is permanently reserved: a stored `false`
// still sits in existing `settings.json` files (Windows has no retired-key
// sweep), so a future configurable feature must use a NEW key rather than
// inherit those values.

// Dictionary sources. Spellings are iOS's verbatim (`SharedSettings.swift`'s source keys)
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

/// Kautian subcollections (`SharedSettings.swift`'s `kautian*Enabled` keys), all default on.
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
pub const IS_KAUTIAN_ALT_READING_ENABLED: SettingsKey<bool> =
    SettingsKey::new("kautianAltReadingEnabled", SUBCOLL.alt_reading);

/// The app UI display language, as a `DisplayLanguage` tag. The roster owns
/// the default: the composing engine never reads which language the UI is in.
pub const DISPLAY_LANGUAGE: SettingsKey<&'static str> =
    SettingsKey::new("displayLanguage", DisplayLanguage::DEFAULT_TAG);

/// Update-check bookkeeping. NAMED DIVERGENCE from macOS's
/// `updateNextCheckDate` (`SettingsStore.swift`): Unix milliseconds, because
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

/// The symbol picker's last picks, most recent first (`symbols::RecentSymbols`).
/// Bookkeeping the picker writes on every pick, like the update keys above:
/// not a choice, so no reset touches it. macOS keeps a Swift twin:
/// `SettingsStore.swift` `recentSymbols`.
pub const RECENT_SYMBOLS: SettingsKey<&'static [&'static str]> =
    SettingsKey::new("recentSymbols", &[]);

/// The settings-window pane the sidebar reopens on.
pub const SELECTED_SETTINGS_PANE: SettingsKey<SettingsPane> =
    SettingsKey::new("selectedSettingsPane", SettingsPane::General);

/// The Learning Records page's kind and order pickers, kept like the pane
/// above so the page reopens on the last pick (#773). Not reset by any
/// pane's reset. macOS keeps Swift twins: `SettingsStore.swift`
/// `learningRecordsKind` / `learningRecordsOrder`.
pub const LEARNING_RECORDS_KIND: SettingsKey<LearningRecordKind> =
    SettingsKey::new("learningRecordsKind", LearningRecordKind::DEFAULT);
pub const LEARNING_RECORDS_ORDER: SettingsKey<LearningRecordOrder> =
    SettingsKey::new("learningRecordsOrder", LearningRecordOrder::DEFAULT);

/// Presentation choices the candidate window and settings window read.
/// Spellings are iOS's where a matching iOS setting exists (`fontType`), the
/// desktop's own otherwise — the same as the macOS Swift twin's
/// (`SettingsStore.Keys`, `SettingsStore.swift`).
pub const CANDIDATE_LAYOUT: SettingsKey<CandidateLayout> =
    SettingsKey::new("candidateLayout", CandidateLayout::Vertical);
pub const APPEARANCE_MODE: SettingsKey<AppearanceMode> =
    SettingsKey::new("candidateAppearanceMode", AppearanceMode::Auto);
/// The candidate window's one size knob. The stored name keeps the spelling
/// from when it sized the text alone, so a size chosen then carries over
/// (`CandidateSizeChoice::from_raw`).
pub const CANDIDATE_SIZE: SettingsKey<CandidateSizeChoice> =
    SettingsKey::new("candidateTextSize", CandidateSizeChoice::Standard);
/// The Linux candidate window's primary size, in px. That is the leading
/// line of a paired cell, and a one-script cell whose script is the mode's
/// leading script (Hanji mode or romanization mode). The stored name stays
/// `hanjiFontSize`, so a size chosen when the row was the Hanji size remains
/// the primary size. The other desktops size both scripts from
/// `candidateTextSize`. Clamped to `CANDIDATE_FONT_PX_RANGE`. macOS removes
/// it in `resetAppearanceSettings`.
pub const HANJI_FONT_SIZE: SettingsKey<i64> = SettingsKey::new("hanjiFontSize", 16);
/// The Linux candidate window's secondary size, in px: the other line of a
/// paired cell, and a one-script cell in the mode's other script. The stored
/// name stays `romanFontSize`. Same range and reset as the primary size.
pub const ROMAN_FONT_SIZE: SettingsKey<i64> = SettingsKey::new("romanFontSize", 16);
/// What a Hanji or romanization size is allowed to be. The settings row
/// stops at the ends; a hand-edited value is clamped to the same range.
pub const CANDIDATE_FONT_PX_RANGE: std::ops::RangeInclusive<i64> = 12..=48;
/// The Linux candidate window's Hanji typeface, a fontconfig family name.
/// Empty is the system sans — the same default as `fontType` = system on
/// macOS and Windows. The other desktops keep one `fontType`.
pub const HANJI_FONT_FAMILY: SettingsKey<&'static str> = SettingsKey::new("hanjiFontFamily", "");
/// The Linux candidate window's romanization typeface. Empty is the system sans,
/// the same default as the Hanji row.
pub const ROMAN_FONT_FAMILY: SettingsKey<&'static str> = SettingsKey::new("romanFontFamily", "");
/// Corner radius of the Linux candidate window, in px. `0` is square.
pub const CANDIDATE_CORNER_RADIUS: SettingsKey<i64> = SettingsKey::new("candidateCornerRadius", 8);
/// The air around the Linux candidate window's text, in px. It is the window
/// pad; the cell pad, the gap, the line gap and the stacked-script gap stay
/// in the same ratio (`taigikeyboard-popup` `pads`). A new spelling:
/// `candidateWindowSize` is retired and must not be reused.
pub const CANDIDATE_WINDOW_PADDING: SettingsKey<i64> =
    SettingsKey::new("candidateWindowPadding", 4);
/// How full the Linux candidate window's highlight is, 0–100. `0` is the
/// cell. `100` meets the window's outer edge, so no pad shows around the
/// highlighted candidate. A new spelling: `candidateWindowSize` stays retired.
pub const CANDIDATE_HIGHLIGHT_FILL: SettingsKey<i64> =
    SettingsKey::new("candidateHighlightFill", 25);
/// What `candidateHighlightFill` is allowed to be.
pub const CANDIDATE_HIGHLIGHT_FILL_RANGE: std::ops::RangeInclusive<i64> = 0..=100;
/// Which color set the Linux candidate window uses. `Custom` reads the four
/// color keys; every other value paints its swatch. A new spelling.
pub const CANDIDATE_COLOR_THEME: SettingsKey<CandidateColorTheme> =
    SettingsKey::new("candidateColorTheme", CandidateColorTheme::Default);
/// `#RRGGBB` for the Linux candidate window, read while the color theme is
/// custom. A hand-edited value that is not six hex digits reads as the default.
pub const CANDIDATE_TEXT_COLOR: SettingsKey<&'static str> =
    SettingsKey::new("candidateTextColor", "#1A1A1A");
/// The highlighted cell's text.
pub const CANDIDATE_SELECTED_TEXT_COLOR: SettingsKey<&'static str> =
    SettingsKey::new("candidateSelectedTextColor", "#FFFFFF");
/// The window background.
pub const CANDIDATE_BACKGROUND_COLOR: SettingsKey<&'static str> =
    SettingsKey::new("candidateBackgroundColor", "#F6F5F4");
/// The highlighted cell's background.
pub const CANDIDATE_SELECTED_BACKGROUND_COLOR: SettingsKey<&'static str> =
    SettingsKey::new("candidateSelectedBackgroundColor", "#1A5FB4");
/// What the corner radius and the window padding are allowed to be.
pub const CANDIDATE_CHROME_PX_RANGE: std::ops::RangeInclusive<i64> = 0..=32;

/// `#RRGGBB` (or `RRGGBB`) as `0xRRGGBB`. Anything else is `fallback`.
pub fn rgb_from_stored(text: &str, fallback: u32) -> u32 {
    let hex = text.trim().trim_start_matches('#');
    if hex.len() == 6 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        u32::from_str_radix(hex, 16).unwrap_or(fallback) & 0x00FF_FFFF
    } else {
        fallback & 0x00FF_FFFF
    }
}

/// `0xRRGGBB` as `#RRGGBB`.
pub fn rgb_to_stored(rgb: u32) -> String {
    format!("#{:06X}", rgb & 0x00FF_FFFF)
}

fn stored_rgb(document: &SettingsDocument, key: &SettingsKey<&'static str>) -> u32 {
    rgb_from_stored(&document.string(key), rgb_from_stored(key.default, 0))
}

fn swatch_colors(swatch: CandidateSwatch) -> CandidateColors {
    CandidateColors {
        text: rgb_from_stored(swatch.text, 0),
        selected_text: rgb_from_stored(swatch.selected_text, 0),
        background: rgb_from_stored(swatch.background, 0),
        selected_background: rgb_from_stored(swatch.selected_background, 0),
    }
}

/// The theme the Appearance picker shows. A stored theme wins. Before that
/// key existed, the four color keys are the choice: an exact swatch match
/// selects that theme, and anything else is custom.
pub fn shown_candidate_color_theme(document: &SettingsDocument) -> CandidateColorTheme {
    if document.contains(CANDIDATE_COLOR_THEME.name) {
        return document.choice(&CANDIDATE_COLOR_THEME);
    }
    let stored = CandidateColors {
        text: stored_rgb(document, &CANDIDATE_TEXT_COLOR),
        selected_text: stored_rgb(document, &CANDIDATE_SELECTED_TEXT_COLOR),
        background: stored_rgb(document, &CANDIDATE_BACKGROUND_COLOR),
        selected_background: stored_rgb(document, &CANDIDATE_SELECTED_BACKGROUND_COLOR),
    };
    CandidateColorTheme::ALL
        .iter()
        .copied()
        .find(|theme| {
            theme
                .swatch()
                .is_some_and(|swatch| swatch_colors(swatch) == stored)
        })
        .unwrap_or(CandidateColorTheme::Custom)
}

/// The four colors the candidate window paints for `document`.
pub fn candidate_colors(document: &SettingsDocument) -> CandidateColors {
    match shown_candidate_color_theme(document).swatch() {
        Some(swatch) => swatch_colors(swatch),
        None => CandidateColors {
            text: stored_rgb(document, &CANDIDATE_TEXT_COLOR),
            selected_text: stored_rgb(document, &CANDIDATE_SELECTED_TEXT_COLOR),
            background: stored_rgb(document, &CANDIDATE_BACKGROUND_COLOR),
            selected_background: stored_rgb(document, &CANDIDATE_SELECTED_BACKGROUND_COLOR),
        },
    }
}

/// Stores `theme` and the four colors it paints, so switching to custom
/// starts from the colors that were on screen.
pub fn apply_candidate_color_theme(document: &mut SettingsDocument, theme: CandidateColorTheme) {
    let painted = theme
        .swatch()
        .map(swatch_colors)
        .unwrap_or_else(|| candidate_colors(document));
    document.set_choice(&CANDIDATE_COLOR_THEME, theme);
    document.set_string(&CANDIDATE_TEXT_COLOR, &rgb_to_stored(painted.text));
    document.set_string(
        &CANDIDATE_SELECTED_TEXT_COLOR,
        &rgb_to_stored(painted.selected_text),
    );
    document.set_string(
        &CANDIDATE_BACKGROUND_COLOR,
        &rgb_to_stored(painted.background),
    );
    document.set_string(
        &CANDIDATE_SELECTED_BACKGROUND_COLOR,
        &rgb_to_stored(painted.selected_background),
    );
}
// RETIRED 2026-09-23 (USER): `candidateWindowSize` (`small` / `medium` /
// `large`), the separate knob for the air around the text. The air now scales
// with the text at one fixed ratio (`CandidateMetrics::resolve`). The spelling
// is permanently reserved: a stored value still sits in existing
// `settings.json` files (Windows has no retired-key sweep), so a future
// padding knob must use a NEW key rather than inherit those values.
pub const FONT_TYPE: SettingsKey<CandidateFontChoice> =
    SettingsKey::new("fontType", CandidateFontChoice::System);

/// Which typeface the user added the candidate window is set in, by the file
/// name the font library stores it as. Read only while `fontType` holds
/// `CandidateFontSelection::CUSTOM_RAW`; the two are written separately, so
/// every reader tolerates one without the other.
///
/// Desktop-only, and NOT part of the iOS schema alignment the keys above keep:
/// a phone keyboard has no font library to name. A file name is local by
/// nature — a settings transfer carries the preference, never the typeface.
/// macOS keeps a Swift twin: `SettingsStore.swift`'s `customFontFile`.
pub const CUSTOM_FONT_FILE: SettingsKey<&'static str> = SettingsKey::new("customFontFile", "");

/// Which OS-installed family the candidate window is set in, by the name the
/// OS reports. Read only while `fontType` holds
/// `CandidateFontSelection::INSTALLED_RAW`; the same two-key shape as
/// `customFontFile`, and its own key rather than that one because a family
/// name is not a file name — `customFontFile` is treated as a path component.
/// Desktop-only and local like a file name. macOS keeps a Swift twin:
/// `SettingsStore.swift`'s `installedFontFamily`.
pub const INSTALLED_FONT_FAMILY: SettingsKey<&'static str> =
    SettingsKey::new("installedFontFamily", "");

/// Which weight of `installedFontFamily` the candidate window is set in, by
/// the face name the OS reports for it ("Light", "Bold", "W3"). Read only
/// beside an installed family; "" is the family's default face — what every
/// installed selection drew before this key existed. Desktop-only and local
/// like the family. macOS keeps a Swift twin: `SettingsStore.swift`'s
/// `installedFontFace`.
pub const INSTALLED_FONT_FACE: SettingsKey<&'static str> =
    SettingsKey::new("installedFontFace", "");

/// Which keys type a tone, and so which keys pick a candidate
/// (`ToneInputScheme`). Desktop-only: the phone keyboards have a tone row of
/// their own and no slot keys, so the default is owned by the key contract
/// rather than by `EngineSettings`. macOS keeps a Swift twin:
/// `SettingsStore.swift` `toneInputScheme`.
pub const TONE_INPUT_SCHEME: SettingsKey<crate::keys::ToneInputScheme> =
    SettingsKey::new("toneInputScheme", crate::keys::ToneInputScheme::Standard);

/// The layout TL / POJ keys are read in (`KeyboardLayout`). Windows reads it
/// (the macOS twin is `SettingsStore.swift` `keyboardLayout`); TPS ignores it.
pub const KEYBOARD_LAYOUT: SettingsKey<KeyboardLayout> =
    SettingsKey::new("keyboardLayout", KeyboardLayout::DEFAULT);

/// Whether the candidate window is shown at all. Off means no fetch and no
/// window — the user types romanization and Space / Enter write it as typed
/// (USER 2026-09-08: "for users who just want to type", S33). Desktop-only like
/// `toneInputScheme`: a phone keyboard's candidate bar is the keyboard.
/// macOS keeps a Swift twin: `SettingsStore.swift` `isCandidateWindowEnabled`.
pub const IS_CANDIDATE_WINDOW_ENABLED: SettingsKey<bool> =
    SettingsKey::new("candidateWindowEnabled", true);
// Pinned at compile time so a silent flip to OFF is loud.
const _: () = assert!(IS_CANDIDATE_WINDOW_ENABLED.default);

// RETIRED 2026-09-08 (USER): `candidateSlotModifier` (`bareKeys` / `shift` /
// `control` / `option`). The slot key set is now DERIVED from
// `toneInputScheme` — the letters and the digits are the same keys under
// both schemes with the two jobs swapped — and the ⇧ / ⌃ / ⌥ digit sets went
// with the picker that chose them. The spelling is permanently reserved: a
// stored `control` still sits in existing `settings.json` files (Windows has
// no retired-key sweep), so a future choice of slot keys must use a NEW key
// rather than inherit those values.

/// What a user-cleared composing chord row stores. An absent key means "never
/// touched" and reads as the action's default; the empty string means the
/// user cleared the row, which is why the two cannot be collapsed.
pub const CLEARED_COMPOSING_CHORD: &str = "";

/// The keys the General pane's reset removes — every input setting the pane
/// draws, same shape as `DICTIONARY_SOURCE_KEYS`. Not the display language
/// (USER 2026-09-20: the reset must not switch the UI language under the
/// user), not the update bookkeeping, not the remembered pane. macOS keeps a
/// Swift twin: `SettingsStore.swift` `resetGeneralSettings`. The input mode
/// takes the romanization TPS returns to and the TPS key panel with it; the
/// Telex guide's wish resets beside them.
pub const GENERAL_KEYS: [&str; 12] = [
    INPUT_MODE.name,
    KEYBOARD_LAYOUT.name,
    LAST_ROMANIZATION_MODE.name,
    TPS_KEYBOARD_SHOWN.name,
    TELEX_GUIDE_SHOWN.name,
    TONE_INPUT_SCHEME.name,
    IS_HANJI_FIRST.name,
    IS_AUTO_SPACE_ENABLED.name,
    IS_CANDIDATE_WINDOW_ENABLED.name,
    IS_LITERAL_ROMAN_CANDIDATE_ENABLED.name,
    SYLLABLE_SEPARATOR.name,
    IS_NASAL_MARKER_UPPERCASE_ENABLED.name,
];

/// The keys the Appearance pane's reset removes (`SettingsStore.swift`
/// `resetAppearanceSettings` is the macOS twin).
pub const APPEARANCE_KEYS: [&str; 16] = [
    APPEARANCE_MODE.name,
    CANDIDATE_LAYOUT.name,
    CANDIDATE_DISPLAY_MODE.name,
    CANDIDATE_SIZE.name,
    HANJI_FONT_SIZE.name,
    ROMAN_FONT_SIZE.name,
    HANJI_FONT_FAMILY.name,
    ROMAN_FONT_FAMILY.name,
    CANDIDATE_CORNER_RADIUS.name,
    CANDIDATE_WINDOW_PADDING.name,
    CANDIDATE_HIGHLIGHT_FILL.name,
    CANDIDATE_COLOR_THEME.name,
    CANDIDATE_TEXT_COLOR.name,
    CANDIDATE_SELECTED_TEXT_COLOR.name,
    CANDIDATE_BACKGROUND_COLOR.name,
    CANDIDATE_SELECTED_BACKGROUND_COLOR.name,
];

/// The 13 source toggles + 12 subcollection toggles the Dictionary Sources pane's reset
/// removes (`SettingsStore.swift` `resetDictionarySources` is the macOS twin).
pub const DICTIONARY_SOURCE_KEYS: [&str; 25] = [
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
    IS_KAUTIAN_ALT_READING_ENABLED.name,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_names_are_the_ios_spellings() {
        // trace: iOS `SharedSettings.swift` keys — `khiin` has no `Enabled`
        // suffix, `moeDictEnabled` is the kautian toggle.
        assert_eq!(IS_KHIIN_ENABLED.name, "khiin");
        assert_eq!(IS_KAUTIAN_ENABLED.name, "moeDictEnabled");
        assert_eq!(IS_HANJI_FIRST.name, "isTranslateSwapped");
        assert_eq!(CANDIDATE_DISPLAY_MODE.name, "candidateDisplayMode");
        assert_eq!(HANJI_FONT_SIZE.name, "hanjiFontSize");
        assert_eq!(HANJI_FONT_SIZE.default, 16);
        assert_eq!(ROMAN_FONT_SIZE.name, "romanFontSize");
        assert_eq!(ROMAN_FONT_SIZE.default, 16);
        assert_eq!(CANDIDATE_FONT_PX_RANGE, 12..=48);
        assert_eq!(HANJI_FONT_FAMILY.name, "hanjiFontFamily");
        assert_eq!(HANJI_FONT_FAMILY.default, "");
        assert_eq!(ROMAN_FONT_FAMILY.default, "");
        assert_eq!(CANDIDATE_CORNER_RADIUS.name, "candidateCornerRadius");
        assert_eq!(CANDIDATE_CORNER_RADIUS.default, 8);
        assert_eq!(CANDIDATE_WINDOW_PADDING.name, "candidateWindowPadding");
        assert_eq!(CANDIDATE_WINDOW_PADDING.default, 4);
        assert_eq!(CANDIDATE_HIGHLIGHT_FILL.name, "candidateHighlightFill");
        assert_eq!(CANDIDATE_HIGHLIGHT_FILL.default, 25);
        assert_eq!(CANDIDATE_HIGHLIGHT_FILL_RANGE, 0..=100);
        assert_eq!(CANDIDATE_CHROME_PX_RANGE, 0..=32);
        assert_eq!(CANDIDATE_COLOR_THEME.name, "candidateColorTheme");
        assert_eq!(CANDIDATE_COLOR_THEME.default, CandidateColorTheme::Default);
        assert_eq!(CANDIDATE_TEXT_COLOR.default, "#1A1A1A");
        assert_eq!(CANDIDATE_SELECTED_TEXT_COLOR.default, "#FFFFFF");
        assert_eq!(CANDIDATE_BACKGROUND_COLOR.default, "#F6F5F4");
        assert_eq!(CANDIDATE_SELECTED_BACKGROUND_COLOR.default, "#1A5FB4");
        assert_eq!(rgb_from_stored("#d6e4ff", 0), 0x00D6_E4FF);
        assert_eq!(rgb_from_stored("nope", 0x11), 0x11);
        assert_eq!(rgb_to_stored(0x1A1A1A), "#1A1A1A");
        assert_eq!(
            CANDIDATE_DISPLAY_MODE.default,
            CandidateDisplayMode::SideBySide
        );
        // The macOS twin's spellings (`SettingsStore.swift` `Keys`).
        assert_eq!(LEARNING_RECORDS_KIND.name, "learningRecordsKind");
        assert_eq!(LEARNING_RECORDS_ORDER.name, "learningRecordsOrder");
    }

    #[test]
    fn reset_rosters_have_no_duplicates() {
        fn assert_no_duplicates(roster: &[&str]) {
            let mut sorted = roster.to_vec();
            sorted.sort_unstable();
            sorted.dedup();
            assert_eq!(sorted.len(), roster.len());
        }
        assert_no_duplicates(&GENERAL_KEYS);
        assert_no_duplicates(&APPEARANCE_KEYS);
        assert_no_duplicates(&DICTIONARY_SOURCE_KEYS);
    }

    #[test]
    fn a_color_theme_paints_its_swatch_until_custom() {
        let fresh = SettingsDocument::default();
        assert_eq!(
            shown_candidate_color_theme(&fresh),
            CandidateColorTheme::Default
        );
        assert_eq!(candidate_colors(&fresh).text, 0x1A_1A_1A);
        assert_eq!(candidate_colors(&fresh).selected_background, 0x1A_5FB4);

        let mut document = SettingsDocument::default();
        apply_candidate_color_theme(&mut document, CandidateColorTheme::Sakura);
        assert_eq!(candidate_colors(&document).background, 0xE6_E5E3);
        assert_eq!(candidate_colors(&document).selected_background, 0xA5_3945);
        apply_candidate_color_theme(&mut document, CandidateColorTheme::Gold);
        assert_eq!(candidate_colors(&document).selected_background, 0xD3_9D19);
        apply_candidate_color_theme(&mut document, CandidateColorTheme::SeaBreeze);
        assert_eq!(candidate_colors(&document).text, 0xFF_FFFF);
        assert_eq!(candidate_colors(&document).background, 0x60_7A99);
        assert_eq!(candidate_colors(&document).selected_background, 0x2B_4A70);
        apply_candidate_color_theme(&mut document, CandidateColorTheme::Jade);
        assert_eq!(candidate_colors(&document).selected_background, 0x5C_8273);
        apply_candidate_color_theme(&mut document, CandidateColorTheme::Wisteria);
        assert_eq!(candidate_colors(&document).text, 0xFF_FFFF);
        assert_eq!(candidate_colors(&document).background, 0x7E_6785);
        assert_eq!(candidate_colors(&document).selected_background, 0x56_3F68);
        apply_candidate_color_theme(&mut document, CandidateColorTheme::Catppuccin);
        assert_eq!(candidate_colors(&document).text, 0xEB_EBEB);
        assert_eq!(candidate_colors(&document).background, 0x1A_1A1A);
        assert_eq!(candidate_colors(&document).selected_background, 0x31_4968);

        apply_candidate_color_theme(&mut document, CandidateColorTheme::Custom);
        assert_eq!(
            shown_candidate_color_theme(&document),
            CandidateColorTheme::Custom
        );
        assert_eq!(candidate_colors(&document).selected_background, 0x31_4968);
        document.set_string(&CANDIDATE_TEXT_COLOR, "#010203");
        assert_eq!(candidate_colors(&document).text, 0x01_0203);
    }

    #[test]
    fn colors_chosen_before_the_theme_key_still_select_a_swatch() {
        let mut document = SettingsDocument::default();
        document.set_string(&CANDIDATE_BACKGROUND_COLOR, "#E6E5E3");
        document.set_string(&CANDIDATE_SELECTED_BACKGROUND_COLOR, "#A53945");
        assert_eq!(
            shown_candidate_color_theme(&document),
            CandidateColorTheme::Sakura
        );
        document.set_string(&CANDIDATE_TEXT_COLOR, "#112233");
        assert_eq!(
            shown_candidate_color_theme(&document),
            CandidateColorTheme::Custom
        );
    }
}
