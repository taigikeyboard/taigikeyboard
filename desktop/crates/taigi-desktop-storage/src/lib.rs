//! What the desktop input methods keep on disk beyond the engine's user-data
//! stores: `settings.json`, the user's copied-in fonts, and the directory
//! each desktop platform hands in (`%APPDATA%\TaigiKeyboard` on Windows, the
//! XDG config / data directories on Linux).
//!
//! The learning databases and the custom dictionary moved to the engine
//! `userdata` crate (`docs/architecture/user-data-engine-roadmap.md` P1); they
//! are re-exported here so the Windows and Linux shells keep their imports
//! until the desktop switch (P5).
//!
//! Host-testable: nothing here touches a Windows API, so the tests run on
//! the Mac against temporary directories. Only `directory::user_data_directory`
//! reads `%APPDATA%`, and it is a pure environment lookup.

mod directory;
mod font_library;
mod settings_file;

pub use directory::{created, user_data_directory, DirectoryError, APPLICATION_FOLDER_NAME};
pub use font_library::{
    copy_in, fonts_directory, remove_stored, sanitized_stem, stored_file_names, ImportError,
    ALLOWED_EXTENSIONS, FONTS_FOLDER_NAME, MAX_FILE_SIZE,
};
pub use settings_file::{LiveSettings, SettingsFileError, SettingsFileStore};
pub use userdata::{
    immediate_transaction, utc_timestamp_now, AssociationRow, CustomDictionaryCSV,
    CustomDictionaryCSVError, CustomDictionaryError, CustomDictionaryIdentity,
    CustomDictionaryImportResult, CustomDictionaryRow, CustomDictionaryStore, LearnedPhraseRow,
    LearnedPhraseStore, LearningCapacity, SearchKeyDeriver, UserAssociationStore, UserDataCSV,
    UserDataDatabase, UserDataDatabaseError, UserDataStores, UserFrequencyStore,
};
