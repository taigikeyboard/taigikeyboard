//! Public façade for the composing crate. Defines `Engine`, `EngineState`,
//! `Phase`, `Intent`, and `ComposingError`. Implementation of state
//! transitions lives in `transition.rs`; this module is the stable surface
//! that `dispatch.rs` and external crates consume.

// 中文: 組字 crate 的對外 API:Engine、EngineState、Phase、Intent、ComposingError。
// 中文: 真正的狀態轉移實作在 transition.rs,本檔案只定義穩定的型別介面。

use lexicon::{compound_hanji_exists, EngineHandle as LexiconHandle};
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
    /// v3.5.8 §10.2 segmented-spacing contract: adjacent segments (and
    /// the nailed prefix ↔ pending tail) are joined by a single space
    /// when the rendered script is roman-ish (roman-first / both-scripts);
    /// hanji-first / TPS render as-is with no separator. See
    /// [`continuous_word_space`] / [`nailed_prefix`]. This is the exact
    /// string a hard finalize (Enter / final-commit) writes to the
    /// document, so the separator policy applies identically there.
    // 中文: §10.2 / I1 — host 在單一 marked region 渲染的組字緩衝區本體。
    // 中文: Continuous = nailed_prefix(§10.2 詞界分隔)+ pending raw 衍生形;
    // 中文: roman-ish 才加空格,漢字優先/TPS 不加;hard finalize 寫入文件的就是此字串。
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

/// v3.5.8 — word-boundary separator policy for the Model B continuous
/// composing buffer (`docs/engine/continuous-input-ranking.md` §10.2
/// segmented-spacing contract). A single ASCII space joins adjacent
/// nailed segments (and the nailed prefix ↔ pending tail) **only when
/// the rendered script is roman-ish**: roman-first, or both-scripts
/// (`hit (彼)`). Hanji-first (`is_translate_swapped` without
/// `output_both_scripts`) and TPS render the hanji/bopomofo as-is with
/// no inter-segment space. This mirrors the platform
/// `appendAutoSpaceIfApplicable` predicate so the marked region and the
/// final-commit auto-space stay consistent. `is_translate_swapped`
/// alone cannot distinguish hanji-first from both-scripts (both set it
/// `true`) — hence the `output_both_scripts` AppConfig field
/// (Codex pre-impl 2026-05-18).
// 中文: §10.2 連續組字緩衝區的詞界空格策略 — 僅 roman-ish(羅馬字優先或雙腳本)
// 中文:   才在 nailed 段間 / nailed↔pending 加單一 ASCII 空格;漢字優先 / TPS 不加。
// 中文:   is_translate_swapped 無法區分漢字優先 vs 雙腳本(都為 true),故需 output_both_scripts。
fn continuous_word_space(config: &AppConfig) -> bool {
    let effective_swapped = config.is_translate_swapped || config.input_mode == "tps";
    // De Morgan of the platform `appendAutoSpaceIfApplicable` guard
    // `if (effectiveSwapped && !outputBothScripts) return`: roman-ish =
    // not swapped, OR both-scripts is on.
    !effective_swapped || config.output_both_scripts
}

