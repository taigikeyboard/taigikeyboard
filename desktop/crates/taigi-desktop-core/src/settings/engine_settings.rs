//! The snapshot one engine operation reads. Every default matches iOS /
//! Android. macOS keeps a Swift twin of the default table
//! (`macos/Sources/TaigiInputMethodCore/Settings/EngineSettings.swift`,
//! `DictionarySourceToggles.swift`).

use super::choices::SettingChoice;

/// What the user types: one of the two romanizations, or TPS (方音符號), which
/// is not a romanization — its keys type glyphs on the Dachen-based layout
/// (`keys/tps_layout.rs`, `docs/architecture/desktop-tps-roadmap.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum InputMode {
    Tl,
    Poj,
    Tps,
}

impl InputMode {
    /// The romanization this mode is, or `None` for TPS.
    pub fn romanization(self) -> Option<Romanization> {
        match self {
            Self::Tl => Some(Romanization::Tl),
            Self::Poj => Some(Romanization::Poj),
            Self::Tps => None,
        }
    }

    /// Whether a change from `self` to `other` enters or leaves TPS — where
    /// the raw buffer changes alphabet, so a composition cannot carry over.
    pub fn crosses_tps(self, other: Self) -> bool {
        (self == Self::Tps) != (other == Self::Tps)
    }

    /// The `AppConfig.input_mode` wire spelling.
    pub fn wire(self) -> &'static str {
        self.raw()
    }

    /// The picker row's i18n key, as every other choice type carries one
    /// (the input-script picker in `GeneralSettingsView.swift` is the macOS twin).
    pub fn label_key(self) -> crate::strings::StringKey {
        use crate::strings::StringKey;
        match self {
            Self::Tl => StringKey::SettingsTlMode,
            Self::Poj => StringKey::SettingsPojMode,
            Self::Tps => StringKey::SettingsTpsMode,
        }
    }
}

impl SettingChoice for InputMode {
    const ALL: &'static [Self] = &[Self::Tl, Self::Poj, Self::Tps];
    const DEFAULT: Self = Self::Tl;
    fn raw(self) -> &'static str {
        match self {
            Self::Tl => "tl",
            Self::Poj => "poj",
            Self::Tps => "tps",
        }
    }
}

/// One of the two romanizations — the value Switch TPS returns to
/// (`keys::LAST_ROMANIZATION_MODE`). Its own type so that TPS cannot be
/// stored there: a stored `"tps"` is a value this type does not name and
/// reads as the default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Romanization {
    Tl,
    Poj,
}

impl Romanization {
    pub fn input_mode(self) -> InputMode {
        match self {
            Self::Tl => InputMode::Tl,
            Self::Poj => InputMode::Poj,
        }
    }

    pub fn other(self) -> Self {
        match self {
            Self::Tl => Self::Poj,
            Self::Poj => Self::Tl,
        }
    }
}

impl SettingChoice for Romanization {
    const ALL: &'static [Self] = &[Self::Tl, Self::Poj];
    const DEFAULT: Self = Self::Tl;
    fn raw(self) -> &'static str {
        self.input_mode().raw()
    }
}

/// What asked for an input-mode change (desktop TPS roadmap D5).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputModeRequest {
    /// The settings picker chose this mode.
    Pick(InputMode),
    /// Switch Romanization: TL ↔ POJ. TPS is not a romanization (U2), so it
    /// never enters TPS; from TPS it leaves for the romanization NOT last
    /// used — the other chord already returns to that one.
    ToggleRomanization,
    /// Switch TPS: into TPS, or back to the romanization last used.
    ToggleTps,
}

