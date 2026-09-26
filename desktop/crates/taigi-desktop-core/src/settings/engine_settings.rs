//! The snapshot one engine operation reads. Port of
//! `macos/Sources/TaigiInputMethodCore/Settings/EngineSettings.swift` and
//! `DictionarySourceToggles.swift`; every default matches iOS / Android / macOS.

use super::choices::SettingChoice;

/// The romanization the user types. Desktop ships TL and POJ only — TPS is
/// out of scope on Windows as on macOS (`docs/architecture/windows-roadmap.md`
/// § Goal), which is why there is no `Tps` variant to fall through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputMode {
    Tl,
    Poj,
}

impl InputMode {
    /// The `AppConfig.input_mode` wire spelling.
    pub fn wire(self) -> &'static str {
        self.raw()
    }

    /// The picker row's i18n key, as every other choice type carries one
    /// (`GeneralSettingsView.swift`).
    pub fn label_key(self) -> crate::strings::StringKey {
        use crate::strings::StringKey;
        match self {
            Self::Tl => StringKey::SettingsTlMode,
            Self::Poj => StringKey::SettingsPojMode,
        }
    }
}

impl SettingChoice for InputMode {
    const ALL: &'static [Self] = &[Self::Tl, Self::Poj];
    const DEFAULT: Self = Self::Tl;
    fn raw(self) -> &'static str {
        match self {
            Self::Tl => "tl",
            Self::Poj => "poj",
        }
    }
}

/// What a candidate cell shows: both scripts (the swap decides which leads),
/// the romanization alone, or both in ONE label led by the hanji (Hanji with Romanization,
/// `Combined`). Stored spellings and the per-mode rules are the same on every
/// platform (`SettingsModels.swift` `CandidateDisplayMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CandidateDisplayMode {
    SideBySide,
    RomanOnly,
    Combined,
}

impl CandidateDisplayMode {
    /// Whether the cell shows any hanji — `false` only for `RomanOnly`.
    pub const fn shows_hanji(self) -> bool {
        !matches!(self, Self::RomanOnly)
    }

    /// Whether the swap shortcut writes the stored swap — exactly where hanji
    /// is on screen. Under `Combined` the cells are split per script, so the
    /// shortcut only picks the punctuation width (USER 2026-09-13: "Hanji with Romanization needs an
    /// isTranslateSwapped button"); `RomanOnly` leaves it inert and the stored
    /// swap waits for the way back.
    pub fn allows_swap_toggle(self) -> bool {
        self.shows_hanji()
    }

    /// Effective swap for a stored flag. `Combined` leads with — and commits —
    /// the hanji: forcing the pair on is a compatibility projection of that,
    /// so every reader of the pair (auto-space, the nextword gates) behaves
    /// as today's hanji-first mode (invariants §42); full-width punctuation
    /// reads `effective_full_width_punctuation` instead.
    /// `RomanOnly` has no hanji to lead with.
    /// CROSS-PLATFORM INVARIANT — mirrors macOS `EngineSettings.swift`
    /// `CandidateDisplayMode.effectiveTranslateSwapped`, iOS
    /// `SettingsModels.swift`, Android `CandidateDisplayMode.kt`.
    pub const fn effective_translate_swapped(self, stored: bool) -> bool {
        matches!(self, Self::Combined) || (stored && self.shows_hanji())
    }

    /// Effective Annotate in Brackets for a stored flag — off only where there is no hanji
    /// to bracket; `Combined` keeps it (`Hanji (romanization)`).
    pub fn effective_output_both_scripts(self, stored: bool) -> bool {
        stored && self.shows_hanji()
    }

    /// Whether a typed punctuation key becomes full-width (`，` for `,`) for a
    /// stored swap flag — the stored flag masked like Annotate in Brackets, NOT the
    /// candidate projection above, which `Combined` forces on while the swap
    /// shortcut still picks the width. Mirrored on macOS / iOS / Android
    /// beside `effective_output_both_scripts`.
    pub const fn effective_full_width_punctuation(self, stored: bool) -> bool {
        stored && self.shows_hanji()
    }

    /// The `AppConfig.candidate_display_mode` wire value. The engine reads
    /// it through `AppConfig::is_roman_only_display`, so only `RomanOnly`
    /// has to be exact; `SideBySide` is spelled out rather than left
    /// `Unspecified` so a build that sets the field is telling apart from
    /// one that never did. `Combined` has no engine reader either: a combined
    /// cell is distinct by its `(Hanji, romanization)` pair, so nothing collapses.
    pub fn wire(self) -> protos::engine::CandidateDisplayMode {
        match self {
            Self::SideBySide => protos::engine::CandidateDisplayMode::SideBySide,
            Self::RomanOnly => protos::engine::CandidateDisplayMode::RomanOnly,
            Self::Combined => protos::engine::CandidateDisplayMode::Combined,
        }
    }

