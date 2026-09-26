//! Where the four stores' files live — the one piece of user data the
//! platform decides (roadmap U5). Built without the `sqlite` feature too, so
//! a shell that only talks to the engine through `dispatch` can name the
//! files it asks the engine to open.

use std::path::{Path, PathBuf};

/// Every platform's file names for the four stores.
pub const FREQUENCY_FILE: &str = "user_frequency.db";
pub const ASSOCIATION_FILE: &str = "user_association.db";
pub const CUSTOM_DICTIONARY_FILE: &str = "custom_dictionary.db";
pub const LEARNED_PHRASES_FILE: &str = "learned_phrases.db";

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
            frequency: directory.join(FREQUENCY_FILE),
            association: directory.join(ASSOCIATION_FILE),
            custom_dictionary: directory.join(CUSTOM_DICTIONARY_FILE),
            learned_phrases: directory.join(LEARNED_PHRASES_FILE),
        }
    }
}
