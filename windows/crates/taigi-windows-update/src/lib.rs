//! The Windows half of updates: the download, verification and two-stage
//! install offer, port of the macOS `UpdateInstallation` /
//! `UpdatePackageVerifier` (roadmap W9), plus the toast. The check itself —
//! manifest, outcome, transport — is `taigi-desktop-update`; the schedule is
//! `taigi_desktop_core::settings::update_schedule`.
//! The Windows-only parts (Authenticode, VERSIONINFO, the toast) live in
//! their own modules with host stubs.
//!
//! What the check is triggered by is NOT in here (Codex W9): the installer's
//! scheduled task runs `--check-updates`, the window checks when
//! overdue at launch, the user checks from the 一般 pane or the lang-bar
//! menu, and the DLL only ever READS the pending manifest.

pub mod installation;
pub mod toast;
pub mod verify;

pub use installation::{Offer, UpdateInstallation};
pub use verify::{Admission, PackageIdentity, Rejection};

/// Where the Windows manifest is published (`windows/updates/README.md`).
/// Compiled into every shipped build; old installs request it forever, so
/// it stays on a domain the project controls.
pub const PUBLISHED_URL: &str = "https://taigikeyboard.tw/appcast/windows.json";
