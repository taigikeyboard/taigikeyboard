//! Colours: light / dark from the appearance setting or the system, the
//! highlight from the Windows accent, and — when a high-contrast theme is
//! on — every colour from the system, nothing of ours.
//!
//! The values are the WinUI common theme resources, so this popup reads as
//! a Windows 11 flyout rather than as the Mac panel it was ported from
//! (`Common_themeresources_any.xaml`). Each is named on the constant it
//! sets. NAMED DIVERGENCE from macOS: no vibrancy / glass backdrop on
//! Windows — an opaque surface with a flyout stroke; and the Sequoia accent
//! darkening (`CandidateAccentColor.sequoiaAdjusted`) is NOT applied — it is
//! a measured fit of what an AppKit window does to the accent, and a
//! selection here is meant to be the Windows system accent exactly.
//!
//! The system's answers are read once ([`SystemTheme::read`]) and cached by
//! the caller until Windows says they changed (`WM_SETTINGCHANGE` /
//! `WM_THEMECHANGED` / `WM_DWMCOLORIZATIONCOLORCHANGED`) — not per keystroke.

// 顏色主題 — 亮/暗、系統強調色、高對比全用系統色;色值取自 WinUI common theme resources(不是 Mac 的灰);不套 Sequoia 修正;Windows 無毛玻璃,改用不透明面 + flyout 邊框(具名差異)。

use taigi_windows_core::settings::AppearanceMode;
use taigi_windows_platform::{HighContrastColors, Rgb};
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Theme {
    pub is_dark: bool,
    pub background: D2D1_COLOR_F,
    pub text: D2D1_COLOR_F,
    pub secondary_text: D2D1_COLOR_F,
    pub tertiary_text: D2D1_COLOR_F,
    pub separator: D2D1_COLOR_F,
    /// The hairline around the whole popup (`SurfaceStrokeColorFlyout`).
    /// Windows 11 draws one on every flyout; the Mac panel had none.
    pub border: D2D1_COLOR_F,
    pub highlight: D2D1_COLOR_F,
    pub highlighted_text: D2D1_COLOR_F,
}

const fn rgb(r: u8, g: u8, b: u8) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
}

const fn rgba(r: u8, g: u8, b: u8, a: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a,
    }
}

/// The accent when the system reports none: Windows' default blue.
const FALLBACK_ACCENT: D2D1_COLOR_F = rgb(0x00, 0x78, 0xD4);

const fn from_rgb((r, g, b): Rgb) -> D2D1_COLOR_F {
    rgb(r, g, b)
}

/// Above this relative luminance a highlight is light enough that white
/// text on it fails contrast; the text goes dark instead (a pale accent
/// such as the yellow or mint presets).
const LIGHT_HIGHLIGHT_LUMINANCE: f32 = 0.55;

/// What the system says about appearance, read at one moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SystemTheme {
    pub prefers_dark: bool,
    /// The colorization colour, opaque: `DwmGetColorizationColor`'s alpha
    /// describes the DWM frame blend, not a colour a highlight can carry,
    /// so it is dropped on purpose.
    pub accent: D2D1_COLOR_F,
    /// The system colours when a high-contrast theme is on. They win over
    /// the mode, the accent and every colour of ours: the user chose them
    /// to be able to read, and a card in our greys defeats that.
    pub high_contrast: Option<HighContrastColors>,
}

impl SystemTheme {
    /// One registry read, one DWM call, one `SystemParametersInfo`.
    pub fn read() -> Self {
        Self {
            prefers_dark: taigi_windows_platform::system_prefers_dark(),
            accent: taigi_windows_platform::system_accent().map_or(FALLBACK_ACCENT, from_rgb),
            high_contrast: taigi_windows_platform::high_contrast_colors(),
        }
    }
}

