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
/// shadow; `Continuous { raw, nailed }` is the v3.5.8 multi-segment state.
///
/// **Model B (mainstream-aligned, see `docs/engine/continuous-input-ranking.md`
/// §10):** `nailed` segments are **NOT** in the host document. The whole
/// composition — `Σ nailed[i].display_text` followed by the derived display
/// of the pending `raw` tail — lives in **one** marked / composing region
/// until a hard finalize (Enter / final-commit / external-suggestion commit).
/// A candidate tap *nails* a segment inside the composition; only a hard
/// finalize writes literal text to the document. Reset/abort clears the
/// whole region and drops all `nailed` (nothing was written).
// 中文: 組字階段。Idle 無預編輯;Composing 單段數字調 raw;
// 中文: Continuous 是 v3.5.8 連續輸入多段狀態。
// 中文: Model B(對齊主流,§10):nailed 段**未**寫入文件;整段組字
// 中文: (Σ nailed.display_text + pending raw 衍生形)留在單一 marked region,
// 中文: 直到 hard finalize 才一次寫字面字。abort 清整段並丟棄所有 nailed。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    // 中文: 閒置狀態,無預編輯內容。
    Idle,
    // 中文: 組字中,raw 為使用者尚未上屏的數字調原始輸入。
    Composing {
        raw: String,
    },
    // 中文: 連續輸入中,nailed 為組字區內已釘的 segments(未寫入文件),raw 為 pending 尾段。
    Continuous {
        raw: String,
        nailed: Vec<NailedSegment>,
    },
}

impl Phase {
    /// Pending-tail display form rendered through the derived-display chain
    /// (POJ doubletap → tone marks → nasal-case adjust; TPS pass-through).
    /// For `Phase::Continuous` this is strictly the pending `raw` tail, not
    /// the original keystroke history nor the nailed prefix; for
    /// `Phase::Composing` it is the single-segment raw; for `Phase::Idle` it
    /// is the empty string.
    ///
    /// **Model B (§10):** this is **no longer the composing-buffer surface** —
    /// it is one internal *component* of it. The composing buffer the host
    /// renders is [`Phase::composing_display`] (`Σ nailed.display_text` +
    /// this pending-tail form). `raw_input` remains the still-editable raw
    /// tail used by span-local candidate fetch and by callers that need the
    /// pending-only form.
    ///
    /// User-typed hyphens are preserved as conversion boundaries (the
    /// derived-display chain splits on `-` for tone-mark application); the
    /// engine does NOT validate whether each chunk is a real syllable, and
    /// does NOT auto-insert hyphens. See §10.2 amendment 2026-05-13.
    // 中文: pending-tail 衍生顯示字串 (TPS 原樣 / POJ-TL 走 derived chain)。
    // 中文: Model B:這已**不是**組字緩衝區本體,只是其一個內部組件;
    // 中文: 組字緩衝區是 Phase::composing_display(Σ nailed.display_text + 本字串)。
    pub fn raw_input(&self, config: &AppConfig) -> String {
        match self {
            Phase::Idle => String::new(),
            Phase::Composing { raw } | Phase::Continuous { raw, .. } => {
                crate::derived::derived_display(raw, config)
            }
        }
    }

    /// The composing-buffer surface the host renders in its single
    /// marked / composing region (`docs/engine/continuous-input-ranking.md`
    /// §10.2 / §10.4 invariant I1, Model B).
    ///
    /// - `Idle` → empty string.
    /// - `Composing { raw }` → derived display of `raw` (identity with
    ///   [`Phase::raw_input`]; no nailed segments exist).
    /// - `Continuous { raw, nailed }` → `Σ nailed[i].display_text`
    ///   concatenated with the derived display of the pending `raw` tail.
    ///   Nailed segments are **not** in the document; they are part of the
    ///   marked region until a hard finalize.
    ///
    /// No inter-segment spaces (the hanji line is logographic; the roman
    /// pending tail keeps user-typed hyphens). This is the string a hard
    /// finalize (Enter / final-commit) writes to the document verbatim.
    // 中文: §10.2 / I1 — host 在單一 marked region 渲染的組字緩衝區本體。
    // 中文: Continuous = Σ nailed.display_text + pending raw 衍生形,無段間空格;
    // 中文: nailed 未在文件,hard finalize 才把此字串一次寫入文件。
    pub fn composing_display(&self, config: &AppConfig) -> String {
        match self {
            Phase::Idle => String::new(),
            Phase::Composing { raw } => crate::derived::derived_display(raw, config),
            Phase::Continuous { raw, nailed } => combined_display(nailed, raw, config),
        }
    }
}

