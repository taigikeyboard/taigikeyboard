//! Public façade for the nextword crate. Defines `Engine`, `PersistedState`,
//! `Intent`, and `NextWordError`. Implementation of decide / filter / score
//! / boost lives in submodules; this module is the stable surface that
//! `dispatch.rs` and external crates consume.

use protos::engine::{AppConfig, DecideResult, FilterResult, RawNextWordPrediction, StateSnapshot};
use thiserror::Error;

/// Long-lived state the engine mutates. Mirrors iOS
/// `NextWordPersistedState` / Android `NextWordPersistedState` 1:1
/// pre-migration.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PersistedState {
    pub last_selected_word: Option<String>,
    pub last_selected_roman: Option<String>,
    pub last_selection_time_ms: i64,
    pub is_showing: bool,
    /// Monotonic counter bumped on every state-mutating intent (except
    /// `UpdateLastSelectedWord` per audit §5 #5). The platform tags
    /// in-flight prediction queries with the generation at dispatch time;
    /// a mismatch at query resolution drops the result. Wraps via
    /// `wrapping_add` — `u64::MAX + 1 = 0` is a fresh current value, NOT
    /// a reset.
    pub current_generation: u64,
}

/// Decoded intent set. Mirrors iOS `NextWordIntent` / Android
/// `NextWordIntent` 1:1, plus `UpdateLastSelectedWord` — Android-only when it
/// was added (audit §5 #5 / Codex v1 P1), now also emitted by iOS's
/// continuous-input mid-commit handshake and by macOS. Decoded from
/// `protos::engine::NextWordRequest::method` inside `dispatch::handle`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Intent {
    /// Candidate selected — main platform write path; may record an association and predict.
    WordSelected {
        text: String,
        roman: String,
        require_roman_mode: bool,
        trigger_prediction: bool,
        now_ms: i64,
    },
    /// Backspace — re-queries using the last character as context.
    Backspace { last_char: String, now_ms: i64 },
    /// Context timeout — resets state and clears candidates after the 30 s idle window.
    ContextTimeoutFired { now_ms: i64 },
    /// New composing — hides candidates, keeps context; emits ClearPredictionsUI if showing.
    ClearForNewComposing { now_ms: i64 },
    /// Full reset — same as the context timeout, clearing all state.
    ResetFull { now_ms: i64 },
    /// Android-only Space-path. Mutates state without timer effects or
    /// generation bump; emits compound-only effect.
    UpdateLastSelectedWord {
        text: String,
        roman: String,
        now_ms: i64,
    },
    /// Platform-driven UI visibility sync. Mutates `state.is_showing` only;
    /// no effects, no generation bump. Called by the platform after
    /// rendering the result of an async predict() so subsequent clear/reset
    /// paths know whether to emit `ClearPredictionsUI`.
    SetIsShowing { is_showing: bool },
}

/// Engine errors surface as `ErrorCode::FailInvariant` at the FFI seam.
#[derive(Debug, Error)]
pub enum NextWordError {
    #[error("nextword request missing method")]
    MissingMethod,
    #[error("intent missing decision input")]
    MissingDecisionInput,
    #[error("invalid Source on RawNextWordPrediction")]
    InvalidSource,
    #[error("invalid Platform on AppConfig")]
    InvalidPlatform,
}

/// State machine. Held inside `Mutex<Engine>` at the FFI boundary.
#[derive(Clone, Debug, Default)]
pub struct Engine {
    pub(crate) state: PersistedState,
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pure read — no state mutation. Used by `Intent::QueryState` and
    /// the generation-mismatch path's post-drop re-snapshot.
    pub(crate) fn snapshot(&self) -> StateSnapshot {
        StateSnapshot {
            last_selected_word: self.state.last_selected_word.clone().unwrap_or_default(),
            is_showing: self.state.is_showing,
            current_generation: self.state.current_generation,
        }
    }

    /// Apply `intent` against the current state, mutate, and return the
    /// resulting decide response (effects + new state echo). Delegates to
    /// the pure decide table in `decide.rs`. May fail when the request
    /// envelope's `Platform` is `Unspecified` — a legacy check, kept because
    /// no decision reads the value any more (`behavioral-invariants.md` §40)
    /// and dropping it would be an observable change of its own.
    pub(crate) fn apply(
        &mut self,
        intent: Intent,
        config: &AppConfig,
    ) -> Result<DecideResult, NextWordError> {
        crate::decide::apply(&mut self.state, intent, config)
    }

    /// Score + merge + sort + limit raw rows. Generation-mismatch path
    /// returns `was_stale=true`. Delegates to `filter.rs`.
    pub(crate) fn filter(
        &self,
        raw: Vec<RawNextWordPrediction>,
        query_generation: u64,
        now_ms: i64,
        limit: i32,
        config: &AppConfig,
    ) -> Result<FilterResult, NextWordError> {
        crate::filter::filter(&self.state, raw, query_generation, now_ms, limit, config)
    }

    /// Reset on envelope-mismatch (IME-session ID change). Zeroes the
    /// state struct EXCEPT `current_generation`, which is bumped by 1
    /// (`wrapping_add`) so pre-reset in-flight async results
    /// (`FilterPredictions` tagged with the pre-reset generation)
    /// reliably fail the equality check in `filter.rs` and surface as
    /// `was_stale=true`. Silent — no effects emitted from the drop.
    ///
    /// Codex post-impl PR #198 review (comment id 3171677157).
    pub(crate) fn reset(&mut self) {
        let bumped_gen = self.state.current_generation.wrapping_add(1);
        self.state = PersistedState {
            current_generation: bumped_gen,
            ..PersistedState::default()
        };
    }
}
