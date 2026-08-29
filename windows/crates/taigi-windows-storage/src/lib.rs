//! What the input method keeps on disk for the user: the three learning
//! databases (`user_frequency.db`, `user_association.db`,
//! `custom_dictionary.db`) and `settings.json`, under `%APPDATA%\TaigiKeyboard`.
//!
//! Port of `macos/Sources/TaigiInputMethodCore/Storage/` over rusqlite. The
//! SQL is byte-identical to the macOS stores (which mirror iOS / Android), so
//! a `.taigi` export or a hand-copied database means the same thing on every
//! platform. User-writable SQLite stays platform-native by policy
//! (`.claude/rules/rust-migration-policy.md` §6). Roadmap W2 / W10; PR4.
//!
//! Host-testable: nothing here touches a Windows API, so the tests run on
//! the Mac against temporary directories. Only `directory::user_data_directory`
//! reads `%APPDATA%`, and it is a pure environment lookup.

// 中文: 使用者資料落地層 — 三個學習資料庫 + settings.json;SQL 與 macOS/iOS/Android 逐字相同。

mod association;
mod capacity;
mod csv;
mod custom_dictionary;
mod database;
mod directory;
mod frequency;
mod settings_file;
mod timestamp;

pub use association::{AssociationRow, UserAssociationStore};
pub use capacity::LearningCapacity;
pub use csv::{CustomDictionaryCSV, CustomDictionaryCSVError, UserDataCSV};
pub use custom_dictionary::{
    CustomDictionaryError, CustomDictionaryIdentity, CustomDictionaryImportResult,
    CustomDictionaryRow, CustomDictionaryStore, SearchKeyDeriver,
};
pub use database::{immediate_transaction, UserDataDatabase, UserDataDatabaseError};
pub use directory::{created, user_data_directory, DirectoryError, APPLICATION_FOLDER_NAME};
pub use frequency::UserFrequencyStore;
pub use settings_file::{LiveSettings, SettingsFileError, SettingsFileStore};
pub use timestamp::utc_timestamp_now;

use std::path::PathBuf;
use std::sync::Arc;

/// The user-data stores, constructed against one directory and opened
/// together. Separate files rather than tables in one database, matching iOS
/// and Android: different capacity policies, very different write rates, and
/// a user clearing one kind of data keeps the others.
pub struct UserDataStores {
    pub frequency: Arc<UserFrequencyStore>,
    pub association: Arc<UserAssociationStore>,
    pub custom_dictionary: Arc<CustomDictionaryStore>,
}

impl UserDataStores {
    /// Stores over `directory`, using the engine's own search-key derivation
    /// for the custom dictionary. Nothing is opened yet.
    pub fn new(directory: PathBuf) -> Self {
        Self {
            frequency: Arc::new(UserFrequencyStore::new(
                directory.clone(),
                UserFrequencyStore::shipped_capacity(),
            )),
            association: Arc::new(UserAssociationStore::new(
                directory.clone(),
                UserAssociationStore::shipped_capacity(),
            )),
            custom_dictionary: Arc::new(CustomDictionaryStore::new(
                directory,
                Arc::new(taigi_windows_core::engine::derive_custom_search_keys),
                CustomDictionaryStore::MAX_ENTRIES,
            )),
        }
    }

    /// Opens all of them, off the calling thread. Safe to call more than
    /// once — each store opens its file exactly once.
    pub fn open(&self) {
        self.frequency.open();
        self.association.open();
        self.custom_dictionary.open();
    }
}
