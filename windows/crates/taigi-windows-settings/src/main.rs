//! `TaigiKeyboardSettings.exe`: the settings window the DLL spawns
//! (`settings_launcher`), a WinUI 3 port of the macOS settings window —
//! same panes, same controls, same `settings.json` the TIP live-reads
//! (roadmap W1 / W10 / W17). The chrome is WinUI's; the information
//! architecture is `SettingsSplitView.swift`'s.

// 中文: 設定視窗主程式 — 讀命令列、開 settings.json、開 WinUI 視窗。

#![cfg_attr(windows, windows_subsystem = "windows")]

mod cli;
mod presentation;
mod prewarm;
mod search;
mod settings_writer;
mod updates;
mod user_data;
#[cfg(windows)]
mod winui;
mod work;

use cli::LaunchOptions;
use presentation::strings_for;
use std::process::ExitCode;
use taigi_windows_core::settings::SettingsPane;
use taigi_windows_storage::{user_data_directory, LiveSettings, SettingsFileStore};

/// The per-session mutex that makes the window single-instance.
const SINGLE_INSTANCE_NAME: &str = "Local\\TaigiKeyboardSettings";

fn main() -> ExitCode {
    taigi_windows_platform::install_debug_logger();
    let launch = LaunchOptions::parse(std::env::args().skip(1));
    // Before everything, including the single-instance claim: a prewarm
    // maps the runtime and leaves, and it must be able to do that while
    // the real window is open.
    if launch.prewarm {
        prewarm::run();
        return ExitCode::SUCCESS;
    }
    if launch.headless_check {
        headless_check();
        return ExitCode::SUCCESS;
    }
    // One window per user: a second launch (the menu row pressed twice)
    // exits — two windows would each own a download stage and a recorder.
    if !taigi_windows_platform::acquire_named_claim(SINGLE_INSTANCE_NAME) {
        log::info!("settings.already_running");
        return ExitCode::SUCCESS;
    }
    // No per-user directory (`%APPDATA%` unset): the window opens on the
    // defaults and refuses every write, saying so. Never a file somewhere
    // else — the DLL would not read it, and a save that "worked" into a
    // temp folder would be a lie (roadmap W2).
    let (directory, is_read_only) = match user_data_directory() {
        Ok(directory) => (directory, false),
        Err(error) => {
            log::error!("settings.no_user_data_directory error={error}");
            (std::env::temp_dir(), true)
        }
    };
    let live = LiveSettings::new(SettingsFileStore::new(&directory));
    let document = live.refresh_if_changed();
    let pane: SettingsPane = launch.pane.unwrap_or_else(|| {
        document.choice(&taigi_windows_core::settings::keys::SELECTED_SETTINGS_PANE)
    });
    drop(document);
    open_window(live, directory, pane, is_read_only, launch.check_now)
}

#[cfg(windows)]
fn open_window(
    live: LiveSettings,
    directory: std::path::PathBuf,
    pane: SettingsPane,
    is_read_only: bool,
    is_check_now: bool,
) -> ExitCode {
    if winui::run(live, directory, pane, is_read_only, is_check_now) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// The macOS host builds this crate to run its tests; there is no window
/// to open there.
#[cfg(not(windows))]
fn open_window(
    _live: LiveSettings,
    _directory: std::path::PathBuf,
    _pane: SettingsPane,
    _is_read_only: bool,
    _is_check_now: bool,
) -> ExitCode {
    log::error!("settings.window_unavailable");
    ExitCode::FAILURE
}

/// `--check-updates`: the scheduled task's daily check (roadmap W9) — no
/// window. Due ⇒ fetch, record the outcome the way the window would, and
/// toast a version not announced before.
fn headless_check() {
    use taigi_windows_update::checker;
    let Ok(directory) = user_data_directory() else {
        return;
    };
    let store = SettingsFileStore::new(&directory);
    let Ok(document) = store.load() else {
        return;
    };
    let now = updates::now_ms();
    if !checker::is_due(&document, now) {
        return;
    }
    if store
        .update(|document| checker::stamp_next_check(document, now))
        .is_err()
    {
        return;
    }
    let outcome = checker::check(
        &taigi_windows_update::HttpTransport,
        updates::INSTALLED_VERSION,
    );
    let Ok(document) = store.update(|document| checker::record(document, &outcome)) else {
        return;
    };
    // The window's side effects, without a window: a package staged for
    // another version goes; the announcement is claimed under the lock.
    let keep = match &outcome {
        checker::Outcome::UpdateAvailable(manifest) => Some(manifest.version.as_str()),
        checker::Outcome::UpToDate => None,
        checker::Outcome::Failed => return,
    };
    if let Some(local) = updates::local_data_directory() {
        let staging = taigi_windows_update::installation::staging_directory(&local);
        taigi_windows_update::installation::remove_staged_packages_other_than(&staging, keep);
    }
    if let checker::Outcome::UpdateAvailable(manifest) = &outcome {
        let version = manifest.version.clone();
        let mut claimed = false;
        if store
            .update(|document| claimed = checker::claim_announcement(document, &version))
            .is_err()
        {
            return;
        }
        if claimed {
            updates::post_toast(&strings_for(&document), manifest);
        }
    }
}
