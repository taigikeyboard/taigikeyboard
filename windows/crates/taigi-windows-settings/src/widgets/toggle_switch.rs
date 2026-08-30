//! WinUI's `ToggleSwitch`, the on/off control of every Settings page: a
//! 40×20 track — a ring when off, the accent when on — with a thumb that
//! slides across. No On/Off caption: the row's header names the setting,
//! and the switch carries that name for assistive technology.

// 中文: WinUI 開關 — 關為描邊環、開為強調色軌道,圓鈕滑動;無 On/Off 字樣。

use crate::theme;

const TRACK_SIZE: egui::Vec2 = egui::vec2(40.0, 20.0);
/// `ToggleSwitchKnob` radius off / on; hover grows it by one.
const THUMB_RADIUS_OFF: f32 = 6.0;
const THUMB_RADIUS_ON: f32 = 7.0;
const THUMB_HOVER_GROWTH: f32 = 1.0;
/// The thumb's centre from the track's edge, either end.
const THUMB_INSET: f32 = 10.0;

/// The switch bound to `on`; `accessible_label` is what a screen reader
/// calls it (the row's header). The response reports `changed()`.
pub fn show(ui: &mut egui::Ui, on: &mut bool, accessible_label: &str) -> egui::Response {
    let (rect, mut response) = ui.allocate_exact_size(TRACK_SIZE, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let is_on = *on;
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::Checkbox,
            ui.is_enabled(),
            is_on,
            accessible_label,
        )
    });
    if !ui.is_rect_visible(rect) {
        return response;
    }
    let palette = theme::palette(ui.visuals());
    let accent = ui.visuals().selection.bg_fill;
    let position = ui.ctx().animate_bool(response.id, is_on);
    let is_hovered = response.hovered() || response.is_pointer_button_down_on();
    // Off: a strong ring on nothing (hover fills it subtly); on: the
    // accent, a shade lighter under the pointer (`AccentFillColorSecondary`).
    let (fill, stroke) = if is_on {
        let fill = if is_hovered {
            theme::accent_hover(accent)
        } else {
            accent
        };
        (fill, egui::Stroke::NONE)
    } else {
        let fill = if is_hovered {
            palette.subtle_fill
        } else {
            egui::Color32::TRANSPARENT
        };
        (
            fill,
            egui::Stroke::new(1.0_f32, palette.control_strong_stroke),
        )
    };
    let painter = ui.painter();
    let track_radius = TRACK_SIZE.y / 2.0;
    painter.rect(rect, track_radius, fill, stroke, egui::StrokeKind::Inside);
    let thumb_x = egui::lerp(
        (rect.left() + THUMB_INSET)..=(rect.right() - THUMB_INSET),
        position,
    );
    let mut thumb_radius = egui::lerp(THUMB_RADIUS_OFF..=THUMB_RADIUS_ON, position);
    if is_hovered {
        thumb_radius += THUMB_HOVER_GROWTH;
    }
    // On the accent, the text colour the theme already chose for it.
    let thumb_color = if is_on {
        ui.visuals().selection.stroke.color
    } else {
        palette.text_secondary
    };
    painter.circle_filled(
        egui::pos2(thumb_x, rect.center().y),
        thumb_radius,
        thumb_color,
    );
    theme::paint_focus_ring(ui, &response, track_radius);
    response
}
