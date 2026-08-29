//! The window's side of updates: the overdue check at launch, the manual
//! check (`--check-now`, the 一般 pane's button), what each outcome leaves
//! in `settings.json`, the toast for an automatic find, and the two-stage
//! install the pending row drives. The decisions are the update crate's;
//! this runs them on a thread and reads the answer each frame.

// 中文: 設定視窗這一側的更新流程 — 啟動逾期檢查、手動檢查、結果落地、toast、兩段式安裝。

use crate::app::SettingsApp;
use crate::widgets::alert::PageMessage;
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

pub struct UpdateState {
    transport: Arc<HttpTransport>,
    check: Option<PendingWork<Outcome>>,
    /// Whether the check in flight answers a press (an alert) rather than
    /// the daily schedule (a toast, once per version).
    is_manual: bool,
    /// A press while a check is in flight is answered when it lands.
    is_manual_outcome_wanted: bool,
    pub manual_outcome: Option<ManualOutcome>,
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
    pub fn check_if_due(&mut self, app: &mut SettingsApp) {
        if checker::is_due(app.document(), now_ms()) {
            self.start_check(app, false);
        }
    }

    /// The 檢查更新 press (`checkManually`).
    pub fn check_manually(&mut self, app: &mut SettingsApp) {
        self.start_check(app, true);
    }

    fn start_check(&mut self, app: &mut SettingsApp, is_manual: bool) {
        if self.check.is_some() {
            if is_manual {
                self.is_manual_outcome_wanted = true;
            }
            return;
        }
        self.is_manual = is_manual;
        app.update_document(|document| checker::stamp_next_check(document, now_ms()));
        let transport = Arc::clone(&self.transport);
        self.check = Some(PendingWork::spawn_quiet(move || {
            checker::check(&*transport, INSTALLED_VERSION)
        }));
    }

    pub fn is_checking(&self) -> bool {
        self.check.is_some()
    }

    /// Collects a finished check and delivers its outcome.
    pub fn poll(&mut self, app: &mut SettingsApp) {
        self.installation.poll();
        let Some(finished) = self.check.as_ref().and_then(PendingWork::poll) else {
            return;
        };
        self.check = None;
        let outcome = finished.unwrap_or(Outcome::Failed);
        self.deliver(app, outcome);
    }

    fn deliver(&mut self, app: &mut SettingsApp, outcome: Outcome) {
        app.update_document(|document| checker::record(document, &outcome));
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
            announce(app, manifest);
        }
    }

    /// The pending row's primary action for the offer it shows.
    pub fn act_on_offer(&mut self, app: &mut SettingsApp, manifest: &UpdateManifest) {
        use taigi_windows_update::Offer;
        match self.installation.offer(manifest) {
            Offer::DownloadPage | Offer::PackageRejected => open_download_page(app, manifest),
            Offer::StartDownload | Offer::DownloadFailed => {
                self.installation.start_download(manifest)
            }
            Offer::Install(_) | Offer::InstallerOpenFailed(_) => self.installation.install(),
            Offer::Downloading => {}
        }
    }
}

/// The automatic check's one notice per version: the announcement is
/// CLAIMED inside the locked settings write first (the scheduled task and
/// this window cannot both win), then the toast posted
/// (`UpdateAnnouncement.post`, claim-then-post rather than post-then-record).
pub fn announce(app: &mut SettingsApp, manifest: &UpdateManifest) {
    let version = manifest.version.clone();
    let mut claimed = false;
    app.update_document(|document| claimed = checker::claim_announcement(document, &version));
    if claimed {
        let strings = app.strings();
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

pub fn open_download_page(app: &mut SettingsApp, manifest: &UpdateManifest) {
    if !taigi_windows_platform::open_url(&manifest.download_page_url) {
        app.message = Some(PageMessage::UrlFailed(manifest.download_page_url.clone()));
    }
}
