//! The per-process state every text-service instance shares: the live
//! settings, the directory the engine keeps the user's data in, the engine's
//! lexicon, and what this host is allowed to touch. One process may create
//! several text services (one per thread manager); the files and the engine
//! are one per process.
//!
//! Lazy by contract (roadmap W3): `shared()` only probes paths and reads the
//! settings file; the engine opens the user data and the lexicon loads on
//! the first key the TIP handles (`prepare_for_first_key`, PR5b), never in
//! `Activate`.

use crate::module::install_directory;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use taigi_desktop_core::composing::ComposingSessionCoordinator;
use taigi_desktop_core::dictionary_artifacts::DictionaryArtifacts;
use taigi_desktop_core::engine::{lexicon_install, user_data, LexiconInstallStats};
use taigi_desktop_core::keys::ShortcutConflicts;
use taigi_desktop_core::settings::{
    keys, SettingsDocument, SettingsProvider, StaticSettingsProvider,
};
use taigi_desktop_core::strings::{DisplayLanguage, StringResolver};
use taigi_desktop_storage::{user_data_directory, LiveSettings, SettingsFileStore};

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
    /// Where the engine keeps the user's data; `None` in an AppContainer
    /// host that cannot reach `%APPDATA%` — nothing is learned there.
    data_directory: Option<PathBuf>,
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
        let capability = DataCapability {
            settings: settings_store.is_some(),
            learning: data_directory.is_some(),
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
            data_directory,
            first_key: OnceLock::new(),
            coordinator: OnceLock::new(),
        }
    }

    /// One write to `settings.json` from the key path — under the file's
    /// lock, on the document as it is now, so a write from the settings
    /// window is not lost. `what` names the write in the log. Answers
    /// whether there was a store to write to (an AppContainer host has
    /// none); a write that failed is logged and still answers true, since
    /// what the chord does next does not depend on the disk.
    pub fn update_settings(&self, what: &str, mutate: impl FnOnce(&mut SettingsDocument)) -> bool {
        let Some(store) = &self.settings_store else {
            log::warn!("settings.no_store what={what}");
            return false;
        };
        if let Err(error) = store.update(mutate) {
            log::error!("settings.update_failed what={what} error={error}");
        }
        true
    }

    /// The coordinator only if a key has already built it — for callbacks
    /// that must not bring the engine up (termination, focus loss).
    pub fn coordinator_if_built(&self) -> Option<&Mutex<ComposingSessionCoordinator>> {
        self.coordinator.get()
    }

    /// The coordinator locked without waiting: `None` when no key has built
    /// it yet OR when it is already held (a host re-entered us from inside
    /// our own session). NEVER blocking — a callback that waited on the
    /// engine would deadlock the session holding it.
    pub(crate) fn try_coordinator(&self) -> Option<MutexGuard<'_, ComposingSessionCoordinator>> {
        self.coordinator_if_built()?.try_lock().ok()
    }

    /// The coordinator, built on first use; picks reach the engine only where
    /// this host has a data directory. `prepare_for_first_key` must have run.
    pub fn coordinator(&self) -> &Mutex<ComposingSessionCoordinator> {
        self.coordinator.get_or_init(|| {
            let settings: Arc<dyn taigi_desktop_core::settings::SettingsProvider> =
                Arc::clone(&self.settings) as _;
            Mutex::new(ComposingSessionCoordinator::for_desktop(
                settings,
                self.data_directory.is_some(),
            ))
        })
    }

    /// Everything the first CONSUMED key needs, done once per process (never
    /// for a key merely observed — the classifier decides first, PR5b):
    /// the lexicon installed from the install directory, the engine told
    /// to open the user data (it finishes on a thread of its own), and the
    /// shortcut registries reconciled (`AppDelegate.swift:56-70`). Idempotent.
    pub fn prepare_for_first_key(&self) -> &FirstKeySetup {
        self.first_key.get_or_init(|| {
            let lexicon = self.install_lexicon();
            if let Some(directory) = &self.data_directory {
                // The engine puts the stores in use before this returns and
                // finishes opening on a thread of its own.
                user_data::open(directory);
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
        install_directory().map(|directory| directory.join(DictionaryArtifacts::DIRECTORY_NAME))
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
