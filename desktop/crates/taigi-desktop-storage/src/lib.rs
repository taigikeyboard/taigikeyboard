//! What the desktop input methods keep on disk beyond the engine's user-data
//! stores: `settings.json` (and the settings windows' writer over it), the
//! user's copied-in fonts, the atomic replace the settings file and the
//! custom-dictionary export write through, and the directory each desktop
//! platform hands in (`%APPDATA%\TaigiKeyboard` on Windows, the XDG config /
//! data directories on Linux).
//!
//! The learning databases and the custom dictionary are the engine's
//! (`engine/userdata`, `docs/architecture/user-data-engine-roadmap.md`); the
//! desktops reach them through `taigi_desktop_core::engine::user_data`.
//!
//! Host-testable: nothing here touches a Windows API, so the tests run on
//! the Mac against temporary directories. Only `directory::user_data_directory`
//! reads `%APPDATA%`, and it is a pure environment lookup.

mod directory;
mod font_library;
mod replace_file;
mod settings_file;
mod settings_writer;

pub use directory::{created, user_data_directory, DirectoryError, APPLICATION_FOLDER_NAME};
pub use font_library::{
    copy_in, fonts_directory, remove_stored, sanitized_stem, stored_file_names, ImportError,
    ALLOWED_EXTENSIONS, FONTS_FOLDER_NAME, MAX_FILE_SIZE,
};
pub use replace_file::replace_file;
pub use settings_file::{LiveSettings, SettingsFileError, SettingsFileStore};
pub use settings_writer::{SettingsWriter, IDLE_REFRESH_INTERVAL};