    /// The picker row's i18n key.
    pub fn label_key(self) -> crate::strings::StringKey {
        use crate::strings::StringKey;
        match self {
            Self::SideBySide => StringKey::SettingsCandidateDisplayModeSideBySide,
            Self::RomanOnly => StringKey::SettingsCandidateDisplayModeRomanOnly,
            Self::Combined => StringKey::SettingsCandidateDisplayModeCombined,
        }
    }

    /// The mode after this one in picker order — what the
    /// `CycleCandidateDisplayMode` shortcut steps to: Pairing → Combined → Romanization Only →
    /// Pairing. CROSS-PLATFORM INVARIANT — mirrors macOS `CandidateDisplayMode.next`.
    pub fn next(self) -> Self {
        match self {
            Self::SideBySide => Self::Combined,
            Self::Combined => Self::RomanOnly,
            Self::RomanOnly => Self::SideBySide,
        }
    }
}

impl SettingChoice for CandidateDisplayMode {
    const ALL: &'static [Self] = &[Self::SideBySide, Self::Combined, Self::RomanOnly];
    /// Today's behaviour, byte for byte.
    const DEFAULT: Self = Self::SideBySide;
    fn raw(self) -> &'static str {
        match self {
            Self::SideBySide => "sideBySide",
            Self::RomanOnly => "romanOnly",
            Self::Combined => "combined",
        }
    }
}

/// Immutable snapshot of everything the engine needs to render a composition.
///
/// A snapshot rather than a set of getters because a single user intent can
/// issue several engine calls (an `Append` is immediately followed by an
/// `EnterContinuous`), and those calls must agree (`EngineSettings.swift:14-20`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineSettings {
    pub input_mode: InputMode,
    /// Word-boundary spacing inputs for the engine's `continuous_word_space`
    /// predicate (`docs/engine/continuous-input-ranking.md` §10.2). Both are
    /// the EFFECTIVE values: the stored toggles AND-ed with `candidate_display_mode
    /// != RomanOnly`, and the swap forced true under `Combined`
    /// (`SettingsDocument::engine_settings`), never the raw document bools —
    /// the raw ones stay untouched so leaving either mode restores them.
    pub is_translate_swapped: bool,
    pub is_output_both_scripts: bool,
    /// `CandidateDisplayMode::effective_full_width_punctuation(stored)` —
    /// read by the TSF session's `document_punctuation` only.
    pub is_full_width_punctuation: bool,
    /// What a candidate cell shows; `AppConfig.candidate_display_mode`.
    /// CROSS-PLATFORM INVARIANT — mirrors
    /// `macos/Sources/TaigiInputMethodCore/Settings/EngineSettings.swift`
    /// `candidateDisplayMode`; every platform defaults to side-by-side.
    pub candidate_display_mode: CandidateDisplayMode,
    /// §34/S22 — inverted onto `FetchAtPos.literal_roman_candidate_disabled`.
    /// CROSS-PLATFORM INVARIANT — mirrors `isLiteralRomanCandidateEnabled`
    /// (`ios/.../SharedSettings.swift`) and `literalRomanCandidateEnabled`
    /// (`android/.../PrefHelper.kt`), both default ON.
    pub is_literal_roman_candidate_enabled: bool,
    /// No Hyphens (`behavioral-invariants.md` §49) — `AppConfig.hyphenless_roman`
    /// on the base config; no TPS layout here, so no fold.
    /// CROSS-PLATFORM INVARIANT — mirrors `isHyphenlessRomanEnabled`
    /// (`macos/.../EngineSettings.swift`, `ios/.../SharedSettings.swift`) and
    /// `hyphenlessRomanEnabled` (`android/.../PrefHelper.kt`), all OFF.
    pub is_hyphenless_roman_enabled: bool,
    /// ⁿ becomes ᴺ in capitals (`behavioral-invariants.md` §53) — the POJ nasal marker follows
    /// the case of the letters before it (`SIÂᴺ`); off, always `ⁿ`. Sent
    /// inverted as `AppConfig.force_lowercase_nasal_marker` on the base config.
    /// CROSS-PLATFORM INVARIANT — mirrors `isNasalMarkerUppercaseEnabled`
    /// (`macos/.../EngineSettings.swift`, `ios/.../SharedSettings.swift`) and
    /// `nasalMarkerUppercaseEnabled` (`android/.../PrefHelper.kt`), all ON.
    pub is_nasal_marker_uppercase_enabled: bool,
    /// Read on the write path only; the boost always applies to whatever was
    /// learned. CROSS-PLATFORM INVARIANT — `SharedSettings.swift:48` (ON).
    pub is_frequency_recording_enabled: bool,
    /// Gates the custom-dictionary lookup itself: off means the engine reads
    /// no custom rows (`FetchAtPos.custom_dictionary_disabled`). CROSS-PLATFORM INVARIANT —
    /// `SharedSettings.swift:51` (ON).
    pub is_custom_dict_enabled: bool,
    pub dictionary_sources: DictionarySourceToggles,
}

