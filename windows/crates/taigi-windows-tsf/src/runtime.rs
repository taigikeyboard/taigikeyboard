//! The per-process state every text-service instance shares: the live
//! settings, the user-data stores, the engine's lexicon, and what this host
//! is allowed to touch. One process may create several text services (one
//! per thread manager); the files and the engine are one per process.
//!
//! Lazy by contract (roadmap W3): `shared()` only probes paths and reads the
//! settings file; the stores open and the lexicon loads on the first key
//! the TIP handles (`prepare_for_first_key`, PR5b), never in `Activate`.

// 中文: 每個程序一份的執行期狀態 — 即時設定、使用者資料庫、引擎詞庫;辭典與資料庫延後到第一個按鍵才載入。

use crate::module::install_directory;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use taigi_windows_core::composing::{
    AssociationSink, ComposingManager, ComposingSessionCoordinator, CustomDictionarySource,
    FrequencySource, NextWordLearner, NoStores, SystemClock,
};
use taigi_windows_core::dictionary_artifacts::DictionaryArtifacts;
use taigi_windows_core::engine::{lexicon_install, LexiconInstallStats};
use taigi_windows_core::keys::ShortcutConflicts;
use taigi_windows_core::settings::{
    keys, SettingsDocument, SettingsProvider, StaticSettingsProvider,
};
use taigi_windows_core::strings::{DisplayLanguage, StringResolver};
use taigi_windows_storage::{user_data_directory, LiveSettings, SettingsFileStore, UserDataStores};

/// The install directory's dictionary folder — where the installer (PR10)
/// copies `ios/Resources/Dictionaries/` (no third committed copy, W2).
pub const DICTIONARIES_DIR_NAME: &str = "Dictionaries";

/// What this host process may touch, probed ONCE and logged once (Codex W2:
/// an AppContainer host cannot read `%APPDATA%`; the TIP then runs on
/// shipped defaults and learns nothing, and never re-probes per keystroke).
///
/// Both flags mean "the per-user directory could be resolved and created";
/// a directory that exists but refuses a read or a write degrades later
/// and per store (`LiveSettings` keeps defaults, a database open logs and
/// stays not-ready) — these flags are the probe, not a promise.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DataCapability {
    /// `settings.json` has a directory to live in.
    pub settings: bool,
    /// The three learning databases have a directory to live in.
    pub learning: bool,
}

pub struct Runtime {
    pub capability: DataCapability,
    pub settings: Arc<dyn SettingsProvider + Send + Sync>,
    settings_store: Option<SettingsFileStore>,
    stores: Option<UserDataStores>,
    first_key: OnceLock<FirstKeySetup>,
    /// The one composing engine driver per process, keyed by context
    /// token (roadmap W3). Behind a mutex because one process may host
    /// thread managers on several threads; held for the length of one
    /// synchronous edit session, never across a callback into the host.
    coordinator: OnceLock<Mutex<ComposingSessionCoordinator>>,
}

