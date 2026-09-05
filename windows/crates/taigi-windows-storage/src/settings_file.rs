//! `settings.json` on disk: atomic replace on write, last-known-good on a
//! bad read, and a cheap mtime/size change detector so the TIP and the
//! settings window (two processes) see one document. Roadmap W10; the
//! reload-by-mtime pattern follows rakukan `engine/config.rs:330-443`.

// settings.json 落地 — tmp+rename 原子寫入、讀壞保留舊值、mtime/size 偵測變更後才採用新 revision。

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime};
use taigi_windows_core::settings::{SettingsDocument, SettingsProvider};

#[derive(Debug, thiserror::Error)]
pub enum SettingsFileError {
    #[error("could not read {path}: {source}")]
    Read {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("could not write {path}: {source}")]
    Write {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("{path} is not a settings document: {source}")]
    Parse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
}

/// What changed on disk without reading the file: the modification time
/// and the size. `%APPDATA%` lives on the system volume, which is NTFS
/// (100 ns `FILETIME`), so two atomic replaces inside one tick with the
/// same length is not a case this has to survive; on a 2-second FAT stamp it
/// would be, and FAT is not a supported profile volume. NAMED LIMITATION.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Fingerprint {
    modified: Option<SystemTime>,
    len: u64,
}

impl Fingerprint {
    fn of(path: &Path) -> Option<Self> {
        let metadata = fs::metadata(path).ok()?;
        Some(Self {
            modified: metadata.modified().ok(),
            len: metadata.len(),
        })
    }
}

/// The file, with the write discipline both processes share.
#[derive(Clone, Debug)]
pub struct SettingsFileStore {
    path: PathBuf,
}

impl SettingsFileStore {
    pub const FILE_NAME: &'static str = "settings.json";
    /// A rename onto a file the other process is reading fails with a
    /// sharing violation on Windows; a few short retries outlast any read.
    const REPLACE_ATTEMPTS: u32 = 5;
    const REPLACE_RETRY_DELAY: Duration = Duration::from_millis(20);
    /// How long `update` waits for the other process's load → save to
    /// finish before giving up, and how old a lock file has to be before it
    /// is treated as left behind by a crash.
    const LOCK_ATTEMPTS: u32 = 50;
    const LOCK_RETRY_DELAY: Duration = Duration::from_millis(20);
    const STALE_LOCK_AGE: Duration = Duration::from_secs(10);

