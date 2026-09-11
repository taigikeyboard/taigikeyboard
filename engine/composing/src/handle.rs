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
//! Read-only intents (`Intent::is_read_only`: `FetchAtPos`, `QueryState`)
//! never mutate. A platform may run the candidate search on a worker
//! thread (Android does), so a fetch can finish after the main thread has
//! already moved the engine to a newer generation. Letting that stale
//! fetch reset the engine would wipe the new context's composing state;
//! instead a mismatched read-only request answers `Engine::idle_snapshot`
//! and leaves both the state and `last_generation` alone. A matching one
//! runs against a clone of the engine with the mutex released, so the (up
//! to tens of ms) dictionary scan never blocks a concurrent main-thread
//! `Append` / `DeleteBackward`.

use crate::api::{ComposingError, Engine};
use crate::dispatch;
use once_cell::sync::OnceCell;
use protos::engine::{AppConfig, ComposingRequest, ComposingResponse};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

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
        let intent = dispatch::decode_intent(req)?;
        if intent.is_read_only() {
            // Lock-free fast reject for the common stale case …
            if self.last_generation.load(Ordering::Acquire) != generation {
                return Ok(Engine::idle_snapshot(config));
            }
            let snapshot = {
                let engine = self
                    .composing
                    .lock()
                    .expect("composing engine mutex poisoned");
                // … re-checked under the mutex: a mutating request may have
                // moved the engine to a newer generation between the load
                // and the lock, and that state must not be answered as ours.
                if self.last_generation.load(Ordering::Acquire) != generation {
                    return Ok(Engine::idle_snapshot(config));
                }
                engine.clone()
            };
            return Ok(dispatch::query(&intent, &snapshot, config));
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
        Ok(dispatch::apply(intent, &mut engine, config))
    }
}

impl Default for EngineHandle {
    fn default() -> Self {
        Self::new()
    }
}