impl Theme {
    /// Resolves the theme for `mode` against `system`: `Auto` follows
    /// `AppsUseLightTheme`, the highlight is the accent — unless a
    /// high-contrast theme is on, which dictates every colour.
    pub fn resolve(mode: AppearanceMode, system: &SystemTheme) -> Self {
        if let Some(high_contrast) = system.high_contrast {
            return Self::high_contrast(high_contrast);
        }
        let is_dark = match mode {
            AppearanceMode::Light => false,
            AppearanceMode::Dark => true,
            AppearanceMode::Auto => system.prefers_dark,
        };
        let highlight = system.accent;
        let highlighted_text = if relative_luminance(highlight) > LIGHT_HIGHLIGHT_LUMINANCE {
            rgba(0x00, 0x00, 0x00, 0.85)
        } else {
            rgb(0xFF, 0xFF, 0xFF)
        };
        if is_dark {
            Self {
                is_dark,
                // SolidBackgroundFillColorBase
                background: rgb(0x20, 0x20, 0x20),
                // TextFillColorPrimary / Secondary / Tertiary
                text: rgb(0xFF, 0xFF, 0xFF),
                secondary_text: rgba(0xFF, 0xFF, 0xFF, 0.7725),
                tertiary_text: rgba(0xFF, 0xFF, 0xFF, 0.5294),
                // DividerStrokeColorDefault
                separator: rgba(0xFF, 0xFF, 0xFF, 0.0837),
                // SurfaceStrokeColorFlyout
                border: rgba(0x00, 0x00, 0x00, 0.2),
                highlight,
                highlighted_text,
            }
        } else {
            Self {
                is_dark,
                background: rgb(0xF3, 0xF3, 0xF3),
                text: rgba(0x00, 0x00, 0x00, 0.8942),
                secondary_text: rgba(0x00, 0x00, 0x00, 0.6063),
                tertiary_text: rgba(0x00, 0x00, 0x00, 0.4458),
                separator: rgba(0x00, 0x00, 0x00, 0.0578),
                border: rgba(0x00, 0x00, 0x00, 0.0578),
                highlight,
                highlighted_text,
            }
        }
    }
}

impl Theme {
    /// Every colour from the high-contrast scheme: `COLOR_WINDOW` /
    /// `COLOR_WINDOWTEXT` for the card, `COLOR_HIGHLIGHT` /
    /// `COLOR_HIGHLIGHTTEXT` for the selection, `COLOR_GRAYTEXT` for
    /// everything secondary — no alpha, no accent fit: a high-contrast
    /// scheme is opaque by definition. The border is `COLOR_WINDOWTEXT`,
    /// not the flyout's alpha: the whole point of the scheme is that the
    /// edge of a surface is visible.
    fn high_contrast(colors: HighContrastColors) -> Self {
        let background = from_rgb(colors.window);
        let gray = from_rgb(colors.gray_text);
        Self {
            is_dark: relative_luminance(background) < 0.5,
            background,
            text: from_rgb(colors.window_text),
            secondary_text: gray,
            tertiary_text: gray,
            separator: gray,
            border: from_rgb(colors.window_text),
            highlight: from_rgb(colors.highlight),
            highlighted_text: from_rgb(colors.highlight_text),
        }
    }
}

