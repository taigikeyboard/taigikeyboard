//! The update check the Windows settings window runs (Linux has none: the
//! distribution's package manager updates an input method): the
//! published manifest and its version, what a check concludes and leaves in
//! `settings.json`, and the network behind two traits with a `ureq`
//! implementation. Port of the macOS `UpdateChecker`; the schedule lives in
//! `taigi_desktop_core::settings::update_schedule` so the input-method
//! processes can read it without linking this crate's TLS stack.
//!
//! What triggers a check, how its answer is shown and what installs a
//! package are each platform's: `taigi-windows-update` keeps the Windows
//! download, Authenticode verification and toast.

pub mod checker;
pub mod manifest;
pub mod manual;
pub mod scheduled;
pub mod transport;

pub use checker::Outcome;
pub use manifest::{DottedVersion, ManifestError, PublishedPackage, UpdateManifest};
pub use manual::ManualOutcome;
pub use scheduled::{run_scheduled_check, ScheduledCheck};
pub use transport::{FetchError, HttpTransport, ManifestFetcher, PackageDownloader};
