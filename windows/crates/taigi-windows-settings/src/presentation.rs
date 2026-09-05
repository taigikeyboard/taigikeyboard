//! Toolkit-neutral presentation: the display-language resolver and the
//! labels built on it, the pane titles, the links a page offers, and what
//! a page has to tell the user after a job. The egui window reads them
//! today, the WinUI one from roadmap W17 — so neither owns them.

// 與 UI 套件無關的呈現層 — 介面語言解析與標籤、pane 標題、對外連結、頁面訊息。

use std::sync::OnceLock;
use taigi_windows_core::settings::{keys, SettingsDocument, SettingsPane};
use taigi_windows_core::strings::{DisplayLanguage, StringKey, StringResolver};

/// `GeneralSettingsView.sponsorURL`.
pub const SPONSOR_URL: &str = "https://p.ecpay.com.tw/AA663DE";

/// The machine's UI language, read once. Windows requires a sign-out to
/// change it, so a running window can hold the answer instead of asking
/// `GetUserPreferredUILanguages` again for every redraw.
fn system_locale() -> &'static str {
    static LOCALE: OnceLock<String> = OnceLock::new();
    LOCALE.get_or_init(taigi_windows_platform::system_locale)
}

/// The resolver for the document's display language, `system` resolved
/// against the machine (`DisplayLanguageStore.syncFromSettings`).
pub fn strings_for(document: &SettingsDocument) -> StringResolver {
    let tag = document.string(&keys::DISPLAY_LANGUAGE);
    let language = DisplayLanguage::from_tag(&tag).effective(system_locale());
    StringResolver::new(language)
}

/// The window title = the selected pane's label (`SettingsSplitViewController`).
pub fn pane_title(strings: &StringResolver, pane: SettingsPane) -> String {
    strings
        .resolve(pane.title_key().unwrap_or(StringKey::DesktopGeneralTab))
        .to_owned()
}

/// A language picker row: the endonym, so a user can find their own
/// language whatever the UI currently reads in; `System` is the one
/// translated row.
pub fn display_language_label(language: DisplayLanguage, strings: &StringResolver) -> String {
    language.endonym().map_or_else(
        || {
            strings
                .resolve(StringKey::SettingsDisplayLanguageAutomatic)
                .to_owned()
        },
        str::to_owned,
    )
}

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

    pub fn title(&self, strings: &StringResolver) -> String {
        let key = match self {
            Self::Failure { title, .. } => *title,
            Self::Done(key) => *key,
            Self::NotUtf8 => StringKey::CommonImportFailed,
            Self::Imported { .. } => StringKey::DesktopImportComplete,
            Self::UrlFailed(_) => StringKey::DesktopOpenURLFailed,
        };
        strings.resolve(key).to_owned()
    }

    pub fn detail(&self, strings: &StringResolver) -> Option<String> {
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

/// Opens `url` in the browser, answering the message to show when it
/// refused: a button that silently does nothing is indistinguishable from
/// a broken one (`ExternalLinkButton.swift:740-744`).
pub fn open_url(url: &str) -> Option<PageMessage> {
    (!taigi_windows_platform::open_url(url)).then(|| PageMessage::UrlFailed(url.to_owned()))
}
