//! `TaigiKeyboardSettings.exe`: the settings window the DLL spawns
//! (`settings_launcher`), an eframe/egui port of the macOS settings window
//! — same panes, same controls, same `settings.json` the TIP live-reads
//! (roadmap W1 / W10 / W15). The chrome is egui's; the information
//! architecture is `SettingsSplitView.swift`'s.

// 中文: 設定視窗主程式 — 讀命令列、開 settings.json、建 egui 視窗。

#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod cli;
mod fonts;
mod keys;
mod panes;
mod search;
mod theme;
mod updates;
mod widgets;
#[cfg(windows)]
mod winui;
mod work;

use app::SettingsApp;
use cli::LaunchOptions;
use taigi_windows_core::settings::SettingsPane;
use taigi_windows_storage::{user_data_directory, LiveSettings, SettingsFileStore};

/// `SettingsPaneLayout.swift:668-690`: sidebar 215 + detail 545, one width
/// in both directions; the height is the user's, floored.
const WINDOW_WIDTH: f32 = 215.0 + 545.0;
const MINIMUM_HEIGHT: f32 = 470.0;
const INITIAL_HEIGHT: f32 = 560.0;
/// The Mac puts no ceiling on the height; winit wants a number, and no
/// monitor is this tall.
const MAXIMUM_HEIGHT: f32 = 16_384.0;

/// The per-session mutex that makes the window single-instance.
const SINGLE_INSTANCE_NAME: &str = "Local\\TaigiKeyboardSettings";

/// Where the window's own frame is remembered between launches — beside
/// the settings, never inside them (the TIP re-reads `settings.json` on
/// every change, and a drag must not be one).
const WINDOW_STATE_FILE: &str = "settings-window.ron";

fn main() -> eframe::Result {
    taigi_windows_platform::install_debug_logger();
    // W17-A0: the WinUI foundation smoke — a window on the staged runtime,
    // nothing of the app. Goes with the egui entry at the W17-C cutover.
    #[cfg(windows)]
    if std::env::args().any(|argument| argument == winui::SMOKE_FLAG) {
        if !winui::run_smoke() {
            std::process::exit(1);
        }
        return Ok(());
    }
    let launch = LaunchOptions::parse(std::env::args().skip(1));
    if launch.headless_check {
        headless_check();
        return Ok(());
    }
    // One window per user: a second launch (the menu row pressed twice)
    // exits — two windows would each own a download stage and a recorder.
    if !taigi_windows_platform::acquire_single_instance(SINGLE_INSTANCE_NAME) {
        log::info!("settings.already_running");
        return Ok(());
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
    let title = app::pane_title(&app::strings_for(&document), pane);
    let window_state = directory.join(WINDOW_STATE_FILE);
    let directory = directory.clone();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(title)
            .with_inner_size([WINDOW_WIDTH, INITIAL_HEIGHT])
            .with_min_inner_size([WINDOW_WIDTH, MINIMUM_HEIGHT])
            .with_max_inner_size([WINDOW_WIDTH, MAXIMUM_HEIGHT]),
        // Centred the first time only (`window.center()` then the autosaved
        // frame, `SettingsWindowController.swift:659-662`): eframe's
        // `centered` would override a remembered position on every launch.
        centered: !window_state.exists(),
        persist_window: !is_read_only,
        persistence_path: Some(window_state),
        ..Default::default()
    };
    eframe::run_native(
        "TaigiKeyboardSettings",
        options,
        Box::new(move |creation| {
            Ok(Box::new(SettingsApp::new(
                creation,
                live,
                directory,
                pane,
                is_read_only,
                launch.check_now,
            )))
        }),
    )
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
            updates::post_toast(&app::strings_for(&document), manifest);
        }
    }
}
