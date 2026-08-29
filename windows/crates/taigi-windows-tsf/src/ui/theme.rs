//! Colours: light / dark from the appearance setting or the system, the
//! highlight from the Windows accent. NAMED DIVERGENCE from macOS: no
//! vibrancy / glass backdrop on Windows — an opaque card. The Sequoia
//! accent darkening (`CandidateAccentColor.sequoiaAdjusted`) is kept so the
//! highlight reads like the native window's. The system's answers are read
//! once ([`SystemTheme::read`]) and cached by the caller until Windows says
//! they changed (`WM_SETTINGCHANGE` / `WM_THEMECHANGED` /
//! `WM_DWMCOLORIZATIONCOLORCHANGED`) — not per keystroke.

// 中文: 顏色主題 — 亮/暗、系統強調色;Windows 無毛玻璃,改用不透明卡片(具名差異);系統值快取到主題變更訊息才重讀。

use taigi_windows_core::settings::AppearanceMode;
use windows::core::w;
use windows::core::BOOL;
use windows::Win32::Foundation::ERROR_SUCCESS;
use windows::Win32::Graphics::Direct2D::Common::D2D1_COLOR_F;
use windows::Win32::Graphics::Dwm::DwmGetColorizationColor;
use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};

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
}

impl SystemTheme {
    /// One registry read and one DWM call.
    pub fn read() -> Self {
        Self {
            prefers_dark: system_prefers_dark(),
            accent: system_accent().unwrap_or(FALLBACK_ACCENT),
        }
    }
}

impl Theme {
    /// Resolves the theme for `mode` against `system`: `Auto` follows
    /// `AppsUseLightTheme`, the highlight is the accent.
    pub fn resolve(mode: AppearanceMode, system: &SystemTheme) -> Self {
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

/// `HKCU\…\Themes\Personalize\AppsUseLightTheme` = 0 → dark. Missing (older
/// Windows) → light.
fn system_prefers_dark() -> bool {
    let mut value: u32 = 1;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: a DWORD read into a local of the size passed.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut _),
            Some(&mut size),
        )
    };
    status == ERROR_SUCCESS && value == 0
}

/// The DWM colorization colour (`0xAARRGGBB`, alpha dropped), the closest
/// thing to the user's accent an in-proc DLL can read without WinRT.
fn system_accent() -> Option<D2D1_COLOR_F> {
    let mut color = 0u32;
    let mut opaque = BOOL(0);
    // SAFETY: out-pointers to locals.
    unsafe { DwmGetColorizationColor(&mut color, &mut opaque) }.ok()?;
    Some(rgb(
        ((color >> 16) & 0xFF) as u8,
        ((color >> 8) & 0xFF) as u8,
        (color & 0xFF) as u8,
    ))
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
        };
        let theme = Theme::resolve(AppearanceMode::Light, &pale);
        assert!(theme.highlighted_text.r < 0.5);
        let deep = SystemTheme {
            prefers_dark: true,
            accent: FALLBACK_ACCENT,
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
        };
        assert!(!Theme::resolve(AppearanceMode::Light, &dark_system).is_dark);
        assert!(Theme::resolve(AppearanceMode::Dark, &dark_system).is_dark);
    }
}
