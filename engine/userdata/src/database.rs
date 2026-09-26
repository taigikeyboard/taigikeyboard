//! Opens a learning database and serialises every access to it. Port of
//! `Storage/UserDataDatabase.swift`: one serial worker owns the file — the
//! open, every queued write and every `perform` run on it in order, as on
//! the macOS `DispatchQueue` — while reads answer on the caller's thread
//! through a second, read-only connection that never waits.

use rusqlite::{Connection, OpenFlags};
use std::cell::Cell;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, SyncSender, TrySendError};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::Duration;

/// Why work the user asked for could not run.
#[derive(Debug, thiserror::Error)]
pub enum UserDataDatabaseError {
    /// The file is not open — still opening at launch, or failed to open.
    /// Surfaced so the UI can say the action did not happen.
    #[error("{0} is not open")]
    NotOpen(String),
    /// `perform` was called from inside a job already running on the
    /// store's worker — it would wait for itself. A programming error made
    /// loud rather than a hang.
    #[error("{0}: perform called from inside the store's own worker")]
    Reentrant(String),
    /// The worker thread is gone (a panic inside a job took it down).
    #[error("{0}: the store's worker is no longer running")]
    WorkerGone(String),
    /// The file carries a `user_version` newer than any shape this engine
    /// knows — written by a later build. Left closed and untouched rather
    /// than migrated by guesswork (user-data-engine-roadmap U7).
    #[error("{0}: user_version {1} is newer than this build knows")]
    FutureVersion(String, i64),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    /// The pre-takeover copy could not be published.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// How a store's file journals — a platform parameter, not a store one
/// (user-data-engine-roadmap U3). The schema and SQL are the same either way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JournalMode {
    /// Write-ahead log: the desktop, whose reader never waits on a writer.
    Wal,
    /// Rollback journal with `synchronous = NORMAL`: iOS, whose App Group
    /// files are shared by two processes and excluded from backup without
    /// `-wal` / `-shm` sidecars (behavioral-invariants §29).
    Delete,
}

impl JournalMode {
    fn pragma(self) -> &'static str {
        match self {
            Self::Wal => "WAL",
            Self::Delete => "DELETE",
        }
    }

    /// Under WAL a reader is never blocked by a writer, so it never waits.
    /// Under a rollback journal every commit briefly locks readers out; a
    /// short wait keeps a keystroke from losing its learned data to each
    /// commit, and a read that still times out answers `None` (neutral
    /// ranking) as before.
    fn reader_busy_timeout(self) -> Duration {
        match self {
            Self::Wal => Duration::ZERO,
            Self::Delete => Duration::from_millis(20),
        }
    }
}

/// `PRAGMA application_id` of a file the engine has taken over: its schema
/// converged and its derived data rebuilt by this crate. No platform's native
/// store sets an application id, so its absence means "not yet taken over"
/// whatever `user_version` says — three platforms number the custom
/// dictionary differently (user-data-engine-roadmap U7). ASCII `TAIG`.
pub const TAIGI_APPLICATION_ID: i32 = 0x5441_4947;

/// What opening one store's file needs to know about it.
#[derive(Clone, Copy)]
pub(crate) struct StoreSchema {
    /// Brings any known shape — a fresh file, the engine's own, or a native
    /// platform store's — to the current one. Shape-detecting and idempotent.
    pub apply: fn(&Connection) -> rusqlite::Result<()>,
    /// The highest `user_version` any known writer — this engine or a
    /// platform's native store — ever stamped on this file. Above it the
    /// file is from a later build and stays closed.
    pub max_known_version: i64,
    /// False when the takeover finishes later than the open: the custom
    /// dictionary is taken over once its search keys are re-derived.
    pub marks_takeover_on_open: bool,
}

/// True when the engine has already taken this file over.
pub(crate) fn is_taken_over(connection: &Connection) -> rusqlite::Result<bool> {
    let id: i32 = connection.pragma_query_value(None, "application_id", |row| row.get(0))?;
    Ok(id == TAIGI_APPLICATION_ID)
}

/// Records the takeover. Idempotent.
pub(crate) fn mark_taken_over(connection: &Connection) -> rusqlite::Result<()> {
    connection.pragma_update(None, "application_id", TAIGI_APPLICATION_ID)
}

