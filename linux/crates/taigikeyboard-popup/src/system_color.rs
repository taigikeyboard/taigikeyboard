//! The Default theme's colors, from the desktop's color scheme and accent.
//!
//! Surfaces are libadwaita 1.5's popover (`_defaults.scss`): a candidate
//! window is a popover, not a window. Light foreground is black at 80%
//! opacity flattened onto that background, because this buffer stores
//! opaque pixels. The highlight is the accent, or Adwaita `@blue_3` when
//! the desktop names none. Highlighted text is white, or black at 85%
//! opacity when the accent is pale — the same luminance gate the Windows
//! candidate window uses.

use crate::paint;
use crate::Chrome;

/// What the desktop portal last said. `accent` is `0xRRGGBB`, absent when
/// the desktop did not name one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SystemAppearance {
    pub prefers_dark: bool,
    pub accent: Option<u32>,
}

/// Adwaita `@blue_3`, `accent_bg_color` in both variants.
const ACCENT_BG: u32 = 0x35_84E4;
/// `popover_bg_color` / `popover_fg_color`, light.
const LIGHT_BG: u32 = 0xFF_FFFF;
/// `popover_bg_color`, dark. Dark foreground is white.
const DARK_BG: u32 = 0x38_3838;
/// `transparentize(black, .2)` is black at 80% opacity.
const LIGHT_TEXT_OPACITY: u32 = 204;
/// Black at 85% opacity, for a highlight light enough that white fails.
const PALE_TEXT_OPACITY: u32 = 217;
/// Above this relative luminance, white text on the highlight fails.
const LIGHT_HIGHLIGHT_LUMINANCE: f32 = 0.55;

/// The four colors a Default page paints for `appearance`.
pub fn palette(appearance: SystemAppearance) -> (u32, u32, u32, u32) {
    let background = if appearance.prefers_dark {
        DARK_BG
    } else {
        LIGHT_BG
    };
    let text = if appearance.prefers_dark {
        0xFF_FFFF
    } else {
        paint::mix(0, background, LIGHT_TEXT_OPACITY)
    };
    let selected_background = appearance.accent.unwrap_or(ACCENT_BG);
    let selected_text = if relative_luminance(selected_background) > LIGHT_HIGHLIGHT_LUMINANCE {
        paint::mix(0, selected_background, PALE_TEXT_OPACITY)
    } else {
        0xFF_FFFF
    };
    (text, selected_text, background, selected_background)
}

/// Replaces `chrome`'s four colors when it follows the system and the
/// desktop has answered. Otherwise the colors already on `chrome` stand,
/// which is the shipped palette when the portal could not be read.
pub fn paint_chrome(chrome: Chrome, appearance: Option<SystemAppearance>) -> Chrome {
    let Some(appearance) = appearance.filter(|_| chrome.follow_system) else {
        return chrome;
    };
    let (text, selected_text, background, selected_background) = palette(appearance);
    Chrome {
        text_color: text,
        selected_text_color: selected_text,
        background_color: background,
        selected_background_color: selected_background,
        ..chrome
    }
}

/// sRGB relative luminance (WCAG).
fn relative_luminance(color: u32) -> f32 {
    let linear = |channel: f32| {
        if channel <= 0.03928 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    let channel = |shift: u32| linear(((color >> shift) & 0xff) as f32 / 255.0);
    0.2126 * channel(16) + 0.7152 * channel(8) + 0.0722 * channel(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_uses_the_popover_and_adwaita_blue() {
        // trace: popover light is #ffffff; black at 80% over it is
        // 255 * 0.2 = 51 = #333333; accent_bg is @blue_3 #3584e4, and its
        // luminance is well under 0.55, so the text on it is white.
        let (text, selected_text, background, selected_background) = palette(SystemAppearance {
            prefers_dark: false,
            accent: None,
        });
        assert_eq!(background, 0xFF_FFFF);
        assert_eq!(text, 0x33_3333);
        assert_eq!(selected_background, 0x35_84E4);
        assert_eq!(selected_text, 0xFF_FFFF);
    }

    #[test]
    fn dark_uses_the_dark_popover_and_white_text() {
        // trace: popover dark is #383838; popover_fg dark is white.
        let (text, _, background, _) = palette(SystemAppearance {
            prefers_dark: true,
            accent: None,
        });
        assert_eq!(background, 0x38_3838);
        assert_eq!(text, 0xFF_FFFF);
    }

    #[test]
    fn a_pale_accent_gets_dark_text_and_a_deep_one_white() {
        // trace: luminance(#F0E68C) ≈ 0.81 → black at 85% over the accent;
        // luminance(#3584e4) ≈ 0.21 → white. The accent itself is not retinted.
        let (_, pale_text, _, pale_bg) = palette(SystemAppearance {
            prefers_dark: false,
            accent: Some(0xF0_E68C),
        });
        assert_eq!(pale_bg, 0xF0_E68C);
        assert_eq!(pale_text, paint::mix(0, 0xF0_E68C, PALE_TEXT_OPACITY));
        let (_, deep_text, _, _) = palette(SystemAppearance {
            prefers_dark: true,
            accent: Some(ACCENT_BG),
        });
        assert_eq!(deep_text, 0xFF_FFFF);
    }

    #[test]
    fn a_named_theme_and_a_missing_portal_keep_the_colors_they_were_sent() {
        let shipped = Chrome::default();
        assert_eq!(paint_chrome(shipped.clone(), None), shipped);
        let mut sakura = shipped.clone();
        sakura.background_color = 0xE6_E5E3;
        sakura.selected_background_color = 0xA5_3945;
        let appearance = SystemAppearance {
            prefers_dark: true,
            accent: Some(0x33_D17A),
        };
        assert_eq!(paint_chrome(sakura.clone(), Some(appearance)), sakura);
        let mut follow = shipped;
        follow.follow_system = true;
        assert_eq!(
            paint_chrome(follow.clone(), None).background_color,
            0xF6_F5F4
        );
        assert_eq!(
            paint_chrome(follow, Some(appearance)).background_color,
            DARK_BG
        );
    }
}
