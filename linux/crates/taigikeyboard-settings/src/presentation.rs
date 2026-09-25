//! Toolkit-neutral presentation: the display-language resolver and the
//! labels built on it, the pane titles, the links the About page offers.
//! Port of the Windows settings crate's `presentation.rs`.

use taigi_desktop_core::settings::{keys, SettingsDocument, SettingsPane};
use taigi_desktop_core::strings::{DisplayLanguage, StringKey, StringResolver};

/// `AboutPage.sponsorURL`.
pub const SPONSOR_URL: &str = "https://p.ecpay.com.tw/AA663DE";
/// `AboutPage.websiteURL` — also the General pane's download link (roadmap L10).
pub const WEBSITE_URL: &str = "https://taigikeyboard.tw";
/// `AboutPage.githubURL`.
pub const GITHUB_URL: &str = "https://github.com/taigikeyboard";
/// `AboutPage.discordURL`.
pub const DISCORD_URL: &str = "https://discord.gg/kXhtQfWvK";
/// `AboutPage.emailURL`.
pub const EMAIL_URL: &str = "mailto:info@taigikeyboard.tw";

/// The display language the document asks for, `system` resolved against
/// the machine (`DisplayLanguageStore.syncFromSettings`).
pub fn display_language_of(document: &SettingsDocument) -> DisplayLanguage {
    let tag = document.string(&keys::DISPLAY_LANGUAGE);
    DisplayLanguage::from_tag(&tag).effective(&taigi_linux_platform::system_locale())
}

pub fn strings_for(document: &SettingsDocument) -> StringResolver {
    StringResolver::new(display_language_of(document))
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
    Failure { title: StringKey, detail: String },
    Done(StringKey),
    NotUtf8,
    Imported { imported: usize, skipped: usize },
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
        }
    }
}
