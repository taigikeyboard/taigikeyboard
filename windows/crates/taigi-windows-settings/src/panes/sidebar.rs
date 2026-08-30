//! The sidebar: every pane, one flat list, in `SettingsPane::SIDEBAR` order
//! (`SettingsSidebarView`, `SettingsSplitView.swift:88-115`), drawn as
//! WinUI's `NavigationView` draws its items — a rounded row that fills
//! subtly under the pointer, the selected one marked by the accent pill at
//! its left edge — each an icon and a label (the Mac's SF Symbols become
//! Segoe Fluent Icons, `SettingsSplitView.swift:36-44`). No section
//! headers (USER 2026-08-18). Selecting persists `selectedSettingsPane`.

// 中文: 側欄 — 五個 pane 一列,WinUI NavigationView 樣:圓角列、hover 淡填、選取列左緣強調色指示條;選取即持久化。

use crate::app::SettingsApp;
use crate::fonts::icon_family;
use crate::theme::{self, CONTROL_CORNER_RADIUS};
use taigi_windows_core::settings::SettingsPane;

/// `SettingsPaneLayout.sidebarWidth`.
const SIDEBAR_WIDTH: f32 = 215.0;
/// The pane's own inset around the rows.
const PANE_MARGIN: egui::Margin = egui::Margin {
    left: 8,
    right: 8,
    top: 12,
    bottom: 8,
};
/// `NavigationViewItem`: 36 tall, the icon 12 in from the edge, the label
/// 16 after the 16px icon.
const ROW_HEIGHT: f32 = 36.0;
const ICON_INSET: f32 = 12.0;
const ICON_SIZE: f32 = 16.0;
const LABEL_GAP: f32 = 16.0;
/// The selection indicator: a 3×16 pill on the row's left edge.
const INDICATOR_SIZE: egui::Vec2 = egui::vec2(3.0, 16.0);

/// The Segoe Fluent Icons / MDL2 Assets glyph for each pane — the same
/// code points in both faces — matching the Mac's symbol per pane:
/// gearshape → Settings, paintpalette → Color, keyboard → KeyboardClassic,
/// character.book.closed → Dictionary, books.vertical → Library.
fn icon_glyph(pane: SettingsPane) -> &'static str {
    match pane {
        SettingsPane::General => "\u{E713}",
        SettingsPane::Appearance => "\u{E790}",
        SettingsPane::Shortcuts => "\u{E765}",
        SettingsPane::CustomDictionary => "\u{E82D}",
        SettingsPane::DictionarySources | SettingsPane::DictionarySearch => "\u{E8F1}",
    }
}

pub fn show(ctx: &egui::Context, app: &mut SettingsApp) {
    let strings = app.strings();
    let current = app.pane();
    let has_icon_face = app.fonts().has_icon_face;
    let mut chosen = None;

    // No rule between the sidebar and the form: Settings draws none, the
    // cards make the split.
    egui::SidePanel::left("sidebar")
        .exact_width(SIDEBAR_WIDTH)
        .resizable(false)
        .show_separator_line(false)
        .frame(egui::Frame::side_top_panel(&ctx.style()).inner_margin(PANE_MARGIN))
        .show(ctx, |ui| {
            for pane in SettingsPane::SIDEBAR {
                let Some(key) = pane.title_key() else {
                    continue;
                };
                let label = strings.resolve(key);
                if navigation_row(ui, pane, label, pane == current, has_icon_face)
                    && pane != current
                {
                    chosen = Some(pane);
                }
            }
        });
    if let Some(pane) = chosen {
        app.select_pane(pane);
    }
}

/// One item; answers whether it was pressed. The icon is drawn only when
/// the machine has the icon face (egui would draw a box otherwise), and
/// the row's text is the label alone for assistive technology.
fn navigation_row(
    ui: &mut egui::Ui,
    pane: SettingsPane,
    label: &str,
    is_selected: bool,
    has_icon_face: bool,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), ROW_HEIGHT),
        egui::Sense::click(),
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            is_selected,
            label,
        )
    });
    if !ui.is_rect_visible(rect) {
        return response.clicked();
    }
    let palette = theme::palette(ui.visuals());
    let fill = if response.is_pointer_button_down_on() {
        palette.subtle_fill_pressed
    } else if is_selected || response.hovered() {
        palette.subtle_fill
    } else {
        egui::Color32::TRANSPARENT
    };
    let painter = ui.painter();
    painter.rect_filled(rect, CONTROL_CORNER_RADIUS, fill);
    if is_selected {
        let indicator = egui::Rect::from_center_size(
            egui::pos2(rect.left() + INDICATOR_SIZE.x / 2.0, rect.center().y),
            INDICATOR_SIZE,
        );
        painter.rect_filled(
            indicator,
            INDICATOR_SIZE.x / 2.0,
            ui.visuals().selection.bg_fill,
        );
    }
    let mut x = rect.left() + ICON_INSET;
    if has_icon_face {
        painter.text(
            egui::pos2(x, rect.center().y),
            egui::Align2::LEFT_CENTER,
            icon_glyph(pane),
            egui::FontId::new(ICON_SIZE, icon_family()),
            ui.visuals().text_color(),
        );
        x += ICON_SIZE + LABEL_GAP;
    }
    painter.text(
        egui::pos2(x, rect.center().y),
        egui::Align2::LEFT_CENTER,
        label,
        egui::TextStyle::Body.resolve(ui.style()),
        ui.visuals().text_color(),
    );
    theme::paint_focus_ring(ui, &response, CONTROL_CORNER_RADIUS);
    response.clicked()
}
