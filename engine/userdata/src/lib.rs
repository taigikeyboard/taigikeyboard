//! The user's own data on disk: the learning databases
//! (`user_frequency.db`, `user_association.db`, `learned_phrases.db`) and the
//! user's `custom_dictionary.db`, in the directory the platform hands in.
//!
//! Engine-owned per `docs/contributing/rust-migration-policy.md` §6; migration
//! plan and status in `docs/architecture/user-data-engine-roadmap.md`.
//! Moved from `desktop/crates/taigi-desktop-storage` (roadmap P1), itself a
//! port of `macos/Sources/TaigiInputMethodCore/Storage/` over rusqlite: the
//! SQL is byte-identical to the macOS stores (which mirror iOS / Android), so
//! a `.taigi` export or a hand-copied database means the same thing on every
//! platform.
//!
//! `UserDataHandle` is the process's one open of the stores and answers
//! every `UserDataRequest`; `engine/dispatch` routes the request to it and
//! maps `RequestError` to the wire's error code.
//!
//! Host-testable: nothing here touches a platform API, so the tests run
//! against temporary directories.

mod association;
mod backup;
mod capacity;
mod csv;
mod custom_dictionary;
mod database;
mod frequency;
mod handle;
mod learned_phrases;
mod learning_records;
mod paging;
mod paths;
mod requests;
mod stores;
mod timestamp;

pub use association::{AssociationPair, AssociationRow, FollowingRow, UserAssociationStore};
pub use backup::{export_backup, import_backup, BackupError, BackupImported, BACKUP_VERSION};
pub use capacity::LearningCapacity;
pub use csv::{CustomDictionaryCSV, CustomDictionaryCSVError, UserDataCSV};
pub use custom_dictionary::{
    CustomDictionaryError, CustomDictionaryIdentity, CustomDictionaryImportResult,
    CustomDictionaryRow, CustomDictionaryStore, SearchKeyDeriver,
};
pub use database::{
    deferred_transaction, immediate_transaction, JournalMode, UserDataDatabase,
    UserDataDatabaseError, TAIGI_APPLICATION_ID,
};
pub use frequency::{FrequencyRow, UserFrequencyStore};
pub use handle::UserDataHandle;
pub use learned_phrases::{LearnedPhraseRow, LearnedPhraseStore};
pub use paths::{
    UserDataPaths, ASSOCIATION_FILE, CUSTOM_DICTIONARY_FILE, FREQUENCY_FILE, LEARNED_PHRASES_FILE,
    USER_DATA_FILES,
};
pub use requests::RequestError;
pub use stores::{derive_custom_search_keys, UserDataStores};
pub use timestamp::{unix_millis_now, unix_seconds_now, utc_timestamp_now};