type Job = Box<dyn FnOnce(Option<&Connection>) + Send>;

thread_local! {
    /// True while the worker is inside a job — what `perform` reads to
    /// refuse re-entry instead of deadlocking on its own barrier.
    static INSIDE_WORKER_JOB: Cell<bool> = const { Cell::new(false) };
}

/// The two connections and the "is it usable yet" answer every user-data
/// store needs.
///
/// Writes are queued to the worker and best-effort: a frequency or
/// association write is a side effect of a keystroke already rendered, and
/// losing one costs one increment of a counter — so the queue is BOUNDED
/// (roadmap W3) and a write that finds it full is dropped, never a keystroke
/// made to wait. Reads are synchronous on the caller's thread over their
/// own connection with `busy_timeout = 0` (W3): under WAL a reader is never
/// blocked by a writer, and the one case SQLite would still wait on
/// (recovery, checkpoint) answers busy → `None` → no learned data this
/// keystroke. NAMED DIVERGENCE from the Mac, whose sync read waits behind
/// the queued writes.
pub struct UserDataDatabase {
    name: &'static str,
    path: PathBuf,
    journal: JournalMode,
    schema: StoreSchema,
    reader: Arc<Mutex<Option<Connection>>>,
    is_open: Arc<AtomicBool>,
    worker: OnceLock<SyncSender<WorkerMessage>>,
    open_requested: AtomicBool,
}

impl UserDataDatabase {
    /// SQLite's own retry window for a lock another PROCESS holds — the
    /// settings window importing while the TIP records. Only the writer
    /// connection carries it; the reader's is zero.
    const WRITER_BUSY_TIMEOUT: Duration = Duration::from_millis(250);
    /// The writer's wait while the file opens and is taken over — never on a
    /// key path (a background open, or a page request that waits for it). Two
    /// processes sharing one container (the iOS app and its keyboard) can
    /// both be opening the same file on a first launch; a failed open leaves
    /// the store closed for the process's life, so the second one waits out
    /// the first one's takeover rather than giving up after 250 ms.
    /// The open puts the normal wait back once it is done.
    const OPEN_BUSY_TIMEOUT: Duration = Duration::from_secs(10);
    /// Queued jobs the worker may fall behind by before writes are dropped.
    /// A keystroke queues one; 256 is seconds of typing against a stalled
    /// disk, after which forgetting a count is the right degrade.
    const QUEUE_CAPACITY: usize = 256;

    pub(crate) fn new(
        name: &'static str,
        path: PathBuf,
        journal: JournalMode,
        schema: StoreSchema,
    ) -> Self {
        Self {
            name,
            path,
            journal,
            schema,
            reader: Arc::new(Mutex::new(None)),
            is_open: Arc::new(AtomicBool::new(false)),
            worker: OnceLock::new(),
            open_requested: AtomicBool::new(false),
        }
    }

    /// The file's name, for the "is not open" message the UI shows.
    fn file_name(&self) -> String {
        self.path.file_name().map_or_else(
            || self.path.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        )
    }

    /// True once the file is open and the schema has been applied.
    pub fn is_ready(&self) -> bool {
        self.is_open.load(Ordering::Acquire)
    }

    /// Opens the file and applies the schema, on the worker — so every write
    /// queued after this call runs after the open, whatever thread queued it.
    /// Called once at launch; a failure is logged and leaves the store
    /// permanently not-ready rather than retrying.
    pub fn open(&self) {
        if self.open_requested.swap(true, Ordering::AcqRel) {
            return;
        }
        let path = self.path.clone();
        let journal = self.journal;
        let schema = self.schema;
        let name = self.name;
        let reader = Arc::clone(&self.reader);
        let is_open = Arc::clone(&self.is_open);
        // The open is a job like any other, so it cannot be overtaken. It
        // carries its own connection out through the closure's return path:
        // the worker keeps whatever `Open` hands it.
        let job = WorkerMessage::Open(Box::new(move || {
            match Self::open_connections(&path, journal, schema) {
                Ok((writer, read_only)) => {
                    *reader
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(read_only);
                    is_open.store(true, Ordering::Release);
                    log::info!("storage.open name={name}");
                    Some(writer)
                }
                Err(error) => {
                    log::error!("storage.open_failed name={name} error={error}");
                    None
                }
            }
        }));
        self.worker().send(job).ok();
    }

