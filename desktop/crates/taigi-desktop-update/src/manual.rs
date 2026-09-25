//! What a manual check (the Check for Updates press) says: an alert with its own
//! buttons (`UpdateAlertPresenter`). The words and the first button are
//! decided here; each settings window draws them in its own toolkit.

use crate::checker::Outcome;
use taigi_desktop_core::strings::{StringKey, StringResolver};

/// What a manual check answers with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManualOutcome {
    pub outcome: Outcome,
    /// Whether the first button downloads and installs in-app (the manifest
    /// named a package and there is somewhere to stage it) or opens the
    /// download page.
    pub installs_in_app: bool,
}

impl ManualOutcome {
    /// The alert's title and the line under it — an update names its
    /// version, a failure says what to do, up-to-date needs no second line.
    pub fn alert_text(&self, strings: &StringResolver) -> (String, Option<String>) {
        match &self.outcome {
            Outcome::UpdateAvailable(manifest) => (
                strings
                    .resolve(StringKey::DesktopUpdateAvailableTitle)
                    .to_owned(),
                Some(strings.format(
                    StringKey::DesktopUpdateAvailableMessage,
                    &[&manifest.version],
                )),
            ),
            Outcome::UpToDate => (
                strings
                    .resolve(StringKey::DesktopUpdateUpToDateTitle)
                    .to_owned(),
                None,
            ),
            Outcome::Failed => (
                strings
                    .resolve(StringKey::DesktopUpdateCheckFailedTitle)
                    .to_owned(),
                Some(
                    strings
                        .resolve(StringKey::DesktopUpdateCheckFailedMessage)
                        .to_owned(),
                ),
            ),
        }
    }

    /// The first button's label; `None` when the only answer is OK.
    pub fn proceed_key(&self) -> Option<StringKey> {
        match self.outcome {
            Outcome::UpdateAvailable(_) if self.installs_in_app => {
                Some(StringKey::DesktopUpdateDownloadAndInstallAction)
            }
            Outcome::UpdateAvailable(_) => Some(StringKey::DesktopUpdateDownloadAction),
            Outcome::UpToDate | Outcome::Failed => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UpdateManifest;

    fn manifest() -> UpdateManifest {
        UpdateManifest {
            version: "9.9.9".to_owned(),
            download_page_url: "https://taigikeyboard.tw/download".to_owned(),
            package: None,
        }
    }

    #[test]
    fn the_manual_alert_names_the_version_and_offers_the_right_first_button() {
        let strings = StringResolver::new(taigi_desktop_core::strings::DisplayLanguage::Hanji);
        let available = ManualOutcome {
            outcome: Outcome::UpdateAvailable(manifest()),
            installs_in_app: false,
        };
        let (title, detail) = available.alert_text(&strings);
        assert_eq!(
            title,
            strings.resolve(StringKey::DesktopUpdateAvailableTitle)
        );
        assert!(detail
            .expect("an update names its version")
            .contains("9.9.9"));
        assert_eq!(
            available.proceed_key(),
            Some(StringKey::DesktopUpdateDownloadAction)
        );

        let in_app = ManualOutcome {
            installs_in_app: true,
            ..available
        };
        assert_eq!(
            in_app.proceed_key(),
            Some(StringKey::DesktopUpdateDownloadAndInstallAction)
        );

        // Up to date and a failed check answer with OK alone.
        let up_to_date = ManualOutcome {
            outcome: Outcome::UpToDate,
            installs_in_app: false,
        };
        assert_eq!(up_to_date.alert_text(&strings).1, None);
        assert_eq!(up_to_date.proceed_key(), None);
        let failed = ManualOutcome {
            outcome: Outcome::Failed,
            installs_in_app: false,
        };
        assert!(failed.alert_text(&strings).1.is_some());
        assert_eq!(failed.proceed_key(), None);
    }
}
