//! The four stores opened together, where the platform keeps their files,
//! and the in-process search-key derivation they write and query with.

use crate::types::CustomSearchKey;
use crate::{
    association, custom_dictionary, frequency, learned_phrases, CustomDictionaryStore, JournalMode,
    LearnedPhraseStore, SearchKeyDeriver, UserAssociationStore, UserFrequencyStore,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Every key a stored custom-dictionary entry or learned phrase is findable
/// under — the engine's own derivation, called in-process. Never `None`: unlike
/// the `DeriveCustomSearchKeys` op it cannot fail, so it logs no failure; the
/// `Option` is the `SearchKeyDeriver` shape, where `None` means the
/// derivation could not answer.
pub fn derive_custom_search_keys(roman: &str) -> Option<Vec<CustomSearchKey>> {
    Some(
        phonetics::api::derive_custom_search_keys(roman)
            .into_iter()
            .map(CustomSearchKey::from)
            .collect(),
    )
}

/// The one key the current `input` is looked up by, under settings
/// `input_mode` (`tl` / `poj` / `tps`) — the read-side twin of
/// [`derive_custom_search_keys`]; the engine's own fetch path (roadmap P3)
/// queries the stores with it.
pub fn derive_custom_query_key(input: &str, input_mode: &str) -> Option<CustomSearchKey> {
    phonetics::api::derive_custom_query_key(input, input_mode).map(CustomSearchKey::from)
}

/// Where each store's file lives. The platform decides (roadmap U5): one
/// directory everywhere except Android, whose `user_association.db` has
/// always lived in `filesDir` beside the others' `databases/`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UserDataPaths {
    pub frequency: PathBuf,
    pub association: PathBuf,
    pub custom_dictionary: PathBuf,
    pub learned_phrases: PathBuf,
}

impl UserDataPaths {
    /// All four files in `directory`, under their shared names.
    pub fn in_directory(directory: &Path) -> Self {
        Self {
            frequency: directory.join(frequency::FILE_NAME),
            association: directory.join(association::FILE_NAME),
            custom_dictionary: directory.join(custom_dictionary::FILE_NAME),
            learned_phrases: directory.join(learned_phrases::FILE_NAME),
        }
    }
}

/// The user-data stores, constructed together and opened together. Separate
/// files rather than tables in one database, matching iOS and Android:
/// different capacity policies, very different write rates, and a user
/// clearing one kind of data keeps the others.
pub struct UserDataStores {
    pub frequency: Arc<UserFrequencyStore>,
    pub association: Arc<UserAssociationStore>,
    pub custom_dictionary: Arc<CustomDictionaryStore>,
    pub learned_phrases: Arc<LearnedPhraseStore>,
}

impl UserDataStores {
    /// The desktop layout: all four files in `directory`, write-ahead logged.
    /// Nothing is opened yet.
    pub fn new(directory: PathBuf) -> Self {
        Self::at(UserDataPaths::in_directory(&directory), JournalMode::Wal)
    }

    /// Stores at `paths`, journaled per `journal`, using the engine's own
    /// search-key derivation. Nothing is opened yet.
    pub fn at(paths: UserDataPaths, journal: JournalMode) -> Self {
        let derive_search_keys: SearchKeyDeriver = Arc::new(derive_custom_search_keys);
        Self {
            frequency: Arc::new(UserFrequencyStore::new(
                paths.frequency,
                journal,
                UserFrequencyStore::shipped_capacity(),
            )),
            association: Arc::new(UserAssociationStore::new(
                paths.association,
                journal,
                UserAssociationStore::shipped_capacity(),
            )),
            custom_dictionary: Arc::new(CustomDictionaryStore::new(
                paths.custom_dictionary,
                journal,
                Arc::clone(&derive_search_keys),
                CustomDictionaryStore::MAX_ENTRIES,
            )),
            learned_phrases: Arc::new(LearnedPhraseStore::new(
                paths.learned_phrases,
                journal,
                derive_search_keys,
                LearnedPhraseStore::MAX_ENTRIES,
            )),
        }
    }

    /// Opens all of them, off the calling thread. Safe to call more than
    /// once — each store opens its file exactly once.
    pub fn open(&self) {
        self.frequency.open();
        self.association.open();
        self.custom_dictionary.open();
        self.learned_phrases.open();
    }
}
