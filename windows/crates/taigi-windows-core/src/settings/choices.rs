//! String-backed settings choices. Each raw value is what `settings.json`
//! stores; each roster order is the order the settings window lists them in.

/// A setting whose stored form is one of a fixed set of strings.
///
/// A stored value the type does not name — a hand-edited file, or a choice a
/// later version removed — reads as the default rather than leaving the
/// reader with nothing (`SettingsStore.swift:366-370`).
pub trait SettingChoice: Copy + PartialEq + std::fmt::Debug + 'static {
    /// Every value, in picker order.
    const ALL: &'static [Self];
    /// What a fresh install uses.
    const DEFAULT: Self;
    /// The persisted spelling.
    fn raw(self) -> &'static str;

    fn from_raw(raw: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|choice| choice.raw() == raw)
    }
}

/// The candidate window's layout. Order = the 外觀 pane's pop-up
/// (`SettingsStore.swift:236-239`, `CandidateLayout.swift:11-14`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CandidateLayout {
    Expandable,
    Horizontal,
    Vertical,
}

impl CandidateLayout {
    /// The picker row's i18n key (`AppearanceSettingsView.swift:259-263`).
    pub fn label_key(self) -> crate::strings::StringKey {
        use crate::strings::StringKey;
        match self {
            Self::Expandable => StringKey::DesktopCandidateLayoutExpandable,
            Self::Horizontal => StringKey::DesktopCandidateLayoutHorizontal,
            Self::Vertical => StringKey::DesktopCandidateLayoutVertical,
        }
    }
}

impl SettingChoice for CandidateLayout {
    const ALL: &'static [Self] = &[Self::Expandable, Self::Horizontal, Self::Vertical];
    /// MacishType's own default, carried by the macOS port (USER 2026-08-28).
    const DEFAULT: Self = Self::Expandable;
    fn raw(self) -> &'static str {
        match self {
            Self::Expandable => "expandable",
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }
}

/// Light / dark / follow the system. Stored under the name it had when only
/// the candidate window read it (`candidateAppearanceMode`), like macOS.
/// Order = the 外觀 pane's thumbnail row: light, dark, auto
/// (`AppearanceSettingsView.swift:106`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AppearanceMode {
    Light,
    Dark,
    Auto,
}

impl AppearanceMode {
    /// The thumbnail caption's i18n key (`AppearanceMode.swift:35-40`): `Auto` shares the display-language picker's "automatic" word.
    pub fn label_key(self) -> crate::strings::StringKey {
        use crate::strings::StringKey;
        match self {
            Self::Light => StringKey::DesktopCandidateAppearanceLight,
            Self::Dark => StringKey::DesktopCandidateAppearanceDark,
            Self::Auto => StringKey::SettingsDisplayLanguageAutomatic,
        }
    }
}

impl SettingChoice for AppearanceMode {
    const ALL: &'static [Self] = &[Self::Light, Self::Dark, Self::Auto];
    const DEFAULT: Self = Self::Auto;
    fn raw(self) -> &'static str {
        match self {
            Self::Light => "light",
            Self::Dark => "dark",
            Self::Auto => "auto",
        }
    }
}

/// How big the candidate text renders. The point sizes are the macOS ladder
/// (`CandidateMetrics.swift:21-27`); medium is one step above the size the
/// window originally rendered at (USER 2026-08-21).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CandidateTextSizeChoice {
    Small,
    Medium,
    Large,
}

impl CandidateTextSizeChoice {
    /// The three named steps share the window-size row's words (`AppearanceSettingsView.swift:268-277`).
    pub fn label_key(self) -> crate::strings::StringKey {
        use crate::strings::StringKey;
        match self {
            Self::Small => StringKey::DesktopSizeSmall,
            Self::Medium => StringKey::DesktopSizeMedium,
            Self::Large => StringKey::DesktopSizeLarge,
        }
    }

    /// Candidate font size in points.
    /// CROSS-PLATFORM INVARIANT — mirrors `macos/.../Candidates/CandidateMetrics.swift:21-27`.
    pub fn font_size(self) -> f32 {
        match self {
            Self::Small => 16.0,
            Self::Medium => 20.0,
            Self::Large => 23.0,
        }
    }
}