    /// Opens synchronously — for tests and the settings window, which has
    /// nothing to do until the file is there.
    pub fn open_blocking(&self) {
        self.open();
        self.wait_for_queued_writes();
    }

    /// Opens the writer, refuses a file from a later build, snapshots a file
    /// the engine is about to take over, then converges its shape.
    fn open_connections(
        path: &Path,
        journal: JournalMode,
        schema: StoreSchema,
    ) -> Result<(Connection, Connection), UserDataDatabaseError> {
        // The directory too: the files are the engine's, so a fresh install
        // (Android's `databases/` before anything wrote there) must not
        // depend on a platform having made it first.
        if let Some(directory) = path.parent() {
            std::fs::create_dir_all(directory)?;
        }
        let writer = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        writer.busy_timeout(Self::OPEN_BUSY_TIMEOUT)?;
        let version = user_version(&writer)?;
        if version > schema.max_known_version {
            return Err(UserDataDatabaseError::FutureVersion(
                path.display().to_string(),
                version,
            ));
        }
        let taken_over = is_taken_over(&writer)?;
        if !taken_over && has_tables(&writer)? {
            snapshot_before_takeover(&writer, path)?;
        }
        writer.pragma_update(None, "journal_mode", journal.pragma())?;
        if journal == JournalMode::Delete {
            writer.pragma_update(None, "synchronous", "NORMAL")?;
        }
        (schema.apply)(&writer)?;
        // Written once: a header write per launch is a lock the iOS app and
        // its extension would contend for.
        if !taken_over && schema.marks_takeover_on_open {
            mark_taken_over(&writer)?;
        }
        // The open is over: from here on another process's lock is waited
        // out briefly.
        writer.busy_timeout(Self::WRITER_BUSY_TIMEOUT)?;
        let read_only = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        read_only.busy_timeout(journal.reader_busy_timeout())?;
        Ok((writer, read_only))
    }

    fn worker(&self) -> &SyncSender<WorkerMessage> {
        self.worker.get_or_init(|| {
            let (sender, receiver) = mpsc::sync_channel::<WorkerMessage>(Self::QUEUE_CAPACITY);
            let name = self.name;
            thread::Builder::new()
                .name(format!("taigi-store-{name}"))
                .spawn(move || {
                    let mut writer: Option<Connection> = None;
                    while let Ok(message) = receiver.recv() {
                        match message {
                            WorkerMessage::Open(open) => {
                                if writer.is_none() {
                                    writer = open();
                                }
                            }
                            WorkerMessage::Run(job) => {
                                INSIDE_WORKER_JOB.with(|inside| inside.set(true));
                                job(writer.as_ref());
                                INSIDE_WORKER_JOB.with(|inside| inside.set(false));
                            }
                            WorkerMessage::Barrier(done) => {
                                done.send(()).ok();
                            }
                        }
                    }
                })
                .ok();
            sender
        })
    }

