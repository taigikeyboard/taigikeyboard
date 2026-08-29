//! Opens the settings window: a separate process (`TaigiKeyboardSettings.exe`
//! beside the DLL), because a window cannot live inside every host process
//! the TIP is loaded into (roadmap W1; rakukan `settings_launcher.rs`).
//! The command line is the contract PR7 implements.

// 中文: 設定視窗是另一個程序;這裡只負責用約定的命令列啟動它。

use crate::module::install_directory;
use std::path::PathBuf;
use std::process::Command;
use taigi_windows_core::settings::launch::{CHECK_NOW_FLAG, PANE_FLAG, SETTINGS_EXE_NAME};
use taigi_windows_core::settings::{SettingChoice, SettingsPane};

pub fn settings_exe_path() -> Option<PathBuf> {
    install_directory().map(|directory| directory.join(SETTINGS_EXE_NAME))
}

/// The settings window on whichever pane the user left it on.
pub fn open_settings() {
    launch(&[]);
}

/// The window on 一般 with an update check running — where the answer lands.
pub fn check_for_updates() {
    launch(&[PANE_FLAG, SettingsPane::General.raw(), CHECK_NOW_FLAG]);
}

fn launch(arguments: &[&str]) {
    let Some(exe) = settings_exe_path() else {
        log::error!("settings.launch_failed reason=no_install_directory");
        return;
    };
    match Command::new(&exe)
        .args(arguments)
        .current_dir(exe.parent().unwrap_or(&exe))
        .spawn()
    {
        Ok(_) => log::info!("settings.launched args={arguments:?}"),
        Err(error) => log::error!(
            "settings.launch_failed path={} error={error}",
            exe.display()
        ),
    }
}
