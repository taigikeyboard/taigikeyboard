//! Public façade for the nextword crate. Defines `Engine`, `PersistedState`,
//! `Intent`, and `NextWordError`. Implementation of decide / filter / score
//! / boost lives in submodules; this module is the stable surface that
//! `dispatch.rs` and external crates consume.

// 中文: nextword crate 的公開門面,定義 `Engine`、`PersistedState`、`Intent`、`NextWordError`。
// 中文: 決策 / 過濾 / 評分 / 加權的實作在子模組,這裡只暴露穩定介面給 dispatch 與外部 crate。

use protos::engine::{AppConfig, DecideResult, FilterResult, RawNextWordPrediction, StateSnapshot};
use thiserror::Error;

/// Long-lived state the engine mutates. Mirrors iOS
/// `NextWordPersistedState` / Android `NextWordPersistedState` 1:1
/// pre-migration.
// 中文: 引擎會變更的長期狀態,與 iOS / Android 平台側結構 1:1 對齊。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct PersistedState {
    // 中文: 上次選定的詞(顯示文字)。
    pub last_selected_word: Option<String>,
    // 中文: 上次選定詞的羅馬字拼寫。
    pub last_selected_roman: Option<String>,
    // 中文: 上次選詞的時間戳記,單位毫秒。
    pub last_selection_time_ms: i64,
    // 中文: 目前是否正顯示下一個詞候選。
    pub is_showing: bool,
    /// Monotonic counter bumped on every state-mutating intent (except
    /// `UpdateLastSelectedWord` per audit §5 #5). The platform tags
    /// in-flight prediction queries with the generation at dispatch time;
    /// a mismatch at query resolution drops the result. Wraps via
    /// `wrapping_add` — `u64::MAX + 1 = 0` is a fresh current value, NOT
    /// a reset.
    // 中文: 單調遞增世代計數,用來丟棄過期的非同步預測結果(wrapping_add 換算非重置)。
    pub current_generation: u64,
}

/// Decoded intent set. Mirrors iOS `NextWordIntent` / Android
/// `NextWordIntent` 1:1, plus `UpdateLastSelectedWord` — Android-only when it
/// was added (audit §5 #5 / Codex v1 P1), now also emitted by iOS's
/// continuous-input mid-commit handshake and by macOS. Decoded from
/// `protos::engine::NextWordRequest::method` inside `dispatch::handle`.
// 中文: 解碼後的 intent enum,對齊 iOS / Android 平台 intent,另含 Android-only 的 Space 路徑。
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Intent {
    // 中文: 選詞:平台主要寫入路徑,可能觸發 association 紀錄與下一輪預測。
    WordSelected {
        text: String,
        roman: String,
        require_roman_mode: bool,
        trigger_prediction: bool,
        now_ms: i64,
    },
    // 中文: 退格:把最後一個字當作上下文,重新發查詢。
    Backspace {
        last_char: String,
        now_ms: i64,
    },
    // 中文: 上下文逾時:30 秒未動作後重置狀態並清候選詞。
    ContextTimeoutFired {
        now_ms: i64,
    },
    // 中文: 開始新的組字:藏候選詞但保留上下文;若先前在顯示則發 ClearPredictionsUI。
    ClearForNewComposing {
        now_ms: i64,
    },
    // 中文: 完整重置:等同上下文逾時,清空所有狀態。
    ResetFull {
        now_ms: i64,
    },
    /// Android-only Space-path. Mutates state without timer effects or
    /// generation bump; emits compound-only effect.
    // 中文: Android 限定的 Space 路徑;只更新狀態,不重排計時器、不增世代,只發複合詞 association。
    UpdateLastSelectedWord {
        text: String,
        roman: String,
        now_ms: i64,
    },
    /// Platform-driven UI visibility sync. Mutates `state.is_showing` only;
    /// no effects, no generation bump. Called by the platform after
    /// rendering the result of an async predict() so subsequent clear/reset
    /// paths know whether to emit `ClearPredictionsUI`.
    // 中文: 平台告知候選詞顯示狀態;只同步 is_showing,不發 effect 也不增世代。
    SetIsShowing {
        is_showing: bool,
    },
}

/// Engine errors surface as `ErrorCode::FailInvariant` at the FFI seam.
// 中文: 引擎錯誤;在 FFI 邊界一律映射為 `ErrorCode::FailInvariant`。
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
// 中文: 狀態機本體;在 FFI 邊界包進 `Mutex<Engine>` 確保跨執行緒安全。
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
    // 中文: 純讀取狀態快照,不變更狀態;供 QueryState 與 envelope 重置後回報使用。
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
    /// envelope's `Platform` is `Unspecified` — the platform divergence
    /// branches require an explicit platform.
    // 中文: 套用 intent 變更狀態,回傳 effects 與最新狀態快照;Platform 未指定時回傳錯誤。
    pub(crate) fn apply(
        &mut self,
        intent: Intent,
        config: &AppConfig,
    ) -> Result<DecideResult, NextWordError> {
        crate::decide::apply(&mut self.state, intent, config)
    }

    /// Score + merge + sort + limit raw rows. Generation-mismatch path
    /// returns `was_stale=true`. Delegates to `filter.rs`.
    // 中文: 評分、合併、排序、截斷原始候選列;世代不符時回傳 was_stale=true。
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
    // 中文: IME session 切換時的靜默重置;清空狀態但 current_generation 仍 +1,讓在途查詢一定過期。
    pub(crate) fn reset(&mut self) {
        let bumped_gen = self.state.current_generation.wrapping_add(1);
        self.state = PersistedState {
            current_generation: bumped_gen,
            ..PersistedState::default()
        };
    }
}
