//! The two-stage install: a finished download turns the 一般 pane's button
//! into 安裝 rather than launching the installer on its own, so nothing
//! takes the focus away from a document the user may have gone back to
//! typing in. Port of `UpdateInstallation` + `UpdatePackageDownload`. The
//! download and the verification run on a thread; the pane polls.

// 中文: 兩段式安裝狀態機 — 下載+驗簽在背景,按鈕變成「安裝」後才由使用者啟動安裝程式。

use crate::manifest::UpdateManifest;
use crate::transport::PackageDownloader;
use crate::verify::{self, PackageIdentity};
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use taigi_windows_core::strings::StringKey;

/// The staging folder, under the per-user local (non-roaming) data.
const STAGING_FOLDER: &str = "Updates";

/// What the pending-update row offers next (`UpdateInstallation.Offer`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Offer {
    /// No in-app install (no `packageURL`, or an unsigned running copy):
    /// the download page, in the browser.
    DownloadPage,
    StartDownload,
    Downloading,
    /// Downloaded and verified; the second press opens the installer.
    Install(PathBuf),
    DownloadFailed,
    PackageRejected,
    InstallerOpenFailed(PathBuf),
}

impl Offer {
    pub fn staged_package(&self) -> Option<&Path> {
        match self {
            Self::Install(package) | Self::InstallerOpenFailed(package) => Some(package),
            _ => None,
        }
    }

    /// The label of the button that acts on the state, so a control and
    /// the state it acts on cannot be paired wrongly. `None` while a
    /// download runs: there is nothing to press, only a spinner.
    pub fn action_key(&self) -> Option<StringKey> {
        match self {
            Self::DownloadPage | Self::PackageRejected => {
                Some(StringKey::DesktopUpdateDownloadAction)
            }
            Self::StartDownload => Some(StringKey::DesktopUpdateDownloadAndInstallAction),
            Self::Install(_) | Self::InstallerOpenFailed(_) => {
                Some(StringKey::DesktopUpdateInstallAction)
            }
            Self::DownloadFailed => Some(StringKey::DesktopUpdateRetryAction),
            Self::Downloading => None,
        }
    }

    /// The note that travels with the state, so a control and an
    /// explanation cannot be paired wrongly.
    pub fn note_key(&self) -> Option<StringKey> {
        match self {
            Self::DownloadFailed | Self::PackageRejected | Self::InstallerOpenFailed(_) => {
                Some(StringKey::DesktopUpdateInstallFailedNote)
            }
            Self::DownloadPage | Self::StartDownload | Self::Downloading | Self::Install(_) => None,
        }
    }
}

enum DownloadOutcome {
    Verified(PathBuf),
    Rejected(PathBuf),
    Failed(Option<PathBuf>),
}

pub struct UpdateInstallation {
    progress: Offer,
    target_version: Option<String>,
    /// Which download the state belongs to: a result from an older one
    /// (a re-check moved the version) is discarded on arrival.
    current_download: u64,
    receiver: Option<mpsc::Receiver<(u64, DownloadOutcome)>>,
    downloader: Arc<dyn PackageDownloader>,
    /// The running copy's own signature, read once: what a package must
    /// be signed with. `None` = unsigned build ⇒ download page only.
    running_identity: Option<PackageIdentity>,
    /// Where packages are staged; `None` (no non-roaming per-user folder)
    /// ⇒ download page only, never a roaming stage.
    staging_directory: Option<PathBuf>,
}

impl UpdateInstallation {
    pub fn new(
        downloader: Arc<dyn PackageDownloader>,
        running_identity: Option<PackageIdentity>,
        local_data_directory: Option<PathBuf>,
    ) -> Self {
        Self {
            progress: Offer::StartDownload,
            target_version: None,
            current_download: 0,
            receiver: None,
            downloader,
            running_identity,
            staging_directory: local_data_directory.map(|local| local.join(STAGING_FOLDER)),
        }
    }

    /// Whether this copy can download and install by itself: a package to
    /// fetch, a signature of its own to hold it against, somewhere to stage.
    pub fn can_install_in_app(&self, manifest: &UpdateManifest) -> bool {
        manifest.package_url.is_some()
            && self.running_identity.is_some()
            && self.staging_directory.is_some()
    }

    pub fn offer(&self, manifest: &UpdateManifest) -> Offer {
        if !self.can_install_in_app(manifest) {
            return Offer::DownloadPage;
        }
        if self.target_version.as_deref() == Some(manifest.version.as_str()) {
            self.progress.clone()
        } else {
            Offer::StartDownload
        }
    }

    /// A check found another version (or none): a package staged for a
    /// different one is thrown away.
    pub fn discard_staged_package(&mut self, other_than: Option<&str>) {
        if other_than.is_some() && self.target_version.as_deref() == other_than {
            return;
        }
        self.reset();
    }