impl SettingChoice for CandidateTextSizeChoice {
    const ALL: &'static [Self] = &[Self::Small, Self::Medium, Self::Large];
    const DEFAULT: Self = Self::Medium;
    fn raw(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }
}

/// How much air the candidate window puts around its text, as a multiplier
/// over the cell paddings (`CandidateMetrics.swift:42-48`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CandidateWindowSizeChoice {
    Small,
    Medium,
    Large,
}

impl CandidateWindowSizeChoice {
    /// The picker row's i18n key (`AppearanceSettingsView.swift:268-272`).
    pub fn label_key(self) -> crate::strings::StringKey {
        use crate::strings::StringKey;
        match self {
            Self::Small => StringKey::DesktopSizeSmall,
            Self::Medium => StringKey::DesktopSizeMedium,
            Self::Large => StringKey::DesktopSizeLarge,
        }
    }

    /// NAMED DIVERGENCE from `macos/.../Candidates/CandidateMetrics.swift:42-48`
    /// (USER 2026-09-01, first real-Windows dogfood): every step of the Windows
    /// ladder is one notch tighter than the Mac's `0.7 / 0.85 / 1.0`. The point
    /// values are read as DIPs here and as points on the Mac, so the same
    /// number lands differently against the platform's own chrome; the window
    /// carried too much air on Windows. The text ladder is untouched — this is
    /// the chrome knob, and the two stay independent.
    pub fn scale(self) -> f32 {
        match self {
            Self::Small => 0.6,
            Self::Medium => 0.72,
            Self::Large => 0.85,
        }
    }
}

impl SettingChoice for CandidateWindowSizeChoice {
    const ALL: &'static [Self] = &[Self::Small, Self::Medium, Self::Large];
    const DEFAULT: Self = Self::Medium;
    fn raw(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
        }
    }
}

/// Which typeface the candidate window draws in. The stored spelling is iOS's
/// (`fontType`); the DEFAULT is the desktop's own — a fresh install draws in
/// the system font (USER 2026-08-23) where iOS starts on Open Huninn.
/// Roster + file names mirror `macos/.../Candidates/CandidateFontChoice.swift:22-49`;
/// the files themselves ship from the repo-root `fonts/font/`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CandidateFontChoice {
    System,
    OpenHuninn,
    Iansui,
    GenYoMin,
    GenYoGothic,
}

impl CandidateFontChoice {
    /// The picker row's i18n key (`CandidateFontChoice.swift:54-62`).
    pub fn label_key(self) -> crate::strings::StringKey {
        use crate::strings::StringKey;
        match self {
            Self::System => StringKey::CommonFontSystemDefault,
            Self::OpenHuninn => StringKey::CommonFontOpenHuninn,
            Self::Iansui => StringKey::CommonFontIansui,
            Self::GenYoMin => StringKey::CommonFontGenYoMin,
            Self::GenYoGothic => StringKey::CommonFontGenYoGothic,
        }
    }

    /// The bundled file under the install dir's `fonts\`, or `None` for the
    /// system face.
    ///
    /// File names, not PostScript names: the shared `fonts/font/` directory uses
    /// Android's resource-naming rules so every platform reads one copy of the
    /// bytes, and the face names inside the files are unchanged.
    pub fn file_name(self) -> Option<&'static str> {
        match self {
            Self::System => None,
            Self::OpenHuninn => Some("jf_openhuninn_2_1.ttf"),
            Self::Iansui => Some("iansui_regular.ttf"),
            Self::GenYoMin => Some("genyomin2tw_r.otf"),
            Self::GenYoGothic => Some("genyogothic2tw_r.otf"),
        }
    }
}

/// A typeface the user added, as this process knows it.
///
/// An opaque id rather than the stored file name, because it travels in
/// `FontSpec` and ends up in the cached-text-format key: those must be
/// `Copy`, and a hash of the name would neither be collision-free nor notice
/// a file replaced under the same name. The renderer hands out one id per
/// LOADED resource (`ui::render`), so a replacement gets a new id and cannot
/// draw out of the previous one's cached format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CustomFontId(pub u32);

