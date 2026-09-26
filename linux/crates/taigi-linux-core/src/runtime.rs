//! The per-process state every engine object shares: the live settings, the
//! directory the engine keeps the user's data in, the engine's lexicon, and
//! the one composing coordinator (roadmap L13). Port of
//! `taigi-windows-tsf::runtime`, with the XDG directories in place of
//! `%APPDATA%` and no AppContainer degradation — a Linux session without a
//! home directory has no user to learn from, and nothing is learned.
//!
//! Lazy by contract: `probe` only resolves paths and reads the settings
//! file; the engine opens the user data and the lexicon loads on the first
//! key an engine CONSUMES (`prepare_for_first_key`), never at `CreateEngine`.

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use taigi_desktop_core::composing::{ComposingSessionCoordinator, ContextToken};
use taigi_desktop_core::dictionary_artifacts::DictionaryArtifacts;
use taigi_desktop_core::engine::{lexicon_install, user_data, LexiconInstallStats};
use taigi_desktop_core::keys::ShortcutConflicts;
use taigi_desktop_core::settings::{
    keys, SettingsDocument, SettingsProvider, StaticSettingsProvider,
};
use taigi_desktop_core::strings::{DisplayLanguage, StringResolver};
use taigi_desktop_storage::{created, LiveSettings, SettingsFileStore};
use taigi_linux_platform::{dictionaries_directory, system_locale, UserDirectories};

pub struct Runtime {
    pub settings: Arc<dyn SettingsProvider + Send + Sync>,
    settings_store: Option<SettingsFileStore>,
    /// Where the engine keeps the user's data; `None` = no learning.
    data_directory: Option<PathBuf>,
    dictionaries: PathBuf,
    first_key: OnceLock<FirstKeySetup>,
    /// The one composing engine driver per process, keyed by context token
    /// (one per engine object). Held for the length of one key, never
    /// across a signal emission.
    coordinator: OnceLock<Mutex<ComposingSessionCoordinator>>,
    next_token: AtomicUsize,
}

/// What the first handled key set up, kept so later keys skip it.
#[derive(Clone, Copy, Debug)]
pub struct FirstKeySetup {
    pub lexicon: Option<LexiconInstallStats>,
}

/// A runtime over a temporary XDG tree with no dictionaries: settings are
/// live (a file), the lexicon is absent, and nothing is learned — the
/// engine's user data is one per process, and several of these share a
/// test binary.
#[cfg(test)]
pub(crate) fn temporary_runtime() -> (tempfile::TempDir, Runtime) {
    let directory = tempfile::tempdir().expect("tempdir");
    let mut runtime = Runtime::from_directories(
        Some(UserDirectories {
            config: directory.path().join("config"),
            data: directory.path().join("data"),
        }),
        directory.path().join("no-dictionaries"),
    );
    runtime.data_directory = None;
    (directory, runtime)
}

impl Runtime {
    /// Resolves the user's directories and reads the settings file. Nothing
    /// else is touched.
    pub fn probe() -> Self {
        #[cfg(feature = "e2e-trace")]
        crate::trace::open();
        Self::from_directories(UserDirectories::resolve(), dictionaries_directory())
    }

    /// `probe` over explicit directories — what a test builds over a
    /// temporary tree.
    pub fn from_directories(directories: Option<UserDirectories>, dictionaries: PathBuf) -> Self {
        let config = directories
            .as_ref()
            .and_then(|d| match created(d.config.clone()) {
                Ok(path) => Some(path),
                Err(error) => {
                    log::warn!("runtime.no_config_directory error={error} — shipped defaults");
                    None
                }
            });
        let data = directories
            .as_ref()
            .and_then(|d| match created(d.data.clone()) {
                Ok(path) => Some(path),
                Err(error) => {
                    log::warn!("runtime.no_data_directory error={error} — no learning");
                    None
                }
            });
        if directories.is_none() {
            log::warn!("runtime.no_user_directories — HOME unset; shipped defaults, no learning");
        }
        let settings_store = config
            .as_ref()
            .map(|directory| SettingsFileStore::new(directory));
        let settings: Arc<dyn SettingsProvider + Send + Sync> = match &settings_store {
            Some(store) => Arc::new(LiveSettings::new(store.clone())),
            None => Arc::new(StaticSettingsProvider::new(SettingsDocument::default())),
        };
        log::info!(
            "runtime.probe settings={} learning={} dictionaries={}",
            settings_store.is_some(),
            data.is_some(),
            dictionaries.display()
        );
        Self {
            settings,
            settings_store,
            data_directory: data,
            dictionaries,
            first_key: OnceLock::new(),
            coordinator: OnceLock::new(),
            next_token: AtomicUsize::new(1),
        }
    }