/// What the first handled key set up, kept so later keys skip it.
#[derive(Clone, Copy, Debug)]
pub struct FirstKeySetup {
    pub lexicon: Option<LexiconInstallStats>,
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
        let stores = data_directory.map(UserDataStores::new);
        let capability = DataCapability {
            settings: settings_store.is_some(),
            learning: stores.is_some(),
        };
        log::info!(
            "runtime.capability settings={} learning={}",
            capability.settings,
            capability.learning
        );
        Self {
            capability,
            settings,
            settings_store,
            stores,
            first_key: OnceLock::new(),
            coordinator: OnceLock::new(),
        }
    }

    pub fn settings_store(&self) -> Option<&SettingsFileStore> {
        self.settings_store.as_ref()
    }

    /// The coordinator only if a key has already built it — for callbacks
    /// that must not bring the engine up (termination, focus loss).
    pub fn coordinator_if_built(&self) -> Option<&Mutex<ComposingSessionCoordinator>> {
        self.coordinator.get()
    }

    /// The coordinator, built on first use over the stores this host has
    /// (`NoStores` where it has none). `prepare_for_first_key` must have run.
    pub fn coordinator(&self) -> &Mutex<ComposingSessionCoordinator> {
        self.coordinator.get_or_init(|| {
            let settings: Arc<dyn taigi_windows_core::settings::SettingsProvider> =
                Arc::clone(&self.settings) as _;
            let (frequency, custom, association): (
                Box<dyn FrequencySource>,
                Box<dyn CustomDictionarySource>,
                Box<dyn AssociationSink>,
            ) = match &self.stores {
                Some(stores) => (
                    Box::new(Arc::clone(&stores.frequency)),
                    Box::new(Arc::clone(&stores.custom_dictionary)),
                    Box::new(Arc::clone(&stores.association)),
                ),
                None => (Box::new(NoStores), Box::new(NoStores), Box::new(NoStores)),
            };
            let learner = NextWordLearner::new(association, Box::new(SystemClock));
            let manager = ComposingManager::new(
                settings,
                frequency,
                custom,
                learner,
                Box::new(SystemClock),
                1,
            );
            Mutex::new(ComposingSessionCoordinator::new(manager))
        })
    }

    /// Everything the first CONSUMED key needs, done once per process (never
    /// for a key merely observed — the classifier decides first, PR5b):
    /// the lexicon installed from the install directory, the stores opened
    /// (seed + key re-derivation queued behind the open, as on macOS
    /// `openUserDataStores`), and the shortcut registries reconciled
    /// (`AppDelegate.swift:56-70`). Idempotent.
    pub fn prepare_for_first_key(&self) -> &FirstKeySetup {
        self.first_key.get_or_init(|| {
            let lexicon = self.install_lexicon();
            if let Some(stores) = &self.stores {
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
            }
            self.reconcile_shortcuts();
            FirstKeySetup { lexicon }
        })
    }

    /// Loads the dictionary data the installer shipped. Failures are logged
    /// and left alone: an uninstalled engine returns no candidates, which is
    /// a keyboard that types romanization but suggests nothing.
    fn install_lexicon(&self) -> Option<LexiconInstallStats> {
        let directory = Self::dictionaries_directory()?;
        let artifacts = match DictionaryArtifacts::locate(&directory) {
            Ok(artifacts) => artifacts,
            Err(error) => {
                log::error!("lexicon.not_installed error={error}");
                return None;
            }
        };
        let stats = lexicon_install(&artifacts, dictionary_version());
        match &stats {
            Some(stats) => log::info!(
                "lexicon.installed records={} prefix_entries={} version={}",
                stats.dictionary_record_count,
                stats.prefix_index_entry_count,
                dictionary_version()
            ),
            None => log::error!("lexicon.install_returned_nothing"),
        }
        stats
    }

    pub fn dictionaries_directory() -> Option<PathBuf> {
        install_directory().map(|directory| directory.join(DICTIONARIES_DIR_NAME))
    }

    /// The launch-time pass over both shortcut registries; writes only when
    /// something changed (the store skips a save whose revision did not move).
    fn reconcile_shortcuts(&self) {
        let Some(store) = &self.settings_store else {
            return;
        };
        if let Err(error) = store.update(ShortcutConflicts::resolve_across_registries) {
            log::error!("shortcuts.reconcile_failed error={error}");
        }
    }

    /// The language the UI strings are drawn in: the setting, or the
    /// machine's when it says `system`.
    pub fn display_language(&self) -> DisplayLanguage {
        let tag = self.settings.current().string(&keys::DISPLAY_LANGUAGE);
        DisplayLanguage::from_tag(&tag).effective(&taigi_windows_platform::system_locale())
    }

    pub fn strings(&self) -> StringResolver {
        StringResolver::new(self.display_language())
    }
}

/// The dictionary stamp the engine caches under: the crate version as one
/// integer (`3.6.6` → `30606`), the same number the macOS `CFBundleVersion`
/// carries, because the data is rebuilt by the release that bumps it. Every
/// component is assumed < 100 (the release train's shape); a larger one
/// would collide and is refused loudly.
pub fn dictionary_version() -> u32 {
    let mut parts = env!("CARGO_PKG_VERSION")
        .split('.')
        .map(|part| part.parse::<u32>().unwrap_or(0));
    let major = parts.next().unwrap_or(0);
    let minor = parts.next().unwrap_or(0);
    let patch = parts.next().unwrap_or(0);
    assert!(
        minor < 100 && patch < 100,
        "version components must stay below 100"
    );
    major * 10_000 + minor * 100 + patch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dictionary_version_matches_the_macos_bundle_version_shape() {
        assert_eq!(dictionary_version() % 100, 6);
        assert!(dictionary_version() >= 30_606);
    }
}
