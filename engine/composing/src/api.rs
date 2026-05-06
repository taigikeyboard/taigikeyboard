//! Public façade for the composing crate. Defines `Engine`, `EngineState`,
//! `Phase`, `Intent`, and `ComposingError`. Implementation of state
//! transitions lives in `transition.rs`; this module is the stable surface
//! that `dispatch.rs` and external crates consume.

// 中文: 組字 crate 的對外 API:Engine、EngineState、Phase、Intent、ComposingError。
// 中文: 真正的狀態轉移實作在 transition.rs,本檔案只定義穩定的型別介面。

use protos::engine::{AppConfig, ComposingResponse};
use thiserror::Error;

/// Composition phase. `Idle` means no preedit; `Composing { raw }` carries
/// the numeric-tone ASCII raw input that the platform-side state used to
/// shadow.
// 中文: 組字階段。Idle 表示無預編輯;Composing { raw } 攜帶數字調 ASCII 原始輸入。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    // 中文: 閒置狀態,無預編輯內容。
    Idle,
    // 中文: 組字中,raw 為使用者尚未上屏的數字調原始輸入。
    Composing { raw: String },
}

/// Engine state — the platform no longer shadows this.
// 中文: 引擎狀態,平台端不再額外複製一份。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EngineState {
    // 中文: 目前的組字階段 (Idle / Composing)。
    pub phase: Phase,
    // 中文: 目前選取的候選詞索引;Idle 時為 -1。
    pub selected_candidate_index: i32,
}

impl Default for EngineState {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            selected_candidate_index: -1,
        }
    }
}

/// Mirrors the iOS `ComposingState.Intent` / Android `ComposingState.Intent`
/// case set 1:1. Decoded from `protos::engine::ComposingRequest::method`
/// inside `dispatch::handle`.
// 中文: 與 iOS / Android 平台 ComposingState.Intent 一對一對應的意圖 enum。
// 中文: 由 dispatch::handle 從 ComposingRequest::method 解碼產生。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    // 中文: 以指定文字開始組字。
    Start { text: String },
    // 中文: 在組字區尾端附加一個字元。
    Append { ch: String },
    // 中文: 附加連字號 "-",內部轉為 Append 處理。
    AppendHyphen,
    // 中文: 把組字區最後一個字元換成 replacement (TPS 自動修正用)。
    ReplaceLast { replacement: String },
    // 中文: 退格刪除組字區尾端字元。
    DeleteBackward,
    // 中文: 上屏目前組字區的衍生顯示形 (加調號/正規化後)。
    CommitDerived,
    // 中文: 直接上屏原始 raw 輸入,不做衍生轉換。
    CommitRaw,
    // 中文: 採用候選/聯想詞 text 上屏。
    SelectSuggestion { text: String },
    // 中文: 先上屏目前組字區衍生形,再插入外部字串 text。
    CommitPreeditThenInsertExternal { text: String },
    // 中文: 使用者主動重設,清空預編輯並重置候選。
    Reset,
    // 中文: 直接設定選取的候選詞索引。
    SetSelectedCandidateIndex { index: i32 },
    // 中文: 純讀取目前狀態,不變更狀態也不發出 Effect。
    QueryState,
}

// 中文: 組字流程的錯誤型別,目前僅有 method 欄位缺漏一種。
#[derive(Debug, Error)]
pub enum ComposingError {
    #[error("composing request missing method")]
    MissingMethod,
}

/// State machine. Held inside `Mutex<Engine>` at the FFI boundary.
// 中文: 組字狀態機本體,FFI 邊界以 Mutex<Engine> 包覆共享。
#[derive(Clone, Debug, Default)]
pub struct Engine {
    state: EngineState,
}

impl Engine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Pure read — no state mutation, no effects emitted. Used by
    /// `Intent::QueryState` and the generation-mismatch path's post-drop
    /// re-snapshot. `config` is the request's `AppConfig` so
    /// `Preedit.display_text` reflects the caller's actual mode/toggles
    /// (per Codex PR #197 r3169707395).
    // 中文: 純讀取目前狀態的快照,不變更狀態也不發出 Effect。
    pub fn snapshot(&self, config: &AppConfig) -> ComposingResponse {
        crate::transition::apply(&mut self.state.clone(), Intent::QueryState, config)
    }

    /// Apply `intent` against the current state, mutate, and return the
    /// resulting response (preedit + ordered effects + new index +
    /// is_composing). Delegates to the pure transition table in
    /// `transition.rs`.
    // 中文: 套用 intent、更新狀態並回傳組字回應 (預編輯/Effect 序列/索引)。
    pub fn apply(&mut self, intent: Intent, config: &AppConfig) -> ComposingResponse {
        crate::transition::apply(&mut self.state, intent, config)
    }

    /// Idempotent reset. Called from the generation-mismatch path inside
    /// `dispatch::handle`. NOT public API — external callers always go
    /// through `dispatch::handle`. The user-initiated `Intent::Reset` path
    /// goes through `apply(Intent::Reset, ...)`, which emits the
    /// `ClearPreeditWithoutCommit + ResetAutocomplete` effects when
    /// composing; this helper is silent (no effects) for the
    /// generation-mismatch drop. Call site is `EngineHandle::handle`.
    // 中文: 靜默重設 (無 Effect),僅供 generation 不一致時的內部丟棄路徑使用。
    pub(crate) fn reset(&mut self) {
        self.state = EngineState::default();
    }
}
