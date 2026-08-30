//! The one alert a page raises: a title, an optional detail, OK. What a
//! page has to tell the user after a job (`PageMessage`) and what a
//! refused URL says (`ExternalLinkButton`) both come through here.

// 中文: 頁面唯一的警示視窗 — 標題、可選細節、確定。

use crate::presentation::PageMessage;
use taigi_windows_core::strings::{StringKey, StringResolver};

/// Shows `message` until OK dismisses it.
pub fn show(ctx: &egui::Context, strings: &StringResolver, message: &mut Option<PageMessage>) {
    let Some(current) = message.as_ref() else {
        return;
    };
    let title = current.title(strings);
    let detail = current.detail(strings);
    let mut dismissed = false;
    egui::Window::new(title)
        .id(egui::Id::new("page_alert"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.set_max_width(360.0);
            if let Some(detail) = detail {
                ui.label(detail);
                ui.add_space(8.0);
            }
            ui.vertical_centered(|ui| {
                if ui.button(strings.resolve(StringKey::CommonOk)).clicked() {
                    dismissed = true;
                }
            });
        });
    if dismissed {
        *message = None;
    }
}
