//! A link out to the web, with the failure shown rather than swallowed
//! (`ExternalLinkButton.swift`): a button that silently does nothing is
//! indistinguishable from a broken one. `footer` is the understated inline
//! form for an attribution line.

// 中文: 對外連結 — 開失敗要說(走頁面共用的 alert);頁尾樣式低調。

use crate::app::SettingsApp;
use crate::widgets::alert::PageMessage;

/// The footer form: no icon, the surrounding fine print's weight.
pub fn footer(ui: &mut egui::Ui, app: &mut SettingsApp, text: &str, url: &str) {
    if ui
        .add(egui::Link::new(egui::RichText::new(text).weak()))
        .clicked()
        && !taigi_windows_platform::open_url(url)
    {
        app.message = Some(PageMessage::UrlFailed(url.to_owned()));
    }
}