impl EngineSettings {
    /// What a fresh install types with. Every value matches the iOS, Android
    /// and macOS default for the same setting (`EngineSettings.swift:79-88`).
    /// A `const` so the settings keys (`keys.rs`) can read their defaults
    /// from it rather than restate them.
    ///
    /// Hanji-first (USER 2026-09-18): the stored swap is on, and the two
    /// effective fields are DERIVED from it under side-by-side the way
    /// `SettingsDocument::engine_settings` derives them, so the snapshot
    /// cannot say one thing about the swap and another about the width.
    pub const DEFAULT: Self = {
        const STORED_SWAP: bool = true;
        const MODE: CandidateDisplayMode = CandidateDisplayMode::SideBySide;
        Self {
            input_mode: InputMode::Tl,
            is_translate_swapped: MODE.effective_translate_swapped(STORED_SWAP),
            is_output_both_scripts: false,
            is_full_width_punctuation: MODE.effective_full_width_punctuation(STORED_SWAP),
            candidate_display_mode: MODE,
            is_literal_roman_candidate_enabled: true,
            is_hyphenless_roman_enabled: false,
            is_nasal_marker_uppercase_enabled: true,
            is_frequency_recording_enabled: true,
            is_custom_dict_enabled: true,
            dictionary_sources: DictionarySourceToggles::DEFAULT,
        }
    };
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Which bundled dictionaries the user has switched on, in the shape the
/// engine's `compute_filters` op reads them. Field order mirrors
/// `engine/protos/proto/lexicon.proto::DictionaryToggles`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionarySourceToggles {
    /// 教育部臺灣台語常用詞辭典 (settings key `moeDictEnabled`).
    pub kautian: bool,
    /// 台語新詞辭庫.
    pub taigitv: bool,
    /// iTaigi 華台對照典.
    pub itaigi: bool,
    /// 台灣植物名彙.
    pub sitbut: bool,
    /// 台華線頂對照典.
    pub taihoa: bool,
    /// 台日大辭典.
    pub taijit: bool,
    /// 台語工藝詞庫.
    pub kungge: bool,
    /// 學科術語辭典.
    pub stti: bool,
    /// 腔口補充資料.
    pub khpoo: bool,
    /// Variant Characters (異用字).
    pub variant: bool,
    /// Conventional Characters (在來字).
    pub khiin: bool,
    /// LKK Han-Lo Recommended Characters.
    pub lkk: bool,
    /// Supplementary Word List (詞庫增補檔案).
    pub dev: bool,
    /// Always populated: an absent subcollection message tells the engine to
    /// skip the gate (`lexicon.proto` `DictionaryToggles.kautian_subcoll`).
    pub kautian_subcollections: KautianSubcollections,
}

impl DictionarySourceToggles {
    /// CROSS-PLATFORM INVARIANT — mirrors `ios/.../SharedSettings.swift:53-66`
    /// and `macos/.../DictionarySourceToggles.swift:99-114`.
    pub const DEFAULT: Self = Self {
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
        kautian_subcollections: KautianSubcollections::DEFAULT,
    };
}

impl Default for DictionarySourceToggles {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// The per-subcollection state of the kautian source. Order mirrors
/// `KautianSubcollToggles` (`lexicon.proto`), the `config.yaml`
/// `dialect_columns` order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KautianSubcollections {
    pub accent_lukang: bool,
    pub accent_sansia: bool,
    pub accent_taipak: bool,
    pub accent_gilan: bool,
    pub accent_tainan: bool,
    pub accent_kaohsiung: bool,
    pub accent_kinmen: bool,
    pub accent_makung: bool,
    pub accent_sintik: bool,
    pub accent_taichung: bool,
    pub name_appendix: bool,
}

impl KautianSubcollections {
    /// CROSS-PLATFORM INVARIANT — every subcollection defaults ON
    /// (`ios/.../SharedSettings.swift:74-84`).
    pub const DEFAULT: Self = Self {
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
    };
}

impl Default for KautianSubcollections {
    fn default() -> Self {
        Self::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_mode_cycle_follows_the_picker_and_returns_in_three_steps() {
        // trace: `SettingChoice::ALL` = [SideBySide, Combined, RomanOnly];
        // `next` walks that ring, so the third step is back at the start.
        let start = CandidateDisplayMode::SideBySide;
        let one = start.next();
        let two = one.next();
        assert_eq!(one, CandidateDisplayMode::Combined);
        assert_eq!(two, CandidateDisplayMode::RomanOnly);
        assert_eq!(two.next(), start);
        for (index, mode) in CandidateDisplayMode::ALL.iter().enumerate() {
            let successor =
                CandidateDisplayMode::ALL[(index + 1) % CandidateDisplayMode::ALL.len()];
            assert_eq!(
                mode.next(),
                successor,
                "{mode:?} steps off the picker order"
            );
        }
    }
}
