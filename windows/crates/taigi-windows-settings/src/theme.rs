//! The window's look: WinUI 3's Fluent tokens (`themeresources.xaml`)
//! mapped onto egui's `Visuals` and `Style`, so the window reads as a
//! Windows 11 Settings page — the system ground with cards on it, 14px
//! body text, 32px controls, 4px corners, 8px flyouts — in whichever of
//! light / dark the resolved theme is, with the user's accent for every
//! selection (roadmap W15). Mica is out of reach (egui paints an opaque
//! client area), so the ground is WinUI's own no-Mica fallback,
//! `SolidBackgroundFillColorBase`. Translucent tokens stay translucent:
//! egui composites them over whatever they land on, as WinUI does.

// 中文: 視窗外觀 — WinUI 3 Fluent 色票對映到 egui;亮 / 暗兩套,強調色跟系統。

use std::sync::LazyLock;

/// The Fluent tokens the window draws with, one set per theme. Names are
/// WinUI's, so a value can be checked against `themeresources.xaml`.
pub struct Palette {
    /// `SolidBackgroundFillColorBase`: the window ground.
    pub ground: egui::Color32,
    /// `CardBackgroundFillColorDefault` / `CardStrokeColorDefault`.
    pub card_fill: egui::Color32,
    pub card_stroke: egui::Color32,
    /// `ControlFillColorDefault` / `Secondary` (hover) / `Tertiary` (pressed).
    pub control_fill: egui::Color32,
    pub control_fill_hover: egui::Color32,
    pub control_fill_pressed: egui::Color32,
    /// `ControlStrokeColorDefault`; `ControlStrongStrokeColorDefault` is
    /// the switch's off-state ring.
    pub control_stroke: egui::Color32,
    pub control_strong_stroke: egui::Color32,
    /// `SubtleFillColorSecondary` / `Tertiary`: the navigation rows' hover
    /// and pressed grounds — theirs alone, not egui's `faint_bg_color`.
    pub subtle_fill: egui::Color32,
    pub subtle_fill_pressed: egui::Color32,
    /// `TextFillColorPrimary` / `Secondary`.
    pub text_primary: egui::Color32,
    pub text_secondary: egui::Color32,
    /// `DividerStrokeColorDefault`.
    pub divider: egui::Color32,
    /// The flyout surface (`AcrylicBackgroundFillColorDefault`'s solid
    /// fallback) and `SurfaceStrokeColorFlyout`: menus, pop-ups, dialogs.
    pub flyout_fill: egui::Color32,
    pub flyout_stroke: egui::Color32,
    /// `SystemFillColorCritical` / `SystemFillColorCaution`.
    pub critical: egui::Color32,
    pub caution: egui::Color32,
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
}

static LIGHT: LazyLock<Palette> = LazyLock::new(|| Palette {
    ground: rgba(0xF3, 0xF3, 0xF3, 0xFF),
    card_fill: rgba(0xFF, 0xFF, 0xFF, 0xB3),
    card_stroke: rgba(0x00, 0x00, 0x00, 0x0F),
    control_fill: rgba(0xFF, 0xFF, 0xFF, 0xB3),
    control_fill_hover: rgba(0xF9, 0xF9, 0xF9, 0x80),
    control_fill_pressed: rgba(0xF9, 0xF9, 0xF9, 0x4D),
    control_stroke: rgba(0x00, 0x00, 0x00, 0x0F),
    control_strong_stroke: rgba(0x00, 0x00, 0x00, 0x72),
    subtle_fill: rgba(0x00, 0x00, 0x00, 0x0A),
    subtle_fill_pressed: rgba(0x00, 0x00, 0x00, 0x06),
    text_primary: rgba(0x00, 0x00, 0x00, 0xE4),
    text_secondary: rgba(0x00, 0x00, 0x00, 0x9E),
    divider: rgba(0x00, 0x00, 0x00, 0x0F),
    flyout_fill: rgba(0xFC, 0xFC, 0xFC, 0xFF),
    flyout_stroke: rgba(0x00, 0x00, 0x00, 0x0F),
    critical: rgba(0xC4, 0x2B, 0x1C, 0xFF),
    caution: rgba(0x9D, 0x5D, 0x00, 0xFF),
});

