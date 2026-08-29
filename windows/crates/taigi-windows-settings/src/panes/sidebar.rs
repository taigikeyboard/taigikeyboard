//! The sidebar: every pane, one flat list, in `SettingsPane::SIDEBAR` order
//! (`SettingsSidebarView`, `SettingsSplitView.swift:88-115`). No section
//! headers (USER 2026-08-18). Selecting persists `selectedSettingsPane`.

// 中文: 側欄 — 五個 pane 一列,選取即持久化。

use crate::app::SettingsApp;
use taigi_windows_core::settings::SettingsPane;

/// `SettingsPaneLayout.sidebarWidth`.
const SIDEBAR_WIDTH: f32 = 215.0;
const ROW_HEIGHT: f32 = 28.0;

pub fn show(ctx: &egui::Context, app: &mut SettingsApp) {
    let strings = app.strings();
    let current = app.pane();
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
                    egui::SelectableLabel::new(pane == current, label),
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