/// Pure `Σ nailed[i].display_text` join, parameterized by the
/// roman-ish `space` predicate and a `is_compound` oracle. **Single
/// source of truth** for the nailed-prefix concatenation; do not
/// re-inline this loop.
///
/// Inter-segment boundary policy (only when `space`, i.e. roman-ish —
/// hanji-first / TPS have no separator at all):
/// - boundary after a hyphen-continuation segment (`tai-`, `s` ends
///   with `-`): emit **nothing** (existing mid-word rule, mirrors the
///   platform `appendAutoSpaceIfApplicable` `endsWith("-")` guard) and
///   treat the boundary as already hyphenated for chain-prevention;
/// - else a `-` (instead of the word-boundary space) iff the previous
///   boundary was NOT a hyphen (left-to-right non-overlapping bigram,
///   blocks `A-B-C` over-gluing while still allowing `A B-C`), both
///   adjacent segments are single-syllable, and
///   `is_compound(prev.canonical_text + seg.canonical_text)` — i.e.
///   the pair reconstructs a known 2-syllable dictionary compound
///   (查某 → `tsa-bóo`); otherwise a single space.
///
/// The separator is a render/commit join concern, **dictionary-informed**
/// for known two-syllable compounds (Codex pre-impl 2026-05-18), and is
/// NEVER stored in `NailedSegment.display_text` / `raw_text` —
/// backspace-pop restores the editable tail from `raw_text`, so segment
/// data stays separator-free; the hyphen is derived fresh on every
/// render from the segments' own `canonical_text` + `syllable_count`.
// 中文: 純串接迴圈;唯一真相來源,勿內聯。分隔符屬 render/commit 呈現層,
// 中文:   但對「詞庫已知 2 音節複合詞」用連字號(查某→tsa-bóo);絕不寫入
// 中文:   NailedSegment(backspace 由 raw_text 還原,連字號每次 render 由
// 中文:   canonical_text + syllable_count 即時推導)。左→右不重疊 bigram
// 中文:   (prev_hyphen 防 A-B-C 過度黏連);trailing-`-` 視為已連字號。
fn nailed_prefix_with_oracle(
    nailed: &[NailedSegment],
    space: bool,
    is_compound: impl Fn(&str) -> bool,
) -> String {
    let mut s = String::new();
    // The previous emitted inter-segment boundary rendered as a hyphen
    // (an auto-compound `-` OR a user-typed trailing `-` that
    // suppressed the separator). Blocks `A-B-C` over-gluing (Codex
    // pre-impl R4 2026-05-18).
    let mut prev_hyphen = false;
    for (i, seg) in nailed.iter().enumerate() {
        if i > 0 && space {
            if s.ends_with('-') {
                // Mid-word hyphen-continuation: emit nothing; the
                // boundary is already hyphenated.
                prev_hyphen = true;
            } else {
                let prev = &nailed[i - 1];
                let compound = !prev_hyphen
                    && prev.syllable_count == 1
                    && seg.syllable_count == 1
                    && is_compound(&format!("{}{}", prev.canonical_text, seg.canonical_text));
                if compound {
                    s.push('-');
                    prev_hyphen = true;
                } else {
                    s.push(' ');
                    prev_hyphen = false;
                }
            }
        }
        s.push_str(&seg.display_text);
    }
    s
}

/// `Σ nailed[i].display_text` joined with the §10.2 word-boundary
/// separator (see [`continuous_word_space`]), with two adjacent
/// single-syllable segments that reconstruct a **known 2-syllable
/// dictionary compound** joined by an internal hyphen instead
/// (查某 → `tsa-bóo`, v3.5.8 §10.2 Option A — manual single-syllable
/// tap path).
///
/// One lexicon lock for the whole join (Codex pre-impl R2 2026-05-18).
/// Cheap pre-gate first: a compound hyphen can only fire when the
/// render is roman-ish AND there is at least one adjacent
/// single-syllable pair — otherwise the lexicon is never touched. When
/// no dictionary is installed (`with_state` → `Err`, e.g. unit tests)
/// the join degrades to the pure space-join: byte-identical to the
/// pre-Option-A behaviour.
// 中文: §10.2 詞界分隔;相鄰兩單音節段若還原成詞庫已知 2 音節複合詞改用連字號。
// 中文: 整段 join 只鎖一次 lexicon(R2);cheap 前置閘擋掉非 roman-ish / 無單音節對;
// 中文: 未安裝詞庫(with_state Err,如單元測試)→ 退化為純空格 join(行為不變)。
pub(crate) fn nailed_prefix(nailed: &[NailedSegment], config: &AppConfig) -> String {
    let space = continuous_word_space(config);
    let eligible = space
        && nailed.len() >= 2
        && nailed
            .windows(2)
            .any(|w| w[0].syllable_count == 1 && w[1].syllable_count == 1);
    if !eligible {
        return nailed_prefix_with_oracle(nailed, space, |_| false);
    }
    LexiconHandle::with_state(|state| {
        let (Some(prefix), Some(dict)) = (state.prefix_index.as_ref(), state.dictionary.as_ref())
        else {
            return Ok(nailed_prefix_with_oracle(nailed, space, |_| false));
        };
        Ok(nailed_prefix_with_oracle(nailed, space, |h| {
            compound_hanji_exists(h, prefix, dict)
        }))
    })
    .unwrap_or_else(|_| nailed_prefix_with_oracle(nailed, space, |_| false))
}

