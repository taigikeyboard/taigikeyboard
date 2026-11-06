//! How the Linux input method builds the shared per-process runtime
//! (`taigi_desktop_core::runtime::DesktopRuntime`): over the XDG
//! directories, with no AppContainer degradation — a Linux session without
//! a home directory has no user to learn from, and nothing is learned. Plus
//! the one thing only this side needs: a token per engine object.
//!
//! Lazy by contract: `probe` only resolves paths and reads the settings
//! file; the engine opens the user data and the lexicon loads on the first
//! key an engine CONSUMES (`prepare_for_first_key`), never at `CreateEngine`.

use std::ops::Deref;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use taigi_desktop_core::composing::{ContextToken, SystemClock};
use taigi_desktop_core::runtime::{DesktopRuntime, RuntimeParts};
use taigi_desktop_core::settings::{SettingsDocument, SettingsProvider, StaticSettingsProvider};
use taigi_desktop_storage::{created, LiveSettings, SettingsFileStore};
use taigi_linux_platform::{dictionaries_directory, system_locale, UserDirectories};

pub use taigi_desktop_core::runtime::FirstKeySetup;

pub struct Runtime {
    shared: DesktopRuntime,
    next_token: AtomicUsize,
}

/// Everything the shared runtime answers — settings, the coordinator, the
/// first-key setup, the display language — read straight through.
impl Deref for Runtime {
    type Target = DesktopRuntime;

    fn deref(&self) -> &DesktopRuntime {
        &self.shared
    }
}

/// A runtime over a temporary XDG tree with no dictionaries: settings are
/// live (a file), the lexicon is absent, and nothing is learned — the
/// engine's user data is one per process, and several of these share a
/// test binary.
#[cfg(test)]
pub(crate) fn temporary_runtime() -> (tempfile::TempDir, Runtime) {
    let directory = tempfile::tempdir().expect("tempdir");
    let runtime = Runtime::build(
        Some(UserDirectories {
            config: directory.path().join("config"),
            data: directory.path().join("data"),
        }),
        directory.path().join("no-dictionaries"),
        /* is_learning_wanted */ false,
    );
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
        Self::build(
            directories,
            dictionaries,
            /* is_learning_wanted */ true,
        )
    }

    fn build(
        directories: Option<UserDirectories>,
        dictionaries: PathBuf,
        is_learning_wanted: bool,
    ) -> Self {
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
            })
            .filter(|_| is_learning_wanted);
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
        let shared = DesktopRuntime::new(RuntimeParts {
            settings,
            settings_store: settings_store.map(|store| Box::new(store) as _),
            data_directory: data,
            dictionaries: Box::new(move || Some(dictionaries.clone())),
            dictionary_version: dictionary_version(),
            system_locale: Box::new(system_locale),
            platform: taigi_linux_platform::DESKTOP_PLATFORM,
            clock: Arc::new(SystemClock),
        });
        Self {
            shared,
            next_token: AtomicUsize::new(1),
        }
    }

    /// A fresh token for a new engine object — a counter, never an address
    /// (`ComposingSessionCoordinator` header).
    pub fn allocate_token(&self) -> ContextToken {
        ContextToken(self.next_token.fetch_add(1, Ordering::Relaxed))
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
