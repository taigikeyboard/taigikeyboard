//! The snapshot one engine operation reads. Port of
//! `macos/Sources/TaigiInputMethodCore/Settings/EngineSettings.swift` and
//! `DictionarySourceToggles.swift`.

// 中文: 引擎單次操作讀取的設定快照;所有預設值與 iOS/Android/macOS 對齊。

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
/// the romanization alone, or both in ONE label led by the hanji (漢羅合用,
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
    pub fn shows_hanji(self) -> bool {
        self != Self::RomanOnly
    }

    /// Only side-by-side has a lead script the swap shortcut can flip; the
    /// other two fix it, so the shortcut is inert and the stored swap waits
    /// for the way back.
    pub fn allows_swap_toggle(self) -> bool {
        self == Self::SideBySide
    }

    /// Effective swap for a stored flag. `Combined` leads with — and commits —
    /// the hanji: forcing the pair on is a compatibility projection of that,
    /// so every reader of the pair (auto-space, full-width, the nextword
    /// gates) behaves as today's hanji-first mode (invariants §42).
    /// `RomanOnly` has no hanji to lead with.
    /// CROSS-PLATFORM INVARIANT — mirrors macOS `EngineSettings.swift`
    /// `CandidateDisplayMode.effectiveTranslateSwapped`, iOS
    /// `SettingsModels.swift`, Android `CandidateDisplayMode.kt`.
    // 中文: 推導 swap — 合用恆 true(投影到既有 pair)、羅馬字恆 false、並排照 stored。
    pub fn effective_translate_swapped(self, stored: bool) -> bool {
        self == Self::Combined || (stored && self.shows_hanji())
    }

    /// Effective 括號標註 for a stored flag — off only where there is no hanji
    /// to bracket; `Combined` keeps it (`漢字 (羅馬字)`).
    pub fn effective_output_both_scripts(self, stored: bool) -> bool {
        stored && self.shows_hanji()
    }

    /// The `AppConfig.candidate_display_mode` wire value. The engine reads
    /// it through `AppConfig::is_roman_only_display`, so only `RomanOnly`
    /// has to be exact; `SideBySide` is spelled out rather than left
    /// `Unspecified` so a build that sets the field is telling apart from
    /// one that never did. `Combined` has no engine reader either: a combined
    /// cell is distinct by its `(漢字, 羅馬字)` pair, so nothing collapses.
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
// 中文: 引擎設定快照;一個意圖內多次呼叫共用同一份。
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
    /// What a candidate cell shows; `AppConfig.candidate_display_mode`.
    /// CROSS-PLATFORM INVARIANT — mirrors
    /// `macos/Sources/TaigiInputMethodCore/Settings/EngineSettings.swift`
    /// `candidateDisplayMode`; every platform defaults to side-by-side.
    pub candidate_display_mode: CandidateDisplayMode,
    /// §34/S22 — inverted onto `FetchAtPos.literal_roman_candidate_disabled`.
    /// CROSS-PLATFORM INVARIANT — mirrors `ios/.../SharedSettings.swift:50`
    /// and `android/.../PrefHelper.kt:326`, both default OFF.
    pub is_literal_roman_candidate_enabled: bool,
    /// Read on the write path only; the boost always applies to whatever was
    /// learned. CROSS-PLATFORM INVARIANT — `SharedSettings.swift:48` (ON).
    pub is_frequency_recording_enabled: bool,
    /// `AppConfig.is_association_recording_enabled`, gating the
    /// `RecordAssociation` effects (`engine/nextword/src/decide.rs:130`).
    /// CROSS-PLATFORM INVARIANT — `SharedSettings.swift:49` (ON).
    pub is_association_recording_enabled: bool,
    /// Gates the custom-dictionary lookup itself: off means
    /// `FetchAtPos.custom_entries` goes out empty. CROSS-PLATFORM INVARIANT —
    /// `SharedSettings.swift:51` (ON).
    pub is_custom_dict_enabled: bool,
    pub dictionary_sources: DictionarySourceToggles,
}

impl Default for EngineSettings {
    /// What a fresh install types with. Every value matches the iOS, Android
    /// and macOS default for the same setting (`EngineSettings.swift:79-88`).
    fn default() -> Self {
        Self {
            input_mode: InputMode::Tl,
            is_translate_swapped: false,
            is_output_both_scripts: false,
            candidate_display_mode: CandidateDisplayMode::SideBySide,
            is_literal_roman_candidate_enabled: false,
            is_frequency_recording_enabled: true,
            is_association_recording_enabled: true,
            is_custom_dict_enabled: true,
            dictionary_sources: DictionarySourceToggles::default(),
        }
    }
}

/// Which bundled dictionaries the user has switched on, in the shape the
/// engine's `compute_filters` op reads them. Field order mirrors
/// `engine/protos/proto/lexicon.proto::DictionaryToggles`.
// 中文: 使用者開啟的辭典來源;欄位順序照 lexicon.proto。
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
    /// 異用字.
    pub variant: bool,
    /// 在來字.
    pub khiin: bool,
    /// LKK 漢羅合用建議用字.
    pub lkk: bool,
    /// 詞庫增補檔案.
    pub dev: bool,
    /// Always populated: an absent subcollection message tells the engine to
    /// skip the gate (`lexicon.proto` `DictionaryToggles.kautian_subcoll`).
    pub kautian_subcollections: KautianSubcollections,
}

impl Default for DictionarySourceToggles {
    /// CROSS-PLATFORM INVARIANT — mirrors `ios/.../SharedSettings.swift:53-66`
    /// and `macos/.../DictionarySourceToggles.swift:99-114`.
    fn default() -> Self {
        Self {
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
            kautian_subcollections: KautianSubcollections::default(),
        }
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

impl Default for KautianSubcollections {
    /// CROSS-PLATFORM INVARIANT — every subcollection defaults ON
    /// (`ios/.../SharedSettings.swift:74-84`).
    fn default() -> Self {
        Self {
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
        }
    }
}
