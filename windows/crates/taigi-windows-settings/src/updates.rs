//! The window's side of updates: the overdue check at launch, the manual
//! check (`--check-now`, the 一般 pane's button), what each outcome leaves
//! in `settings.json`, the toast for an automatic find, and the two-stage
//! install the pending row drives. The decisions are the update crate's;
//! this runs them on a thread and reads the answer when the window looks.
//!
//! Written once for both windows: everything here takes the shared
//! `SettingsWriter` and answers with a `PageMessage` when there is
//! something for the window to say, so no toolkit reaches in here.

// 中文: 設定視窗這一側的更新流程 — 啟動逾期檢查、手動檢查、結果落地、toast、兩段式安裝。

use crate::presentation::{self, PageMessage};
use crate::settings_writer::SettingsWriter;
use crate::work::PendingWork;
use std::path::PathBuf;
use std::sync::Arc;
use taigi_windows_core::strings::{StringKey, StringResolver};
use taigi_windows_update::{
    checker, toast, verify, HttpTransport, Outcome, UpdateInstallation, UpdateManifest,
};

/// The running build's version (`AppVersion.installed`).
pub const INSTALLED_VERSION: &str = env!("CARGO_PKG_VERSION");

/// What a manual check answers with, shown as an alert with its own
/// buttons (`UpdateAlertPresenter`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ManualOutcome {
    pub outcome: Outcome,
    /// Whether the first button downloads and installs in-app (a signed
    /// copy with a `packageURL`) or opens the download page.
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

pub struct UpdateState {
    transport: Arc<HttpTransport>,
    check: Option<PendingWork<Outcome>>,
    /// Whether the check in flight answers a press (an alert) rather than
    /// the daily schedule (a toast, once per version).
    is_manual: bool,
    /// A press while a check is in flight is answered when it lands.
    is_manual_outcome_wanted: bool,
    manual_outcome: Option<ManualOutcome>,
    pub installation: UpdateInstallation,
}

/// `%LOCALAPPDATA%\TaigiKeyboard`: non-roaming, for staged packages. No
/// such folder ⇒ no in-app install (never a roaming stage).
pub fn local_data_directory() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(|local| PathBuf::from(local).join(taigi_windows_storage::APPLICATION_FOLDER_NAME))
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}

impl UpdateState {
    pub fn new() -> Self {
        let transport = Arc::new(HttpTransport);
        let installation = UpdateInstallation::new(
            transport.clone(),
            verify::running_identity(),
            local_data_directory(),
        );
        // Stale staged packages from earlier runs go at launch (the window
        // is single-instance, so no other window is mid-download).
        installation.remove_staged_packages(None);
        Self {
            transport,
            check: None,
            is_manual: false,
            is_manual_outcome_wanted: false,
            manual_outcome: None,
            installation,
        }
    }

    /// The daily check, when it is due (`checkAutomatically`).
    pub fn check_if_due(&mut self, settings: &mut SettingsWriter) {
        if checker::is_due(settings.document(), now_ms()) {
            self.start_check(settings, false);
        }
    }

    /// The 檢查更新 press (`checkManually`).
    pub fn check_manually(&mut self, settings: &mut SettingsWriter) {
        self.start_check(settings, true);
    }

    fn start_check(&mut self, settings: &mut SettingsWriter, is_manual: bool) {
        if self.check.is_some() {
            if is_manual {
                self.is_manual_outcome_wanted = true;
            }
            return;
        }
        self.is_manual = is_manual;
        settings.update(|document| checker::stamp_next_check(document, now_ms()));
        let transport = Arc::clone(&self.transport);
        self.check = Some(PendingWork::spawn_quiet(move || {
            checker::check(&*transport, INSTALLED_VERSION)
        }));
    }

    pub fn is_checking(&self) -> bool {
        self.check.is_some()
    }

    /// Collects a finished check and delivers its outcome.
    pub fn poll(&mut self, settings: &mut SettingsWriter) {
        self.installation.poll();
        let Some(finished) = self.check.as_ref().and_then(PendingWork::poll) else {
            return;
        };
        self.check = None;
        let outcome = finished.unwrap_or(Outcome::Failed);
        self.deliver(settings, outcome);
    }

    fn deliver(&mut self, settings: &mut SettingsWriter, outcome: Outcome) {
        settings.update(|document| checker::record(document, &outcome));
        match &outcome {
            Outcome::UpdateAvailable(manifest) => self
                .installation
                .discard_staged_package(Some(&manifest.version)),
            Outcome::UpToDate => self.installation.discard_staged_package(None),
            Outcome::Failed => {}
        }
        let answers_press = self.is_manual || self.is_manual_outcome_wanted;
        self.is_manual_outcome_wanted = false;
        if answers_press {
            let installs_in_app = match &outcome {
                Outcome::UpdateAvailable(manifest) => {
                    self.installation.can_install_in_app(manifest)
                }
                _ => false,
            };
            self.manual_outcome = Some(ManualOutcome {
                outcome,
                installs_in_app,
            });
            return;
        }
        if let Outcome::UpdateAvailable(manifest) = &outcome {
            announce(settings, manifest);
        }
    }

