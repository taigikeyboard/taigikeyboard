//! Every pane's view must PLAN — the regression net for the whole window.
//!
//! The reactor refuses a tree whose shape it cannot realize and answers
//! `PumpError::StructureUnsupported`; what that costs is written out once,
//! on `cards::frame`. Two such trees shipped in W17 because nothing ever
//! mounted these pages, and the crash reaches the user with no message.
//!
//! `RecordingRuntime` is the reactor's headless host, so planning runs with
//! no WinUI runtime and no desktop — exactly the layer both defects were
//! in. **Scope: each pane in its LAUNCH state.** A subtree that only
//! appears once the user has done something — the busy overlay, the entry
//! dialog, a search result row — returns `View::empty()` here and is not
//! covered; so is everything below planning (the Windows App SDK ABI, COM
//! apartments, real layout, theme, focus). Those stay device dogfood.

use super::list_selection::recorded::{insertion_parents, selections};
use super::window::{SettingsWindow, SettingsWindowInput};
use taigi_windows_core::settings::{keys, SettingChoice, SettingsPane};
use taigi_windows_storage::{LiveSettings, SettingsFileStore, UserDataStores};
use tempfile::TempDir;
use windows_reactor::{Pump, RecordingRuntime, View};

/// Far enough ahead that `checker::is_due` says no: a writable launch runs
/// the overdue update check in `create`, and a test must not reach the
/// network.
const NEVER_DUE_MS: i64 = i64::MAX;

/// One settings directory for every mount below: they all want the same
/// stamped defaults, and none of them writes.
fn stamped_directory() -> TempDir {
    let directory = tempfile::tempdir().expect("a temp directory");
    SettingsFileStore::new(directory.path())
        .update(|document| document.set_i64(&keys::UPDATE_NEXT_CHECK_MS, NEVER_DUE_MS))
        .expect("stamp the update check away");
    directory
}

/// Mounts the whole window on `pane` against the headless runtime.
///
/// The stores are handed over unopened — `user_data::open_at_launch`'s job
/// is the launch's, and a test that only plans a view tree must not run its
/// migrations. A closed store answers a query the way a read-only launch's
/// does, and the tree under test is the same.
fn plan(directory: &TempDir, pane: SettingsPane, is_read_only: bool) -> Result<(), String> {
    planned(directory, pane, is_read_only).map(|_| ())
}

/// The same mount, keeping the runtime so a test can read what was applied.
fn planned(
    directory: &TempDir,
    pane: SettingsPane,
    is_read_only: bool,
) -> Result<Pump<RecordingRuntime>, String> {
    let input = SettingsWindowInput::new(
        LiveSettings::new(SettingsFileStore::new(directory.path())),
        UserDataStores::new(directory.path().to_path_buf()),
        is_read_only,
        pane,
        false,
    );
    let mut pump = Pump::new(RecordingRuntime::default());
    pump.mount_view(View::component::<SettingsWindow>(input))
        .map_err(|error| format!("{error:?}"))?;
    Ok(pump)
}

/// No pane's LAUNCH plan may select a row in the batch that inserts it.
///
/// Reactor plans a node's properties before its children, so a
/// `ListViewSelectedIndex` sent in the same batch as the items reaches a list
/// that is still one render behind it. XAML answers `E_INVALIDARG` for an
/// index it does not number, and reactor turns a failed native command into
/// `std::process::abort()` — the settings window died this way the first time
/// a user added a custom typeface (2026-09-11). `winui::list_selection` is
/// what holds the index back, and its own reactor tests drive the handover
/// across renders; this is the launch-state half — conservative on purpose
/// (a batch may insert OTHER rows harmlessly), and blind to everything a user
/// has to do something to reach.
#[test]
fn no_pane_selects_a_row_in_the_render_that_inserts_it() {
    let directory = stamped_directory();
    for pane in SettingsPane::ALL {
        let pump = planned(&directory, *pane, false).expect("the pane plans");
        for (batch_index, batch) in pump.runtime().commands().iter().enumerate() {
            let parents = insertion_parents(batch);
            for (node, index) in selections(batch) {
                assert!(
                    !parents.contains(&node),
                    "pane {} batch {batch_index}: selects index {index} of a list \
                     the same batch fills",
                    pane.raw(),
                );
            }
        }
    }
}

#[test]
fn every_pane_plans_writable_and_read_only() {
    let directory = stamped_directory();
    // Read-only is its own tree, not a cosmetic difference: the window
    // draws a banner and greys the controls (`SettingsWriter`, roadmap W2).
    for is_read_only in [false, true] {
        for pane in SettingsPane::ALL {
            assert_eq!(
                plan(&directory, *pane, is_read_only),
                Ok(()),
                "pane {} does not plan (read_only={is_read_only})",
                pane.raw()
            );
        }
    }
}
