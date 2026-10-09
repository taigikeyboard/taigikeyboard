//! The settings document: the JSON shape both processes read and write, with
//! typed access that answers the default for an absent or unreadable key.
//!
//! Absent means "never touched"; a reset REMOVES keys rather than writing the
//! defaults over them, so a written-through default cannot be mistaken for a
//! choice the user made, and a later version's changed default reaches
//! installs that never chose. macOS keeps a Swift twin: `SettingsStore.swift`
//! `removeStoredValues`.
//!
//! `revision` is the change counter the TIP compares before adopting a
//! reload (roadmap W10): mtime/size are the cheap detector, the revision is
//! the truth. Every mutation bumps it, so a writer cannot save new content
//! under an old revision by forgetting a call.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::choices::{KeyboardLayout, SettingChoice};
use super::engine_settings::{
    next_input_mode, CandidateDisplayMode, DictionarySourceToggles, EngineSettings, InputMode,
    InputModeRequest, KautianSubcollections, SyllableSeparator,
};
use super::keys;
use crate::strings::DisplayLanguage;
use crate::symbols::RecentSymbols;

/// The name of one setting, paired with the value used when the user has
/// never touched it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsKey<T: Copy + 'static> {
    pub name: &'static str,
    pub default: T,
}

impl<T: Copy + 'static> SettingsKey<T> {
    pub const fn new(name: &'static str, default: T) -> Self {
        Self { name, default }
    }
}

/// The whole document. `values` holds only what was explicitly stored.
/// Unknown top-level fields are dropped on a round trip — schema metadata
/// added later must live inside `values` or bump this shape.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SettingsDocument {
    /// Monotonically increasing per mutation (saturating, never wrapping).
    /// `0` = never written.
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    values: BTreeMap<String, Value>,
}