/// The mode `request` moves `current` to. `last_romanization` is the one
/// TPS was entered from (`keys::LAST_ROMANIZATION_MODE`).
pub fn next_input_mode(
    current: InputMode,
    last_romanization: Romanization,
    request: InputModeRequest,
) -> InputMode {
    match (request, current.romanization()) {
        (InputModeRequest::Pick(mode), _) => mode,
        (InputModeRequest::ToggleRomanization, romanization) => romanization
            .unwrap_or(last_romanization)
            .other()
            .input_mode(),
        (InputModeRequest::ToggleTps, Some(_)) => InputMode::Tps,
        (InputModeRequest::ToggleTps, None) => last_romanization.input_mode(),
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
    /// the hanji: forcing the swap on is a compatibility projection of that,
    /// so every reader of the swap (auto-space, the nextword gates) behaves
    /// as today's hanji-first mode (invariants §42); full-width punctuation
    /// reads `effective_full_width_punctuation` instead.
    /// `RomanOnly` has no hanji to lead with.
    /// CROSS-PLATFORM INVARIANT — mirrors iOS `SettingsModels.swift`
    /// `CandidateDisplayMode.effectiveHanjiFirst`, Android
    /// `CandidateDisplayMode.kt`.
    pub const fn effective_hanji_first(self, stored: bool) -> bool {
        matches!(self, Self::Combined) || (stored && self.shows_hanji())
    }

    /// Whether a typed punctuation key becomes full-width (`，` for `,`) for a
    /// stored swap flag — the stored flag masked by the display mode, NOT the
    /// candidate projection above, which `Combined` forces on while the swap
    /// shortcut still picks the width. Mirrored on macOS / iOS / Android.
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
    /// Pairing. CROSS-PLATFORM INVARIANT — macOS keeps a Swift twin: `CandidateDisplayMode.next`.
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

/// How the rendered romanization separates syllables (Syllable Separator,
/// `behavioral-invariants.md` §49): the dictionary hyphen (`tâi-uân`), a space
/// (`tâi uân`), or nothing (`tâiuân`). Stored spellings are the same on every
/// platform (`SettingsModels.swift` `SyllableSeparator`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SyllableSeparator {
    Hyphen,
    Space,
    None,
}

impl SyllableSeparator {
    /// The `AppConfig.syllable_separator` wire value, sent as stored: the
    /// engine never rewrites TPS (`AppConfig::rendered_syllable_joiner`).
    pub fn wire(self) -> protos::engine::SyllableSeparator {
        match self {
            Self::Hyphen => protos::engine::SyllableSeparator::Hyphen,
            Self::Space => protos::engine::SyllableSeparator::Space,
            Self::None => protos::engine::SyllableSeparator::None,
        }
    }

    /// The picker row's i18n key.
    pub fn label_key(self) -> crate::strings::StringKey {
        use crate::strings::StringKey;
        match self {
            Self::Hyphen => StringKey::DesktopTelexGuideHyphen,
            Self::Space => StringKey::SettingsSyllableSeparatorSpace,
            Self::None => StringKey::SettingsSyllableSeparatorNone,
        }
    }
}

impl SettingChoice for SyllableSeparator {
    const ALL: &'static [Self] = &[Self::Hyphen, Self::Space, Self::None];
    /// The dictionary form.
    const DEFAULT: Self = Self::Hyphen;
    fn raw(self) -> &'static str {
        match self {
            Self::Hyphen => "hyphen",
            Self::Space => "space",
            Self::None => "none",
        }
    }
}

/// Immutable snapshot of everything the engine needs to render a composition.
///
/// A snapshot rather than a set of getters because a single user intent can
/// issue several engine calls (a commit, then the next-word handshake), and
/// those calls must agree: reading the settings twice could straddle a change
/// and render the two halves of one intent under different rules.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineSettings {
    pub input_mode: InputMode,
    /// Word-boundary spacing input for the engine's `continuous_word_space`
    /// predicate (`docs/engine/continuous-commit-and-display.md` §10.2). The
    /// EFFECTIVE value: the stored toggle AND-ed with `candidate_display_mode
    /// != RomanOnly`, and forced true under `Combined`
    /// (`SettingsDocument::engine_settings`), never the raw document bool —
    /// the raw one stays untouched so leaving either mode restores it.
    pub is_hanji_first: bool,
    /// `CandidateDisplayMode::effective_full_width_punctuation(stored)` —
    /// read by the composing intent executor's `document_punctuation`.
    pub is_full_width_punctuation: bool,
    /// What a candidate cell shows; `AppConfig.candidate_display_mode`.
    /// CROSS-PLATFORM INVARIANT — every platform defaults to side-by-side.
    /// macOS keeps a Swift twin:
    /// `macos/Sources/TaigiInputMethodCore/Settings/EngineSettings.swift`
    /// `candidateDisplayMode`.
    pub candidate_display_mode: CandidateDisplayMode,
    /// §34/S22 — inverted onto `FetchAtPos.literal_roman_candidate_disabled`.
    /// CROSS-PLATFORM INVARIANT — mirrors `isLiteralRomanCandidateEnabled`
    /// (`ios/.../SharedSettings.swift`) and `literalRomanCandidateEnabled`
    /// (`android/.../PrefHelper.kt`), both default OFF (USER 2026-10-02).
    pub is_literal_roman_candidate_enabled: bool,
    /// Syllable Separator (`behavioral-invariants.md` §49) —
    /// `AppConfig.syllable_separator` on the base config, sent as stored.
    /// CROSS-PLATFORM INVARIANT — default Hyphen on every platform; mirrored by
    /// `syllableSeparator` (`ios/.../SharedSettings.swift`) and
    /// `syllableSeparator` (`android/.../PrefHelper.kt`). macOS keeps a Swift
    /// twin: `macos/.../EngineSettings.swift` `syllableSeparator`.
    pub syllable_separator: SyllableSeparator,
    /// Nasal mark in POJ capitals (`behavioral-invariants.md` §53) — the POJ nasal marker follows
    /// the case of the letters before it (`SIÂᴺ`); off, always `ⁿ`. Sent
    /// inverted as `AppConfig.force_lowercase_nasal_marker` on the base config.
    /// CROSS-PLATFORM INVARIANT — default ON on every platform; mirrored by
    /// `isNasalMarkerUppercaseEnabled` (`ios/.../SharedSettings.swift`) and
    /// `isNasalMarkerUppercaseEnabled` (`android/.../PrefHelper.kt`). macOS keeps
    /// a Swift twin: `macos/.../EngineSettings.swift` `isNasalMarkerUppercaseEnabled`.
    pub is_nasal_marker_uppercase_enabled: bool,
    /// Gates the custom-dictionary lookup itself: off means the engine reads
    /// no custom rows (`FetchAtPos.custom_dictionary_disabled`). CROSS-PLATFORM INVARIANT —
    /// `SharedSettings.swift` `isCustomDictEnabledKey` (ON).
    pub is_custom_dict_enabled: bool,
    pub dictionary_sources: DictionarySourceToggles,
}

