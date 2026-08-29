//! Where this input method keeps what it learns from the user:
//! `%APPDATA%\TaigiKeyboard` (roaming, so learned rankings follow a domain
//! profile — the one place Windows offers for per-user data that is not a
//! cache). Port of `Storage/UserDataDirectory.swift`.

// 中文: 使用者資料目錄 — %APPDATA%\TaigiKeyboard,不存在就建立。

use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum DirectoryError {
    /// `%APPDATA%` is not set — a host running without a user profile (an
    /// AppContainer, a service). The stores then run as `NoStores`.
    #[error("APPDATA is not set; no per-user directory to keep learning data in")]
    NoAppData,
    #[error("could not create {path}: {source}")]
    Create {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// The application's folder name under `%APPDATA%`. Also what the installer
/// and the settings window agree on.
pub const APPLICATION_FOLDER_NAME: &str = "TaigiKeyboard";

/// The shipped location, created if absent.
pub fn user_data_directory() -> Result<PathBuf, DirectoryError> {
    let app_data = std::env::var_os("APPDATA").ok_or(DirectoryError::NoAppData)?;
    created(Path::new(&app_data).join(APPLICATION_FOLDER_NAME))
}

/// Creates `directory` if it does not exist and returns it. Split out so a
/// test can hand the stores a temporary directory without going near the
/// user's real one.
pub fn created(directory: PathBuf) -> Result<PathBuf, DirectoryError> {
    std::fs::create_dir_all(&directory).map_err(|source| DirectoryError::Create {
        path: directory.clone(),
        source,
    })?;
    Ok(directory)
}