    /// Stale packages from earlier runs, removed at launch — except the
    /// one for `keep` (a headless check that just found that version). A
    /// folder Windows refuses to remove (an installer still running from it)
    /// is left alone.
    pub fn remove_staged_packages(&self, keep: Option<&str>) {
        if let Some(staging) = &self.staging_directory {
            remove_staged_packages_other_than(staging, keep);
        }
    }

    pub fn start_download(&mut self, manifest: &UpdateManifest) {
        let (Some(package_url), Some(identity), Some(staging)) = (
            manifest.package_url.clone(),
            self.running_identity.clone(),
            self.staging_directory.clone(),
        ) else {
            return;
        };
        match self.offer(manifest) {
            Offer::Downloading | Offer::Install(_) | Offer::InstallerOpenFailed(_) => return,
            Offer::DownloadPage
            | Offer::StartDownload
            | Offer::DownloadFailed
            | Offer::PackageRejected => {}
        }
        self.reset();
        self.target_version = Some(manifest.version.clone());
        self.progress = Offer::Downloading;
        let download = self.current_download;
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        let downloader = Arc::clone(&self.downloader);
        let version = manifest.version.clone();
        std::thread::spawn(move || {
            let outcome = run_download(&*downloader, &staging, &version, &package_url, &identity);
            // Nobody listening (a re-check moved on): the package this
            // download produced is the worker's to remove.
            if let Err(mpsc::SendError((
                _,
                DownloadOutcome::Verified(package) | DownloadOutcome::Rejected(package),
            ))) = sender.send((download, outcome))
            {
                discard(&package);
            }
        });
    }

    /// Collects a finished download; answers whether the offer changed.
    pub fn poll(&mut self) -> bool {
        let Some(receiver) = &self.receiver else {
            return false;
        };
        let (download, outcome) = match receiver.try_recv() {
            Ok(answer) => answer,
            Err(mpsc::TryRecvError::Empty) => return false,
            Err(mpsc::TryRecvError::Disconnected) => {
                (self.current_download, DownloadOutcome::Failed(None))
            }
        };
        self.receiver = None;
        if download != self.current_download {
            if let DownloadOutcome::Verified(package) | DownloadOutcome::Rejected(package) = outcome
            {
                discard(&package);
            }
            return false;
        }
        self.progress = match outcome {
            DownloadOutcome::Verified(package) => Offer::Install(package),
            DownloadOutcome::Rejected(package) => {
                discard(&package);
                Offer::PackageRejected
            }
            DownloadOutcome::Failed(package) => {
                if let Some(package) = package {
                    discard(&package);
                }
                Offer::DownloadFailed
            }
        };
        true
    }

    pub fn is_downloading(&self) -> bool {
        self.receiver.is_some()
    }

    /// The second press: opens the installer the user asked for.
    pub fn install(&mut self) {
        let Some(package) = self.progress.staged_package().map(Path::to_path_buf) else {
            return;
        };
        let opened = taigi_windows_platform::open_url(&package.to_string_lossy());
        self.progress = if opened {
            Offer::Install(package)
        } else {
            Offer::InstallerOpenFailed(package)
        };
    }

    fn reset(&mut self) {
        if let Some(staged) = self.progress.staged_package() {
            discard(staged);
        }
        self.progress = Offer::StartDownload;
        self.target_version = None;
        self.current_download += 1;
        self.receiver = None;
    }
}

/// The package lands in its own folder (`<staging>\<uuid>\<version>.exe`),
/// so discarding it is removing the folder.
fn stage(staging: &Path, version: &str) -> Result<PathBuf, std::io::Error> {
    let folder = staging.join(uuid::Uuid::new_v4().to_string().to_uppercase());
    std::fs::create_dir_all(&folder)?;
    Ok(folder.join(format!("{version}.exe")))
}

fn discard(package: &Path) {
    if let Some(folder) = package.parent() {
        std::fs::remove_dir_all(folder).ok();
    }
}

/// Every staged folder under `staging` except one holding `<keep>.exe`.
/// Shared with the headless check, which has no `UpdateInstallation`.
pub fn remove_staged_packages_other_than(staging: &Path, keep: Option<&str>) {
    let Ok(entries) = std::fs::read_dir(staging) else {
        return;
    };
    let kept_name = keep.map(|version| format!("{version}.exe"));
    for entry in entries.flatten() {
        let folder = entry.path();
        let holds_kept = kept_name
            .as_deref()
            .is_some_and(|name| folder.join(name).is_file());
        if !holds_kept {
            std::fs::remove_dir_all(&folder).ok();
        }
    }
}