/// One **nailed** segment inside `Phase::Continuous`. "Nailed" means the
/// user accepted a candidate for this part of the buffer, but — under
/// Model B (`docs/engine/continuous-input-ranking.md` §10) — it is **NOT**
/// yet written to the host document; it lives inside the active marked /
/// composing region until a hard finalize. `raw_span` records the byte
/// offsets in the original raw input the user typed (start = end of the
/// previous segment, end = start + raw_text.len()). `syllable_count` lets
/// span-local fetch distinguish e.g. `tsua` → 紙(1) vs 珠仔(2).
// 中文: Continuous 階段「已釘」的單一 segment。Model B(§10):**未**寫入文件,
// 中文: 留在 active marked region 內直到 hard finalize。raw_span 為原始 raw byte 區間,
// 中文: syllable_count 區分同 toneless key 不同音節數的候選。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NailedSegment {
    // 中文: 此段在組字區內的顯示文字 (e.g., "紙")。v3.5.8 Phase 9 Bug 1:
    // 中文: swap/TPS/both-scripts 格式化字串;hard finalize 時併入整段一次寫入文件。
    pub display_text: String,
    // 中文: 規範字典鍵 (`hanji.unwrap_or(roman)`)。v3.5.8 Phase 9 Bug 1 (Option A):
    // 中文: backspace pop 時的 NextWord last-selected 修正用它,確保關聯學習
    // 中文: 與顯示模式無關 (decision b)。非 swap 時等同 display_text。
    pub canonical_text: String,
    // 中文: 對應消耗的原始輸入 (e.g., "tsua")。
    pub raw_text: String,
    // 中文: 在原 raw 輸入中的 byte 偏移 (start, end);用於 Phase 5 span-local 查詢。
    pub raw_span: (usize, usize),
    // 中文: 此 segment 包含的音節數,1 為單音節、2+ 為複合詞。
    pub syllable_count: u8,
}

/// `Σ nailed[i].display_text` — the nailed-prefix string (already
/// swap/TPS/both-scripts-formatted at nail time). **Single source of truth**
/// for the nailed-prefix concatenation; do not re-inline this loop.
// 中文: Σ nailed.display_text;nailed 前綴串接的唯一真相來源,勿再內聯。
pub(crate) fn nailed_prefix(nailed: &[NailedSegment]) -> String {
    let mut s = String::new();
    for seg in nailed {
        s.push_str(&seg.display_text);
    }
    s
}

