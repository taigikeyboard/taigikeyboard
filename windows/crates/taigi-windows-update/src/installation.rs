//! The two-stage install: a finished download turns the 一般 pane's button
//! into 安裝 rather than launching the installer on its own, so nothing
//! takes the focus away from a document the user may have gone back to
//! typing in. Port of `UpdateInstallation` + `UpdatePackageDownload`. The
//! download and the verification run on a thread; the pane polls. What a
//! downloaded package has to prove is `verify::Admission`'s business, not
//! this module's.

// 兩段式安裝狀態機 — 下載+驗證在背景,按鈕變成「安裝」後才由使用者啟動安裝程式。

use crate::manifest::{PublishedPackage, UpdateManifest};
use crate::transport::PackageDownloader;
use crate::verify::Admission;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc};
use taigi_windows_core::strings::StringKey;

/// The staging folder, under the per-user local (non-roaming) data.
const STAGING_FOLDER: &str = "Updates";

/// What the pending-update row offers next (`UpdateInstallation.Offer`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Offer {
    /// No in-app install (the manifest names no package, or there is
    /// nowhere to stage one): the download page, in the browser.
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

impl DownloadOutcome {
    /// What this outcome left on disk, so no caller has to keep its own
    /// list of which ones did.
    fn staged_package(self) -> Option<PathBuf> {
        match self {
            Self::Verified(package) | Self::Rejected(package) | Self::Failed(Some(package)) => {
                Some(package)
            }
            Self::Failed(None) => None,
        }
    }
}

pub struct UpdateInstallation {
    progress: Offer,
    target_version: Option<String>,
    /// Which download the state belongs to: a result from an older one
    /// (a re-check moved the version) is discarded on arrival.
    current_download: u64,
    receiver: Option<mpsc::Receiver<(u64, DownloadOutcome)>>,
    downloader: Arc<dyn PackageDownloader>,
    /// Everything a staged package must clear, held opaque: this module
    /// decides WHEN to verify, never what "verified" means.
    admission: Arc<Admission>,
    /// Where packages are staged; `None` (no non-roaming per-user folder)
    /// ⇒ download page only, never a roaming stage.
    staging_directory: Option<PathBuf>,
}

impl UpdateInstallation {
    pub fn new(
        downloader: Arc<dyn PackageDownloader>,
        admission: Arc<Admission>,
        local_data_directory: Option<PathBuf>,
    ) -> Self {
        Self {
            progress: Offer::StartDownload,
            target_version: None,
            current_download: 0,
            receiver: None,
            downloader,
            admission,
            staging_directory: local_data_directory.map(|local| local.join(STAGING_FOLDER)),
        }
    }

    /// Whether this copy can download and install by itself: a package the
    /// manifest named, and somewhere to stage it. Whether the running copy
    /// is SIGNED is not among these — that decides which checks a staged
    /// package faces, not whether one may be fetched.
    pub fn can_install_in_app(&self, manifest: &UpdateManifest) -> bool {
        self.installable_parts(manifest).is_some()
    }