impl SettingsDocument {
    /// Parses the on-disk JSON. Unknown keys are kept verbatim (a newer version
    /// may have written them); a malformed file is an error the caller keeps
    /// last-known-good over.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let mut document: Self = serde_json::from_str(json)?;
        document.carry_over_hyphenless_roman();
        Ok(document)
    }

    /// The retired No Hyphens switch becomes the Syllable Separator, in
    /// memory: a stored `true` reads as `none` unless a separator is already
    /// stored, and the switch goes either way. Every reader of the file
    /// agrees, and the next write persists it; the revision stays — the
    /// user's choice did not change. Twin of iOS / Android
    /// `carryOverHyphenlessRoman`.
    fn carry_over_hyphenless_roman(&mut self) {
        let Some(retired) = self.values.remove(keys::RETIRED_HYPHENLESS_ROMAN_ENABLED) else {
            return;
        };
        if retired.as_bool() == Some(true) && !self.contains(keys::SYLLABLE_SEPARATOR.name) {
            self.values.insert(
                keys::SYLLABLE_SEPARATOR.name.to_owned(),
                Value::String(SyllableSeparator::None.raw().to_owned()),
            );
        }
    }

    /// Pretty JSON with sorted keys (the map is a `BTreeMap`), so two writes
    /// of the same state are byte-identical and a diff of the file reads.
    pub fn to_json(&self) -> String {
        // A struct with two plain fields cannot fail to serialize.
        serde_json::to_string_pretty(self).unwrap_or_else(|_| String::from("{}"))
    }

    /// Whether the user ever stored `name`.
    pub fn contains(&self, name: &str) -> bool {
        self.values.contains_key(name)
    }

    /// A boolean, or its default when absent or not a boolean. `object(forKey:)
    /// as? Bool ?? default` on macOS — never `bool(forKey:)`, which answers
    /// `false` for an absent key and would turn every default-ON setting off.
    pub fn bool(&self, key: &SettingsKey<bool>) -> bool {
        self.values
            .get(key.name)
            .and_then(Value::as_bool)
            .unwrap_or(key.default)
    }

    /// A string, or its default when absent or not a string.
    pub fn string(&self, key: &SettingsKey<&'static str>) -> String {
        self.values
            .get(key.name)
            .and_then(Value::as_str)
            .map_or_else(|| key.default.to_owned(), str::to_owned)
    }

    /// An integer, or its default when absent or not an integer.
    pub fn i64(&self, key: &SettingsKey<i64>) -> i64 {
        self.values
            .get(key.name)
            .and_then(Value::as_i64)
            .unwrap_or(key.default)
    }

    /// A string-backed choice, or its default when absent or unrecognised.
    /// macOS keeps a Swift twin: `SettingsStore.swift` `choice(_:)`.
    pub fn choice<T: SettingChoice>(&self, key: &SettingsKey<T>) -> T {
        self.values
            .get(key.name)
            .and_then(Value::as_str)
            .and_then(T::from_raw)
            .unwrap_or(key.default)
    }

    /// The raw stored string, if any — for keys whose value space is not a
    /// closed choice (the composing chords: absent = default, `""` = cleared).
    pub fn raw_string(&self, name: &str) -> Option<&str> {
        self.values.get(name).and_then(Value::as_str)
    }

    pub fn set_bool(&mut self, key: &SettingsKey<bool>, value: bool) {
        self.set_raw(key.name, Value::Bool(value));
    }

    pub fn set_string(&mut self, key: &SettingsKey<&'static str>, value: &str) {
        self.set_raw(key.name, Value::String(value.to_owned()));
    }

    pub fn set_i64(&mut self, key: &SettingsKey<i64>, value: i64) {
        self.set_raw(key.name, Value::from(value));
    }

    pub fn set_choice<T: SettingChoice>(&mut self, key: &SettingsKey<T>, value: T) {
        self.set_raw(key.name, Value::String(value.raw().to_owned()));
    }

    /// Stores an arbitrary string under `name` (composing chords).
    pub fn set_raw_string(&mut self, name: &str, value: &str) {
        self.set_raw(name, Value::String(value.to_owned()));
    }

    /// A list of strings, in stored order. A value that is not an array of
    /// strings — one non-string item spoils it, as `UserDefaults
    /// .stringArray(forKey:)` reads it on macOS — reads as the default.
    pub fn string_list(&self, key: &SettingsKey<&'static [&'static str]>) -> Vec<String> {
        self.values
            .get(key.name)
            .and_then(Value::as_array)
            .and_then(|items| {
                items
                    .iter()
                    .map(|item| item.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_else(|| key.default.iter().map(|s| (*s).to_owned()).collect())
    }

    pub fn set_string_list(
        &mut self,
        key: &SettingsKey<&'static [&'static str]>,
        values: &[String],
    ) {
        self.set_raw(key.name, Value::from(values));
    }

    /// The display language as stored — `System` included, which is what
    /// the picker shows selected.
    pub fn display_language(&self) -> DisplayLanguage {
        DisplayLanguage::from_tag(&self.string(&keys::DISPLAY_LANGUAGE))
    }

    /// The language strings are drawn in: the stored one, `System` resolved
    /// against the machine's `system_locale`
    /// (`DisplayLanguageStore.syncFromSettings`).
    pub fn effective_display_language(&self, system_locale: &str) -> DisplayLanguage {
        self.display_language().effective(system_locale)
    }

    /// The symbol picker's recent picks (`SettingsStore.swift` `recentSymbols` is the macOS twin).
    pub fn recent_symbols(&self) -> RecentSymbols {
        RecentSymbols::new(self.string_list(&keys::RECENT_SYMBOLS))
    }

    /// Moves `symbol` to the front of the recent picks. Re-picking the front
    /// symbol writes nothing, so it moves no revision.
    pub fn note_recent_symbol(&mut self, symbol: &str) {
        let before = self.recent_symbols();
        let after = before.noting(symbol);
        if after != before {
            self.set_string_list(&keys::RECENT_SYMBOLS, after.symbols());
        }
    }

    fn set_raw(&mut self, name: &str, value: Value) {
        self.values.insert(name.to_owned(), value);
        self.bump_revision();
    }

    /// Forgets `name`, so it reads as its default again. A no-op (no
    /// revision bump) for a key that was not stored.
    pub fn remove(&mut self, name: &str) {
        if self.values.remove(name).is_some() {
            self.bump_revision();
        }
    }

    fn bump_revision(&mut self) {
        self.revision = self.revision.saturating_add(1);
    }

    /// The one writer of the input mode (desktop TPS roadmap D5): moves it as
    /// `request` asks and, when a romanization is left for TPS, remembers
    /// that romanization for the way back. Answers the mode now in force.
    pub fn switch_input_mode(&mut self, request: InputModeRequest) -> InputMode {
        let current: InputMode = self.choice(&keys::INPUT_MODE);
        let next = next_input_mode(current, self.choice(&keys::LAST_ROMANIZATION_MODE), request);
        if let (Some(left), InputMode::Tps) = (current.romanization(), next) {
            self.set_choice(&keys::LAST_ROMANIZATION_MODE, left);
        }
        self.set_choice(&keys::INPUT_MODE, next);
        next
    }

    /// Whether TPS is the input mode in force.
    pub fn is_typing_tps(&self) -> bool {
        self.choice::<InputMode>(&keys::INPUT_MODE) == InputMode::Tps
    }

    /// The layout keys are read in: the chosen one, except under TPS, whose
    /// glyphs keep their QWERTY positions. macOS twin: the picker binding
    /// and `TaigiInputController` keyboard override (QWERTY under TPS).
    pub fn key_reading_layout(&self) -> KeyboardLayout {
        if self.is_typing_tps() {
            return KeyboardLayout::Qwerty;
        }
        self.choice(&keys::KEYBOARD_LAYOUT)
    }

    /// Whether the on-screen TPS key panel should be up: the user asked for
    /// it and TPS is being typed (desktop TPS roadmap D6). macOS twin:
    /// `SettingsStore.isTpsKeyboardWanted`.
    pub fn is_tps_keyboard_wanted(&self) -> bool {
        self.is_typing_tps() && self.bool(&keys::TPS_KEYBOARD_SHOWN)
    }

    /// Puts every setting the General pane owns back to shipped state.
    pub fn reset_general(&mut self) {
        for name in keys::GENERAL_KEYS {
            self.remove(name);
        }
    }

    /// Puts every key the Appearance pane owns back to shipped state.
    pub fn reset_appearance(&mut self) {
        for name in keys::APPEARANCE_KEYS {
            self.remove(name);
        }
    }

    /// Records `chord` on `action`, or clears the row when `None`
    /// (`SettingsStore.swift` `setComposingChord` is the macOS twin).
    pub fn set_composing_chord(
        &mut self,
        action: crate::keys::ComposingAction,
        chord: Option<&crate::keys::ComposingKeyChord>,
        platform: crate::platform::DesktopPlatform,
    ) {
        let value = chord.map_or_else(
            || keys::CLEARED_COMPOSING_CHORD.to_owned(),
            |chord| chord.raw_value(platform),
        );
        self.set_raw_string(&action.settings_key_name(), &value);
    }

    /// Puts every key the shortcuts pane owns on the composing side back to
    /// shipped state — removed, not written (`SettingsStore.swift`
    /// `resetComposingShortcuts` is the macOS twin).
    pub fn reset_composing_shortcuts(&mut self) {
        for action in crate::keys::ComposingAction::ALL {
            self.remove(&action.settings_key_name());
        }
    }

    /// Puts every global shortcut row back to shipped state — removed, not
    /// written, like the composing rows. The pane's reset calls both.
    pub fn reset_global_shortcuts(&mut self) {
        for action in crate::keys::ShortcutAction::ALL {
            self.remove(&action.settings_key_name());
        }
    }

    /// Puts every toggle the Dictionary Sources pane owns back to shipped state.
    pub fn reset_dictionary_sources(&mut self) {
        for name in keys::DICTIONARY_SOURCE_KEYS {
            self.remove(name);
        }
    }

    /// The engine-facing snapshot as of this document.
    ///
    /// The swap comes out DERIVED — the rules live on `CandidateDisplayMode`
    /// (invariants §42). The stored bool is left alone, so switching back to
    /// side-by-side restores it. Every consumer of the swap reads it from
    /// here, never `bool(&IS_HANJI_FIRST)`
    /// directly; the raw read is for the panes and the toggle shortcut that
    /// write it.
    pub fn engine_settings(&self) -> EngineSettings {
        let candidate_display_mode: CandidateDisplayMode =
            self.choice(&keys::CANDIDATE_DISPLAY_MODE);
        let stored_swap = self.bool(&keys::IS_HANJI_FIRST);
        let input_mode: InputMode = self.choice(&keys::INPUT_MODE);
        EngineSettings {
            input_mode,
            is_hanji_first: candidate_display_mode.effective_hanji_first(stored_swap),
            // Full width always under TPS, as on mobile
            // (`ios/.../SharedSettings.swift` `isFullWidthPunctuation`).
            is_full_width_punctuation: input_mode == InputMode::Tps
                || candidate_display_mode.effective_full_width_punctuation(stored_swap),
            candidate_display_mode,
            // Show Typed Text First is a romanization literal: off under TPS,
            // so the engine never prepends one (a lone tone mark still
            // composes as TL there) and every TPS cell keeps its key.
            is_literal_roman_candidate_enabled: input_mode != InputMode::Tps
                && self.bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED),
            syllable_separator: self.choice(&keys::SYLLABLE_SEPARATOR),
            is_nasal_marker_uppercase_enabled: self.bool(&keys::IS_NASAL_MARKER_UPPERCASE_ENABLED),
            is_custom_dict_enabled: self.bool(&keys::IS_CUSTOM_DICT_ENABLED),
            dictionary_sources: self.dictionary_sources(),
        }
    }

    /// Read as part of `engine_settings` so the toggles the engine filters
    /// by and the settings it composes under come from one instant.
    pub fn dictionary_sources(&self) -> DictionarySourceToggles {
        DictionarySourceToggles {
            kautian: self.bool(&keys::IS_KAUTIAN_ENABLED),
            taigitv: self.bool(&keys::IS_TAIGITV_ENABLED),
            itaigi: self.bool(&keys::IS_ITAIGI_ENABLED),
            sitbut: self.bool(&keys::IS_SITBUT_ENABLED),
            taihoa: self.bool(&keys::IS_TAIHOA_ENABLED),
            taijit: self.bool(&keys::IS_TAIJIT_ENABLED),
            kungge: self.bool(&keys::IS_KUNGGE_ENABLED),
            stti: self.bool(&keys::IS_STTI_ENABLED),
            khpoo: self.bool(&keys::IS_KHPOO_ENABLED),
            variant: self.bool(&keys::IS_VARIANT_ENABLED),
            khiin: self.bool(&keys::IS_KHIIN_ENABLED),
            lkk: self.bool(&keys::IS_LKK_ENABLED),
            dev: self.bool(&keys::IS_DEV_ENABLED),
            kautian_subcollections: KautianSubcollections {
                accent_lukang: self.bool(&keys::IS_KAUTIAN_ACCENT_LUKANG_ENABLED),
                accent_sansia: self.bool(&keys::IS_KAUTIAN_ACCENT_SANSIA_ENABLED),
                accent_taipak: self.bool(&keys::IS_KAUTIAN_ACCENT_TAIPAK_ENABLED),
                accent_gilan: self.bool(&keys::IS_KAUTIAN_ACCENT_GILAN_ENABLED),
                accent_tainan: self.bool(&keys::IS_KAUTIAN_ACCENT_TAINAN_ENABLED),
                accent_kaohsiung: self.bool(&keys::IS_KAUTIAN_ACCENT_KAOHSIUNG_ENABLED),
                accent_kinmen: self.bool(&keys::IS_KAUTIAN_ACCENT_KINMEN_ENABLED),
                accent_makung: self.bool(&keys::IS_KAUTIAN_ACCENT_MAKUNG_ENABLED),
                accent_sintik: self.bool(&keys::IS_KAUTIAN_ACCENT_SINTIK_ENABLED),
                accent_taichung: self.bool(&keys::IS_KAUTIAN_ACCENT_TAICHUNG_ENABLED),
                name_appendix: self.bool(&keys::IS_KAUTIAN_NAME_APPENDIX_ENABLED),
                alt_reading: self.bool(&keys::IS_KAUTIAN_ALT_READING_ENABLED),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{AppearanceMode, CandidateLayout, InputMode};

    #[test]
    fn keys_are_read_in_the_chosen_layout_except_under_tps() {
        // trace: KEYBOARD_LAYOUT default = KeyboardLayout::DEFAULT = Qwerty;
        // INPUT_MODE Tps → `is_typing_tps` → Qwerty whatever is stored;
        // GENERAL_KEYS holds keyboardLayout, so the reset removes it.
        let mut document = SettingsDocument::default();
        assert_eq!(document.key_reading_layout(), KeyboardLayout::Qwerty);
        document.set_choice(&keys::KEYBOARD_LAYOUT, KeyboardLayout::Dvorak);
        assert_eq!(document.key_reading_layout(), KeyboardLayout::Dvorak);
        document.set_choice(&keys::INPUT_MODE, InputMode::Tps);
        assert_eq!(document.key_reading_layout(), KeyboardLayout::Qwerty);
        document.reset_general();
        assert_eq!(
            document.choice(&keys::KEYBOARD_LAYOUT),
            KeyboardLayout::Qwerty
        );
    }

    #[test]
    fn recent_symbols_round_trip_and_an_unchanged_list_moves_no_revision() {
        let mut document = SettingsDocument::default();
        assert!(document.recent_symbols().symbols().is_empty());
        document.note_recent_symbol("。");
        assert_eq!(document.revision, 1);
        assert_eq!(document.recent_symbols().symbols(), ["。"]);
        let reloaded = SettingsDocument::from_json(&document.to_json()).unwrap();
        assert_eq!(reloaded.recent_symbols().symbols(), ["。"]);
        document.note_recent_symbol("。");
        assert_eq!(
            document.revision, 1,
            "re-picking the front symbol writes nothing"
        );
        document.set_raw_string(keys::RECENT_SYMBOLS.name, "not a list");
        assert!(document.recent_symbols().symbols().is_empty());
        document.set_raw(keys::RECENT_SYMBOLS.name, serde_json::json!(["。", 1]));
        assert!(
            document.recent_symbols().symbols().is_empty(),
            "one non-string item spoils the list, as on macOS"
        );
    }

    #[test]
    fn empty_document_reads_every_default() {
        let doc = SettingsDocument::default();
        assert_eq!(doc.engine_settings(), EngineSettings::default());
        // Hanji-first out of the box (USER 2026-09-18), and the punctuation
        // width derived from it under side-by-side follows.
        assert!(doc.engine_settings().is_hanji_first);
        assert!(doc.engine_settings().is_full_width_punctuation);
        assert!(!doc.bool(&keys::IS_AUTO_SPACE_ENABLED));
        assert_eq!(doc.string(&keys::DISPLAY_LANGUAGE), "system");
        assert_eq!(doc.display_language(), DisplayLanguage::System);
        assert_eq!(
            doc.choice(&keys::CANDIDATE_LAYOUT),
            CandidateLayout::Vertical
        );
        assert_eq!(doc.revision, 0);
    }

    #[test]
    fn absent_bool_is_default_not_false() {
        // trace: macOS `UserDefaults.bool(forKey:)` trap — a fresh install must
        // keep the custom dictionary ON even though nothing was ever stored.
        let doc = SettingsDocument::default();
        assert!(doc.bool(&keys::IS_CUSTOM_DICT_ENABLED));
        let mut stored = SettingsDocument::default();
        stored.set_bool(&keys::IS_CUSTOM_DICT_ENABLED, false);
        assert!(!stored.bool(&keys::IS_CUSTOM_DICT_ENABLED));
    }

    #[test]
    fn unknown_choice_and_wrong_type_read_as_default() {
        let doc = SettingsDocument::from_json(
            r#"{"revision": 3, "values": {"inputMode": "bopomofo", "autoSpaceEnabled": "yes", "candidateAppearanceMode": "dark"}}"#,
        )
        .unwrap();
        assert_eq!(doc.choice(&keys::INPUT_MODE), InputMode::Tl);
        assert!(!doc.bool(&keys::IS_AUTO_SPACE_ENABLED));
        assert_eq!(doc.choice(&keys::APPEARANCE_MODE), AppearanceMode::Dark);
        assert_eq!(doc.revision, 3);
    }

    #[test]
    fn json_round_trip_is_stable_and_keeps_unknown_keys() {
        let mut doc = SettingsDocument::default();
        doc.set_choice(&keys::INPUT_MODE, InputMode::Poj);
        doc.set_bool(&keys::IS_KHIIN_ENABLED, true);
        doc.set_raw_string("futureKey", "kept");
        let json = doc.to_json();
        let again = SettingsDocument::from_json(&json).unwrap();
        assert_eq!(again, doc);
        assert_eq!(again.to_json(), json);
        assert_eq!(again.raw_string("futureKey"), Some("kept"));
        assert_eq!(again.revision, 3, "one bump per mutation");
        let mut untouched = again.clone();
        untouched.remove("neverStored");
        assert_eq!(
            untouched.revision, 3,
            "removing an absent key is not a change"
        );
    }

    #[test]
    fn reset_removes_keys_rather_than_writing_defaults() {
        let mut doc = SettingsDocument::default();
        doc.set_choice(&keys::CANDIDATE_LAYOUT, CandidateLayout::Horizontal);
        doc.set_bool(&keys::IS_KAUTIAN_ENABLED, false);
        doc.set_bool(&keys::IS_AUTO_SPACE_ENABLED, false);
        doc.reset_appearance();
        doc.reset_dictionary_sources();
        assert!(!doc.contains(keys::CANDIDATE_LAYOUT.name));
        assert!(!doc.contains(keys::IS_KAUTIAN_ENABLED.name));
        assert!(
            doc.contains(keys::IS_AUTO_SPACE_ENABLED.name),
            "not an appearance or source key"
        );
        assert!(doc.engine_settings().dictionary_sources.kautian);
    }

    #[test]
    fn candidate_size_reads_the_one_knob_and_the_two_knob_spellings() {
        use crate::settings::CandidateSizeChoice;
        // Never touched → the new default; the text ladder's old spellings
        // → the nearest step; the retired window knob changes nothing; a
        // read writes nothing; the Appearance reset removes the key.
        let mut doc = SettingsDocument::default();
        assert_eq!(
            doc.choice(&keys::CANDIDATE_SIZE),
            CandidateSizeChoice::Standard
        );
        for (stored, expected) in [
            ("small", CandidateSizeChoice::Small),
            ("medium", CandidateSizeChoice::Large),
            ("large", CandidateSizeChoice::ExtraLarge),
            ("13", CandidateSizeChoice::ExtraSmall),
            ("23", CandidateSizeChoice::ExtraLarge),
            ("gigantic", CandidateSizeChoice::Standard),
        ] {
            doc.set_raw_string(keys::CANDIDATE_SIZE.name, stored);
            assert_eq!(doc.choice(&keys::CANDIDATE_SIZE), expected, "{stored}");
        }
        doc.set_bool(&SettingsKey::new(keys::CANDIDATE_SIZE.name, false), true);
        assert_eq!(
            doc.choice(&keys::CANDIDATE_SIZE),
            CandidateSizeChoice::Standard,
            "a non-string value reads as the default"
        );
        doc.set_raw_string("candidateWindowSize", "large");
        doc.set_raw_string(keys::CANDIDATE_SIZE.name, "medium");
        let revision = doc.revision;
        assert_eq!(
            doc.choice(&keys::CANDIDATE_SIZE),
            CandidateSizeChoice::Large
        );
        assert_eq!(doc.revision, revision, "reading migrates nothing");
        assert_eq!(doc.raw_string(keys::CANDIDATE_SIZE.name), Some("medium"));
        doc.reset_appearance();
        assert!(!doc.contains(keys::CANDIDATE_SIZE.name));
        assert_eq!(
            doc.choice(&keys::CANDIDATE_SIZE),
            CandidateSizeChoice::Standard
        );
    }

    // INVARIANT_DESKTOP_GENERAL_PANE_OUTPUT_SCRIPT_AND_RESET (behavioral-invariants.md §48)
    #[test]
    fn reset_general_removes_the_pane_s_keys_and_nothing_else() {
        // trace: General owns the swap, the tone keys and auto-space; the
        // display language is the user's UI choice, the candidate layout is
        // Appearance's and the update date is bookkeeping — all three survive.
        // Removed, not written: the swap reads its default (hanji-first)
        // with no key stored.
        let mut doc = SettingsDocument::default();
        doc.set_bool(&keys::IS_HANJI_FIRST, false);
        doc.set_bool(&keys::IS_AUTO_SPACE_ENABLED, true);
        doc.set_raw_string(keys::DISPLAY_LANGUAGE.name, "en");
        doc.set_choice(&keys::CANDIDATE_LAYOUT, CandidateLayout::Horizontal);
        doc.set_i64(&keys::UPDATE_NEXT_CHECK_MS, 42);
        doc.reset_general();
        assert!(!doc.contains(keys::IS_HANJI_FIRST.name));
        assert!(!doc.contains(keys::IS_AUTO_SPACE_ENABLED.name));
        assert!(doc.engine_settings().is_hanji_first);
        assert!(
            doc.contains(keys::DISPLAY_LANGUAGE.name),
            "display language kept"
        );
        assert!(doc.contains(keys::CANDIDATE_LAYOUT.name), "外觀's key");
        assert!(doc.contains(keys::UPDATE_NEXT_CHECK_MS.name), "bookkeeping");
    }

    #[test]
    fn the_tps_key_panel_is_wanted_under_tps_with_its_key_on_and_reset_clears_it() {
        // trace: is_tps_keyboard_wanted = mode Tps && tpsKeyboardShown; the
        // key survives a switch away and back; GENERAL_KEYS removes it.
        let mut doc = SettingsDocument::default();
        doc.set_bool(&keys::TPS_KEYBOARD_SHOWN, true);
        assert!(!doc.is_tps_keyboard_wanted(), "TL: nothing to show");
        doc.switch_input_mode(InputModeRequest::ToggleTps);
        assert!(doc.is_tps_keyboard_wanted());
        doc.switch_input_mode(InputModeRequest::ToggleTps);
        assert!(!doc.is_tps_keyboard_wanted());
        assert!(doc.bool(&keys::TPS_KEYBOARD_SHOWN), "kept for the way back");
        doc.switch_input_mode(InputModeRequest::ToggleTps);
        doc.set_bool(&keys::TPS_KEYBOARD_SHOWN, false);
        assert!(!doc.is_tps_keyboard_wanted());
        doc.set_bool(&keys::TPS_KEYBOARD_SHOWN, true);
        doc.reset_general();
        assert!(!doc.contains(keys::TPS_KEYBOARD_SHOWN.name));
        assert!(!doc.is_tps_keyboard_wanted());
    }

    #[test]
    fn roman_only_masks_the_swap_without_touching_what_is_stored() {
        // trace: stored swap=true; mode=romanOnly → the snapshot reads false
        // while `bool(&key)` still answers true; back to sideBySide → true
        // again with no write in between.
        let mut doc = SettingsDocument::default();
        doc.set_bool(&keys::IS_HANJI_FIRST, true);
        doc.set_choice(
            &keys::CANDIDATE_DISPLAY_MODE,
            CandidateDisplayMode::RomanOnly,
        );
        let snapshot = doc.engine_settings();
        assert_eq!(
            snapshot.candidate_display_mode,
            CandidateDisplayMode::RomanOnly
        );
        assert!(!snapshot.is_hanji_first);
        assert!(doc.bool(&keys::IS_HANJI_FIRST), "stored value untouched");
        let revision = doc.revision;
        doc.set_choice(
            &keys::CANDIDATE_DISPLAY_MODE,
            CandidateDisplayMode::SideBySide,
        );
        assert!(doc.engine_settings().is_hanji_first);
        assert_eq!(doc.revision, revision + 1, "only the mode was written");
    }

    #[test]
    fn combined_forces_the_swap_and_leaves_the_stored_swap_alone() {
        // trace: stored swap=false (written — the fresh default is
        // hanji-first); mode=combined → true: each script is its own adjacent
        // cell, hanji first, and a commit writes the hanji — the projection
        // of that onto the flag is a forced swap. Roman-only still masks to
        // false; back to sideBySide reads the stored false again with no bool
        // written in between.
        let mut doc = SettingsDocument::default();
        doc.set_bool(&keys::IS_HANJI_FIRST, false);
        doc.set_choice(
            &keys::CANDIDATE_DISPLAY_MODE,
            CandidateDisplayMode::Combined,
        );
        let snapshot = doc.engine_settings();
        assert_eq!(
            snapshot.candidate_display_mode,
            CandidateDisplayMode::Combined
        );
        assert!(snapshot.is_hanji_first);
        assert!(!doc.bool(&keys::IS_HANJI_FIRST), "stored value untouched");

        doc.set_choice(
            &keys::CANDIDATE_DISPLAY_MODE,
            CandidateDisplayMode::RomanOnly,
        );
        assert!(!doc.engine_settings().is_hanji_first);

        let revision = doc.revision;
        doc.set_choice(
            &keys::CANDIDATE_DISPLAY_MODE,
            CandidateDisplayMode::SideBySide,
        );
        assert!(!doc.engine_settings().is_hanji_first);
        assert_eq!(doc.revision, revision + 1, "only the mode was written");
    }

    /// The rules `engine_settings()` and the swap shortcut read live on the enum — pinned once.
    #[test]
    fn candidate_display_mode_rules_per_mode() {
        use CandidateDisplayMode::{Combined, RomanOnly, SideBySide};
        assert!(
            SideBySide.allows_swap_toggle()
                && Combined.allows_swap_toggle()
                && !RomanOnly.allows_swap_toggle()
        );
        assert!(SideBySide.shows_hanji() && Combined.shows_hanji() && !RomanOnly.shows_hanji());
        assert!(Combined.effective_hanji_first(false));
        assert!(!RomanOnly.effective_hanji_first(true));
        // Punctuation width follows the STORED swap under side-by-side /
        // combined, never under roman-only.
        assert!(!Combined.effective_full_width_punctuation(false));
        assert!(Combined.effective_full_width_punctuation(true));
        assert!(!RomanOnly.effective_full_width_punctuation(true));
    }

    /// Under combined the candidate projection stays swapped while the
    /// punctuation width follows the stored flag the swap shortcut toggles.
    #[test]
    fn combined_punctuation_width_follows_the_stored_swap() {
        let half = SettingsDocument::from_json(
            r#"{"revision": 1, "values": {"candidateDisplayMode": "combined", "isTranslateSwapped": false}}"#,
        )
        .unwrap()
        .engine_settings();
        assert!(half.is_hanji_first && !half.is_full_width_punctuation);

        let full = SettingsDocument::from_json(
            r#"{"revision": 1, "values": {"candidateDisplayMode": "combined", "isTranslateSwapped": true}}"#,
        )
        .unwrap()
        .engine_settings();
        assert!(full.is_hanji_first && full.is_full_width_punctuation);

        let roman_only = SettingsDocument::from_json(
            r#"{"revision": 1, "values": {"candidateDisplayMode": "romanOnly", "isTranslateSwapped": true}}"#,
        )
        .unwrap()
        .engine_settings();
        assert!(!roman_only.is_full_width_punctuation);
    }

    #[test]
    fn unknown_candidate_display_mode_reads_as_side_by_side() {
        let doc = SettingsDocument::from_json(
            r#"{"revision": 1, "values": {"candidateDisplayMode": "hanlo", "isTranslateSwapped": true}}"#,
        )
        .unwrap();
        assert_eq!(
            doc.choice(&keys::CANDIDATE_DISPLAY_MODE),
            CandidateDisplayMode::SideBySide
        );
        assert!(doc.engine_settings().is_hanji_first);
        assert_eq!(
            SettingsDocument::default().choice(&keys::CANDIDATE_DISPLAY_MODE),
            CandidateDisplayMode::SideBySide
        );
    }

    /// §34/S22 — the OFF default is covered by `empty_document_reads_every_default`;
    /// what only this pins is that `engine_settings()` maps THIS key, so a user
    /// who turned Show Typed Text First on keeps it on across the 2026-10-02 flip.
    #[test]
    fn literal_roman_candidate_honours_a_stored_true() {
        let mut doc = SettingsDocument::default();

        doc.set_bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED, true);

        assert!(doc.engine_settings().is_literal_roman_candidate_enabled);
    }

    #[test]
    fn syllable_separator_reads_the_stored_choice() {
        let mut doc = SettingsDocument::default();
        assert_eq!(
            doc.engine_settings().syllable_separator,
            SyllableSeparator::Hyphen,
            "ships Hyphen"
        );
        doc.set_choice(&keys::SYLLABLE_SEPARATOR, SyllableSeparator::Space);
        assert_eq!(
            doc.engine_settings().syllable_separator,
            SyllableSeparator::Space
        );
        doc.reset_general();
        assert!(!doc.contains(keys::SYLLABLE_SEPARATOR.name), "一般's key");
    }

    #[test]
    fn a_stored_no_hyphens_switch_carries_over_as_none() {
        let parse = |json: &str| SettingsDocument::from_json(json).unwrap();
        let on = parse(r#"{"revision":3,"values":{"hyphenlessRomanEnabled":true}}"#);
        assert_eq!(
            on.choice(&keys::SYLLABLE_SEPARATOR),
            SyllableSeparator::None
        );
        assert!(!on.contains(keys::RETIRED_HYPHENLESS_ROMAN_ENABLED));
        assert_eq!(on.revision, 3, "the user's choice did not change");
        // The carried-over value is what the next write persists.
        assert_eq!(parse(&on.to_json()), on);

        let off = parse(r#"{"values":{"hyphenlessRomanEnabled":false}}"#);
        assert!(!off.contains(keys::SYLLABLE_SEPARATOR.name));
        assert!(!off.contains(keys::RETIRED_HYPHENLESS_ROMAN_ENABLED));

        let both =
            parse(r#"{"values":{"hyphenlessRomanEnabled":true,"syllableSeparator":"space"}}"#);
        assert_eq!(
            both.choice(&keys::SYLLABLE_SEPARATOR),
            SyllableSeparator::Space,
            "a stored successor wins"
        );
        assert!(!both.contains(keys::RETIRED_HYPHENLESS_ROMAN_ENABLED));
    }

    #[test]
    fn nasal_marker_uppercase_reads_the_stored_switch() {
        let mut doc = SettingsDocument::default();
        assert!(
            doc.engine_settings().is_nasal_marker_uppercase_enabled,
            "ships ON"
        );
        doc.set_bool(&keys::IS_NASAL_MARKER_UPPERCASE_ENABLED, false);
        assert!(!doc.engine_settings().is_nasal_marker_uppercase_enabled);
        doc.reset_general();
        assert!(
            !doc.contains(keys::IS_NASAL_MARKER_UPPERCASE_ENABLED.name),
            "一般's key"
        );
    }

    #[test]
    fn dictionary_sources_read_every_toggle() {
        let mut doc = SettingsDocument::default();
        doc.set_bool(&keys::IS_KAUTIAN_ACCENT_GILAN_ENABLED, false);
        doc.set_bool(&keys::IS_DEV_ENABLED, false);
        let sources = doc.dictionary_sources();
        assert!(!sources.kautian_subcollections.accent_gilan);
        assert!(sources.kautian_subcollections.accent_lukang);
        assert!(!sources.dev);
        assert!(sources.lkk);
    }

    /// A stored value that differs from its default reaches the field
    /// `engine_settings()` maps it to: a default-on setting switched off reads
    /// as off, a default-off one switched on reads as on. Ported from the
    /// macOS `SettingsStore.current` tests (P13b).
    #[test]
    fn engine_settings_maps_stored_overrides() {
        let mut doc = SettingsDocument::default();
        doc.set_choice(&keys::INPUT_MODE, InputMode::Poj);
        doc.set_bool(&keys::IS_HANJI_FIRST, false);
        doc.set_bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED, true);
        doc.set_choice(&keys::SYLLABLE_SEPARATOR, SyllableSeparator::None);
        doc.set_bool(&keys::IS_NASAL_MARKER_UPPERCASE_ENABLED, false);
        doc.set_bool(&keys::IS_CUSTOM_DICT_ENABLED, false);
        doc.set_bool(&keys::IS_KAUTIAN_ENABLED, false);
        doc.set_bool(&keys::IS_KHIIN_ENABLED, true);
        doc.set_bool(&keys::IS_KAUTIAN_ACCENT_GILAN_ENABLED, false);

        // trace: stored swap=false under the default SideBySide →
        // effective_hanji_first(false)=false, effective_full_width_punctuation(false)=false;
        // every field not set above keeps its default.
        let expected = EngineSettings {
            input_mode: InputMode::Poj,
            is_hanji_first: false,
            is_full_width_punctuation: false,
            candidate_display_mode: CandidateDisplayMode::SideBySide,
            is_literal_roman_candidate_enabled: true,
            syllable_separator: SyllableSeparator::None,
            is_nasal_marker_uppercase_enabled: false,
            is_custom_dict_enabled: false,
            dictionary_sources: DictionarySourceToggles {
                kautian: false,
                khiin: true,
                kautian_subcollections: KautianSubcollections {
                    accent_gilan: false,
                    ..KautianSubcollections::DEFAULT
                },
                ..DictionarySourceToggles::DEFAULT
            },
        };
        assert_eq!(doc.engine_settings(), expected);
    }

    #[test]
    fn tps_reads_back_and_its_punctuation_is_always_full_width() {
        // trace: `engine_settings` — Romanization Only derives half width
        // under TL; under TPS the width is full whatever the display mode.
        let mut doc = SettingsDocument::default();
        doc.set_choice(
            &keys::CANDIDATE_DISPLAY_MODE,
            CandidateDisplayMode::RomanOnly,
        );
        assert!(!doc.engine_settings().is_full_width_punctuation);
        doc.set_choice(&keys::INPUT_MODE, InputMode::Tps);
        assert_eq!(doc.choice(&keys::INPUT_MODE), InputMode::Tps);
        assert!(doc.engine_settings().is_full_width_punctuation);
    }

    #[test]
    fn show_typed_text_first_reads_off_under_tps_and_the_stored_value_survives() {
        let mut doc = SettingsDocument::default();
        doc.set_bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED, true);
        assert!(doc.engine_settings().is_literal_roman_candidate_enabled);
        doc.set_choice(&keys::INPUT_MODE, InputMode::Tps);
        assert!(!doc.engine_settings().is_literal_roman_candidate_enabled);
        assert!(doc.bool(&keys::IS_LITERAL_ROMAN_CANDIDATE_ENABLED));
    }

    #[test]
    fn a_poj_round_trip_through_tps_returns_to_poj() {
        // trace: POJ →Switch TPS→ TPS, remembering POJ →Switch TPS→ POJ;
        // then →Switch TPS→ TPS →Switch Romanization→ TL (not the POJ last
        // used — that is Switch TPS's way back).
        let mut doc = SettingsDocument::default();
        doc.switch_input_mode(InputModeRequest::Pick(InputMode::Poj));
        assert!(!doc.contains(keys::LAST_ROMANIZATION_MODE.name));
        assert_eq!(
            doc.switch_input_mode(InputModeRequest::ToggleTps),
            InputMode::Tps
        );
        assert_eq!(
            doc.choice(&keys::LAST_ROMANIZATION_MODE),
            super::super::Romanization::Poj
        );
        assert_eq!(
            doc.switch_input_mode(InputModeRequest::ToggleTps),
            InputMode::Poj
        );
        doc.switch_input_mode(InputModeRequest::ToggleTps);
        assert_eq!(
            doc.switch_input_mode(InputModeRequest::ToggleRomanization),
            InputMode::Tl
        );
        assert_eq!(doc.choice(&keys::INPUT_MODE), InputMode::Tl);
    }

    #[test]
    fn tps_with_no_or_a_bad_last_romanization_leaves_as_from_tl() {
        // trace: a document that reached TPS without a switch (a restored
        // mobile backup, a hand edit) has no `lastRomanizationMode`, or one
        // that names TPS — both read as the default TL: Switch TPS → TL,
        // Switch Romanization → POJ. Picking TPS while in TPS writes no
        // romanization.
        for stored in [None, Some("tps")] {
            let mut doc = SettingsDocument::default();
            doc.set_choice(&keys::INPUT_MODE, InputMode::Tps);
            if let Some(raw) = stored {
                doc.set_raw_string(keys::LAST_ROMANIZATION_MODE.name, raw);
            }
            doc.switch_input_mode(InputModeRequest::Pick(InputMode::Tps));
            assert_eq!(
                doc.raw_string(keys::LAST_ROMANIZATION_MODE.name),
                stored,
                "{stored:?}"
            );
            let mut toggled = doc.clone();
            assert_eq!(
                toggled.switch_input_mode(InputModeRequest::ToggleTps),
                InputMode::Tl
            );
            assert_eq!(
                doc.switch_input_mode(InputModeRequest::ToggleRomanization),
                InputMode::Poj
            );
        }
    }

    #[test]
    fn picking_tps_remembers_the_romanization_it_left() {
        let mut doc = SettingsDocument::default();
        doc.switch_input_mode(InputModeRequest::Pick(InputMode::Poj));
        doc.switch_input_mode(InputModeRequest::Pick(InputMode::Tps));
        assert_eq!(
            doc.choice(&keys::LAST_ROMANIZATION_MODE),
            super::super::Romanization::Poj
        );
        doc.reset_general();
        assert!(!doc.contains(keys::LAST_ROMANIZATION_MODE.name));
        assert!(!doc.contains(keys::INPUT_MODE.name));
    }
}