/// The Model B composing-buffer surface for a `(nailed, raw)` pair:
/// `Σ nailed[i].display_text` followed by the derived display of the
/// pending `raw` tail. **Single source of truth** — both
/// [`Phase::composing_display`] and the `transition.rs` Continuous paths
/// route through this so the rendered preedit and the hard-finalize commit
/// can never diverge (Codex post-impl review point).
// 中文: Model B 組字緩衝區本體 (nailed 前綴 + pending raw 衍生形) 的唯一真相來源;
// 中文: Phase::composing_display 與 transition.rs 各 Continuous path 全走此,確保不漂移。
pub(crate) fn combined_display(nailed: &[NailedSegment], raw: &str, config: &AppConfig) -> String {
    let mut s = nailed_prefix(nailed, config);
    let derived = crate::derived::derived_display(raw, config);
    // §10.2 word boundary between the nailed prefix and the pending
    // tail (the tail is the next word). Same predicate + trailing-`-`
    // suppression as the inter-segment join.
    if !s.is_empty() && !derived.is_empty() && continuous_word_space(config) && !s.ends_with('-') {
        s.push(' ');
    }
    s.push_str(&derived);
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
    ///
    /// **v3.5.9 B-4** — `roman` may legitimately be either TL or POJ
    /// display form (whichever the user typed when storing the
    /// entry). The engine treats it as raw on the lattice / dedupe
    /// axis (`composing::shadow::custom_toneless_key` canonicalizes
    /// per-mode for the FST family) and folds it through
    /// `phonetics::api::canonical_tl_form` only when synthesizing the
    /// `user_frequency.db` commit key (`display_text`), keeping the
    /// commit key mode-invariant.
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

#[cfg(test)]
mod tests {
    //! v3.5.8 §10.2 segmented-spacing contract pins for the Model B
    //! composing-buffer join (`nailed_prefix` / `combined_display`).
    //! Behavioral mid/final-commit coverage lives in
    //! `tests/continuous_phase.rs` + `tests/raw_input_pending_tail.rs`;
    //! these unit-pin the predicate matrix directly.

    use super::{
        combined_display, nailed_prefix, nailed_prefix_with_oracle, AppConfig, NailedSegment,
    };

    fn seg(display: &str) -> NailedSegment {
        NailedSegment {
            display_text: display.to_owned(),
            canonical_text: display.to_owned(),
            raw_text: display.to_owned(),
            raw_span: (0, display.len()),
            syllable_count: 1,
        }
    }

    /// Distinct `display_text` vs `canonical_text` (roman-display,
    /// hanji-canonical) + explicit syllable count — the §10.2 Option A
    /// compound-hyphen tests key the oracle off `canonical_text`.
    fn seg_dc(display: &str, canonical: &str, syllable_count: u8) -> NailedSegment {
        NailedSegment {
            display_text: display.to_owned(),
            canonical_text: canonical.to_owned(),
            raw_text: display.to_owned(),
            raw_span: (0, display.len()),
            syllable_count,
        }
    }

    /// `input_mode` + the two swap flags are the only fields the
    /// separator predicate reads; the rest stay at proto defaults.
    fn cfg(input_mode: &str, swapped: bool, both: bool) -> AppConfig {
        AppConfig {
            tone_mode: String::new(),
            input_mode: input_mode.to_owned(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_translate_swapped: swapped,
            is_association_recording_enabled: false,
            platform_id: 0,
            output_both_scripts: both,
        }
    }

    #[test]
    fn roman_first_inserts_word_boundary_space_between_segments() {
        let n = [seg("hit"), seg("tui")];
        assert_eq!(nailed_prefix(&n, &cfg("tl", false, false)), "hit tui");
    }

    #[test]
    fn hanji_first_has_no_inter_segment_space() {
        let n = [seg("彼"), seg("隻")];
        // is_translate_swapped without output_both_scripts → hanji-first.
        assert_eq!(nailed_prefix(&n, &cfg("tl", true, false)), "彼隻");
    }

    #[test]
    fn both_scripts_is_roman_ish_and_spaced_even_when_swapped() {
        let n = [seg("hit (彼)"), seg("tui (隻)")];
        assert_eq!(
            nailed_prefix(&n, &cfg("tl", true, true)),
            "hit (彼) tui (隻)"
        );
    }

    #[test]
    fn tps_renders_as_is_no_space() {
        let n = [seg("ㄏㄧㆵ"), seg("ㄉㄨㄧ")];
        // input_mode == "tps" → effective_swapped regardless of flag.
        assert_eq!(nailed_prefix(&n, &cfg("tps", false, false)), "ㄏㄧㆵㄉㄨㄧ");
    }

    #[test]
    fn trailing_hyphen_segment_suppresses_the_following_space() {
        // A hyphen-continuation segment (`tai-`) is mid-word; no space
        // after it even in roman-first (mirrors the platform
        // `appendAutoSpaceIfApplicable` `endsWith("-")` rule).
        let n = [seg("tai-"), seg("uan")];
        assert_eq!(nailed_prefix(&n, &cfg("tl", false, false)), "tai-uan");
    }

    #[test]
    fn empty_and_single_segment_have_no_leading_or_trailing_space() {
        assert_eq!(nailed_prefix(&[], &cfg("tl", false, false)), "");
        assert_eq!(
            nailed_prefix(&[seg("hit")], &cfg("tl", false, false)),
            "hit"
        );
    }

    #[test]
    fn combined_display_spaces_nailed_prefix_against_pending_tail() {
        let n = [seg("珠")];
        // derived_display("a", tl) == "a" (verbatim, no tone digit) →
        // roman-first inserts the §10.2 boundary space.
        assert_eq!(combined_display(&n, "a", &cfg("tl", false, false)), "珠 a");
        // hanji-first: no boundary space.
        assert_eq!(combined_display(&n, "a", &cfg("tl", true, false)), "珠a");
        // No pending tail → no dangling separator.
        assert_eq!(combined_display(&n, "", &cfg("tl", false, false)), "珠");
        // No nailed prefix → tail only, no leading separator.
        assert_eq!(combined_display(&[], "a", &cfg("tl", false, false)), "a");
    }

    // ---- v3.5.8 §10.2 Option A — dictionary-compound hyphen join ----
    // `nailed_prefix_with_oracle` is the pure join; these pin the
    // separator policy against a hermetic compound oracle (the live
    // lexicon-backed path is locked in `engine/lexicon/tests/
    // compound_hanji.rs` + the graceful no-lexicon path by the §10.2
    // predicate-matrix tests above, which now route through this fn).

    #[test]
    fn oracle_false_everywhere_is_byte_identical_to_plain_space_join() {
        // Regression pin: with no compound ever, the roman-ish join is
        // exactly the pre-Option-A behaviour.
        let n = [seg("hit"), seg("tui")];
        assert_eq!(nailed_prefix_with_oracle(&n, true, |_| false), "hit tui");
        // Hanji-first / TPS (space = false) → no separator at all,
        // oracle irrelevant.
        let h = [seg("彼"), seg("隻")];
        assert_eq!(nailed_prefix_with_oracle(&h, false, |_| true), "彼隻");
    }

    #[test]
    fn known_two_syllable_compound_renders_internal_hyphen() {
        // hit ê tsa bóo → hit ê tsa-bóo (查某 is the only 2-syll
        // compound; "彼个" / others are not in the oracle).
        let n = [
            seg_dc("hit", "彼", 1),
            seg_dc("ê", "个", 1),
            seg_dc("tsa", "查", 1),
            seg_dc("bóo", "某", 1),
        ];
        assert_eq!(
            nailed_prefix_with_oracle(&n, true, |h| h == "查某"),
            "hit ê tsa-bóo"
        );
    }

    #[test]
    fn left_to_right_non_overlapping_bigram_blocks_a_b_c_overgluing() {
        // Oracle says both 查某 and 某人 are 2-syll compounds. Strict
        // non-overlapping left-to-right pairing hyphens (查,某) then
        // blocks (某,人) — "查-某 人", never "查-某-人".
        let n = [
            seg_dc("tsa", "查", 1),
            seg_dc("bóo", "某", 1),
            seg_dc("lâng", "人", 1),
        ];
        assert_eq!(
            nailed_prefix_with_oracle(&n, true, |h| h == "查某" || h == "某人"),
            "tsa-bóo lâng"
        );
    }

    #[test]
    fn multi_syllable_segment_is_not_a_compound_bigram_member() {
        // A 2-syllable nailed segment (e.g. the compound was tapped
        // whole) is never re-hyphenated against a neighbour even if the
        // oracle would match the canonical concatenation.
        let n = [seg_dc("tsa-bóo", "查某", 2), seg_dc("lâng", "人", 1)];
        assert_eq!(
            nailed_prefix_with_oracle(&n, true, |_| true),
            "tsa-bóo lâng"
        );
    }

    #[test]
    fn user_typed_trailing_hyphen_suppresses_then_blocks_next_compound() {
        // "tai-" is a user hyphen-continuation: boundary emits nothing
        // and counts as already-hyphenated, so the following (uan, X)
        // boundary cannot also auto-compound (gets a plain space).
        let n = [
            seg_dc("tai-", "台", 1),
            seg_dc("uan", "灣", 1),
            seg_dc("X", "國", 1),
        ];
        assert_eq!(
            nailed_prefix_with_oracle(&n, true, |h| h == "灣國"),
            "tai-uan X"
        );
    }

    #[test]
    fn no_lexicon_installed_keeps_compound_pairs_space_joined() {
        // `nailed_prefix` (lexicon-wired) with no dictionary installed
        // in the unit-test process → graceful pure space-join.
        let n = [seg_dc("tsa", "查", 1), seg_dc("bóo", "某", 1)];
        assert_eq!(nailed_prefix(&n, &cfg("tl", false, false)), "tsa bóo");
    }
}