    /// Queues a write. Silently does nothing until the store is ready, or
    /// when the queue is full — the same degrade as a write that fails: one
    /// unlearned commit.
    pub fn write(&self, body: impl FnOnce(&Connection) -> rusqlite::Result<()> + Send + 'static) {
        let name = self.name;
        let job: Job = Box::new(move |connection| {
            let Some(connection) = connection else { return };
            if let Err(error) = body(connection) {
                log::error!("storage.write_failed name={name} error={error}");
            }
        });
        match self.worker().try_send(WorkerMessage::Run(job)) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                log::warn!("storage.write_dropped name={name} reason=queue_full")
            }
            Err(TrySendError::Disconnected(_)) => {
                log::error!("storage.write_dropped name={name} reason=worker_gone")
            }
        }
    }

    /// Runs a read now on the read-only connection, or answers `None` when
    /// the store is not ready, another read holds the connection, or the
    /// read failed — never an empty result, so a caller can tell "nothing
    /// learned yet" from "could not look".
    pub fn read<T>(&self, body: impl FnOnce(&Connection) -> rusqlite::Result<T>) -> Option<T> {
        if !self.is_ready() {
            return None;
        }
        let guard = self.reader.try_lock().ok()?;
        let connection = guard.as_ref()?;
        match body(connection) {
            Ok(value) => Some(value),
            Err(error) => {
                log::error!("storage.read_failed name={} error={error}", self.name);
                None
            }
        }
    }

    /// Runs work the user asked for by name — an import, a delete, a clear
    /// — on the worker, in order with every write queued before it, and
    /// waits for the answer. Callers are the settings window's thread, never
    /// the keystroke path, and never a job already on the worker.
    pub fn perform<T, E>(
        &self,
        body: impl FnOnce(&Connection) -> Result<T, E> + Send + 'static,
    ) -> Result<T, E>
    where
        T: Send + 'static,
        E: From<UserDataDatabaseError> + Send + std::fmt::Display + 'static,
    {
        if INSIDE_WORKER_JOB.with(Cell::get) {
            return Err(UserDataDatabaseError::Reentrant(self.name.to_owned()).into());
        }
        let file_name = self.file_name();
        let name = self.name;
        let (reply, answer) = mpsc::channel::<Result<T, E>>();
        let job: Job = Box::new(move |connection| {
            let result = match connection {
                Some(connection) => body(connection),
                None => Err(UserDataDatabaseError::NotOpen(file_name).into()),
            };
            reply.send(result).ok();
        });
        if self.worker().send(WorkerMessage::Run(job)).is_err() {
            return Err(UserDataDatabaseError::WorkerGone(name.to_owned()).into());
        }
        match answer.recv() {
            Ok(result) => result.inspect_err(|error| {
                log::error!("storage.perform_failed name={name} error={error}")
            }),
            Err(_) => Err(UserDataDatabaseError::WorkerGone(name.to_owned()).into()),
        }
    }

    /// For a takeover that runs after the open (the custom dictionary's key
    /// re-derivation): the open's long lock wait while `taking_over`, the
    /// normal one after. A store that never opened has nothing to set.
    pub(crate) fn wait_long_for_locks(&self, taking_over: bool) {
        if !self.is_ready() {
            return;
        }
        let timeout = if taking_over {
            Self::OPEN_BUSY_TIMEOUT
        } else {
            Self::WRITER_BUSY_TIMEOUT
        };
        self.perform::<_, UserDataDatabaseError>(move |connection| {
            Ok(connection.busy_timeout(timeout)?)
        })
        .ok();
    }

    /// Lets every job queued so far land first, so a caller counting rows
    /// sees what the keystrokes before it recorded. A no-op from inside a
    /// job (everything before it has already run).
    pub fn wait_for_queued_writes(&self) {
        if INSIDE_WORKER_JOB.with(Cell::get) {
            return;
        }
        let (done_sender, done_receiver) = mpsc::channel::<()>();
        if self
            .worker()
            .send(WorkerMessage::Barrier(done_sender))
            .is_ok()
        {
            done_receiver.recv().ok();
        }
    }
}

enum WorkerMessage {
    Open(Box<dyn FnOnce() -> Option<Connection> + Send>),
    Run(Job),
    Barrier(mpsc::Sender<()>),
}

/// True when the file already holds a table — a store written before, by this
/// engine or a platform's native code — rather than one SQLite just created.
fn has_tables(connection: &Connection) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table');",
        [],
        |row| row.get(0),
    )
}

/// `PRAGMA user_version` — which schema the last writer says it left.
pub(crate) fn user_version(connection: &Connection) -> rusqlite::Result<i64> {
    connection.pragma_query_value(None, "user_version", |row| row.get(0))
}

/// Whether `table` exists — the migrations read the SHAPE they are given.
pub(crate) fn table_exists(connection: &Connection, table: &str) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1);",
        [table],
        |row| row.get(0),
    )
}

/// Whether `table` has `column`.
pub(crate) fn has_column(
    connection: &Connection,
    table: &str,
    column: &str,
) -> rusqlite::Result<bool> {
    connection.query_row(
        "SELECT EXISTS (SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2);",
        [table, column],
        |row| row.get(0),
    )
}

