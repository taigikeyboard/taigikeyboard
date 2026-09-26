//! The user's own data on disk: the learning databases
//! (`user_frequency.db`, `user_association.db`, `learned_phrases.db`) and the
//! user's `custom_dictionary.db`, in the directory the platform hands in.
//!
//! Engine-owned per `.claude/rules/rust-migration-policy.md` §6; migration
//! plan and status in `docs/architecture/user-data-engine-roadmap.md`.
//! Moved from `desktop/crates/taigi-desktop-storage` (roadmap P1), itself a
//! port of `macos/Sources/TaigiInputMethodCore/Storage/` over rusqlite: the
//! SQL is byte-identical to the macOS stores (which mirror iOS / Android), so
//! a `.taigi` export or a hand-copied database means the same thing on every
//! platform.
//!
//! Host-testable: nothing here touches a platform API, so the tests run
//! against temporary directories.

#[cfg(feature = "sqlite")]
mod association;
#[cfg(feature = "sqlite")]
mod backup;
#[cfg(feature = "sqlite")]
mod capacity;
#[cfg(feature = "sqlite")]
mod csv;
#[cfg(feature = "sqlite")]
mod custom_dictionary;
#[cfg(feature = "sqlite")]
mod database;
#[cfg(feature = "sqlite")]
mod frequency;
#[cfg(feature = "sqlite")]
mod learned_phrases;
mod paths;
mod stores;
#[cfg(feature = "sqlite")]
mod timestamp;
mod types;
#[cfg(feature = "sqlite")]
mod user_data_stores;

#[cfg(feature = "sqlite")]
pub use association::{AssociationRow, FollowingRow, UserAssociationStore};
#[cfg(feature = "sqlite")]
pub use backup::{export_backup, import_backup, BackupError, BackupImported, BACKUP_VERSION};
#[cfg(feature = "sqlite")]
pub use capacity::LearningCapacity;
#[cfg(feature = "sqlite")]
pub use csv::{CustomDictionaryCSV, CustomDictionaryCSVError, UserDataCSV};
#[cfg(feature = "sqlite")]
pub use custom_dictionary::{
    CustomDictionaryError, CustomDictionaryIdentity, CustomDictionaryImportResult,
    CustomDictionaryRow, CustomDictionaryStore, SearchKeyDeriver,
};
#[cfg(feature = "sqlite")]
pub use database::{
    immediate_transaction, JournalMode, UserDataDatabase, UserDataDatabaseError,
    TAIGI_APPLICATION_ID,
};
#[cfg(feature = "sqlite")]
pub use frequency::UserFrequencyStore;
#[cfg(feature = "sqlite")]
pub use learned_phrases::{LearnedPhraseRow, LearnedPhraseStore};
pub use paths::{
    UserDataPaths, ASSOCIATION_FILE, CUSTOM_DICTIONARY_FILE, FREQUENCY_FILE, LEARNED_PHRASES_FILE,
};
pub use stores::{CustomDictionarySource, FrequencySource, LearnedPhraseSource};
#[cfg(feature = "sqlite")]
pub use timestamp::{unix_seconds_now, utc_timestamp_now};
pub use types::{AssociationPair, CustomEntry, CustomSearchKey, FrequencyRow, LearnedPhrase};
#[cfg(feature = "sqlite")]
pub use user_data_stores::{derive_custom_query_key, derive_custom_search_keys, UserDataStores};