/// Which typeface the candidate window is set in: one of the bundled roster,
/// or one the user added.
///
/// `CandidateFontChoice` is the roster the four platforms share and stays
/// exactly that; this is the desktop's extension of it, mirroring
/// `macos/.../Candidates/CandidateFontSelection.swift`. Two different custom
/// typefaces have to be two different values, or the second would draw in the
/// first's cached text format and measured widths.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CandidateFontSelection {
    BuiltIn(CandidateFontChoice),
    Custom(CustomFontId),
}

impl Default for CandidateFontSelection {
    /// What a fresh install draws in — the desktop's own default, the system
    /// face (`CandidateFontChoice::DEFAULT`).
    fn default() -> Self {
        Self::BuiltIn(CandidateFontChoice::DEFAULT)
    }
}

impl CandidateFontSelection {
    /// What `fontType` holds while a custom typeface is selected. Deliberately
    /// not a `CandidateFontChoice` raw value: an older build, or a platform
    /// with no font library, reads it back as an unknown value and falls to the
    /// system font (`SettingsDocument::choice`), which is the honest answer to
    /// "a typeface this build cannot see".
    pub const CUSTOM_RAW: &'static str = "custom";

    /// The bundled face this selection names, or `None` for a custom one.
    pub fn built_in(self) -> Option<CandidateFontChoice> {
        match self {
            Self::BuiltIn(choice) => Some(choice),
            Self::Custom(_) => None,
        }
    }

    pub fn is_custom(self) -> bool {
        matches!(self, Self::Custom(_))
    }
}

impl From<CandidateFontChoice> for CandidateFontSelection {
    fn from(choice: CandidateFontChoice) -> Self {
        Self::BuiltIn(choice)
    }
}

impl SettingChoice for CandidateFontChoice {
    const ALL: &'static [Self] = &[
        Self::System,
        Self::OpenHuninn,
        Self::Iansui,
        Self::GenYoMin,
        Self::GenYoGothic,
    ];
    const DEFAULT: Self = Self::System;
    fn raw(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::OpenHuninn => "openHuninn",
            Self::Iansui => "iansui",
            Self::GenYoMin => "genYoMin",
            Self::GenYoGothic => "genYoGothic",
        }
    }
}

/// The settings window's panes. `SIDEBAR` is what the sidebar lists, in
/// order; `DictionarySearch` is built but unlisted, exactly as on macOS
/// (`SettingsSplitView.swift:13-45,121-125`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsPane {
    General,
    Appearance,
    Shortcuts,
    CustomDictionary,
    DictionarySources,
    /// Last in the sidebar, under 辭典管理 (USER 2026-09-08): the two 管理
    /// panes end the list — what the input method draws FROM, then what it
    /// draws IN.
    FontManagement,
    DictionarySearch,
}

impl SettingsPane {
    /// The panes the sidebar shows, top to bottom.
    pub const SIDEBAR: [SettingsPane; 6] = [
        Self::General,
        Self::Appearance,
        Self::Shortcuts,
        Self::CustomDictionary,
        Self::DictionarySources,
        Self::FontManagement,
    ];

    /// The sidebar row label's i18n key (`SettingsSplitView.swift:26-34`).
    /// `None` for the unlisted search page, which has no row and no title of
    /// its own on macOS either.
    pub fn title_key(self) -> Option<crate::strings::StringKey> {
        use crate::strings::StringKey;
        Some(match self {
            Self::General => StringKey::DesktopGeneralTab,
            Self::Appearance => StringKey::DesktopAppearanceTab,
            Self::Shortcuts => StringKey::DesktopShortcutsTab,
            Self::CustomDictionary => StringKey::DictionaryCustomDictionary,
            Self::DictionarySources => StringKey::DesktopDictionarySourcesLink,
            Self::FontManagement => StringKey::DesktopFontManagementTab,
            Self::DictionarySearch => return None,
        })
    }