static DARK: LazyLock<Palette> = LazyLock::new(|| Palette {
    ground: rgba(0x20, 0x20, 0x20, 0xFF),
    card_fill: rgba(0xFF, 0xFF, 0xFF, 0x0D),
    card_stroke: rgba(0x00, 0x00, 0x00, 0x19),
    control_fill: rgba(0xFF, 0xFF, 0xFF, 0x0F),
    control_fill_hover: rgba(0xFF, 0xFF, 0xFF, 0x15),
    control_fill_pressed: rgba(0xFF, 0xFF, 0xFF, 0x08),
    control_stroke: rgba(0xFF, 0xFF, 0xFF, 0x12),
    control_strong_stroke: rgba(0xFF, 0xFF, 0xFF, 0x9A),
    subtle_fill: rgba(0xFF, 0xFF, 0xFF, 0x0F),
    subtle_fill_pressed: rgba(0xFF, 0xFF, 0xFF, 0x0A),
    text_primary: rgba(0xFF, 0xFF, 0xFF, 0xFF),
    text_secondary: rgba(0xFF, 0xFF, 0xFF, 0xC5),
    divider: rgba(0xFF, 0xFF, 0xFF, 0x15),
    flyout_fill: rgba(0x2C, 0x2C, 0x2C, 0xFF),
    flyout_stroke: rgba(0x00, 0x00, 0x00, 0x33),
    critical: rgba(0xFF, 0x99, 0xA4, 0xFF),
    caution: rgba(0xFC, 0xE1, 0x00, 0xFF),
});

/// `ControlCornerRadius`: buttons, fields, cards, navigation rows.
pub const CONTROL_CORNER_RADIUS: f32 = 4.0;
/// `OverlayCornerRadius`: flyouts, menus, dialogs.
const OVERLAY_CORNER_RADIUS: f32 = 8.0;
/// The focus visual's outer ring (`FocusVisualPrimaryThickness`).
const FOCUS_RING_WIDTH: f32 = 2.0;
/// `AccentFillColorSecondary`: the accent under the pointer, 90% opacity.
const ACCENT_HOVER_OPACITY: f32 = 0.9;

static TITLE_TEXT_STYLE: LazyLock<egui::TextStyle> =
    LazyLock::new(|| egui::TextStyle::Name("title".into()));

/// The palette the given visuals were built from.
pub fn palette(visuals: &egui::Visuals) -> &'static Palette {
    if visuals.dark_mode {
        &DARK
    } else {
        &LIGHT
    }
}

/// The page title's text style (`TitleTextBlockStyle`, 28px): Segoe UI
/// Variable Display Semibold is unreachable — egui takes one instance of a
/// variable face — so this is the regular face at the title's size.
pub fn title_text_style() -> egui::TextStyle {
    TITLE_TEXT_STYLE.clone()
}

/// An accent-filled control under the pointer.
pub fn accent_hover(accent: egui::Color32) -> egui::Color32 {
    accent.gamma_multiply(ACCENT_HOVER_OPACITY)
}