    pub fn new(directory: &Path) -> Self {
        Self {
            path: directory.join(Self::FILE_NAME),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The document on disk. A missing file is the shipped defaults; a
    /// present-but-bad file is an error the caller decides about (the
    /// provider keeps last-known-good, the settings window tells the user).
    pub fn load(&self) -> Result<SettingsDocument, SettingsFileError> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(SettingsDocument::default())
            }
            Err(source) => {
                return Err(SettingsFileError::Read {
                    path: self.path.clone(),
                    source,
                })
            }
        };
        SettingsDocument::from_json(&text).map_err(|source| SettingsFileError::Parse {
            path: self.path.clone(),
            source,
        })
    }

    /// Writes `document` whole: to a sibling temp file, then renamed over
    /// the real one, so a reader never sees a half-written file.
    pub fn save(&self, document: &SettingsDocument) -> Result<(), SettingsFileError> {
        let write_error = |source| SettingsFileError::Write {
            path: self.path.clone(),
            source,
        };
        if let Some(directory) = self.path.parent() {
            fs::create_dir_all(directory).map_err(write_error)?;
        }
        // Unique per save, not per process: two saves in one process (two
        // panes' worker threads) must not share a temp file.
        static SAVE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let sequence = SAVE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = self
            .path
            .with_extension(format!("json.{}.{sequence}.tmp", std::process::id()));
        fs::write(&temporary, document.to_json()).map_err(write_error)?;
        let mut attempt = 0;
        loop {
            match fs::rename(&temporary, &self.path) {
                Ok(()) => return Ok(()),
                Err(error) if attempt + 1 < Self::REPLACE_ATTEMPTS => {
                    log::warn!("settings.replace_retry attempt={attempt} error={error}");
                    attempt += 1;
                    thread::sleep(Self::REPLACE_RETRY_DELAY);
                }
                Err(source) => {
                    fs::remove_file(&temporary).ok();
                    return Err(write_error(source));
                }
            }
        }
    }

    /// Load → mutate → save under a lock, the one shape every settings
    /// mutation takes. CONTRACT: `mutate` changes the document only through
    /// its typed setters (which bump `revision`) and never touches
    /// `revision` itself — the save is skipped when the revision did not move.
    /// The rest of the shape: a write always starts from the file (the other
    /// process may have written since), the document's own per-mutation
    /// revision bump is what the TIP compares, and the lock keeps two
    /// updaters — the settings window and the TIP's launch pass — from
    /// losing each other's write between the load and the save.
    pub fn update(
        &self,
        mutate: impl FnOnce(&mut SettingsDocument),
    ) -> Result<SettingsDocument, SettingsFileError> {
        let _guard = self.lock_for_update()?;
        let mut document = self.load()?;
        let revision_before = document.revision;
        mutate(&mut document);
        // A mutation that changed nothing writes nothing: the file keeps its
        // fingerprint and no reader re-parses it.
        if document.revision != revision_before {
            self.save(&document)?;
        }
        Ok(document)
    }

    /// A cross-process lock: `settings.json.lock` created exclusively,
    /// removed when the guard drops. A lock older than `STALE_LOCK_AGE` was
    /// left by a crash and is taken over.
    fn lock_for_update(&self) -> Result<UpdateLock, SettingsFileError> {
        let lock_path = self.path.with_extension("json.lock");
        let write_error = |source| SettingsFileError::Write {
            path: lock_path.clone(),
            source,
        };
        if let Some(directory) = lock_path.parent() {
            fs::create_dir_all(directory).map_err(write_error)?;
        }
        let mut attempt = 0;
        loop {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&lock_path)
            {
                Ok(_) => return Ok(UpdateLock { path: lock_path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    let is_stale = fs::metadata(&lock_path)
                        .and_then(|metadata| metadata.modified())
                        .ok()
                        .and_then(|modified| SystemTime::now().duration_since(modified).ok())
                        .is_some_and(|age| age > Self::STALE_LOCK_AGE);
                    if is_stale {
                        log::warn!("settings.lock_stale path={}", lock_path.display());
                        fs::remove_file(&lock_path).ok();
                        continue;
                    }
                    if attempt + 1 >= Self::LOCK_ATTEMPTS {
                        return Err(write_error(error));
                    }
                    attempt += 1;
                    thread::sleep(Self::LOCK_RETRY_DELAY);
                }
                Err(error) => return Err(write_error(error)),
            }
        }
    }
}

/// Removes the lock file when the update is over, however it ended.
struct UpdateLock {
    path: PathBuf,
}

impl Drop for UpdateLock {
    fn drop(&mut self) {
        fs::remove_file(&self.path).ok();
    }
}

/// The TIP's view of the file: the parsed document, re-read only when the
/// file's fingerprint changed, and never replaced by a document that fails
/// to parse. `current()` is what every consumer calls at the moment it
/// needs a value (invariant §11); it costs one `stat`.
pub struct LiveSettings {
    store: SettingsFileStore,
    state: Mutex<LiveState>,
}

struct LiveState {
    document: Arc<SettingsDocument>,
    fingerprint: Option<Fingerprint>,
}

impl LiveSettings {
    pub fn new(store: SettingsFileStore) -> Self {
        let fingerprint = Fingerprint::of(store.path());
        let document = match store.load() {
            Ok(document) => document,
            Err(error) => {
                log::error!("settings.initial_load_failed error={error}");
                SettingsDocument::default()
            }
        };
        Self {
            store,
            state: Mutex::new(LiveState {
                document: Arc::new(document),
                fingerprint,
            }),
        }
    }

    pub fn store(&self) -> &SettingsFileStore {
        &self.store
    }

