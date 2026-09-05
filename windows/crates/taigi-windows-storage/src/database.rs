//! Opens a learning database and serialises every access to it. Port of
//! `Storage/UserDataDatabase.swift`: one serial worker owns the file — the
//! open, every queued write and every `perform` run on it in order, as on
//! the macOS `DispatchQueue` — while reads answer on the caller's thread
//! through a second, read-only connection that never waits.

use rusqlite::{Connection, OpenFlags};
use std::cell::Cell;
use std::path::PathBuf;
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
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
}

type Job = Box<dyn FnOnce(Option<&Connection>) + Send>;
type Schema = fn(&Connection) -> rusqlite::Result<()>;

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
    file_name: &'static str,
    name: &'static str,
    directory: PathBuf,
    schema: Schema,
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
    /// Queued jobs the worker may fall behind by before writes are dropped.
    /// A keystroke queues one; 256 is seconds of typing against a stalled
    /// disk, after which forgetting a count is the right degrade.
    const QUEUE_CAPACITY: usize = 256;

    pub fn new(
        file_name: &'static str,
        name: &'static str,
        directory: PathBuf,
        schema: Schema,
    ) -> Self {
        Self {
            file_name,
            name,
            directory,
            schema,
            reader: Arc::new(Mutex::new(None)),
            is_open: Arc::new(AtomicBool::new(false)),
            worker: OnceLock::new(),
            open_requested: AtomicBool::new(false),
        }
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
        let path = self.directory.join(self.file_name);
        let schema = self.schema;
        let name = self.name;
        let reader = Arc::clone(&self.reader);
        let is_open = Arc::clone(&self.is_open);
        // The open is a job like any other, so it cannot be overtaken. It
        // carries its own connection out through the closure's return path:
        // the worker keeps whatever `Open` hands it.
        let job = WorkerMessage::Open(Box::new(move || {
            match Self::open_connections(&path, schema) {
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

    fn open_connections(
        path: &std::path::Path,
        schema: Schema,
    ) -> rusqlite::Result<(Connection, Connection)> {
        let writer = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_CREATE
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        writer.busy_timeout(Self::WRITER_BUSY_TIMEOUT)?;
        writer.pragma_update(None, "journal_mode", "WAL")?;
        schema(&writer)?;
        let read_only = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        read_only.busy_timeout(Duration::ZERO)?;
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
        let file_name = self.file_name.to_owned();
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
