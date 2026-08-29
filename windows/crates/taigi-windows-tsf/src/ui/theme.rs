//! Colours: light / dark from the appearance setting or the system, the
//! highlight from the Windows accent, and — when a high-contrast theme is
//! on — every colour from the system, nothing of ours. NAMED DIVERGENCE
//! from macOS: no vibrancy / glass backdrop on Windows — an opaque card. The
//! Sequoia accent darkening (`CandidateAccentColor.sequoiaAdjusted`) is kept
//! so the highlight reads like the native window's. The system's answers are
//! read once ([`SystemTheme::read`]) and cached by the caller until Windows
//! says they changed (`WM_SETTINGCHANGE` / `WM_THEMECHANGED` /
//! `WM_DWMCOLORIZATIONCOLORCHANGED`) — not per keystroke.

// 中文: 顏色主題 — 亮/暗、系統強調色、高對比全用系統色;Windows 無毛玻璃,改用不透明卡片(具名差異);系統值快取到主題變更訊息才重讀。

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
        let highlight = sequoia_adjusted(system.accent);
        let highlighted_text = if relative_luminance(highlight) > LIGHT_HIGHLIGHT_LUMINANCE {
            rgba(0x00, 0x00, 0x00, 0.85)
        } else {
            rgb(0xFF, 0xFF, 0xFF)
        };
        if is_dark {
            Self {
                is_dark,
                background: rgb(0x2B, 0x2B, 0x2B),
                text: rgba(0xFF, 0xFF, 0xFF, 0.85),
                secondary_text: rgba(0xFF, 0xFF, 0xFF, 0.55),
                tertiary_text: rgba(0xFF, 0xFF, 0xFF, 0.25),
                separator: rgba(0xFF, 0xFF, 0xFF, 0.12),
                highlight,
                highlighted_text,
            }
        } else {
            Self {
                is_dark,
                background: rgb(0xF6, 0xF6, 0xF6),
                text: rgba(0x00, 0x00, 0x00, 0.85),
                secondary_text: rgba(0x00, 0x00, 0x00, 0.50),
                tertiary_text: rgba(0x00, 0x00, 0x00, 0.26),
                separator: rgba(0x00, 0x00, 0x00, 0.10),
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
    /// scheme is opaque by definition.
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
            highlight: from_rgb(colors.highlight),
            highlighted_text: from_rgb(colors.highlight_text),
        }
    }
}

/// Upstream's measured affine fit of what the native window does with the
/// accent (`MacishBasePanel.swift:129-137`).
fn sequoia_adjusted(color: D2D1_COLOR_F) -> D2D1_COLOR_F {
    let adjust = |channel: f32| (channel * 0.9417 - 0.0594).max(0.0);
    D2D1_COLOR_F {
        r: adjust(color.r),
        g: adjust(color.g),
        b: adjust(color.b),
        a: color.a,
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
        // trace: luminance(#F0E68C khaki) ≈ 0.77 after the Sequoia fit → dark
        // text; luminance(#0078D4) ≈ 0.17 → white.
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
            "the accent fit is not applied"
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
            theme.highlight,
            theme.highlighted_text,
        ] {
            assert!((color.a - 1.0).abs() < f32::EPSILON, "every colour opaque");
        }
    }
}
