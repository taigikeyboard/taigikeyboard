//! The per-process state every engine object of a desktop input method
//! shares: the live settings, the directory the engine keeps the user's data
//! in, the engine's lexicon, and the one composing coordinator. Each shell
//! builds it from its own platform lookups (`%APPDATA%` and the DLL's install
//! directory on Windows, the XDG directories on Linux) and keeps nothing
//! else of the kind.
//!
//! Lazy by contract (Windows roadmap W3): building one only resolves paths
//! and reads the settings file; the engine opens the user data and the
//! lexicon loads on the first key an engine CONSUMES
//! ([`DesktopRuntime::prepare_for_first_key`]), never when the input method
//! is merely activated. macOS, whose stores have always opened at launch,
//! asks for it from `applicationDidFinishLaunching` (inventory C6).

use crate::composing::ComposingSessionCoordinator;
use crate::dictionary_artifacts::DictionaryArtifacts;
use crate::engine::{lexicon_install, user_data, LexiconInstallStats};
use crate::keys::ShortcutConflicts;
use crate::platform::DesktopPlatform;
use crate::settings::{SettingsDocument, SettingsProvider, SettingsStore};
use crate::strings::{DisplayLanguage, StringResolver};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

/// What a shell hands in to build a [`DesktopRuntime`].
pub struct RuntimeParts {
    /// What every consumer reads settings through, at the moment it needs
    /// them (behavioural invariant §11).
    pub settings: Arc<dyn SettingsProvider + Send + Sync>,
    /// Where writes from the key path go; `None` = nowhere to write (the
    /// shipped defaults).
    pub settings_store: Option<Box<dyn SettingsStore>>,
    /// Where the engine keeps the user's data; `None` = nothing is learned.
    pub data_directory: Option<PathBuf>,
    /// The dictionaries directory, resolved on the first consumed key.
    pub dictionaries: Box<dyn Fn() -> Option<PathBuf> + Send + Sync>,
    /// The stamp the lexicon is installed under — the shipping crate's
    /// version, so each shell passes its own.
    pub dictionary_version: u32,
    /// The machine's UI language as a BCP-47 tag, read at every call. A
    /// closure, so a shell that is handed the tag (macOS `Configure`) holds
    /// it in its own runtime rather than in a process static.
    pub system_locale: Box<dyn Fn() -> String + Send + Sync>,
    /// Which desktop this is — the shell's own constant.
    pub platform: DesktopPlatform,
}

/// One per process: what every engine object of the input method reads,
/// and the one engine they drive.
pub struct DesktopRuntime {
    pub settings: Arc<dyn SettingsProvider + Send + Sync>,
    settings_store: Option<Box<dyn SettingsStore>>,
    data_directory: Option<PathBuf>,
    dictionaries: Box<dyn Fn() -> Option<PathBuf> + Send + Sync>,
    dictionary_version: u32,
    system_locale: Box<dyn Fn() -> String + Send + Sync>,
    platform: DesktopPlatform,
    first_key: OnceLock<FirstKeySetup>,
    /// The one composing engine driver per process, keyed by context token.
    /// Held for the length of one key, never across a call back into the
    /// host or a signal emission.
    coordinator: OnceLock<Mutex<ComposingSessionCoordinator>>,
}

/// What the first handled key set up, kept so later keys skip it.
#[derive(Clone, Copy, Debug)]
pub struct FirstKeySetup {
    pub lexicon: Option<LexiconInstallStats>,
}

impl DesktopRuntime {
    pub fn new(parts: RuntimeParts) -> Self {
        Self {
            settings: parts.settings,
            settings_store: parts.settings_store,
            data_directory: parts.data_directory,
            dictionaries: parts.dictionaries,
            dictionary_version: parts.dictionary_version,
            system_locale: parts.system_locale,
            platform: parts.platform,
            first_key: OnceLock::new(),
            coordinator: OnceLock::new(),
        }
    }

    /// Whether key-path writes have a settings file to go to.
    pub fn has_settings_store(&self) -> bool {
        self.settings_store.is_some()
    }

    /// Whether the engine keeps the user's data in this process.
    pub fn is_learning(&self) -> bool {
        self.data_directory.is_some()
    }

