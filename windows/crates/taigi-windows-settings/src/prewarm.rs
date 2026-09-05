//! `--prewarm`: map the Windows App Runtime beside this exe and exit.
//!
//! The settings window is a WinUI 3 app over a self-contained Windows App
//! Runtime (`build.rs`), and a first open on a machine that has not run it
//! yet spends its time in the loader, not in our code. Measured on the
//! Windows box (spawn → the first painted control): ~2.5-3.1 s with
//! nothing cached, ~1.0-1.2 s when a process mapped these images first and
//! then EXITED, ~0.3 s once a real window has run. Reading the files
//! instead caps out at ~1.4 s — the image sections are what carries.
//!
//! So this mode is not a window: no `settings.json` write, no user-data
//! stores, no single-instance claim (the real window may be open). It maps
//! the runtime and returns.
//!
//! The remaining ~0.8 s to a warm launch is state only a real WinUI window
//! builds (XAML resource caches, the D3D device, the DWM connection), and
//! `windows-reactor` cannot create a window without showing it — so this
//! mode deliberately buys the loader half and nothing else.

/// The runtime images the window maps from its own directory, spelled as
/// the staging list spells them (dumped from a live settings process on the
/// Windows box, 2026-09-04: 19 modules came from the install directory, one
/// of them the exe itself). The shipped runtime holds more —
/// `Microsoft.Web.WebView2.Core.dll` and thirteen others are staged but
/// never mapped — and loading a DLL the window does not use would run a
/// `DllMain` for nothing, so this is the mapped set, not the staged one.
///
/// This snapshot belongs to one `windows-reactor` revision (the tests'
/// `SNAPSHOT_REACTOR_REVISION`): a bump changes which images the window
/// maps, and `the_snapshot_belongs_to_the_pinned_reactor` goes red until
/// the bump's round re-dumps the module list on the box and updates both.
const RUNTIME_IMAGES: &[&str] = &[
    "microsoft.ui.xaml.dll",
    "microsoft.ui.xaml.controls.dll",
    "microsoft.ui.xaml.phone.dll",
    "microsoft.ui.input.dll",
    "microsoft.ui.windowing.dll",
    "microsoft.ui.windowing.core.dll",
    "microsoft.ui.composition.ossupport.dll",
    "microsoft.internal.frameworkudk.dll",
    "microsoft.directmanipulation.dll",
    "microsoft.inputstatemanager.dll",
    "microsoft.windows.applicationmodel.resources.dll",
    "coremessagingxp.dll",
    "winuiedit.dll",
    "dcompi.dll",
    "dwmcorei.dll",
    "wuceffectsi.dll",
    "marshal.dll",
    "mrm.dll",
];

/// Maps every runtime image beside this exe. A failed load is logged with
/// its path and the rest still go — a prewarm that gets most of the runtime
/// in is worth more than one that gives up on the first miss.
pub fn run() {
    let Some(directory) = taigi_windows_platform::executable_directory() else {
        log::error!("prewarm.no_executable_directory");
        return;
    };
    for name in RUNTIME_IMAGES {
        taigi_windows_platform::preload_library(&directory.join(name));
    }
    log::info!("prewarm.done count={}", RUNTIME_IMAGES.len());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    /// The staging list `scripts/release-app.sh` copies into the installer.
    const SHIPPED_RUNTIME_FILES: &str =
        include_str!("../../../build-support/windows-app-runtime-files.txt");
    /// The workspace manifest, for the pin the snapshot belongs to.
    const WORKSPACE_MANIFEST: &str = include_str!("../../../Cargo.toml");
    /// The `windows-reactor` commit `RUNTIME_IMAGES` was dumped against.
    const SNAPSHOT_REACTOR_REVISION: &str = "dc720b3674c46ceb82d758ed20959977b32e60a9";

    /// Every name we map has to be one the installer puts beside the exe —
    /// otherwise the prewarm loads a path that does not exist on a user's
    /// machine, and the mode quietly stops paying.
    #[test]
    fn every_image_is_shipped_and_listed_once() {
        let shipped: BTreeSet<&str> = SHIPPED_RUNTIME_FILES
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
            .collect();
        for name in RUNTIME_IMAGES {
            assert!(
                shipped.contains(name),
                "{name} is not in build-support/windows-app-runtime-files.txt — \
                 the installer would not put it beside the exe"
            );
        }
        let distinct: BTreeSet<&&str> = RUNTIME_IMAGES.iter().collect();
        assert_eq!(
            distinct.len(),
            RUNTIME_IMAGES.len(),
            "an image is listed twice"
        );
    }

    /// The list is a dump from a live window, and only a live window can
    /// produce another — which `make check-box` cannot do (session 0 kills
    /// WinUI). So the pin is what the gate can hold: move the reactor and
    /// this test says the snapshot is owed a re-dump on the box.
    #[test]
    fn the_snapshot_belongs_to_the_pinned_reactor() {
        assert!(
            WORKSPACE_MANIFEST.contains(SNAPSHOT_REACTOR_REVISION),
            "windows/Cargo.toml no longer pins windows-reactor at \
             {SNAPSHOT_REACTOR_REVISION}: re-dump the settings window's \
             install-directory modules on the Windows box, update \
             RUNTIME_IMAGES, then update this revision"
        );
    }
}
