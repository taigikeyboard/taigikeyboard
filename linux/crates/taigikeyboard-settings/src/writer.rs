//! The window's copy of `settings.json` and the one way it changes: one
//! atomic edit (lock, load, mutate, save) through the same store the engine
//! writes, then this copy follows the file. Port of the Windows
//! `settings_writer.rs`; the read-only mode is the same named rule (roadmap
//! W2 / L7): no per-user directory ⇒ the shipped defaults, held in memory
//! and never a file, every write refused with a banner.

use std::sync::Arc;
use std::time::Duration;
use taigi_desktop_core::settings::SettingsDocument;
use taigi_desktop_core::strings::StringResolver;
use taigi_desktop_storage::{created, LiveSettings, SettingsFileStore};
use taigi_linux_platform::UserDirectories;

/// How long an idle window waits before reading the file again: one `stat`
/// a second is nothing, and a chord's effect showing within a second reads
/// as live (roadmap L9).
pub const REFRESH_INTERVAL: Duration = Duration::from_secs(1);

/// What the banner says when there is no user directory at all: the
/// variables the XDG lookup wanted.
const NO_DIRECTORIES_DETAIL: &str = "HOME / XDG_CONFIG_HOME";

pub struct SettingsWriter {
    /// `None` in read-only mode: nothing on disk is read or tracked.
    live: Option<LiveSettings>,
    document: Arc<SettingsDocument>,
    write_failure: Option<String>,
}

impl SettingsWriter {
    /// Over the user's configuration directory, or read-only when there is
    /// none or it cannot be created — with the real reason as the banner's
    /// detail.
    pub fn at_launch() -> Self {
        let Some(directories) = UserDirectories::resolve() else {
            log::error!("settings.no_user_directories");
            return Self::read_only(NO_DIRECTORIES_DETAIL.to_owned());
        };
        match created(directories.config) {
            Ok(directory) => Self::new(SettingsFileStore::new(&directory)),
            Err(error) => {
                log::error!("settings.no_config_directory error={error}");
                Self::read_only(error.to_string())
            }
        }
    }

    pub fn new(store: SettingsFileStore) -> Self {
        let live = LiveSettings::new(store);
        let document = live.refresh_if_changed();
        Self {
            live: Some(live),
            document,
            write_failure: None,
        }
    }

    /// The shipped defaults, refusing every write; `detail` is what the
    /// banner shows after the message.
    pub fn read_only(detail: String) -> Self {
        Self {
            live: None,
            document: Arc::new(SettingsDocument::default()),
            write_failure: Some(detail),
        }
    }

    pub fn document(&self) -> &SettingsDocument {
        &self.document
    }

    pub fn strings(&self) -> StringResolver {
        crate::presentation::strings_for(&self.document)
    }

    pub fn is_read_only(&self) -> bool {
        self.live.is_none()
    }

    /// The last write that failed, shown as a banner until a write
    /// succeeds: a control that snaps back with no word looks broken.
    pub fn write_failure(&self) -> Option<&str> {
        self.write_failure.as_deref()
    }

    /// Re-reads the file if its fingerprint moved, so a change made outside
    /// — the engine's own write for a chord or a menu row — shows without
    /// a restart. Answers whether the document changed.
    pub fn refresh(&mut self) -> bool {
        let Some(live) = &self.live else {
            return false;
        };
        let document = live.refresh_if_changed();
        let changed = !Arc::ptr_eq(&document, &self.document);
        self.document = document;
        changed
    }

    /// One atomic edit, then this copy follows the file. A failed write is
    /// reported, not swallowed; a read-only window refuses without touching
    /// its banner.
    pub fn update(&mut self, mutate: impl FnOnce(&mut SettingsDocument)) {
        let Some(live) = &self.live else {
            return;
        };
        match live.store().update(mutate) {
            Ok(_) => self.write_failure = None,
            Err(error) => {
                log::error!("settings.update_failed error={error}");
                self.write_failure = Some(error.to_string());
            }
        }
        self.refresh();
    }
}