    /// One write to `settings.json` from the key path — under the file's
    /// lock, on the document as it is now, so a write from the settings
    /// window is not lost. `what` names the write in the log. Answers
    /// whether there was a store to write to; a write that failed is logged
    /// and still answers true, since what the chord does next does not
    /// depend on the disk.
    pub fn update_settings(&self, what: &str, mutate: impl FnOnce(&mut SettingsDocument)) -> bool {
        let Some(store) = &self.settings_store else {
            log::warn!("settings.no_store what={what}");
            return false;
        };
        let mut mutate = Some(mutate);
        let result = store.update_document(&mut |document| {
            if let Some(mutate) = mutate.take() {
                mutate(document);
            }
        });
        if let Err(error) = result {
            log::error!("settings.update_failed what={what} error={error}");
        }
        true
    }

    /// The coordinator only if a key has already built it — for lifecycle
    /// callbacks that must not bring the engine up (focus loss, teardown).
    pub fn coordinator_if_built(&self) -> Option<&Mutex<ComposingSessionCoordinator>> {
        self.coordinator.get()
    }

    /// The coordinator locked without waiting: `None` when no key has built
    /// it yet OR when it is already held (a host that re-entered us from
    /// inside our own session). Never blocking — a callback that waited on
    /// the engine would deadlock the session holding it.
    pub fn try_coordinator(&self) -> Option<MutexGuard<'_, ComposingSessionCoordinator>> {
        self.coordinator_if_built()?.try_lock().ok()
    }

    /// The coordinator, locked and waited for — for a shell whose calls
    /// never re-enter (one executor, in order). A poisoned lock is recovered:
    /// a panic inside one key must not end the engine for the session.
    pub fn lock_coordinator(&self) -> MutexGuard<'_, ComposingSessionCoordinator> {
        self.coordinator()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// The coordinator, built on first use; the engine keeps what it learns
    /// only where this process has a data directory (`prepare_for_first_key`
    /// opens it there). `prepare_for_first_key` must have run.
    pub fn coordinator(&self) -> &Mutex<ComposingSessionCoordinator> {
        self.coordinator.get_or_init(|| {
            let settings: Arc<dyn SettingsProvider> = Arc::clone(&self.settings) as _;
            Mutex::new(ComposingSessionCoordinator::for_desktop(settings))
        })
    }

    /// Everything the first CONSUMED key needs, once per process (never for
    /// a key merely observed — the classifier decides first): the lexicon
    /// installed from the dictionaries directory, the engine told to open
    /// the user data (it finishes on a thread of its own), and the shortcut
    /// registries reconciled where the shell has a settings store (Windows,
    /// Linux; macOS reconciles its own in Swift). Idempotent. macOS calls
    /// it at launch rather than on the first key (inventory C6).
    pub fn prepare_for_first_key(&self) -> &FirstKeySetup {
        self.first_key.get_or_init(|| {
            let lexicon = self.install_lexicon();
            if let Some(directory) = &self.data_directory {
                // The engine puts the stores in use before this returns and
                // finishes opening on a thread of its own.
                user_data::open(directory, self.platform);
            }
            self.reconcile_shortcuts();
            FirstKeySetup { lexicon }
        })
    }

    /// Loads the dictionary data that shipped. Failures are logged and left
    /// alone: an engine with no lexicon types romanization and suggests
    /// nothing.
    fn install_lexicon(&self) -> Option<LexiconInstallStats> {
        let directory = (self.dictionaries)()?;
        let artifacts = match DictionaryArtifacts::locate(&directory) {
            Ok(artifacts) => artifacts,
            Err(error) => {
                log::error!(
                    "lexicon.not_installed directory={} error={error}",
                    directory.display()
                );
                return None;
            }
        };
        let stats = lexicon_install(&artifacts, self.dictionary_version);
        match &stats {
            Some(stats) => log::info!(
                "lexicon.installed records={} prefix_entries={} version={}",
                stats.dictionary_record_count,
                stats.prefix_index_entry_count,
                self.dictionary_version
            ),
            None => log::error!("lexicon.install_returned_nothing"),
        }
        stats
    }

    /// The launch-time pass over both shortcut registries; writes only when
    /// something changed (the store skips a save whose revision did not
    /// move).
    fn reconcile_shortcuts(&self) {
        let Some(store) = &self.settings_store else {
            return;
        };
        if let Err(error) = store.update_document(&mut |document| {
            ShortcutConflicts::resolve_across_registries(document, self.platform)
        }) {
            log::error!("shortcuts.reconcile_failed error={error}");
        }
    }

    /// The language the UI strings are drawn in: the setting, or the
    /// machine's when it says `system`.
    pub fn display_language(&self) -> DisplayLanguage {
        self.settings
            .current()
            .effective_display_language(&(self.system_locale)())
    }

    pub fn strings(&self) -> StringResolver {
        StringResolver::new(self.display_language())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::test_support::TEST_PLATFORM;
    use crate::settings::{keys, StaticSettingsProvider};
    use std::sync::atomic::{AtomicUsize, Ordering};

    /// A store that counts its writes and fails when told to.
    struct CountingStore {
        writes: Arc<AtomicUsize>,
        fails: bool,
    }

    impl SettingsStore for CountingStore {
        fn update_document(
            &self,
            mutate: &mut dyn FnMut(&mut SettingsDocument),
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            if self.fails {
                return Err("locked".into());
            }
            mutate(&mut SettingsDocument::default());
            self.writes.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    fn runtime(store: Option<CountingStore>) -> DesktopRuntime {
        DesktopRuntime::new(RuntimeParts {
            settings: Arc::new(StaticSettingsProvider::new(SettingsDocument::default())),
            settings_store: store.map(|store| Box::new(store) as Box<dyn SettingsStore>),
            data_directory: None,
            dictionaries: Box::new(|| None),
            dictionary_version: 1,
            system_locale: Box::new(|| "ja-JP".to_owned()),
            platform: TEST_PLATFORM,
        })
    }

    #[test]
    fn a_key_path_write_answers_whether_there_was_a_store() {
        assert!(!runtime(None).update_settings("test", |_| {}));
        let writes = Arc::new(AtomicUsize::new(0));
        let with_store = runtime(Some(CountingStore {
            writes: Arc::clone(&writes),
            fails: false,
        }));
        let mut ran = 0;
        assert!(with_store.update_settings("test", |_| ran += 1));
        assert_eq!((ran, writes.load(Ordering::Relaxed)), (1, 1));
        // A failed write is logged; the chord still had somewhere to write.
        let failing = runtime(Some(CountingStore {
            writes: Arc::new(AtomicUsize::new(0)),
            fails: true,
        }));
        assert!(failing.update_settings("test", |_| {}));
    }

    #[test]
    fn the_first_key_setup_runs_once_and_reconciles_the_shortcuts() {
        let writes = Arc::new(AtomicUsize::new(0));
        let runtime = runtime(Some(CountingStore {
            writes: Arc::clone(&writes),
            fails: false,
        }));
        assert!(!runtime.is_learning());
        // No dictionaries directory: no lexicon, nothing else attempted.
        assert!(runtime.prepare_for_first_key().lexicon.is_none());
        runtime.prepare_for_first_key();
        assert_eq!(writes.load(Ordering::Relaxed), 1, "reconciled once");
    }

    #[test]
    fn the_coordinator_is_not_built_by_a_look_and_not_waited_for_when_held() {
        let runtime = runtime(None);
        assert!(runtime.coordinator_if_built().is_none());
        assert!(runtime.try_coordinator().is_none(), "nothing built yet");
        let held = runtime.lock_coordinator();
        assert!(runtime.try_coordinator().is_none(), "held elsewhere");
        drop(held);
        assert!(runtime.try_coordinator().is_some());
    }

    #[test]
    fn system_follows_the_locale_the_shell_reads() {
        let runtime = runtime(None);
        // trace: DISPLAY_LANGUAGE defaults to "system"; the test locale is ja-JP.
        assert_eq!(
            runtime.settings.current().string(&keys::DISPLAY_LANGUAGE),
            "system"
        );
        assert_eq!(runtime.display_language(), DisplayLanguage::Japanese);
    }
}