/// sRGB relative luminance (WCAG), the contrast gate's measure.
fn relative_luminance(color: D2D1_COLOR_F) -> f32 {
    let linear = |channel: f32| {
        if channel <= 0.03928 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pale_accent_gets_dark_highlighted_text_and_a_deep_one_white() {
        // trace: luminance(#F0E68C khaki) ≈ 0.81 → dark text;
        // luminance(#0078D4) ≈ 0.17 → white. No accent fit is applied.
        let pale = SystemTheme {
            prefers_dark: false,
            accent: rgb(0xF0, 0xE6, 0x8C),
            high_contrast: None,
        };
        let theme = Theme::resolve(AppearanceMode::Light, &pale);
        assert!(theme.highlighted_text.r < 0.5);
        let deep = SystemTheme {
            prefers_dark: true,
            accent: FALLBACK_ACCENT,
            high_contrast: None,
        };
        let theme = Theme::resolve(AppearanceMode::Auto, &deep);
        assert!(theme.is_dark);
        assert!(theme.highlighted_text.r > 0.9);
    }

    #[test]
    fn the_highlight_is_the_system_accent_exactly() {
        // The Sequoia affine fit is a measurement of what an AppKit window
        // does to the accent; a Windows selection is the system accent.
        let system = SystemTheme {
            prefers_dark: false,
            accent: rgb(0x00, 0x78, 0xD4),
            high_contrast: None,
        };
        assert_eq!(
            Theme::resolve(AppearanceMode::Light, &system).highlight,
            rgb(0x00, 0x78, 0xD4)
        );
    }

    #[test]
    fn every_mode_carries_a_visible_flyout_border() {
        for mode in [AppearanceMode::Light, AppearanceMode::Dark] {
            let system = SystemTheme {
                prefers_dark: false,
                accent: FALLBACK_ACCENT,
                high_contrast: None,
            };
            let theme = Theme::resolve(mode, &system);
            assert!(theme.border.a > 0.0, "{mode:?} draws no hairline");
            // `SurfaceStrokeColorFlyout` is a black ink in BOTH themes —
            // it does not flip with the mode the way the text ramp does.
            assert_eq!(
                (theme.border.r, theme.border.g, theme.border.b),
                (0.0, 0.0, 0.0),
                "{mode:?}"
            );
        }
    }

    #[test]
    fn the_mode_overrides_the_system_preference() {
        let dark_system = SystemTheme {
            prefers_dark: true,
            accent: FALLBACK_ACCENT,
            high_contrast: None,
        };
        assert!(!Theme::resolve(AppearanceMode::Light, &dark_system).is_dark);
        assert!(Theme::resolve(AppearanceMode::Dark, &dark_system).is_dark);
    }

    #[test]
    fn a_high_contrast_scheme_dictates_every_colour_over_mode_and_accent() {
        // trace: "High Contrast Black" — window #000000, text #FFFFFF,
        // highlight #1AEBFF (cyan), highlight text #000000, gray #00FF00.
        let system = SystemTheme {
            prefers_dark: false,
            accent: rgb(0xF0, 0xE6, 0x8C),
            high_contrast: Some(HighContrastColors {
                window: (0x00, 0x00, 0x00),
                window_text: (0xFF, 0xFF, 0xFF),
                highlight: (0x1A, 0xEB, 0xFF),
                highlight_text: (0x00, 0x00, 0x00),
                gray_text: (0x00, 0xFF, 0x00),
            }),
        };
        // Light mode asked for; the scheme still wins.
        let theme = Theme::resolve(AppearanceMode::Light, &system);
        assert!(theme.is_dark);
        assert_eq!(theme.background, rgb(0x00, 0x00, 0x00));
        assert_eq!(theme.text, rgb(0xFF, 0xFF, 0xFF));
        assert_eq!(
            theme.highlight,
            rgb(0x1A, 0xEB, 0xFF),
            "the system highlight, not the app accent"
        );
        assert_eq!(theme.highlighted_text, rgb(0x00, 0x00, 0x00));
        assert_eq!(theme.secondary_text, rgb(0x00, 0xFF, 0x00));
        assert_eq!(theme.separator, rgb(0x00, 0xFF, 0x00));
        assert!(
            (theme.text.a - 1.0).abs() < f32::EPSILON,
            "opaque, no alpha of ours"
        );
    }

    #[test]
    fn a_light_high_contrast_scheme_reads_as_light_and_stays_opaque_throughout() {
        // trace: "High Contrast White" — window #FFFFFF, text #000000,
        // highlight #37006E, highlight text #FFFFFF, gray #600000.
        let system = SystemTheme {
            prefers_dark: true,
            accent: FALLBACK_ACCENT,
            high_contrast: Some(HighContrastColors {
                window: (0xFF, 0xFF, 0xFF),
                window_text: (0x00, 0x00, 0x00),
                highlight: (0x37, 0x00, 0x6E),
                highlight_text: (0xFF, 0xFF, 0xFF),
                gray_text: (0x60, 0x00, 0x00),
            }),
        };
        let theme = Theme::resolve(AppearanceMode::Dark, &system);
        assert!(
            !theme.is_dark,
            "a white window is a light theme, whatever the mode asked"
        );
        assert_eq!(theme.tertiary_text, rgb(0x60, 0x00, 0x00));
        for color in [
            theme.background,
            theme.text,
            theme.secondary_text,
            theme.tertiary_text,
            theme.separator,
            theme.border,
            theme.highlight,
            theme.highlighted_text,
        ] {
            assert!((color.a - 1.0).abs() < f32::EPSILON, "every colour opaque");
        }
    }
}