/// The Model B composing-buffer surface for a `(nailed, raw)` pair:
/// `Σ nailed[i].display_text` followed by the derived display of the
/// pending `raw` tail. **Single source of truth** — both
/// [`Phase::composing_display`] and the `transition.rs` Continuous paths
/// route through this so the rendered preedit and the hard-finalize commit
/// can never diverge (Codex post-impl review point).
// 中文: Model B 組字緩衝區本體 (nailed 前綴 + pending raw 衍生形) 的唯一真相來源;
// 中文: Phase::composing_display 與 transition.rs 各 Continuous path 全走此,確保不漂移。
pub(crate) fn combined_display(
    nailed: &[NailedSegment],
    raw: &str,
    config: &AppConfig,
) -> String {
    let mut s = nailed_prefix(nailed);
    s.push_str(&crate::derived::derived_display(raw, config));
    s
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
    Start {
        text: String,
    },
    // 中文: 在組字區尾端附加一個字元。
    Append {
        ch: String,
    },
    // 中文: 附加連字號 "-",內部轉為 Append 處理。
    AppendHyphen,
    // 中文: 把組字區最後一個字元換成 replacement (TPS 自動修正用)。
    ReplaceLast {
        replacement: String,
    },
    // 中文: 退格刪除組字區尾端字元。
    DeleteBackward,
    // 中文: 上屏目前組字區的衍生顯示形 (加調號/正規化後)。
    CommitDerived,
    // 中文: 直接上屏原始 raw 輸入,不做衍生轉換。
    CommitRaw,
    // 中文: 採用候選/聯想詞 text 上屏。
    SelectSuggestion {
        text: String,
    },
    // 中文: 先上屏目前組字區衍生形,再插入外部字串 text。
    CommitPreeditThenInsertExternal {
        text: String,
    },
    // 中文: 使用者主動重設,清空預編輯並重置候選。
    Reset,
    // 中文: 直接設定選取的候選詞索引。
    SetSelectedCandidateIndex {
        index: i32,
    },
    // 中文: 純讀取目前狀態,不變更狀態也不發出 Effect。
    QueryState,
    // 中文: 從 Composing 進入 Continuous (連續輸入) 模式;nailed 起始為空。
    EnterContinuous,
    /// v3.5.8 Phase 6 — pure read of span-local continuous-input
    /// candidates for the current `Phase::Continuous { raw }` starting
    /// at `position` (always `0` in v3.5.8; non-zero short-circuits
    /// to an empty candidate list). Resolved by `dispatch::handle`
    /// outside the `transition::apply` pure path because the fetch
    /// needs lexicon state — see `dispatch::handle_fetch_at_pos`.
    /// `transition.rs` only sees this variant via a defensive snapshot
    /// arm; production callers always go through dispatch.
    ///
    /// v3.5.8 Phase 9.3a — carries the per-candidate
    /// `user_frequency.db` snapshot (`frequency_entries`, keyed by
    /// `display_text_key = hanji ?? roman`) and the platform wall
    /// clock (`now_ms`, epoch-ms). Both fields are decoded verbatim
    /// from `FetchAtPos { frequency_entries, now_ms }` and threaded
    /// straight to `handle_fetch_at_pos`. Empty list + `now_ms = 0`
    /// is the backward-compatible "no user-freq plumbing yet" mode
    /// that reproduces PR-9.2 behavior (neutral 1.0 boost, rank 1
    /// everywhere).
    ///
    /// v3.5.8 Phase 9 Item 12 — `custom_entries` carries the
    /// platform's `custom_dictionary.db` matches for the current raw
    /// buffer (raw stored `(roman, hanji)` columns; DB stays native).
    /// Decoded verbatim from `FetchAtPos.custom_entries` and threaded
    /// to `handle_fetch_at_pos`, which synthesizes a full-buffer
    /// `RawCandidate` per entry and dedupes `(roman, hanji)` against
    /// the FST hits. Empty list = no custom matches / feature
    /// disabled — backward-compatible no-op.
    // 中文: Phase 6 新增 — 純讀取 Phase::Continuous 的 span-local 候選列表 (position 目前固定為 0)。
    // 中文: Phase 9.3a — 加帶平台 user_frequency.db 快照與 wall clock,供 SortKey recency + user_freq_boost 計算。
    // 中文: Phase 9 Item 12 — 加帶平台 custom_dictionary.db 命中 (raw roman/hanji),供 engine 合成 + (roman,hanji) 去重。
    FetchAtPos {
        position: u32,
        frequency_entries: Vec<protos::engine::FrequencyEntry>,
        now_ms: i64,
        custom_entries: Vec<protos::engine::CustomDictEntry>,
    },
    /// Nail a candidate segment in `Phase::Continuous`. The engine takes
    /// `pending[..consumed_bytes]` as the nailed segment's raw text and
    /// keeps `pending[consumed_bytes..]` as the new pending tail. When
    /// `consumed_bytes >= pending.len()`, this becomes a final commit and
    /// exits to Idle. Caller (Phase 6+ proto layer) is responsible for
    /// `consumed_bytes` aligning with both UTF-8 char boundaries and TL
    /// syllable boundaries returned by the syllabifier.
    // 中文: 連續輸入下挑選候選 segment;consumed_bytes >= pending.len() 為 final commit。
    CommitContinuous {
        display_text: String,
        // v3.5.8 Phase 9 Bug 1 (Option A): canonical key for freq/NextWord.
        // Empty → engine falls back to `display_text` (legacy callers).
        canonical_text: String,
        consumed_bytes: usize,
        syllable_count: u8,
    },
    // 中文: 中途 abort 連續輸入,清掉整段組字 (nailed + pending),退回 Idle。
    ResetContinuous,
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

    /// Pure-Rust observability of the engine's `EngineState`. Returns a
    /// clone so callers cannot mutate internal state. Phase 4 adds this so
    /// `tests/continuous_phase.rs` can assert `Phase::Continuous`'s
    /// `nailed` / `raw` fields without a corresponding proto carrier
    /// (the proto-side response shape lands in Phase 6). `#[doc(hidden)]`
    /// because this is a Phase-4-internal escape hatch — production
    /// callers should reach state through `apply` / `snapshot`'s
    /// `ComposingResponse` carrier (Codex post-impl note 1).
    // 中文: 回傳 EngineState 副本,讓測試可直接檢查 Phase::Continuous 內部結構。
    // 中文: doc-hidden — 這是 Phase 4 暫時 escape hatch,Phase 6 加 proto 欄位後可移除。
    #[doc(hidden)]
    pub fn snapshot_state(&self) -> EngineState {
        self.state.clone()
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
