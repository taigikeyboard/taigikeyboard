//! `EngineHandle` — singleton process-wide owner of the composing engine.
//!
//! Per plan §3.4 (Codex (a) APPROVED): one `Mutex<Engine>` per process.
//! Both reference IMEs (McBopomofo `KeyHandler`, khiin-rs `EngineController`)
//! converge on this pattern.
//!
//! Generation-mismatch semantics (plan §5b.2): the platform increments
//! `Request.generation` only on a real field change (per §4.2 fallback
//! rules); on mismatch the engine silently drops state to Idle BEFORE
//! applying the request. NO effects emitted from the drop itself —
//! the request's own effects then apply against fresh state.

// 中文: 行程內的 EngineHandle 單例,負責持有 Mutex<Engine> 與 generation 同步。
// 中文: generation 不一致時靜默把狀態退回 Idle,再套用本次請求。

use crate::api::{ComposingError, Engine};
use crate::dispatch;
use once_cell::sync::OnceCell;
use protos::engine::{AppConfig, ComposingRequest, ComposingResponse};
use std::sync::Mutex;

// 中文: 行程內共享的組字引擎 handle,內含引擎本體與上次見到的 generation。
pub struct EngineHandle {
    composing: Mutex<Engine>,
    last_generation: Mutex<u64>,
}

impl EngineHandle {
    pub fn new() -> Self {
        Self {
            composing: Mutex::new(Engine::new()),
            last_generation: Mutex::new(0),
        }
    }

    /// Process-wide singleton. The handle is constructed lazily on first
    /// access. Reference: `references/khiin-rs/khiin/src/engine.rs:57`.
    // 中文: 取得行程級單例,首次存取時延遲建構。
    pub fn instance() -> &'static EngineHandle {
        static HANDLE: OnceCell<EngineHandle> = OnceCell::new();
        HANDLE.get_or_init(EngineHandle::new)
    }

    /// Top-level composing dispatch. Performs generation-mismatch reset
    /// before delegating to the pure transition table.
    // 中文: 對外的組字分派入口,先處理 generation 不一致重設,再委派給 dispatch。
    pub fn handle(
        &self,
        req: &ComposingRequest,
        config: &AppConfig,
        generation: u64,
    ) -> Result<ComposingResponse, ComposingError> {
        let mut engine = self
            .composing
            .lock()
            .expect("composing engine mutex poisoned");
        let mut last_gen = self
            .last_generation
            .lock()
            .expect("last_generation mutex poisoned");
        if *last_gen != generation {
            // Silent state drop — NO effects emitted from the reset itself.
            engine.reset();
            *last_gen = generation;
        }
        dispatch::handle(req, &mut engine, config)
    }
}

impl Default for EngineHandle {
    fn default() -> Self {
        Self::new()
    }
}