    /// Re-reads the file if its fingerprint moved. A parse failure keeps
    /// the last good document AND the old fingerprint, so the next call
    /// tries again rather than adopting the bad file's stamp.
    pub fn refresh_if_changed(&self) -> Arc<SettingsDocument> {
        let mut state = self
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let fingerprint = Fingerprint::of(self.store.path());
        if fingerprint == state.fingerprint {
            return Arc::clone(&state.document);
        }
        match self.store.load() {
            Ok(document) => {
                state.fingerprint = fingerprint;
                // The revision is the adoption truth: a rewrite that changed
                // nothing (same revision, same content) keeps the shared
                // document so consumers holding it see no churn. A rewrite
                // carrying an OLDER revision — a restored backup — is still
                // the file on disk and IS adopted: only the parse gate
                // protects last-known-good. POLICY, stated here.
                if document.revision == state.document.revision && document == *state.document {
                    return Arc::clone(&state.document);
                }
                log::info!(
                    "settings.reload from_revision={} to_revision={}",
                    state.document.revision,
                    document.revision
                );
                state.document = Arc::new(document);
            }
            Err(error) => log::error!("settings.reload_failed error={error}"),
        }
        Arc::clone(&state.document)
    }
}

impl SettingsProvider for LiveSettings {
    fn current(&self) -> Arc<SettingsDocument> {
        self.refresh_if_changed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taigi_windows_core::settings::keys;

    #[test]
    fn a_missing_file_is_the_defaults_and_a_save_round_trips() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsFileStore::new(directory.path());
        assert_eq!(store.load().unwrap(), SettingsDocument::default());
        let saved = store
            .update(|document| document.set_bool(&keys::IS_AUTO_SPACE_ENABLED, false))
            .unwrap();
        assert_eq!(saved.revision, 1);
        assert_eq!(store.load().unwrap(), saved);
        assert!(!directory.path().join("settings.json.tmp").exists());
        assert_eq!(
            fs::read_dir(directory.path()).unwrap().count(),
            1,
            "the temp file is gone after the rename"
        );
    }

    #[test]
    fn live_settings_reload_on_change_and_keep_last_known_good() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsFileStore::new(directory.path());
        let live = LiveSettings::new(store.clone());
        assert_eq!(live.current().revision, 0);
        // Another process writes.
        let mut document = SettingsDocument::default();
        document.set_bool(&keys::IS_AUTO_SPACE_ENABLED, false);
        // A same-tick rewrite can share the mtime; a different length is enough.
        store.save(&document).unwrap();
        let seen = live.current();
        assert_eq!(seen.revision, 1);
        assert!(!seen.bool(&keys::IS_AUTO_SPACE_ENABLED));
        // Garbage on disk: the last good document stays.
        fs::write(store.path(), "{ not json").unwrap();
        assert_eq!(live.current().revision, 1);
        // And a later good write is adopted (the bad fingerprint was not kept).
        document.set_bool(&keys::IS_AUTO_SPACE_ENABLED, true);
        store.save(&document).unwrap();
        assert_eq!(live.current().revision, 2);
        // An identical rewrite keeps the very same shared document.
        let before = live.current();
        store.save(&document).unwrap();
        assert!(
            Arc::ptr_eq(&before, &live.current()),
            "same revision + content = no churn"
        );
        // A restored backup with an older revision is adopted (policy).
        store.save(&SettingsDocument::default()).unwrap();
        assert_eq!(live.current().revision, 0);
    }

    #[test]
    fn update_takes_a_lock_and_a_stale_lock_is_taken_over() {
        let directory = tempfile::tempdir().unwrap();
        let store = SettingsFileStore::new(directory.path());
        let lock = store.path().with_extension("json.lock");
        fs::write(&lock, "").unwrap();
        // Fresh lock held by "another process": update waits then gives up.
        let started = std::time::Instant::now();
        assert!(store.update(|_| {}).is_err());
        assert!(
            started.elapsed() >= Duration::from_millis(500),
            "it waited for the other writer"
        );
        // A lock older than the stale age is taken over.
        let old = SystemTime::now() - Duration::from_secs(60);
        fs::File::options()
            .write(true)
            .open(&lock)
            .unwrap()
            .set_modified(old)
            .unwrap();
        assert!(store.update(|_| {}).is_ok());
        assert!(!lock.exists(), "the lock is released after the update");
        assert!(!store.path().exists(), "a no-op update writes no file");
        store
            .update(|document| document.set_bool(&keys::IS_AUTO_SPACE_ENABLED, false))
            .unwrap();
        assert!(store.path().exists());
        assert!(fs::read_dir(directory.path()).unwrap().all(|entry| !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .ends_with(".tmp")));
    }
}
