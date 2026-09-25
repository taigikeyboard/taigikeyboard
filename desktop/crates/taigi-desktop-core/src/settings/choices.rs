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

/// The candidate window's layout. Order = the Appearance pane's pop-up
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
/// Order = the Appearance pane's thumbnail row: light, dark, auto
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

/// How big the candidate window renders — one knob for the whole window: the
/// text, the gaps and the air around them all scale off the candidate font
/// size (USER 2026-09-23, which merged the separate text-size and window-size
/// pickers and made the default a step smaller). The point sizes are the
/// macOS ladder (`CandidateMetrics.swift` `CandidateSizeChoice`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CandidateSizeChoice {
    ExtraSmall,
    Small,
    Standard,
    Large,
    ExtraLarge,
}

impl CandidateSizeChoice {
    /// The step's name in the size pop-up.
    pub fn label_key(self) -> crate::strings::StringKey {
        use crate::strings::StringKey;
        match self {
            Self::ExtraSmall => StringKey::DesktopSizeExtraSmall,
            Self::Small => StringKey::DesktopSizeSmall,
            Self::Standard => StringKey::DesktopSizeMedium,
            Self::Large => StringKey::DesktopSizeLarge,
            Self::ExtraLarge => StringKey::DesktopSizeExtraLarge,
        }
    }

    /// Candidate font size in points.
    /// CROSS-PLATFORM INVARIANT — mirrors `macos/.../Candidates/CandidateMetrics.swift`
    /// `CandidateSizeChoice.candidateFontSize`.
    pub fn font_size(self) -> f32 {
        match self {
            Self::ExtraSmall => 13.0,
            Self::Small => 15.0,
            Self::Standard => 17.0,
            Self::Large => 20.0,
            Self::ExtraLarge => 23.0,
        }
    }
}

