//! `EngineHandle` — singleton process-wide owner of the composing engine.
//!
//! Per plan §3.4 (Codex (a) APPROVED): one `Mutex<Engine>` per process.
//! Both reference IMEs (McBopomofo `KeyHandler`, khiin-rs `EngineController`)
//! converge on this pattern.
//!
//! Generation-mismatch semantics (plan §5b.2): the platform increments
//! `Request.generation` only on a real field change (per §4.2 fallback
//! rules); on mismatch the engine silently drops state to Idle BEFORE
//! applying a mutating request. NO effects emitted from the drop itself —
//! the request's own effects then apply against fresh state.
//!
//! Read-only intents (`Intent::is_read_only`: `FetchAtPos`)
//! never mutate. A platform may run the candidate search on a worker
//! thread (Android does), so a fetch can finish after the main thread has
//! already moved the engine to a newer generation. Letting that stale
//! fetch reset the engine would wipe the new context's composing state;
//! instead a mismatched read-only request answers `Engine::idle_snapshot`
//! and leaves both the state and `last_generation` alone. A matching one
//! runs against a clone of the engine with the mutex released, so the (up
//! to tens of ms) dictionary scan never blocks a concurrent main-thread
//! `Append` / `DeleteBackward`. A fetch that runs in passes (the user-data
//! re-rank) takes that clone once ([`EngineHandle::engine_at`]) and answers
//! every pass from it, so the rows it read describe the state it answers.

use crate::api::{Applied, ComposingError, ConversionFrequency, Engine, Intent};
use crate::requests;
use once_cell::sync::OnceCell;
use protos::engine::{AppConfig, ComposingRequest, ComposingResponse};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// The composing state a fetch is answered for (`Engine::pending_snapshot`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingSnapshot {
    /// The text the fetch lists — part of the pending buffer, or the last
    /// nailed segment's raw text followed by it (`WordListStart::LastNailed`):
    /// the user's rows are read for it.
    pub listed_raw: String,
    /// The word the listed part follows.
    pub context: ListContext,
}

/// The word a fetch's list follows (§56).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ListContext {
    /// A word of the composition, as `(canonical text, association roman)`.
    Word(String, String),
    /// Nothing in the composition: the last committed word.
    Committed,
    /// Glyphs the user did not convert: no context.
    Cut,
}

pub struct EngineHandle {
    composing: Mutex<Engine>,
    // Only compared and stored; atomic so a stale read-only request can be
    // answered without touching the composing mutex.
    last_generation: AtomicU64,
}

impl EngineHandle {
    pub fn new() -> Self {
        Self {
            composing: Mutex::new(Engine::new()),
            last_generation: AtomicU64::new(0),
        }
    }

    /// Process-wide singleton. The handle is constructed lazily on first
    /// access. Reference: `references/khiin-rs/khiin/src/engine.rs:57`.
    pub fn instance() -> &'static EngineHandle {
        static HANDLE: OnceCell<EngineHandle> = OnceCell::new();
        HANDLE.get_or_init(EngineHandle::new)
    }

    /// Top-level composing dispatch. Performs generation-mismatch reset
    /// before delegating a mutating intent to the pure transition table;
    /// read-only intents take the lock-free path described in the module
    /// doc.
    pub fn handle(
        &self,
        req: &ComposingRequest,
        config: &AppConfig,
        generation: u64,
    ) -> Result<ComposingResponse, ComposingError> {
        self.handle_learning(req, config, generation)
            .map(|applied| applied.response)
    }

    /// [`handle`](Self::handle), with the phrase a final commit taught (§50)
    /// and the usage an R5 commit counts — for the engine's own user-data
    /// stores.
    pub fn handle_learning(
        &self,
        req: &ComposingRequest,
        config: &AppConfig,
        generation: u64,
    ) -> Result<Applied, ComposingError> {
        self.handle_ranked(req, config, generation, None)
    }

    /// [`handle_learning`](Self::handle_learning), a Hanji conversion walk
    /// ranking with the user's learned counts `frequency` supplies (H5).
    pub fn handle_ranked(
        &self,
        req: &ComposingRequest,
        config: &AppConfig,
        generation: u64,
        frequency: Option<&dyn ConversionFrequency>,
    ) -> Result<Applied, ComposingError> {
        let intent = requests::decode_intent(req)?;
        if intent.is_read_only() {
            return Ok(self.query(&intent, config, generation).into());
        }
        let mut engine = self
            .composing
            .lock()
            .expect("composing engine mutex poisoned");
        if self.last_generation.load(Ordering::Acquire) != generation {
            // Silent state drop — NO effects emitted from the reset itself.
            engine.reset();
            self.last_generation.store(generation, Ordering::Release);
        }
        Ok(engine.apply_ranked(intent, config, frequency))
    }

    /// Answers a read-only intent (`Intent::is_read_only`) — a `FetchAtPos`
    /// the engine built with the user's rows (`UserRows`), or one decoded
    /// from the wire. A stale generation answers the idle snapshot (module
    /// doc).
    pub fn query(&self, intent: &Intent, config: &AppConfig, generation: u64) -> ComposingResponse {
        match self.engine_at(generation) {
            Some(snapshot) => requests::query(intent, &snapshot, config),
            None => Engine::idle_snapshot(config),
        }
    }

    /// A clone of the engine as of `generation`, or `None` when `generation`
    /// is stale. A `FetchAtPos` reads what it ranks against from it
    /// (`Engine::pending_snapshot`) — the user-data stores and the bigram
    /// tables (user-data-engine-roadmap P3b, bigram-lm-roadmap P5) — and
    /// answers every pass from it (`requests::query`).
    pub fn engine_at(&self, generation: u64) -> Option<Engine> {
        self.read_at(generation, Engine::clone)
    }

    /// Runs `read` on the engine as of `generation`, or answers `None` when a
    /// mutating request has moved the engine on. A lock-free fast reject for
    /// the common stale case, re-checked under the mutex: a newer generation
    /// may land between the load and the lock, and that state must not be
    /// answered as ours.
    fn read_at<T>(&self, generation: u64, read: impl FnOnce(&Engine) -> T) -> Option<T> {
        if self.last_generation.load(Ordering::Acquire) != generation {
            return None;
        }
        let engine = self
            .composing
            .lock()
            .expect("composing engine mutex poisoned");
        if self.last_generation.load(Ordering::Acquire) != generation {
            return None;
        }
        Some(read(&engine))
    }
}
impl Default for EngineHandle {
    fn default() -> Self {
        Self::new()
    }
}