    /// The same question, answered with what a download needs.
    fn installable_parts(&self, manifest: &UpdateManifest) -> Option<(PublishedPackage, PathBuf)> {
        Some((manifest.package.clone()?, self.staging_directory.clone()?))
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
        let Some((package, staging)) = self.installable_parts(manifest) else {
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
        let admission = Arc::clone(&self.admission);
        let version = manifest.version.clone();
        std::thread::spawn(move || {
            let outcome = run_download(&*downloader, &admission, &staging, &version, &package);
            // Nobody listening (a re-check moved on): whatever this download
            // left on disk is the worker's to remove.
            if let Err(mpsc::SendError((_, outcome))) = sender.send((download, outcome)) {
                if let Some(package) = outcome.staged_package() {
                    discard(&package);
                }
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
            if let Some(package) = outcome.staged_package() {
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
    admission: &Admission,
    staging: &Path,
    version: &str,
    published: &PublishedPackage,
) -> DownloadOutcome {
    let package = match stage(staging, version) {
        Ok(package) => package,
        Err(error) => {
            log::debug!("update.staging_failed error={error}");
            return DownloadOutcome::Failed(None);
        }
    };
    if let Err(error) = downloader.download(&published.url, &package) {
        log::debug!("update.download_failed error={error}");
        return DownloadOutcome::Failed(Some(package));
    }
    match admission.admit(&package, &published.sha256, version) {
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

    /// Writes `STAGED_BODY` wherever it is asked to stage.
    struct Writing;

    impl PackageDownloader for Writing {
        fn download(&self, _url: &str, destination: &Path) -> Result<(), FetchError> {
            std::fs::write(destination, STAGED_BODY)
                .map_err(|error| FetchError::Transport(error.to_string()))
        }
    }

    const STAGED_BODY: &str = "an installer";
    /// `sha256sum` of `STAGED_BODY`, written down rather than computed so the
    /// fixture states what it expects. The encoding it is written in is
    /// pinned against an external oracle by
    /// `verify::tests::an_empty_file_hashes_to_the_published_sha256_of_nothing`.
    const STAGED_BODY_SHA256: &str =
        "b2b3a11c02f14e36f2a5c148db1b5924fa96141da4b2fbfb262a469df53f6750";

    fn manifest(version: &str, package: Option<&str>) -> UpdateManifest {
        manifest_with_digest(
            version,
            package,
            package.map(|_| STAGED_BODY_SHA256.to_owned()),
        )
    }

    fn manifest_with_digest(
        version: &str,
        package: Option<&str>,
        digest: Option<String>,
    ) -> UpdateManifest {
        UpdateManifest {
            version: version.to_owned(),
            download_page_url: "https://taigikeyboard.tw/".to_owned(),
            package: package.zip(digest).map(|(url, sha256)| PublishedPackage {
                url: url.to_owned(),
                sha256,
            }),
        }
    }

    fn unsigned() -> Arc<Admission> {
        Arc::new(Admission::unsigned())
    }

    #[test]
    fn a_published_package_and_somewhere_to_stage_it_admit_an_in_app_install() {
        // trace: can_install_in_app = manifest.package && staging. The
        // running copy's signature is deliberately NOT a term — an unsigned
        // release has none and still installs in-app; what its absence
        // changes is only which checks `Admission::admit` runs.
        let staging = tempfile::tempdir().unwrap();
        let installation = UpdateInstallation::new(
            Arc::new(Refusing),
            unsigned(),
            Some(staging.path().to_path_buf()),
        );
        assert_eq!(
            installation.offer(&manifest("3.7.0", Some("https://x/y.exe"))),
            Offer::StartDownload
        );
        // Half a package is no package: either half missing is the download
        // page, decided once at the wire boundary.
        for incomplete in [
            manifest("3.7.0", None),
            manifest_with_digest("3.7.0", Some("https://x/y.exe"), None),
        ] {
            assert_eq!(installation.offer(&incomplete), Offer::DownloadPage);
        }
        let nowhere_to_stage = UpdateInstallation::new(Arc::new(Refusing), unsigned(), None);
        assert_eq!(
            nowhere_to_stage.offer(&manifest("3.7.0", Some("https://x/y.exe"))),
            Offer::DownloadPage
        );
    }

    #[test]
    fn the_digest_of_the_staged_bytes_is_what_admits_it() {
        // trace: run_download hashes the staged file and `Admission::admit`
        // compares it to the manifest's; an unsigned host copy runs no
        // Authenticode check, so this is the whole bar here.
        let staging = tempfile::tempdir().unwrap();
        let mut installation = UpdateInstallation::new(
            Arc::new(Writing),
            unsigned(),
            Some(staging.path().to_path_buf()),
        );
        let matching = manifest("3.7.0", Some("https://x/y.exe"));
        installation.start_download(&matching);
        while !installation.poll() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(
            matches!(installation.offer(&matching), Offer::Install(_)),
            "the published digest admits the package"
        );

        let wrong = manifest_with_digest("3.8.0", Some("https://x/y.exe"), Some("b".repeat(64)));
        installation.start_download(&wrong);
        while !installation.poll() {
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert_eq!(
            installation.offer(&wrong),
            Offer::PackageRejected,
            "another file than the one published is refused"
        );
        assert_eq!(
            Offer::PackageRejected.action_key(),
            Some(StringKey::DesktopUpdateDownloadAction),
            "a refused package leaves the download page as the way out"
        );
    }

    #[test]
    fn a_refused_download_lands_on_download_failed_and_a_re_check_resets_it() {
        let staging = tempfile::tempdir().unwrap();
        let mut installation = UpdateInstallation::new(
            Arc::new(Refusing),
            unsigned(),
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
