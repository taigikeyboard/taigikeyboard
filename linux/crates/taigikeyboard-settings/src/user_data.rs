//! The user-data stores the window reads and writes (Custom Dictionary, the three
//! learning tables), opened once at launch under `$XDG_DATA_HOME/taigikeyboard`
//! (roadmap L7). Port of the Windows `user_data.rs`: the launch migrations
//! run on a background thread off the open; no directory means no stores
//! and the pages that need them say so.

use std::sync::Arc;
use taigi_desktop_storage::{created, UserDataStores};
use taigi_linux_platform::UserDirectories;

/// `Err` carries what the window says: the data directory could not be
/// created, or there is no user directory at all.
pub fn open_at_launch() -> Result<UserDataStores, String> {
    let directories =
        UserDirectories::resolve().ok_or_else(|| "HOME / XDG_DATA_HOME".to_owned())?;
    let directory = created(directories.data).map_err(|error| {
        log::error!("user_data.no_data_directory error={error}");
        error.to_string()
    })?;
    let stores = UserDataStores::new(directory);
    stores.open();
    let custom_dictionary = Arc::clone(&stores.custom_dictionary);
    std::thread::Builder::new()
        .name("taigi-custom-dictionary-launch".into())
        .spawn(move || custom_dictionary.finish_takeover())
        .ok();
    Ok(stores)
}
