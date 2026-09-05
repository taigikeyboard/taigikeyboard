//! Opening the three user-data stores at launch, and the migrations that
//! go with it. Both windows call this — the egui one today, the WinUI one
//! from roadmap W17 — so the launch-time side effects are one list, not
//! one per toolkit.

use std::path::PathBuf;
use std::sync::Arc;
use taigi_windows_storage::UserDataStores;

/// The stores, opened unless the window is read-only (no `%APPDATA%`: it
/// shows the defaults and writes nothing, so it opens nothing either).
///
/// What the DLL does on its first consumed key is done here too: a fresh
/// install whose first visitor is this window still gets its seeds, and an
/// older dictionary its re-derived keys. Both run off the launch path so a
/// large dictionary cannot hold the window closed.
pub fn open_at_launch(directory: PathBuf, is_read_only: bool) -> UserDataStores {
    let stores = UserDataStores::new(directory);
    if is_read_only {
        return stores;
    }
    stores.open();
    let custom_dictionary = Arc::clone(&stores.custom_dictionary);
    std::thread::Builder::new()
        .name("taigi-custom-dictionary-launch".into())
        .spawn(move || {
            if let Err(error) = custom_dictionary.rederive_search_keys_if_needed() {
                log::error!("custom_dictionary.rederive_failed error={error}");
            }
            if let Err(error) = custom_dictionary.seed_if_empty() {
                log::error!("custom_dictionary.seed_failed error={error}");
            }
        })
        .ok();
    stores
}