impl EngineSettings {
    /// What a fresh install types with. Every value matches the iOS and Android
    /// default for the same setting. macOS keeps a Swift twin:
    /// `EngineSettings.swift` `EngineSettings.defaults`.
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
            is_hanji_first: MODE.effective_hanji_first(STORED_SWAP),
            is_full_width_punctuation: MODE.effective_full_width_punctuation(STORED_SWAP),
            candidate_display_mode: MODE,
            is_literal_roman_candidate_enabled: false,
            syllable_separator: SyllableSeparator::Hyphen,
            is_nasal_marker_uppercase_enabled: true,
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
/// `engine/protos/proto/lexicon.proto::DictionarySourceToggles`.
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
    /// skip the gate (`lexicon.proto` `DictionarySourceToggles.kautian_subcollections`).
    pub kautian_subcollections: KautianSubcollections,
}

impl DictionarySourceToggles {
    /// CROSS-PLATFORM INVARIANT — mirrors the source-toggle keys in `ios/.../SharedSettings.swift`.
    /// macOS keeps a Swift twin: `macos/.../DictionarySourceToggles.swift`
    /// `DictionarySourceToggles.defaults`.
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
/// `KautianSubcollectionToggles` (`lexicon.proto`), the `config.yaml`
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
    pub alt_reading: bool,
}

impl KautianSubcollections {
    /// CROSS-PLATFORM INVARIANT — every subcollection defaults ON
    /// (`ios/.../SharedSettings.swift`, the `kautianAccent*` / `kautianNameAppendix` /
    /// `kautianAltReading` keys).
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
        alt_reading: true,
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

    #[test]
    fn every_mode_request_from_every_mode_and_last_romanization() {
        // trace: roadmap D5 — Pick is the pick; Switch Romanization flips
        // TL ↔ POJ and leaves TPS for the romanization NOT last used; Switch
        // TPS enters TPS from either romanization and leaves it for the one
        // last used. `last` is read only under TPS.
        use InputMode::{Poj, Tl, Tps};
        use InputModeRequest::{Pick, ToggleRomanization, ToggleTps};
        let cases = [
            (Tl, Romanization::Tl, ToggleRomanization, Poj),
            (Poj, Romanization::Tl, ToggleRomanization, Tl),
            (Tps, Romanization::Tl, ToggleRomanization, Poj),
            (Tps, Romanization::Poj, ToggleRomanization, Tl),
            (Tl, Romanization::Poj, ToggleTps, Tps),
            (Poj, Romanization::Tl, ToggleTps, Tps),
            (Tps, Romanization::Tl, ToggleTps, Tl),
            (Tps, Romanization::Poj, ToggleTps, Poj),
            (Tl, Romanization::Tl, Pick(Tps), Tps),
            (Tps, Romanization::Tl, Pick(Tps), Tps),
            (Tps, Romanization::Tl, Pick(Poj), Poj),
            (Poj, Romanization::Tl, Pick(Tl), Tl),
        ];
        for (current, last, request, expected) in cases {
            assert_eq!(
                next_input_mode(current, last, request),
                expected,
                "{current:?} last={last:?} {request:?}"
            );
        }
    }

    #[test]
    fn crossing_tps_is_entering_or_leaving_it() {
        assert!(InputMode::Tl.crosses_tps(InputMode::Tps));
        assert!(InputMode::Tps.crosses_tps(InputMode::Poj));
        assert!(!InputMode::Tl.crosses_tps(InputMode::Poj));
        assert!(!InputMode::Tps.crosses_tps(InputMode::Tps));
    }
}
