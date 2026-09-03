//! Opens the settings window: a separate process (`TaigiKeyboardSettings.exe`
//! beside the DLL), because a window cannot live inside every host process
//! the TIP is loaded into (roadmap W1; rakukan `settings_launcher.rs`).
//! The command line is the contract PR7 implements.

// 中文: 設定視窗是另一個程序;這裡只負責用約定的命令列啟動它。

use crate::module::install_directory;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use taigi_windows_core::settings::launch::{
    CHECK_NOW_FLAG, PANE_FLAG, PREWARM_FLAG, SETTINGS_EXE_NAME,
};
use taigi_windows_core::settings::{SettingChoice, SettingsPane};
use windows::Win32::System::Threading::BELOW_NORMAL_PRIORITY_CLASS;

/// The claim that keeps the many hosts this DLL is loaded into from each
/// spawning a prewarm of their own.
const PREWARM_CLAIM_NAME: &str = "Local\\TaigiKeyboardSettingsPrewarm";

/// What the spawned process runs at.
#[derive(Clone, Copy)]
enum Priority {
    /// This host's own class — what a window the user just asked for
    /// deserves.
    Inherit,
    /// Off the user's path: the prewarm must never take CPU from the
    /// typing that just activated us. Not background mode
    /// (`PROCESS_MODE_BACKGROUND_BEGIN`), which throttles I/O too — and
    /// this work IS the I/O, so it would still be running when the user
    /// reaches for 設定.
    BelowNormal,
}

impl Priority {
    fn creation_flags(self) -> u32 {
        match self {
            Self::Inherit => 0,
            Self::BelowNormal => BELOW_NORMAL_PRIORITY_CLASS.0,
        }
    }
}

pub fn settings_exe_path() -> Option<PathBuf> {
    install_directory().map(|directory| directory.join(SETTINGS_EXE_NAME))
}

/// The settings window on whichever pane the user left it on.
pub fn open_settings() {
    launch(&[], Priority::Inherit);
}

/// The window on 一般 with an update check running — where the answer lands.
pub fn check_for_updates() {
    launch(
        &[PANE_FLAG, SettingsPane::General.raw(), CHECK_NOW_FLAG],
        Priority::Inherit,
    );
}

/// Maps the settings window's WinUI runtime ahead of the first open, in a
/// process that exits as soon as it has (`taigi-windows-settings::prewarm`
/// says what that buys).
///
/// Best-effort: the claim dies with the host that took it, so a later host
/// may prewarm again against an already-mapped runtime. Called from
/// `Activate`, so it may not block — `CreateProcessW` runs every
/// process-creation callback the machine's security software has installed,
/// and that lands on the host's UI thread the moment the user switched to
/// this input method. The spawn goes to a thread of its own; only the
/// claim, which is one syscall, stays inline.
pub fn prewarm_once() {
    // The claim is a kernel object shared across hosts; this flag is this
    // process asking itself, so the second and later activations in a host
    // cost an atomic load instead of a syscall and a thread.
    static PREWARM_ATTEMPTED: AtomicBool = AtomicBool::new(false);
    if PREWARM_ATTEMPTED.swap(true, Ordering::AcqRel) {
        return;
    }
    if !taigi_windows_platform::acquire_named_claim(PREWARM_CLAIM_NAME) {
        return;
    }
    if let Err(error) = thread::Builder::new()
        .name("taigi-settings-prewarm".into())
        .spawn(|| launch(&[PREWARM_FLAG], Priority::BelowNormal))
    {
        log::warn!("settings.prewarm_thread_failed error={error}");
    }
}

fn launch(arguments: &[&str], priority: Priority) {
    use std::os::windows::process::CommandExt;
    let Some(exe) = settings_exe_path() else {
        log::error!("settings.launch_failed reason=no_install_directory");
        return;
    };
    match Command::new(&exe)
        .args(arguments)
        .current_dir(exe.parent().unwrap_or(&exe))
        .creation_flags(priority.creation_flags())
        .spawn()
    {
        Ok(_) => log::info!("settings.launched args={arguments:?}"),
        Err(error) => log::error!(
            "settings.launch_failed path={} error={error}",
            exe.display()
        ),
    }
}
