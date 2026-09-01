//! `EngineHandle` — singleton process-wide owner of the nextword engine.
//!
//! Per plan §3.6: one `Mutex<Engine>` per process. Mirrors
//! `engine/composing/src/handle.rs`.
//!
//! Generation-mismatch semantics (envelope-level): the platform increments
//! `Request.generation` on `onStartInputView` (IME-session change). On
//! mismatch the engine silently drops state to default BEFORE applying the
//! request, AND bumps `current_generation` by 1 so any in-flight async
//! `FilterPredictions` tagged with the pre-reset generation lands as
//! `was_stale=true` (post-impl PR #198 r3171677157 — preserving without
//! bumping would leave a window where a future intent could collide with
//! the in-flight query's generation). NO effects emitted from the drop.
//!
//! Distinct from `PersistedState.current_generation` (engine-internal,
//! per-intent monotonic, returned via `DecideResult.current_generation`
//! and re-checked on `FilterPredictions.query_generation`). Both axes use
//! `wrapping_add(1)`; `u64::MAX + 1 = 0` is a fresh current value, NOT a
//! reset.

// 中文: `EngineHandle` 是 nextword 引擎的全行程單例,每行程一個 `Mutex<Engine>`。
// 中文: envelope 世代不符時靜默重置狀態並 +1 current_generation,確保在途查詢一定判定 stale。

use crate::api::{Engine, NextWordError};
use crate::dispatch;
use once_cell::sync::OnceCell;
use protos::engine::{AppConfig, NextWordRequest, NextWordResponse};
use std::sync::Mutex;

// 中文: 行程單例的引擎包裝;持有引擎本體與最後一次看到的 envelope 世代值。
pub struct EngineHandle {
    nextword: Mutex<Engine>,
    last_generation: Mutex<u64>,
}

impl EngineHandle {
    pub fn new() -> Self {
        Self {
            nextword: Mutex::new(Engine::new()),
            last_generation: Mutex::new(0),
        }
    }

    /// Process-wide singleton. Constructed lazily on first access.
    // 中文: 取得行程單例;首次呼叫時惰性建立。
    pub fn instance() -> &'static EngineHandle {
        static HANDLE: OnceCell<EngineHandle> = OnceCell::new();
        HANDLE.get_or_init(EngineHandle::new)
    }

    /// Top-level nextword dispatch. Performs envelope generation-mismatch
    /// reset before delegating to the pure dispatch table.
    ///
    /// LOCK ORDER: always lock `nextword` first, then `last_generation`.
    /// Never call platform / FFI / log code while either lock is held.
    /// Mirrors composing/src/handle.rs:55-60.
    // 中文: nextword 最上層分派;先做 envelope 世代檢查與重置,再呼叫純分派表。
    // 中文: 鎖順序固定先 nextword 後 last_generation,持鎖期間禁止呼叫平台 / FFI / log。
    pub fn handle(
        &self,
        req: &NextWordRequest,
        config: &AppConfig,
        generation: u64,
    ) -> Result<NextWordResponse, NextWordError> {
        let mut engine = self
            .nextword
            .lock()
            .expect("nextword engine mutex poisoned");
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

#[cfg(test)]
mod tests {
    use super::*;
    use protos::engine::Platform;

    fn ios_config() -> AppConfig {
        AppConfig {
            tone_mode: String::new(),
            input_mode: "tl".to_owned(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_translate_swapped: false,
            is_association_recording_enabled: true,
            platform_id: Platform::Ios as i32,
            output_both_scripts: false,
            candidate_display_mode: 0,
        }
    }

    /// Regression guard for PR #198 r3171677157: preserve-only would let an
    /// in-flight `FilterPredictions` tagged with the pre-reset
    /// `current_generation` collide with state's value after a same-cycle
    /// reset. Bump-on-reset closes the window — the post-reset state's
    /// `current_generation` is strictly newer than any pre-reset value.
    #[test]
    fn envelope_reset_bumps_current_generation_so_in_flight_filter_drops_as_stale() {
        let mut engine = Engine::new();
        engine.state.current_generation = 5;
        engine.state.is_showing = true;
        engine.state.last_selected_word = Some("早安".to_owned());

        // Pre-reset: an async predict() round was tagged at gen=5.
        let pre_reset_query_gen = engine.state.current_generation;

        // Envelope mismatch fires reset (no effects, but current_generation
        // bumps so pre-reset in-flight queries can't collide).
        engine.reset();

        assert_eq!(engine.state.current_generation, 6);
        assert!(!engine.state.is_showing, "is_showing reset");
        assert_eq!(
            engine.state.last_selected_word, None,
            "last_selected_word reset"
        );

        // Filter the pre-reset query against the post-reset state.
        let result = engine
            .filter(Vec::new(), pre_reset_query_gen, 0, 30, &ios_config())
            .expect("filter ok");
        assert!(result.was_stale, "pre-reset query must drop as stale");
    }

    /// Wrap-around at `u64::MAX` mirrors the per-intent bump behavior.
    #[test]
    fn envelope_reset_wraps_at_u64_max() {
        let mut engine = Engine::new();
        engine.state.current_generation = u64::MAX;
        engine.reset();
        assert_eq!(engine.state.current_generation, 0, "wrapping_add expected");
    }
}
