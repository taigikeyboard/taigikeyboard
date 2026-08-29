//! The sidebar: every pane, one flat list, in `SettingsPane::SIDEBAR` order
//! (`SettingsSidebarView`, `SettingsSplitView.swift:88-115`), each row an
//! icon and a label — the Mac's SF Symbols become Segoe Fluent Icons
//! (`SettingsSplitView.swift:36-44`). No section headers (USER 2026-08-18).
//! Selecting persists `selectedSettingsPane`.

// 中文: 側欄 — 五個 pane 一列,每列 icon + 標籤(SF Symbols → Segoe Fluent Icons),選取即持久化。

use crate::app::SettingsApp;
use crate::fonts::icon_family;
use taigi_windows_core::settings::SettingsPane;

/// `SettingsPaneLayout.sidebarWidth`.
const SIDEBAR_WIDTH: f32 = 215.0;
const ROW_HEIGHT: f32 = 28.0;
const ICON_SIZE: f32 = 16.0;
const LABEL_SIZE: f32 = 14.0;

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

/// The row's text: the icon in the icon face, then the label — or just the
/// label when the machine has no icon face (egui would draw a box). The
/// colour is egui's `PLACEHOLDER`, which the widget swaps for its own
/// foreground at paint time, so selected / hovered / focused rows keep
/// egui's colours rather than one fixed here.
fn row_text(pane: SettingsPane, label: &str, has_icon_face: bool) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    if has_icon_face {
        job.append(
            icon_glyph(pane),
            0.0,
            egui::TextFormat {
                font_id: egui::FontId::new(ICON_SIZE, icon_family()),
                color: egui::Color32::PLACEHOLDER,
                valign: egui::Align::Center,
                ..Default::default()
            },
        );
    }
    job.append(
        label,
        if has_icon_face { 10.0 } else { 0.0 },
        egui::TextFormat {
            font_id: egui::FontId::new(LABEL_SIZE, egui::FontFamily::Proportional),
            color: egui::Color32::PLACEHOLDER,
            valign: egui::Align::Center,
            ..Default::default()
        },
    );
    job
}

pub fn show(ctx: &egui::Context, app: &mut SettingsApp) {
    let strings = app.strings();
    let current = app.pane();
    let has_icon_face = app.fonts().has_icon_face;
    let mut chosen = None;
    egui::SidePanel::left("sidebar")
        .exact_width(SIDEBAR_WIDTH)
        .resizable(false)
        .show(ctx, |ui| {
            ui.add_space(12.0);
            for pane in SettingsPane::SIDEBAR {
                let Some(key) = pane.title_key() else {
                    continue;
                };
                let label = strings.resolve(key);
                let response = ui.add_sized(
                    [ui.available_width(), ROW_HEIGHT],
                    egui::SelectableLabel::new(
                        pane == current,
                        row_text(pane, label, has_icon_face),
                    ),
                );
                if response.clicked() && pane != current {
                    chosen = Some(pane);
                }
            }
        });
    if let Some(pane) = chosen {
        app.select_pane(pane);
    }
}
