//! The few Linux-specific pieces the IBus engine and the settings window
//! share, all pure (no D-Bus, no GTK): where the user's data lives (XDG),
//! where the install put the dictionaries, how a framework key event becomes the
//! desktop core's [`taigi_desktop_core::keys::KeyEventSnapshot`], how the
//! engine opens the settings window, and which language the UI draws in.
//!
//! Counterpart of `windows/crates/taigi-windows-platform`, minus everything
//! that needed a Win32 handle. Design record:
//! `docs/architecture/linux-roadmap.md` (L5, L7, L8).

pub mod key_translation;
pub mod launcher;
pub mod locale;
pub mod paths;

pub use key_translation::{snapshot, KeyState, RawKeyEvent};
pub use launcher::{open_settings, open_url};
pub use locale::system_locale;
pub use paths::{
    config_directory, data_directory, dictionaries_directory, install_prefix, settings_binary,
    InstallLayout, UserDirectories,
};

/// A debug-build logger to stderr (`RUST_LOG`, default `info`) — what the
/// dev loop (`make run-engine`) reads. Release builds install nothing: no
/// log leaves a release build (`security-rules.md`; Windows twin
/// `taigi-windows-platform::install_debug_logger`).
#[cfg(debug_assertions)]
pub fn install_debug_logger() {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .try_init();
}

#[cfg(not(debug_assertions))]
pub fn install_debug_logger() {}
