//! How the Windows TIP builds the shared per-process runtime
//! (`taigi_desktop_core::runtime::DesktopRuntime`): the settings and the
//! user data under `%APPDATA%`, the dictionaries beside the DLL. One process
//! may create several text services (one per thread manager); the files
//! and the engine are one per process, so the runtime is a process static.
//!
//! Lazy by contract (roadmap W3): `shared()` only probes paths and reads the
//! settings file; the engine opens the user data and the lexicon loads on
//! the first key the TIP handles (`prepare_for_first_key`, PR5b) — the
//! install directory included — never in `Activate`.
//!
//! What this host may touch is probed ONCE and logged once (Codex W2: an
//! AppContainer host cannot read `%APPDATA%`; the TIP then runs on shipped
//! defaults and learns nothing, and never re-probes per keystroke). A
//! directory that exists but refuses a read or a write degrades later and
//! per store (`LiveSettings` keeps defaults, a database open logs and stays
//! not-ready) — the probe is not a promise.

use crate::module::install_directory;
use std::ops::Deref;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use taigi_desktop_core::composing::SystemClock;
use taigi_desktop_core::dictionary_artifacts::DictionaryArtifacts;
use taigi_desktop_core::runtime::{DesktopRuntime, RuntimeParts};
use taigi_desktop_core::settings::{SettingsDocument, SettingsProvider, StaticSettingsProvider};
use taigi_desktop_storage::{user_data_directory, LiveSettings, SettingsFileStore};

pub struct Runtime(DesktopRuntime);

/// Everything the shared runtime answers — settings, the coordinator, the
/// first-key setup, the display language — read straight through.
impl Deref for Runtime {
    type Target = DesktopRuntime;

    fn deref(&self) -> &DesktopRuntime {
        &self.0
    }
}

static SHARED: OnceLock<Runtime> = OnceLock::new();

impl Runtime {
    pub fn shared() -> &'static Runtime {
        SHARED.get_or_init(Runtime::probe)
    }

    fn probe() -> Self {
        let data_directory = match user_data_directory() {
            Ok(directory) => Some(directory),
            Err(error) => {
                log::warn!(
                    "runtime.no_user_data_directory error={error} — shipped defaults, no learning"
                );
                None
            }
        };
        let settings_store = data_directory
            .as_ref()
            .map(|directory| SettingsFileStore::new(directory));
        let settings: Arc<dyn SettingsProvider + Send + Sync> = match &settings_store {
            Some(store) => Arc::new(LiveSettings::new(store.clone())),
            None => Arc::new(StaticSettingsProvider::new(SettingsDocument::default())),
        };
        log::info!(
            "runtime.capability settings={} learning={}",
            settings_store.is_some(),
            data_directory.is_some()
        );
        Self(DesktopRuntime::new(RuntimeParts {
            settings,
            settings_store: settings_store.map(|store| Box::new(store) as _),
            data_directory,
            dictionaries: Box::new(Self::dictionaries_directory),
            dictionary_version: dictionary_version(),
            system_locale: Box::new(taigi_windows_platform::system_locale),
            platform: taigi_windows_platform::DESKTOP_PLATFORM,
            clock: Arc::new(SystemClock),
        }))
    }

    pub fn dictionaries_directory() -> Option<PathBuf> {
        install_directory().map(|directory| directory.join(DictionaryArtifacts::DIRECTORY_NAME))
    }
}

pub fn dictionary_version() -> u32 {
    taigi_desktop_core::dictionary_artifacts::dictionary_version(env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stamp_is_this_crate_version_in_the_macos_bundle_shape() {
        // trace: 3.6.7 -> 3*10_000 + 6*100 + 7 = 30_607. Recomputed from the
        // crate version, never pinned to a literal patch: the literal was `6`,
        // so the 3.6.7 bump failed a test of the version number instead of the
        // encoding (same trap already fixed in
        // `dictionary_artifacts.rs::the_stamp_has_the_macos_bundle_version_shape`).
        let mut parts = env!("CARGO_PKG_VERSION")
            .split('.')
            .map(|part| part.parse::<u32>().expect("version components are numeric"));
        let (major, minor, patch) = (
            parts.next().expect("major"),
            parts.next().expect("minor"),
            parts.next().expect("patch"),
        );
        assert_eq!(dictionary_version(), major * 10_000 + minor * 100 + patch);
        assert!(
            dictionary_version() >= 30_606,
            "the stamp never goes back past the first shipped desktop version"
        );
    }
}
