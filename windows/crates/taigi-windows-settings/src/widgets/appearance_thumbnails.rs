//! The 淺色 / 深色 / 自動 selector, drawn the way System Settings draws its
//! Appearance row (`AppearanceModeRow`, `AppearanceSettingsView.swift:313-416`):
//! a thumbnail per mode with a caption under it, the selected one ringed
//! in the accent. The thumbnails are miniature candidate bars — three
//! cells, the first highlighted — on the mode's background; 自動 is the two
//! split down the middle. Fixed colours on purpose: each depicts ONE mode.

// 中文: 外觀縮圖列 — 亮 / 暗 / 自動,迷你候選列,選中者描邊。

use taigi_windows_core::settings::AppearanceMode;
use taigi_windows_core::strings::StringResolver;

/// System Settings' order: light, dark, then auto.
const MODES: [AppearanceMode; 3] = [
    AppearanceMode::Light,
    AppearanceMode::Dark,
    AppearanceMode::Auto,
];
const THUMBNAIL_SIZE: egui::Vec2 = egui::vec2(62.0, 40.0);
const THUMBNAIL_CORNER_RADIUS: f32 = 8.0;
const THUMBNAIL_SPACING: f32 = 14.0;
const SELECTION_RING_PADDING: f32 = 2.0;

/// Draws the row bound to `selection`; answers whether it changed.
pub fn show(ui: &mut egui::Ui, selection: &mut AppearanceMode, strings: &StringResolver) -> bool {
    let before = *selection;
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = THUMBNAIL_SPACING;
        for mode in MODES {
            ui.vertical(|ui| {
                let (rect, response) = ui.allocate_exact_size(THUMBNAIL_SIZE, egui::Sense::click());
                if response.clicked() {
                    *selection = mode;
                }
                // The thumbnail's highlighted cell is the user's accent —
                // what the real window highlights in (`sync_accent`).
                let highlight = ui.visuals().selection.bg_fill;
                let painter = ui.painter();
                let corner = THUMBNAIL_CORNER_RADIUS;
                match mode {
                    AppearanceMode::Light => paint_bar(painter, rect, false, highlight),
                    AppearanceMode::Dark => paint_bar(painter, rect, true, highlight),
                    AppearanceMode::Auto => {
                        // Light on the left, dark on the right, split down
                        // the middle — how System Settings depicts it.
                        paint_bar(painter, rect, false, highlight);
                        let mut right = rect;
                        right.min.x = rect.center().x;
                        paint_bar(&painter.with_clip_rect(right), rect, true, highlight);
                    }
                }
                // A hairline so the light thumbnail keeps an edge on a light
                // form background; the ring only on the selected one.
                painter.rect_stroke(
                    rect,
                    corner,
                    egui::Stroke::new(1.0_f32, ui.visuals().widgets.noninteractive.bg_stroke.color),
                    egui::StrokeKind::Inside,
                );
                if *selection == mode {
                    painter.rect_stroke(
                        rect.expand(SELECTION_RING_PADDING),
                        corner + SELECTION_RING_PADDING,
                        egui::Stroke::new(2.0_f32, ui.visuals().selection.bg_fill),
                        egui::StrokeKind::Outside,
                    );
                }
                let caption = strings.resolve(mode.label_key());
                ui.add_space(5.0);
                ui.vertical_centered(|ui| {
                    if *selection == mode {
                        ui.label(caption);
                    } else {
                        ui.weak(caption);
                    }
                });
            });
        }
    });
    *selection != before
}

/// A miniature of what the setting controls: a candidate bar on the
/// mode's background.
fn paint_bar(painter: &egui::Painter, rect: egui::Rect, dark: bool, highlight: egui::Color32) {
    let background = if dark {
        egui::Color32::from_gray(0x29)
    } else {
        egui::Color32::from_gray(0xF0)
    };
    let cell = if dark {
        egui::Color32::from_gray(0x61)
    } else {
        egui::Color32::from_gray(0xBD)
    };
    painter.rect_filled(rect, THUMBNAIL_CORNER_RADIUS, background);
    let height = 7.0;
    let widths = [12.0, 9.0, 9.0];
    let spacing = 2.5;
    let total: f32 = widths.iter().sum::<f32>() + spacing * 2.0;
    let mut x = rect.center().x - total / 2.0;
    let y = rect.center().y - height / 2.0;
    for (index, width) in widths.into_iter().enumerate() {
        let capsule = egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, height));
        let color = if index == 0 { highlight } else { cell };
        painter.rect_filled(capsule, height / 2.0, color);
        x += width + spacing;
    }
}
