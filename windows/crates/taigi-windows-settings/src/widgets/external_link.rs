//! A link out to the web, with the failure shown rather than swallowed
//! (`ExternalLinkButton.swift`): a button that silently does nothing is
//! indistinguishable from a broken one. `footer` is the understated inline
//! form for an attribution line.

// 中文: 對外連結 — 開失敗要說;頁尾樣式低調。

use crate::app::SettingsApp;
use taigi_windows_core::strings::StringKey;

/// The footer form: no icon, the surrounding fine print's weight.
pub fn footer(ui: &mut egui::Ui, app: &mut SettingsApp, text: &str, url: &str) {
    if ui
        .add(egui::Link::new(egui::RichText::new(text).weak()))
        .clicked()
    {
        open(app, url);
    }
}

fn open(app: &mut SettingsApp, url: &str) {
    if !taigi_windows_platform::open_url(url) {
        app.failed_url = Some(url.to_owned());
    }
}

/// The "could not open" alert, until dismissed.
pub fn show_failure(ctx: &egui::Context, app: &mut SettingsApp) {
    let Some(url) = app.failed_url.clone() else {
        return;
    };
    let strings = app.strings();
    let mut dismissed = false;
    egui::Window::new(strings.resolve(StringKey::DesktopOpenURLFailed))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label(&url);
            ui.add_space(8.0);
            if ui.button(strings.resolve(StringKey::CommonOk)).clicked() {
                dismissed = true;
            }
        });
    if dismissed {
        app.failed_url = None;
    }
}