impl SettingChoice for CandidateSizeChoice {
    const ALL: &'static [Self] = &[
        Self::ExtraSmall,
        Self::Small,
        Self::Standard,
        Self::Large,
        Self::ExtraLarge,
    ];
    const DEFAULT: Self = Self::Standard;
    /// The point size, so the stored value never collides with the spellings
    /// the two-knob ladder wrote under the same key (`from_raw`).
    fn raw(self) -> &'static str {
        match self {
            Self::ExtraSmall => "13",
            Self::Small => "15",
            Self::Standard => "17",
            Self::Large => "20",
            Self::ExtraLarge => "23",
        }
    }

    /// The current spellings, plus the three the text-size ladder stored
    /// before the merge, each read as the step nearest the size it rendered:
    /// 16 pt → 15, 20 → 20, 23 → 23 (USER 2026-09-23). A never-touched
    /// install has no key and takes the new default.
    fn from_raw(raw: &str) -> Option<Self> {
        match raw {
            "small" => Some(Self::Small),
            "medium" => Some(Self::Large),
            "large" => Some(Self::ExtraLarge),
            _ => Self::ALL.iter().copied().find(|step| step.raw() == raw),
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

/// A family the OS has installed, as this process knows it — the same opaque
/// shape as `CustomFontId`, for the same reason: it keys cached text formats.
/// The renderer hands out one id per (family, system-collection generation),
/// so a family that came back after a reinstall does not draw out of a format
/// made against the collection it left (`ui::render`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InstalledFontId(pub u32);

/// Which typeface the candidate window is set in: one of the bundled roster,
/// one the user added, or a family the OS has installed.
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
    /// Nothing copied or loaded for it: the OS is the authority on whether
    /// it exists, the way the library's directory is for a custom font.
    Installed(InstalledFontId),
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

    /// What `fontType` holds while an installed family is selected. Outside
    /// the roster for the same reason as `CUSTOM_RAW`; not `"system"`, which
    /// is `CandidateFontChoice::System`'s own raw value.
    pub const INSTALLED_RAW: &'static str = "installed";

    /// The bundled face this selection names, or `None` for any other kind.
    pub fn built_in(self) -> Option<CandidateFontChoice> {
        match self {
            Self::BuiltIn(choice) => Some(choice),
            Self::Custom(_) | Self::Installed(_) => None,
        }
    }

    /// Whether the cell geometry has to measure this face's line box. A
    /// bundled face's is known to fit the height an inline row is given; a
    /// face the user added or the OS supplies carries no such guarantee
    /// (`CandidateMetrics::for_content`).
    pub fn requires_line_box_measurement(self) -> bool {
        matches!(self, Self::Custom(_) | Self::Installed(_))
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
/// order; `DictionarySearch` is built but unlisted, and `About` is listed
/// nowhere but the input-source menu (USER 2026-09-20), exactly as on macOS
/// (`SettingsSplitView.swift`, `SettingsPane.sidebar`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SettingsPane {
    General,
    Appearance,
    Shortcuts,
    /// The sources before the user's own words on top of them (USER
    /// 2026-09-21).
    DictionarySources,
    CustomDictionary,
    /// Last in the sidebar (USER 2026-09-08): what the input method draws IN,
    /// after what it draws FROM.
    FontManagement,
    DictionarySearch,
    About,
}

impl SettingsPane {
    /// The panes the sidebar shows, top to bottom.
    pub const SIDEBAR: [SettingsPane; 6] = [
        Self::General,
        Self::Appearance,
        Self::Shortcuts,
        Self::DictionarySources,
        Self::CustomDictionary,
        Self::FontManagement,
    ];

    /// The sidebar row label's i18n key, and the window title
    /// (`SettingsSplitView.swift`, `labelKey`). `None` for the unlisted
    /// search page, which has no row and no title of its own on macOS
    /// either; About has a title without a row.
    pub fn title_key(self) -> Option<crate::strings::StringKey> {
        use crate::strings::StringKey;
        Some(match self {
            Self::General => StringKey::DesktopGeneralTab,
            Self::Appearance => StringKey::DesktopAppearanceTab,
            Self::Shortcuts => StringKey::DesktopShortcutsTab,
            Self::CustomDictionary => StringKey::DictionaryCustomDictionary,
            Self::DictionarySources => StringKey::DesktopDictionarySourcesLink,
            Self::FontManagement => StringKey::DesktopFontManagementTab,
            Self::About => StringKey::HomeAboutKeyboard,
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
            // Info, matching the Mac's `info.circle`; no row draws it.
            Self::About => "\u{E946}",
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
        Self::About,
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
            Self::About => "about",
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
        round_trips::<CandidateSizeChoice>();
        round_trips::<CandidateFontChoice>();
        round_trips::<SettingsPane>();
    }

    #[test]
    fn the_size_ladder_matches_macos_and_climbs() {
        // trace: CandidateMetricsTests.swift pins [13, 15, 17, 20, 23].
        let sizes: Vec<f32> = CandidateSizeChoice::ALL
            .iter()
            .map(|c| c.font_size())
            .collect();
        assert_eq!(sizes, [13.0, 15.0, 17.0, 20.0, 23.0]);
        assert_eq!(CandidateSizeChoice::DEFAULT.font_size(), 17.0);
    }

    #[test]
    fn the_two_knob_spellings_read_as_the_nearest_step() {
        // trace: old text ladder small 16 / medium 20 / large 23 → 15 / 20 / 23
        // (USER 2026-09-23). `extraLarge` (Extra Large, retired 2026-08-21) was
        // never a spelling of this ladder and stays unknown.
        assert_eq!(
            CandidateSizeChoice::from_raw("small"),
            Some(CandidateSizeChoice::Small)
        );
        assert_eq!(
            CandidateSizeChoice::from_raw("medium"),
            Some(CandidateSizeChoice::Large)
        );
        assert_eq!(
            CandidateSizeChoice::from_raw("large"),
            Some(CandidateSizeChoice::ExtraLarge)
        );
        assert_eq!(CandidateSizeChoice::from_raw("extraLarge"), None);
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

    /// About is a pane — it persists, it titles the window — but not a row:
    /// the input-source menu is its one doorway (USER 2026-09-20).
    #[test]
    fn about_is_a_pane_with_a_title_but_not_in_the_sidebar() {
        assert!(SettingsPane::ALL.contains(&SettingsPane::About));
        assert!(!SettingsPane::SIDEBAR.contains(&SettingsPane::About));
        assert_eq!(
            SettingsPane::About.title_key(),
            Some(crate::strings::StringKey::HomeAboutKeyboard)
        );
        assert_eq!(SettingsPane::from_raw("about"), Some(SettingsPane::About));
    }
}
