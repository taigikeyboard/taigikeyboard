//! Update check, download, verification and the two-stage install offer,
//! port of the macOS `UpdateChecker` / `UpdateInstallation` /
//! `UpdatePackageVerifier` (roadmap W9). The decisions — what the manifest
//! means, when a check is due, what the 一般 pane offers next — are pure and
//! host-tested; the network is behind two traits with a `ureq`
//! implementation; the Windows-only parts (Authenticode, VERSIONINFO, the
//! toast) live in their own modules with host stubs.
//!
//! What the check is triggered by is NOT in here (Codex W9): the installer's
//! scheduled task runs `--check-updates`, the window checks when
//! overdue at launch, the user checks from the 一般 pane or the lang-bar
//! menu, and the DLL only ever READS the pending manifest.

pub mod checker;
pub mod installation;
pub mod manifest;
pub mod toast;
pub mod transport;
pub mod verify;

pub use checker::{Outcome, CHECK_INTERVAL_MS};
pub use installation::{Offer, UpdateInstallation};
pub use manifest::{DottedVersion, ManifestError, PublishedPackage, UpdateManifest, PUBLISHED_URL};
pub use transport::{FetchError, HttpTransport, ManifestFetcher, PackageDownloader};
pub use verify::{Admission, PackageIdentity, Rejection};