/// Where the copy taken before the first takeover lives: `<file>.pre-engine`
/// beside the store.
pub(crate) fn snapshot_path(path: &Path) -> PathBuf {
    let mut name: OsString = path.as_os_str().to_owned();
    name.push(".pre-engine");
    PathBuf::from(name)
}

/// Copies the file, as SQLite sees it, before the engine first changes it —
/// the user's way back if a migration misreads a platform's shape
/// (user-data-engine-roadmap U7). Taken once: an existing copy is the
/// older, truer one. A copy that cannot be written stops the takeover (the
/// store stays closed and the next launch tries again).
///
/// Two processes can take their first open together (the iOS app and its
/// extension), so the copy is written under a per-process name and only a
/// COMPLETE one is published, by a hard link that fails when the final name
/// exists: the first finished copy wins and is never overwritten — not even
/// by a later copy taken after the winner already migrated the file — and a
/// half-written file never carries the final name.
fn snapshot_before_takeover(
    connection: &Connection,
    path: &Path,
) -> Result<(), UserDataDatabaseError> {
    let snapshot = snapshot_path(path);
    if snapshot.exists() {
        return Ok(());
    }
    let mut partial: OsString = snapshot.as_os_str().to_owned();
    partial.push(format!(".{}.partial", std::process::id()));
    let partial = PathBuf::from(partial);
    std::fs::remove_file(&partial).ok();
    connection.execute("VACUUM INTO ?1;", [partial.to_string_lossy()])?;
    let published = match std::fs::hard_link(&partial, &snapshot) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error.into()),
    };
    std::fs::remove_file(&partial).ok();
    published
}

/// `BEGIN IMMEDIATE` … `COMMIT`, rolled back on any error — the body's, or
/// the commit's own: a failed `COMMIT` leaves the transaction open
/// (`SQLiteConnection.swift:160`), and every later `BEGIN` on the
/// connection would fail until something rolls it back.
pub fn immediate_transaction<T, E: From<rusqlite::Error>>(
    connection: &Connection,
    body: impl FnOnce(&Connection) -> Result<T, E>,
) -> Result<T, E> {
    connection.execute_batch("BEGIN IMMEDIATE;")?;
    let result = body(connection).and_then(|value| {
        connection.execute_batch("COMMIT;")?;
        Ok(value)
    });
    if result.is_err() {
        connection.execute_batch("ROLLBACK;").ok();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{UserDataPaths, UserFrequencyStore};

    fn probe_schema() -> StoreSchema {
        StoreSchema {
            apply: |_| Ok(()),
            max_known_version: 0,
            marks_takeover_on_open: true,
        }
    }

    #[test]
    fn perform_from_inside_the_worker_is_refused_rather_than_deadlocking() {
        // trace: Codex PR4 BLOCK — a job that re-enters `perform` would wait for
        // a barrier the worker can never reach.
        let directory = tempfile::tempdir().unwrap();
        let database = UserDataDatabase::new(
            "Probe",
            directory.path().join("probe.db"),
            JournalMode::Wal,
            probe_schema(),
        );
        database.open_blocking();
        let outcome: Result<Result<(), UserDataDatabaseError>, UserDataDatabaseError> =
            database.perform(|_| Ok(Ok(())));
        assert!(outcome.is_ok());
        let handle = Arc::new(database);
        let inner = Arc::clone(&handle);
        let (tx, rx) = mpsc::channel();
        handle.write(move |_| {
            let nested: Result<(), UserDataDatabaseError> = inner.perform(|_| Ok(()));
            tx.send(nested.is_err()).ok();
            Ok(())
        });
        assert!(
            rx.recv_timeout(Duration::from_secs(5)).unwrap(),
            "re-entrant perform is an error"
        );
        // A write queued before open is not lost: the open is a job in the same queue.
        let directory = tempfile::tempdir().unwrap();
        let store = UserFrequencyStore::new(
            UserDataPaths::in_directory(directory.path()).frequency,
            JournalMode::Wal,
            UserFrequencyStore::shipped_capacity(),
        );
        store.open();
        store.record("先", "sian");
        assert_eq!(
            store.all_rows().unwrap().len(),
            1,
            "queued behind the open, not dropped"
        );
    }
}