    /// The pending row's primary action for the offer it shows.
    pub fn act_on_offer(&mut self, manifest: &UpdateManifest) -> Option<PageMessage> {
        use taigi_windows_update::Offer;
        match self.installation.offer(manifest) {
            Offer::DownloadPage | Offer::PackageRejected => {
                return open_download_page(manifest);
            }
            Offer::StartDownload | Offer::DownloadFailed => {
                self.installation.start_download(manifest)
            }
            Offer::Install(_) | Offer::InstallerOpenFailed(_) => self.installation.install(),
            Offer::Downloading => {}
        }
        None
    }

    /// A check that answered a press and has not been dismissed yet.
    pub fn manual_outcome(&self) -> Option<&ManualOutcome> {
        self.manual_outcome.as_ref()
    }

    /// The manual alert's first button: the outcome is answered, then the
    /// update is fetched in-app or the download page opened.
    pub fn proceed_with_manual_outcome(&mut self) -> Option<PageMessage> {
        let outcome = self.manual_outcome.take()?;
        let Outcome::UpdateAvailable(manifest) = &outcome.outcome else {
            return None;
        };
        if outcome.installs_in_app {
            self.installation.start_download(manifest);
            return None;
        }
        open_download_page(manifest)
    }

    /// 稍後, or the OK on an answer with nothing to do.
    pub fn dismiss_manual_outcome(&mut self) {
        self.manual_outcome = None;
    }

    /// Whether a check or a download is in flight — what makes a window
    /// look more often than once a second.
    pub fn is_busy(&self) -> bool {
        self.is_checking() || self.installation.is_downloading()
    }
}

/// The automatic check's one notice per version: the announcement is
/// CLAIMED inside the locked settings write first (the scheduled task and
/// this window cannot both win), then the toast posted
/// (`UpdateAnnouncement.post`, claim-then-post rather than post-then-record).
pub fn announce(settings: &mut SettingsWriter, manifest: &UpdateManifest) {
    let version = manifest.version.clone();
    let mut claimed = false;
    settings.update(|document| claimed = checker::claim_announcement(document, &version));
    if claimed {
        let strings = settings.strings();
        post_toast(&strings, manifest);
    }
}

pub fn post_toast(strings: &StringResolver, manifest: &UpdateManifest) -> bool {
    toast::show(
        strings.resolve(StringKey::DesktopUpdateAvailableTitle),
        &strings.format(
            StringKey::DesktopUpdateAvailableMessage,
            &[&manifest.version],
        ),
    )
}

pub fn open_download_page(manifest: &UpdateManifest) -> Option<PageMessage> {
    presentation::open_url(&manifest.download_page_url)
}

#[cfg(test)]
mod tests {
    use super::*;
    use taigi_windows_storage::{LiveSettings, SettingsFileStore};

    fn writer(directory: &std::path::Path) -> SettingsWriter {
        SettingsWriter::new(
            std::rc::Rc::new(LiveSettings::new(SettingsFileStore::new(directory))),
            false,
        )
    }

    fn manifest() -> UpdateManifest {
        UpdateManifest {
            version: "9.9.9".to_owned(),
            download_page_url: "https://taigikeyboard.tw/download".to_owned(),
            package_url: None,
        }
    }

    #[test]
    fn announce_claims_the_version_before_posting_and_only_once() {
        // trace: claim_announcement records the version and answers true;
        // a second call for the same version finds it recorded and answers
        // false, so the toast is posted once however often the check runs.
        let directory = tempfile::tempdir().expect("a temporary settings directory");
        let mut settings = writer(directory.path());
        announce(&mut settings, &manifest());
        let after_first = settings.document().clone();
        announce(&mut settings, &manifest());
        assert_eq!(
            settings.document(),
            &after_first,
            "the second attempt finds the version already claimed"
        );
        assert!(
            settings.write_failure().is_none(),
            "both attempts wrote the settings file"
        );
    }

    #[test]
    fn a_read_only_window_refuses_the_write_and_keeps_saying_why() {
        // trace: no `%APPDATA%` ⇒ the window opens on the defaults with the
        // banner already up, and a check's stamp must not clear it.
        let directory = tempfile::tempdir().expect("a temporary settings directory");
        let mut settings = SettingsWriter::new(
            std::rc::Rc::new(LiveSettings::new(SettingsFileStore::new(directory.path()))),
            true,
        );
        let before = settings.document().clone();
        announce(&mut settings, &manifest());
        assert_eq!(settings.document(), &before, "nothing was written");
        assert_eq!(settings.write_failure(), Some("APPDATA"));
    }

    #[test]
    fn the_manual_alert_names_the_version_and_offers_the_right_first_button() {
        let strings = StringResolver::new(taigi_windows_core::strings::DisplayLanguage::Hanji);
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

    #[test]
    fn an_answered_manual_outcome_leaves_nothing_for_the_window_to_show() {
        // trace: 稍後 dismisses; proceeding on an outcome with nothing to
        // fetch also clears it, and opens no URL.
        let mut updates = UpdateState::new();
        updates.manual_outcome = Some(ManualOutcome {
            outcome: Outcome::UpToDate,
            installs_in_app: false,
        });
        assert!(updates.proceed_with_manual_outcome().is_none());
        assert!(updates.manual_outcome().is_none());

        updates.manual_outcome = Some(ManualOutcome {
            outcome: Outcome::Failed,
            installs_in_app: false,
        });
        updates.dismiss_manual_outcome();
        assert!(updates.manual_outcome().is_none());
    }
}
