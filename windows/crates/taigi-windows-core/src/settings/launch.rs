//! The command line between the DLL and the settings window: one place
//! both sides read, so the spawner (`taigi-windows-tsf::settings_launcher`)
//! and the parser (`taigi-windows-settings::cli`) cannot drift.

// 中文: DLL 與設定視窗之間的命令列契約 — 兩邊共用同一份常數。

/// The settings window's executable, beside the DLL in the install
/// directory (roadmap W1).
pub const SETTINGS_EXE_NAME: &str = "TaigiKeyboardSettings.exe";
/// `--pane <raw>`: open on this pane (absent = where the user left it).
pub const PANE_FLAG: &str = "--pane";
/// `--check-now`: run an update check with the window up (the menu's
/// 檢查更新 row, `TaigiInputController.swift:375-383`).
pub const CHECK_NOW_FLAG: &str = "--check-now";
/// `--check-updates`: the headless check the scheduled task runs (W9);
/// no window.
pub const CHECK_UPDATES_FLAG: &str = "--check-updates";
/// `--prewarm`: map the Windows App Runtime this exe's window is built on
/// and exit. No window, no settings write, no stores — the process is
/// gone in about a second, and the images it mapped stay cached for the
/// launch that really opens the window (measured: a cold first open drops
/// from ~2.5 s to ~1.1 s to painted content).
pub const PREWARM_FLAG: &str = "--prewarm";
