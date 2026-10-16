//! Toolkit-neutral presentation both settings windows draw from: the
//! resolver for the document's display language and the labels built on
//! it, the pane titles, the links a page offers, and what a page has to
//! tell the user after a job. The shells pass the machine's locale in; how
//! it is read (and how often) is theirs.

use super::{SettingsDocument, SettingsPane};
use crate::strings::{DisplayLanguage, StringKey, StringResolver};

/// `AboutPage.sponsorURL`.
pub const SPONSOR_URL: &str = "https://p.ecpay.com.tw/AA663DE";
/// `AboutPage.websiteURL` — also the Linux General pane's download link
/// (roadmap L10).
pub const WEBSITE_URL: &str = "https://taigikeyboard.tw";
/// `AboutPage.githubURL`.
pub const GITHUB_URL: &str = "https://github.com/taigikeyboard";
/// `AboutPage.discordURL`.
pub const DISCORD_URL: &str = "https://discord.gg/UFn9RdynPu";
/// `AboutPage.facebookURL`.
pub const FACEBOOK_URL: &str = "https://www.facebook.com/profile.php?id=61589036184924";
/// `AboutPage.instagramURL`.
pub const INSTAGRAM_URL: &str = "https://www.instagram.com/taigikeyboardtw/";
/// `AboutPage.threadsURL`.
pub const THREADS_URL: &str = "https://www.threads.com/@siansiansu";
/// `AboutPage.emailURL`.
pub const EMAIL_URL: &str = "mailto:info@taigikeyboard.tw";

/// The resolver for the document's display language, `system` resolved
/// against the machine's `system_locale`.
pub fn strings_for(document: &SettingsDocument, system_locale: &str) -> StringResolver {
    StringResolver::new(document.effective_display_language(system_locale))
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

/// The Output pop-up's roster: the stored swap as the two scripts it picks
/// between, Hanji (the default) first.
pub const OUTPUT_SCRIPTS: &[bool] = &[true, false];

/// The Output pop-up's row for the stored swap: which script leads.
pub fn output_script_label(is_hanji: bool) -> StringKey {
    if is_hanji {
        StringKey::SettingsOutputScriptHanji
    } else {
        StringKey::SettingsOutputScriptRoman
    }
}

/// The Nasal mark in POJ capitals pop-up's roster: the stored switch as the
/// two markers it picks between, ᴺ (the default) first.
pub const NASAL_MARKER_STYLES: &[bool] = &[true, false];

/// The Nasal mark in POJ capitals pop-up's row for the stored switch.
pub fn nasal_marker_style_label(is_uppercase: bool) -> StringKey {
    if is_uppercase {
        StringKey::SettingsNasalMarkerUppercaseCapital
    } else {
        StringKey::SettingsNasalMarkerUppercaseSmall
    }
}

/// What a page reports after a job (`UserDataPageChrome.swift` `UserDataPageMessage`
/// is the macOS twin).
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

#[cfg(test)]
mod tests {
    use super::super::keys;
    use super::*;

    #[test]
    fn a_system_display_language_follows_the_machine_and_a_stored_one_does_not() {
        let mut document = SettingsDocument::default();
        // trace: no stored tag → "system" → resolve_automatic("ja-JP") = Japanese.
        assert_eq!(
            strings_for(&document, "ja-JP").language,
            DisplayLanguage::Japanese
        );
        document.set_string(&keys::DISPLAY_LANGUAGE, "en");
        assert_eq!(document.display_language(), DisplayLanguage::English);
        assert_eq!(
            strings_for(&document, "ja-JP").language,
            DisplayLanguage::English
        );
    }

    #[test]
    fn a_page_message_titles_and_details_itself() {
        let strings = StringResolver::new(DisplayLanguage::English);
        // trace: desktop.importComplete en = "Import Complete";
        // dictionary.importResult en = "Imported {0}, skipped {1}".
        let imported = PageMessage::Imported {
            imported: 3,
            skipped: 1,
        };
        assert_eq!(imported.title(&strings), "Import Complete");
        assert_eq!(
            imported.detail(&strings).as_deref(),
            Some("Imported 3, skipped 1")
        );
        // trace: common.importFailed en = "Import failed";
        // desktop.notUTF8Detail en = "This file is not UTF-8 text.".
        assert_eq!(PageMessage::NotUtf8.title(&strings), "Import failed");
        assert_eq!(
            PageMessage::NotUtf8.detail(&strings).as_deref(),
            Some("This file is not UTF-8 text.")
        );
        // trace: desktop.openURLFailed en = "Could Not Open the Page"; a
        // failure's detail is the error's own text (here, the URL).
        let refused = PageMessage::failure(StringKey::DesktopOpenURLFailed, WEBSITE_URL);
        assert_eq!(refused.title(&strings), "Could Not Open the Page");
        assert_eq!(refused.detail(&strings).as_deref(), Some(WEBSITE_URL));
        let done = PageMessage::Done(StringKey::DesktopImportComplete);
        assert_eq!(done.title(&strings), "Import Complete");
        assert_eq!(done.detail(&strings), None);
    }
}