    /// The sidebar row's Segoe Fluent Icons / MDL2 Assets glyph — the same
    /// code points in both faces — matching the Mac's symbol per pane:
    /// gearshape → Settings, paintpalette → Color, keyboard →
    /// KeyboardClassic, character.book.closed → Dictionary,
    /// books.vertical → Library (`SettingsSplitView.swift:36-44`).
    pub fn icon_glyph(self) -> &'static str {
        match self {
            Self::General => "\u{E713}",
            Self::Appearance => "\u{E790}",
            Self::Shortcuts => "\u{E765}",
            Self::CustomDictionary => "\u{E82D}",
            Self::DictionarySources | Self::DictionarySearch => "\u{E8F1}",
            // Font, the glyph Windows itself puts on a typeface list —
            // matching the Mac's `textformat`.
            Self::FontManagement => "\u{E8D2}",
        }
    }
}

impl SettingChoice for SettingsPane {
    const ALL: &'static [Self] = &[
        Self::General,
        Self::Appearance,
        Self::Shortcuts,
        Self::CustomDictionary,
        Self::DictionarySources,
        Self::FontManagement,
        Self::DictionarySearch,
    ];
    const DEFAULT: Self = Self::General;
    fn raw(self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Appearance => "appearance",
            Self::Shortcuts => "shortcuts",
            Self::CustomDictionary => "customDictionary",
            Self::DictionarySources => "dictionarySources",
            Self::FontManagement => "fontManagement",
            Self::DictionarySearch => "dictionarySearch",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trips<T: SettingChoice>() {
        for choice in T::ALL {
            assert_eq!(T::from_raw(choice.raw()), Some(*choice));
        }
        assert_eq!(T::from_raw("not-a-choice"), None);
        assert!(T::ALL.contains(&T::DEFAULT));
    }

    #[test]
    fn candidate_display_mode_roster_carries_all_three_values() {
        use crate::settings::CandidateDisplayMode;
        // The picker rows come straight off `ALL`, so the roster IS the UI.
        assert_eq!(
            CandidateDisplayMode::ALL,
            &[
                CandidateDisplayMode::SideBySide,
                CandidateDisplayMode::Combined,
                CandidateDisplayMode::RomanOnly,
            ]
        );
    }

    #[test]
    fn every_choice_round_trips_and_has_a_listed_default() {
        round_trips::<CandidateLayout>();
        round_trips::<crate::settings::CandidateDisplayMode>();
        round_trips::<AppearanceMode>();
        round_trips::<CandidateTextSizeChoice>();
        round_trips::<CandidateWindowSizeChoice>();
        round_trips::<CandidateFontChoice>();
        round_trips::<SettingsPane>();
    }

    #[test]
    fn the_text_ladder_matches_macos_and_the_chrome_ladder_is_a_notch_tighter() {
        // trace: CandidateMetricsTests.swift:55-58 pins [16, 20, 23] and
        // [0.7, 0.85, 1.0]. The text ladder is shared; the chrome ladder is the
        // named divergence (USER 2026-09-01) — each step one notch tighter, and
        // the whole ladder still under the Mac's, never over it.
        let sizes: Vec<f32> = CandidateTextSizeChoice::ALL
            .iter()
            .map(|c| c.font_size())
            .collect();
        assert_eq!(sizes, [16.0, 20.0, 23.0]);
        let scales: Vec<f32> = CandidateWindowSizeChoice::ALL
            .iter()
            .map(|c| c.scale())
            .collect();
        assert_eq!(scales, [0.6, 0.72, 0.85]);
        assert!(
            scales.windows(2).all(|pair| pair[0] < pair[1]),
            "the ladder still climbs"
        );
    }

    #[test]
    fn dictionary_search_is_a_pane_but_not_in_the_sidebar() {
        assert!(SettingsPane::ALL.contains(&SettingsPane::DictionarySearch));
        assert!(!SettingsPane::SIDEBAR.contains(&SettingsPane::DictionarySearch));
        assert_eq!(SettingsPane::SIDEBAR[0], SettingsPane::General);
        assert!(SettingsPane::DictionarySearch.title_key().is_none());
        assert!(SettingsPane::SIDEBAR
            .iter()
            .all(|pane| pane.title_key().is_some()));
    }
}