/// Typography and spacing, applied once to both themes' styles: WinUI's
/// body 14 / caption 12 / subtitle 20 (`BodyTextBlockStyle` and kin), a
/// 32px control height, `Button`'s 11×5 padding. Colour is `visuals`'.
pub fn apply_style(style: &mut egui::Style) {
    use egui::{FontFamily, FontId, TextStyle};
    style.text_styles = [
        (
            TextStyle::Small,
            FontId::new(12.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(14.0, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(14.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Heading,
            FontId::new(20.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(14.0, FontFamily::Monospace),
        ),
        (
            title_text_style(),
            FontId::new(28.0, FontFamily::Proportional),
        ),
    ]
    .into();
    style.spacing.interact_size = egui::vec2(40.0, 32.0);
    style.spacing.button_padding = egui::vec2(11.0, 5.0);
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
}

/// The visuals for `theme` with the user's `accent`, built from egui's
/// own so what the tokens do not name (cursor, shadows, scroll bars) stays
/// egui's. Every surface egui draws maps to one Fluent token; the pair is
/// named beside each line.
pub fn visuals(theme: egui::Theme, accent: egui::Color32) -> egui::Visuals {
    let (mut visuals, palette) = match theme {
        egui::Theme::Light => (egui::Visuals::light(), &*LIGHT),
        egui::Theme::Dark => (egui::Visuals::dark(), &*DARK),
    };
    let corner = egui::CornerRadius::from(CONTROL_CORNER_RADIUS);
    let widget = |bg_fill, bg_stroke, fg| egui::style::WidgetVisuals {
        bg_fill,
        weak_bg_fill: bg_fill,
        bg_stroke: egui::Stroke::new(1.0_f32, bg_stroke),
        corner_radius: corner,
        fg_stroke: egui::Stroke::new(1.0_f32, fg),
        expansion: 0.0,
    };
    // Labels, separators, table rules; `weak_bg_fill` is also what
    // disabled controls and weak text fade toward — the ground.
    visuals.widgets.noninteractive = widget(palette.ground, palette.divider, palette.text_primary);
    visuals.widgets.inactive = widget(
        palette.control_fill,
        palette.control_stroke,
        palette.text_primary,
    );
    visuals.widgets.hovered = widget(
        palette.control_fill_hover,
        palette.control_stroke,
        palette.text_primary,
    );
    // `active.fg_stroke` doubles as egui's strong-text colour: primary,
    // not WinUI's secondary pressed foreground, or every `strong` label
    // would dim.
    visuals.widgets.active = widget(
        palette.control_fill_pressed,
        palette.control_stroke,
        palette.text_primary,
    );
    visuals.widgets.open = visuals.widgets.hovered;
    visuals.panel_fill = palette.ground;
    // `TextEdit`'s ground and code spans: a control surface.
    visuals.extreme_bg_color = palette.control_fill;
    visuals.code_bg_color = palette.control_fill;
    // Table stripes: `LayerOnMicaBaseAltFillColorSecondary`, the same value
    // as the subtle fill in both themes.
    visuals.faint_bg_color = palette.subtle_fill;
    visuals.window_fill = palette.flyout_fill;
    visuals.window_stroke = egui::Stroke::new(1.0_f32, palette.flyout_stroke);
    visuals.window_corner_radius = OVERLAY_CORNER_RADIUS.into();
    visuals.menu_corner_radius = OVERLAY_CORNER_RADIUS.into();
    visuals.selection.bg_fill = accent;
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, accent_text_color(accent));
    visuals.hyperlink_color = accent;
    visuals.error_fg_color = palette.critical;
    visuals.warn_fg_color = palette.caution;
    visuals
}

/// White on a deep accent, near-black on a pale one (the yellow / mint
/// presets) — the same WCAG luminance gate the candidate window applies
/// (`ui/theme.rs::LIGHT_HIGHLIGHT_LUMINANCE`).
pub fn accent_text_color(accent: egui::Color32) -> egui::Color32 {
    let linear = |channel: u8| {
        let channel = f32::from(channel) / 255.0;
        if channel <= 0.03928 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance =
        0.2126 * linear(accent.r()) + 0.7152 * linear(accent.g()) + 0.0722 * linear(accent.b());
    if luminance > 0.55 {
        egui::Color32::from_black_alpha(217)
    } else {
        egui::Color32::WHITE
    }
}

/// WinUI's keyboard focus visual around a custom-drawn control: a 2px
/// ring in the text colour just outside the control's own edge. Drawn
/// only for keyboard focus, as egui's stock widgets do.
pub fn paint_focus_ring(ui: &egui::Ui, response: &egui::Response, corner_radius: f32) {
    if !response.has_focus() {
        return;
    }
    ui.painter().rect_stroke(
        response.rect.expand(FOCUS_RING_WIDTH),
        corner_radius + FOCUS_RING_WIDTH,
        egui::Stroke::new(FOCUS_RING_WIDTH, ui.visuals().text_color()),
        egui::StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deep_accent_takes_white_text_and_pale_accent_dark_text() {
        // Windows' default blue, then the yellow preset.
        assert_eq!(
            accent_text_color(egui::Color32::from_rgb(0x00, 0x78, 0xD4)),
            egui::Color32::WHITE
        );
        assert_eq!(
            accent_text_color(egui::Color32::from_rgb(0xFF, 0xB9, 0x00)),
            egui::Color32::from_black_alpha(217)
        );
    }

    #[test]
    fn both_themes_carry_the_accent_and_their_ground() {
        let accent = egui::Color32::from_rgb(0x00, 0x78, 0xD4);
        let light = visuals(egui::Theme::Light, accent);
        let dark = visuals(egui::Theme::Dark, accent);
        assert!(!light.dark_mode && dark.dark_mode);
        assert_eq!(light.panel_fill, LIGHT.ground);
        assert_eq!(dark.panel_fill, DARK.ground);
        assert_eq!(light.selection.bg_fill, accent);
        assert_eq!(dark.hyperlink_color, accent);
        // Strong text stays the primary colour in both.
        assert_eq!(light.strong_text_color(), LIGHT.text_primary);
        assert_eq!(dark.strong_text_color(), DARK.text_primary);
    }

    #[test]
    fn style_carries_the_title_text_style() {
        let mut style = egui::Style::default();
        apply_style(&mut style);
        assert_eq!(style.text_styles[&title_text_style()].size, 28.0);
        assert_eq!(style.text_styles[&egui::TextStyle::Body].size, 14.0);
        assert_eq!(style.spacing.interact_size.y, 32.0);
    }
}