    /// A fresh token for a new engine object — a counter, never an address
    /// (`ComposingSessionCoordinator` header).
    pub fn allocate_token(&self) -> ContextToken {
        ContextToken(self.next_token.fetch_add(1, Ordering::Relaxed))
    }

    /// The coordinator only if a key has already built it — for lifecycle
    /// callbacks that must not bring the engine up (focus loss, destroy).
    pub fn coordinator_if_built(&self) -> Option<&Mutex<ComposingSessionCoordinator>> {
        self.coordinator.get()
    }

    /// The coordinator, locked. Every D-Bus method on the engine runs in
    /// order on one executor (`spawn = false`), so the lock is never
    /// contended by a re-entrant call; a poisoned lock is recovered because
    /// a panic inside one key must not end the engine for the session.
    pub fn lock_coordinator(&self) -> MutexGuard<'_, ComposingSessionCoordinator> {
        self.coordinator()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The coordinator, built on first use; picks reach the engine only where
    /// this process has a data directory. `prepare_for_first_key` must have
    /// run.
    pub fn coordinator(&self) -> &Mutex<ComposingSessionCoordinator> {
        self.coordinator.get_or_init(|| {
            let settings: Arc<dyn SettingsProvider> = Arc::clone(&self.settings) as _;
            Mutex::new(ComposingSessionCoordinator::for_desktop(
                settings,
                self.data_directory.is_some(),
            ))
        })
    }

    /// Everything the first CONSUMED key needs, once per process: the
    /// lexicon installed from the dictionaries directory, the engine told to
    /// open the user data (it finishes on a thread of its own), the shortcut
    /// registries reconciled. Idempotent.
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

    /// Loads the dictionary data the package installed. Failures are logged
    /// and left alone: an engine with no lexicon types romanization and
    /// suggests nothing.
    fn install_lexicon(&self) -> Option<LexiconInstallStats> {
        let artifacts = match DictionaryArtifacts::locate(&self.dictionaries) {
            Ok(artifacts) => artifacts,
            Err(error) => {
                log::error!(
                    "lexicon.not_installed directory={} error={error}",
                    self.dictionaries.display()
                );
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

    /// One write to `settings.json` from the key path — under the file's
    /// lock, on the document as it is now, so a write from the settings
    /// window is not lost (`taigi-windows-tsf::runtime::update_settings`).
    /// `what` names the write in the log. Answers whether there was a store
    /// to write to; a write that failed is logged and still answers true,
    /// since what the chord does next does not depend on the disk.
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

    /// The language the UI strings are drawn in: the setting, or the
    /// machine's when it says `system`.
    pub fn display_language(&self) -> DisplayLanguage {
        let tag = self.settings.current().string(&keys::DISPLAY_LANGUAGE);
        DisplayLanguage::from_tag(&tag).effective(&system_locale())
    }

    pub fn strings(&self) -> StringResolver {
        StringResolver::new(self.display_language())
    }

    /// The launch-time pass over both shortcut registries; writes only when
    /// something changed.
    fn reconcile_shortcuts(&self) {
        let Some(store) = &self.settings_store else {
            return;
        };
        if let Err(error) = store.update(ShortcutConflicts::resolve_across_registries) {
            log::error!("shortcuts.reconcile_failed error={error}");
        }
    }
}

/// The dictionary stamp: the desktop train's version in the macOS bundle
/// shape (`taigi-windows-tsf::runtime::dictionary_version`).
pub fn dictionary_version() -> u32 {
    taigi_desktop_core::dictionary_artifacts::dictionary_version(env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stamp_is_this_crate_version_in_the_macos_bundle_shape() {
        let mut parts = env!("CARGO_PKG_VERSION")
            .split('.')
            .map(|part| part.parse::<u32>().expect("numeric"));
        let (major, minor, patch) = (
            parts.next().unwrap(),
            parts.next().unwrap(),
            parts.next().unwrap(),
        );
        assert_eq!(dictionary_version(), major * 10_000 + minor * 100 + patch);
    }
}