/// The staging folder under a per-user local data directory.
pub fn staging_directory(local_data_directory: &Path) -> PathBuf {
    local_data_directory.join(STAGING_FOLDER)
}

fn run_download(
    downloader: &dyn PackageDownloader,
    staging: &Path,
    version: &str,
    package_url: &str,
    identity: &PackageIdentity,
) -> DownloadOutcome {
    let package = match stage(staging, version) {
        Ok(package) => package,
        Err(error) => {
            log::debug!("update.staging_failed error={error}");
            return DownloadOutcome::Failed(None);
        }
    };
    if let Err(error) = downloader.download(package_url, &package) {
        log::debug!("update.download_failed error={error}");
        return DownloadOutcome::Failed(Some(package));
    }
    match verify::verify(&package, identity, version) {
        Ok(()) => DownloadOutcome::Verified(package),
        Err(rejection) => {
            log::debug!("update.package_refused rejection={rejection:?}");
            DownloadOutcome::Rejected(package)
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_offer_pairs_its_button_with_its_note_and_only_a_download_shows_neither() {
        // trace: the pending row draws `action_key`'s button, or a spinner
        // when there is none, and `note_key`'s line under it. A download in
        // flight is the one state with nothing to press.
        let staged = std::path::PathBuf::from("staged.exe");
        for offer in [
            Offer::DownloadPage,
            Offer::StartDownload,
            Offer::DownloadFailed,
            Offer::PackageRejected,
            Offer::Install(staged.clone()),
            Offer::InstallerOpenFailed(staged),
        ] {
            assert!(
                offer.action_key().is_some(),
                "{offer:?} leaves the user nothing to press"
            );
        }
        assert_eq!(Offer::Downloading.action_key(), None);
        assert_eq!(Offer::Downloading.note_key(), None);
    }

    use super::*;
    use crate::transport::FetchError;

    struct Refusing;

    impl PackageDownloader for Refusing {
        fn download(&self, _url: &str, _destination: &Path) -> Result<(), FetchError> {
            Err(FetchError::Rejected)
        }
    }

    fn manifest(version: &str, package: Option<&str>) -> UpdateManifest {
        UpdateManifest {
            version: version.to_owned(),
            download_page_url: "https://taigikeyboard.tw/".to_owned(),
            package_url: package.map(str::to_owned),
        }
    }

    fn identity() -> PackageIdentity {
        PackageIdentity {
            signer_thumbprint: vec![1, 2, 3],
            product_name: "Taigi Keyboard".to_owned(),
        }
    }

    #[test]
    fn without_a_package_or_a_signed_running_copy_the_offer_is_the_download_page() {
        // trace: canInstallInApp = packageURL != nil && expectedIdentity != nil.
        let staging = tempfile::tempdir().unwrap();
        let signed = UpdateInstallation::new(
            Arc::new(Refusing),
            Some(identity()),
            Some(staging.path().to_path_buf()),
        );
        assert_eq!(signed.offer(&manifest("3.7.0", None)), Offer::DownloadPage);
        assert_eq!(
            signed.offer(&manifest("3.7.0", Some("https://x/y.exe"))),
            Offer::StartDownload
        );
        let unsigned =
            UpdateInstallation::new(Arc::new(Refusing), None, Some(staging.path().to_path_buf()));
        assert_eq!(
            unsigned.offer(&manifest("3.7.0", Some("https://x/y.exe"))),
            Offer::DownloadPage
        );
    }

    #[test]
    fn a_refused_download_lands_on_download_failed_and_a_re_check_resets_it() {
        let staging = tempfile::tempdir().unwrap();
        let mut installation = UpdateInstallation::new(
            Arc::new(Refusing),
            Some(identity()),
            Some(staging.path().to_path_buf()),
        );
        let manifest = manifest("3.7.0", Some("https://x/y.exe"));
        installation.start_download(&manifest);
        assert_eq!(installation.offer(&manifest), Offer::Downloading);
        while !installation.poll() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(installation.offer(&manifest), Offer::DownloadFailed);
        assert_eq!(
            Offer::DownloadFailed.note_key(),
            Some(StringKey::DesktopUpdateInstallFailedNote)
        );
        // The staged folder is gone with the failure.
        assert_eq!(
            std::fs::read_dir(staging.path().join(STAGING_FOLDER))
                .map(|d| d.count())
                .unwrap_or(0),
            0
        );
        installation.discard_staged_package(Some("3.7.0"));
        assert_eq!(
            installation.offer(&manifest),
            Offer::DownloadFailed,
            "same version keeps its state"
        );
        installation.discard_staged_package(Some("3.8.0"));
        assert_eq!(installation.offer(&manifest), Offer::StartDownload);
    }
}
