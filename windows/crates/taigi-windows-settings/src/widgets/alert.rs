//! The one alert a page raises: a title, an optional detail, OK. What a
//! page has to tell the user after a job (`UserDataPageMessage`) and what
//! a refused URL says (`ExternalLinkButton`) both come through here.

// 中文: 頁面唯一的警示視窗 — 標題、可選細節、確定。

use taigi_windows_core::strings::{StringKey, StringResolver};

/// What a page reports after a job (`UserDataPageChrome.swift:19-52`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageMessage {
    Failure {
        title: StringKey,
        detail: String,
    },
    Done(StringKey),
    NotUtf8,
    Imported {
        imported: usize,
        skipped: usize,
    },
    /// A URL the browser refused to open (`ExternalLinkButton.swift:740-744`).
    UrlFailed(String),
}

impl PageMessage {
    pub fn failure(title: StringKey, error: impl std::fmt::Display) -> Self {
        Self::Failure {
            title,
            detail: error.to_string(),
        }
    }

    fn title(&self, strings: &StringResolver) -> String {
        let key = match self {
            Self::Failure { title, .. } => *title,
            Self::Done(key) => *key,
            Self::NotUtf8 => StringKey::CommonImportFailed,
            Self::Imported { .. } => StringKey::DesktopImportComplete,
            Self::UrlFailed(_) => StringKey::DesktopOpenURLFailed,
        };
        strings.resolve(key).to_owned()
    }

    fn detail(&self, strings: &StringResolver) -> Option<String> {
        match self {
            Self::Failure { detail, .. } => Some(detail.clone()),
            Self::Done(_) => None,
            Self::NotUtf8 => Some(strings.resolve(StringKey::DesktopNotUTF8Detail).to_owned()),
            Self::Imported { imported, skipped } => {
                Some(strings.format(StringKey::DictionaryImportResult, &[imported, skipped]))
            }
            Self::UrlFailed(url) => Some(url.clone()),
        }
    }
}

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
