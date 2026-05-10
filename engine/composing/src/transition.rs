//! Pure transition function: `(state, intent, config) → ComposingResponse`.
//! Pure — no logging, no FFI, no platform types. The dispatcher in
//! `dispatch.rs` runs this and returns the response to callers.
//!
//! Effect ordering matters; iOS/Android downstream wrappers consume effects
//! in proto-list order. `prost` preserves order on `repeated Effect` fields.
//!
//! `Phase::Continuous` (v3.5.8) follows MOE-app UX: committed segments live
//! in the document (already inserted via earlier `CommitTextReplacingPreedit`
//! effects), while only the un-committed `raw` tail occupies the marked /
//! preedit region. `ComposingResponse.preedit` therefore carries pending-only
//! state during Continuous; tests inspect committed via
//! `Engine::snapshot_state` until Phase 6 lands proto carriers.

// 中文: 純粹的狀態轉移函式,不做 log/FFI/平台型別轉換。
// 中文: Effect 順序攸關平台端消化,prost 對 repeated Effect 會保留順序。
// 中文: Phase::Continuous 沿用 MOE app UX — committed segments 已寫入文件,
// 中文: 只有 pending raw 留在 marked text 區。preedit 欄位只反映 pending。

use crate::api::{CommittedSegment, EngineState, Intent, Phase};
use crate::derived::derived_display;
use protos::engine::composing_response::Preedit;
use protos::engine::effect;
use protos::engine::AppConfig;
use protos::engine::{
    ClearPreeditWithoutCommit, CommitTextReplacingPreedit, ComposingResponse,
    DeleteBackwardFromDocument, Effect, NextWordClearForNewComposing,
    NextWordUpdateLastSelectedWord, NextWordWordSelected, PerformAutocomplete, ResetAutocomplete,
    ResetAutocompleteContext, UpdatePreedit,
};

/// Apply `intent` against `state`, mutate, return the proto response.
// 中文: 依 intent 變更 state,並回傳 proto 組字回應 (狀態機核心)。
pub(crate) fn apply(
    state: &mut EngineState,
    intent: Intent,
    config: &AppConfig,
) -> ComposingResponse {
    match intent {
        Intent::Start { text } => match &state.phase {
            Phase::Continuous { .. } => start_under_continuous(state, text, config),
            _ => enter_composing(state, text, config),
        },
        Intent::Append { ch } => match &state.phase {
            Phase::Idle => enter_composing(state, ch, config),
            Phase::Composing { raw } => {
                let mut next = raw.clone();
                next.push_str(&ch);
                enter_composing(state, next, config)
            }
            Phase::Continuous { .. } => append_continuous(state, ch, config),
        },
        Intent::AppendHyphen => apply(
            state,
            Intent::Append {
                ch: "-".to_string(),
            },
            config,
        ),
        Intent::ReplaceLast { replacement } => replace_last(state, replacement, config),
        Intent::DeleteBackward => delete_backward(state, config),
        Intent::CommitDerived => match &state.phase {
            Phase::Continuous { .. } => noop(state, config),
            _ => commit_derived(state, config),
        },
        Intent::CommitRaw => match &state.phase {
            Phase::Continuous { .. } => noop(state, config),
            _ => commit_raw(state, config),
        },
        Intent::SelectSuggestion { text } => match &state.phase {
            Phase::Continuous { .. } => select_suggestion_under_continuous(state, text, config),
            _ => select_suggestion(state, text, config),
        },
        Intent::CommitPreeditThenInsertExternal { text } => match &state.phase {
            Phase::Continuous { .. } => {
                commit_preedit_then_insert_external_under_continuous(state, text, config)
            }
            _ => commit_preedit_then_insert_external(state, text, config),
        },
        Intent::Reset => reset(state, config),
        Intent::SetSelectedCandidateIndex { index } => {
            set_selected_candidate_index(state, index, config)
        }
        Intent::QueryState => snapshot(state, config),
        Intent::EnterContinuous => enter_continuous(state, config),
        // FetchAtPos is a read-only query that needs lexicon state; the
        // dispatcher short-circuits before reaching `apply`. Reaching
        // here means a caller bypassed dispatch (test path or future
        // refactor) — return a snapshot rather than panic so the
        // invariant "transition is total" holds (Codex post-impl
        // continuous_phase findings #2/#3 pattern).
        Intent::FetchAtPos { .. } => snapshot(state, config),
        Intent::CommitContinuous {
            display_text,
            consumed_bytes,
            syllable_count,
        } => commit_continuous(state, display_text, consumed_bytes, syllable_count, config),
        Intent::ResetContinuous => reset_continuous(state, config),
    }
}

// ---- Helpers ------------------------------------------------------

/// Drop the last `char` from `s` in a single UTF-8 walk via `Chars::as_str`.
/// Returns "" if `s` is empty.
// 中文: 安全移除字串最後一個 Unicode 字元 (單次 UTF-8 走訪),空字串回傳 ""。
fn drop_last_char(s: &str) -> String {
    let mut it = s.chars();
    it.next_back();
    it.as_str().to_string()
}

/// Build a `composing` step response (typing / replace-last / delete-backward
/// non-empty branches). All three update the preedit + request a fresh
/// autocomplete query against the new buffer.
// 中文: 組成「組字進行中」的回應,負責同時更新預編輯並觸發候選詞查詢。
fn step_response(raw: String, display: String, selected_index: i32) -> ComposingResponse {
    ComposingResponse {
        preedit: Some(Preedit {
            raw_input: raw,
            display_text: display.clone(),
        }),
        effect: vec![update_preedit(display), perform_autocomplete()],
        selected_candidate_index: selected_index,
        is_composing: true,
        continuous: None,
    }
}

/// Enter or update the composing phase. A fresh composition step resets
/// `selected_candidate_index` to 0.
// 中文: 進入或更新 Composing 階段;每次新輸入會把候選索引重設為 0。
fn enter_composing(state: &mut EngineState, raw: String, config: &AppConfig) -> ComposingResponse {
    state.phase = Phase::Composing { raw: raw.clone() };
    state.selected_candidate_index = 0;
    let display = derived_display(&raw, config);
    step_response(raw, display, 0)
}

/// TPS auto-correct. Preserves `selected_candidate_index` (correction on top
/// of an in-progress selection). Under `Phase::Continuous` operates on the
/// pending tail only — committed segments are untouched.
// 中文: TPS 自動修正:替換尾端字元,保留目前候選索引 (修正疊在已選擇之上)。
// 中文: Continuous phase 下只動 pending 尾,committed segments 不受影響。
fn replace_last(
    state: &mut EngineState,
    replacement: String,
    config: &AppConfig,
) -> ComposingResponse {
    match &state.phase {
        Phase::Composing { raw } => {
            if raw.is_empty() {
                return noop(state, config);
            }
            let mut new_raw = drop_last_char(raw);
            new_raw.push_str(&replacement);
            state.phase = Phase::Composing {
                raw: new_raw.clone(),
            };
            let display = derived_display(&new_raw, config);
            step_response(new_raw, display, state.selected_candidate_index)
        }
        Phase::Continuous { raw, committed } => {
            if raw.is_empty() {
                return noop(state, config);
            }
            let mut new_pending = drop_last_char(raw);
            new_pending.push_str(&replacement);
            // Empty pending + empty committed = degenerate Continuous state
            // (Codex post-impl finding #2). Exit to Idle and clear nextword.
            if new_pending.is_empty() && committed.is_empty() {
                return exit_to_idle(
                    state,
                    vec![
                        clear_preedit_without_commit(),
                        reset_autocomplete(),
                        next_word_clear_for_new_composing(),
                    ],
                );
            }
            let pending_display = derived_display(&new_pending, config);
            let preserved_index = state.selected_candidate_index;
            state.phase = Phase::Continuous {
                raw: new_pending.clone(),
                committed: committed.clone(),
            };
            continuous_step_response(new_pending, pending_display, preserved_index)
        }
        Phase::Idle => noop(state, config),
    }
}

fn delete_backward(state: &mut EngineState, config: &AppConfig) -> ComposingResponse {
    match &state.phase {
        Phase::Composing { raw } => {
            if raw.is_empty() {
                return noop(state, config);
            }
            let new_raw = drop_last_char(raw);
            if new_raw.is_empty() {
                return exit_to_idle(
                    state,
                    vec![
                        clear_preedit_without_commit(),
                        reset_autocomplete(),
                        delete_backward_from_document(),
                    ],
                );
            }
            state.phase = Phase::Composing {
                raw: new_raw.clone(),
            };
            state.selected_candidate_index = 0;
            let display = derived_display(&new_raw, config);
            step_response(new_raw, display, 0)
        }
        Phase::Continuous { raw, committed } => {
            delete_backward_continuous(state, raw.clone(), committed.clone(), config)
        }
        Phase::Idle => noop(state, config),
    }
}

/// `DeleteBackward` under `Phase::Continuous`. Three branches:
///   1. pending non-empty → drop last char of pending; if pending now empty
///      and committed is also empty, exit to Idle (would-be DeleteBackwardFromDocument
///      forwarded so the platform deletes a real document char on the next press).
///   2. pending empty AND committed non-empty → pop the last segment, restore
///      its `raw_text` as the new pending, emit N `DeleteBackwardFromDocument`
///      effects to remove the segment's display chars from the document, then
///      `UpdatePreedit` with the restored pending. N = popped.display_text char count.
///   3. pending empty AND committed empty → exit to Idle (degenerate case;
///      shouldn't occur in steady state but guarded).
// 中文: Continuous phase 下的 backspace 折回:有 pending 砍尾;沒 pending 但有
// 中文: committed 則 pop 最後一段並從文件刪掉對應字數;兩者皆空則退到 Idle。
fn delete_backward_continuous(
    state: &mut EngineState,
    pending: String,
    committed: Vec<CommittedSegment>,
    config: &AppConfig,
) -> ComposingResponse {
    if !pending.is_empty() {
        let new_pending = drop_last_char(&pending);
        if new_pending.is_empty() && committed.is_empty() {
            return exit_to_idle(
                state,
                vec![
                    clear_preedit_without_commit(),
                    reset_autocomplete(),
                    next_word_clear_for_new_composing(),
                    delete_backward_from_document(),
                ],
            );
        }
        state.phase = Phase::Continuous {
            raw: new_pending.clone(),
            committed,
        };
        state.selected_candidate_index = 0;
        let pending_display = derived_display(&new_pending, config);
        return continuous_step_response(new_pending, pending_display, 0);
    }

    // pending empty branches
    if committed.is_empty() {
        return exit_to_idle(
            state,
            vec![
                clear_preedit_without_commit(),
                reset_autocomplete(),
                next_word_clear_for_new_composing(),
                delete_backward_from_document(),
            ],
        );
    }

    let mut new_committed = committed;
    // JUSTIFICATION: `committed.is_empty()` was checked at the branch above;
    // popping a non-empty Vec is a programmer-invariant guarantee, not a
    // data path.
    let popped = new_committed
        .pop()
        .expect("committed non-empty checked above");
    let restore_chars = popped.display_text.chars().count();
    let new_pending = popped.raw_text;
    let pending_display = derived_display(&new_pending, config);
    // Codex post-impl finding #1: roll the popped segment back out of
    // nextword's `last_selected_word` so the next final commit can't record
    // a false association from a word no longer in the document.
    let nextword_correction = match new_committed.last() {
        Some(prev) => {
            next_word_update_last_selected_word(prev.display_text.clone(), prev.raw_text.clone())
        }
        None => next_word_clear_for_new_composing(),
    };
    state.phase = Phase::Continuous {
        raw: new_pending.clone(),
        committed: new_committed,
    };
    state.selected_candidate_index = 0;

    let mut effects: Vec<Effect> = (0..restore_chars)
        .map(|_| delete_backward_from_document())
        .collect();
    effects.push(nextword_correction);
    effects.push(update_preedit(pending_display.clone()));
    effects.push(perform_autocomplete());
    ComposingResponse {
        preedit: Some(Preedit {
            raw_input: new_pending,
            display_text: pending_display,
        }),
        effect: effects,
        selected_candidate_index: 0,
        is_composing: true,
        continuous: None,
    }
}

fn commit_derived(state: &mut EngineState, config: &AppConfig) -> ComposingResponse {
    let Phase::Composing { raw } = &state.phase else {
        return noop(state, config);
    };
    let display = derived_display(raw, config);
    if display.is_empty() {
        return noop(state, config);
    }
    exit_to_idle(
        state,
        vec![
            commit_text_replacing_preedit(display),
            reset_autocomplete(),
            reset_autocomplete_context(),
        ],
    )
}

fn commit_raw(state: &mut EngineState, config: &AppConfig) -> ComposingResponse {
    let Phase::Composing { raw } = &state.phase else {
        return noop(state, config);
    };
    if raw.is_empty() {
        return noop(state, config);
    }
    let text = raw.clone();
    exit_to_idle(
        state,
        vec![
            commit_text_replacing_preedit(text),
            reset_autocomplete(),
            reset_autocomplete_context(),
        ],
    )
}

fn select_suggestion(
    state: &mut EngineState,
    text: String,
    config: &AppConfig,
) -> ComposingResponse {
    if !matches!(state.phase, Phase::Composing { .. }) {
        return noop(state, config);
    }
    exit_to_idle(
        state,
        vec![
            commit_text_replacing_preedit(text),
            reset_autocomplete(),
            reset_autocomplete_context(),
        ],
    )
}

fn commit_preedit_then_insert_external(
    state: &mut EngineState,
    external: String,
    config: &AppConfig,
) -> ComposingResponse {
    if external.is_empty() {
        return noop(state, config);
    }
    if let Phase::Composing { raw } = &state.phase {
        let mut combined = derived_display(raw, config);
        combined.push_str(&external);
        return exit_to_idle(
            state,
            vec![
                commit_text_replacing_preedit(combined),
                reset_autocomplete(),
                reset_autocomplete_context(),
            ],
        );
    }
    // Idle → plain insert. `CommitTextReplacingPreedit` is no-op-on-empty-preedit
    // safe on both platforms.
    exit_to_idle(state, vec![commit_text_replacing_preedit(external)])
}

/// User-initiated reset. `Phase::Continuous` adds `NextWordClearForNewComposing`
/// to the standard Composing reset effect pair so the platform tears down
/// nextword's continuous-mode candidate strip.
// 中文: 使用者主動 reset;Continuous 額外發 NextWordClearForNewComposing 通知平台。
fn reset(state: &mut EngineState, config: &AppConfig) -> ComposingResponse {
    match state.phase {
        Phase::Idle => noop(state, config),
        Phase::Composing { .. } => exit_to_idle(
            state,
            vec![clear_preedit_without_commit(), reset_autocomplete()],
        ),
        Phase::Continuous { .. } => exit_to_idle(
            state,
            vec![
                clear_preedit_without_commit(),
                reset_autocomplete(),
                next_word_clear_for_new_composing(),
            ],
        ),
    }
}

fn set_selected_candidate_index(
    state: &mut EngineState,
    index: i32,
    config: &AppConfig,
) -> ComposingResponse {
    state.selected_candidate_index = index;
    snapshot(state, config)
}

fn snapshot(state: &EngineState, config: &AppConfig) -> ComposingResponse {
    let (raw, display, is_composing) = match &state.phase {
        Phase::Idle => (String::new(), String::new(), false),
        Phase::Composing { raw } => (raw.clone(), derived_display(raw, config), true),
        Phase::Continuous { raw, .. } => (raw.clone(), derived_display(raw, config), true),
    };
    ComposingResponse {
        preedit: Some(Preedit {
            raw_input: raw,
            display_text: display,
        }),
        effect: Vec::new(),
        selected_candidate_index: state.selected_candidate_index,
        is_composing,
        continuous: None,
    }
}

fn exit_to_idle(state: &mut EngineState, effects: Vec<Effect>) -> ComposingResponse {
    state.phase = Phase::Idle;
    state.selected_candidate_index = -1;
    ComposingResponse {
        preedit: Some(Preedit::default()),
        effect: effects,
        selected_candidate_index: -1,
        is_composing: false,
        continuous: None,
    }
}

fn noop(state: &EngineState, config: &AppConfig) -> ComposingResponse {
    snapshot(state, config)
}

// ---- Continuous-phase helpers --------------------------------------

/// Step response for `Phase::Continuous` mid-composition mutations (typing,
/// TPS replace-last, segment-pop). Pending-only display in `preedit`;
/// committed segments live in the document, not the marked region.
// 中文: Continuous phase 內仍在編輯中的回應;preedit 只反映 pending。
fn continuous_step_response(
    pending: String,
    pending_display: String,
    selected_index: i32,
) -> ComposingResponse {
    ComposingResponse {
        preedit: Some(Preedit {
            raw_input: pending,
            display_text: pending_display.clone(),
        }),
        effect: vec![update_preedit(pending_display), perform_autocomplete()],
        selected_candidate_index: selected_index,
        is_composing: true,
        continuous: None,
    }
}

/// `Phase::Composing { raw }` → `Phase::Continuous { raw, committed: [] }`.
/// Marked text was already derived from the same `raw`, so no preedit
/// refresh is necessary; emit zero effects. Idle / already-Continuous /
/// empty-raw Composing → noop (the `Continuous { raw: "", committed: [] }`
/// state is invalid; entering it from a degenerate empty Composing buffer
/// would violate the "Continuous is non-empty in at least one of pending /
/// committed" invariant — Codex post-impl finding #2).
// 中文: Composing → Continuous;raw 不變 committed 起始為空,不發 effect。
// 中文: Composing.raw 為空時不轉,維持「Continuous 至少 pending 或 committed 一邊非空」不變式。
fn enter_continuous(state: &mut EngineState, config: &AppConfig) -> ComposingResponse {
    let Phase::Composing { raw } = &state.phase else {
        return noop(state, config);
    };
    if raw.is_empty() {
        return noop(state, config);
    }
    let raw = raw.clone();
    let pending_display = derived_display(&raw, config);
    state.phase = Phase::Continuous {
        raw: raw.clone(),
        committed: Vec::new(),
    };
    ComposingResponse {
        preedit: Some(Preedit {
            raw_input: raw,
            display_text: pending_display,
        }),
        effect: Vec::new(),
        selected_candidate_index: state.selected_candidate_index,
        is_composing: true,
        continuous: None,
    }
}

/// `Intent::Start { text }` arriving while in `Phase::Continuous`. Codex
/// post-impl finding #3: silently dropping `text` would lose user input.
/// Treats the intent as "drop continuous state, then begin a fresh
/// Composing buffer with `text`". Emits the ResetContinuous effect trio
/// followed by the regular Composing entry effects; final state is
/// `Phase::Composing { raw: text }` (or Idle if `text` is empty).
// 中文: Continuous 下收到 Start { text }:先 drop 連續狀態,再 enter_composing(text)。
fn start_under_continuous(
    state: &mut EngineState,
    text: String,
    config: &AppConfig,
) -> ComposingResponse {
    // Drop continuous state to Idle first.
    state.phase = Phase::Idle;
    state.selected_candidate_index = -1;
    let mut effects = vec![
        clear_preedit_without_commit(),
        reset_autocomplete(),
        next_word_clear_for_new_composing(),
    ];
    let resp = enter_composing(state, text, config);
    effects.extend(resp.effect);
    ComposingResponse {
        effect: effects,
        ..resp
    }
}

/// `Intent::SelectSuggestion { text }` arriving while in `Phase::Continuous`.
/// Codex post-impl finding #3: silently dropping `text` would lose user
/// selection. Drops continuous state and commits `text` to the document
/// (replacing the marked pending region). Emits the ResetContinuous
/// trio + the regular SelectSuggestion commit effects.
// 中文: Continuous 下收到 SelectSuggestion { text }:drop 連續狀態,把 text 上屏。
fn select_suggestion_under_continuous(
    state: &mut EngineState,
    text: String,
    config: &AppConfig,
) -> ComposingResponse {
    if text.is_empty() {
        // Empty suggestion: drop continuous state without inserting. Mirrors
        // SelectSuggestion-on-Idle being a no-op.
        return reset_continuous(state, config);
    }
    let _ = config;
    exit_to_idle(
        state,
        vec![
            commit_text_replacing_preedit(text),
            reset_autocomplete(),
            reset_autocomplete_context(),
            next_word_clear_for_new_composing(),
        ],
    )
}

/// `Intent::CommitPreeditThenInsertExternal { text }` under Continuous.
/// Codex post-impl finding #3. Combines the pending derived display with
/// the external text into a single document commit (mirroring the
/// Composing branch), then exits Continuous. Empty `text` collapses to
/// `noop` (matches Composing semantics).
// 中文: Continuous 下:把 pending 衍生形 + external 合成單次 commit,退出。
fn commit_preedit_then_insert_external_under_continuous(
    state: &mut EngineState,
    external: String,
    config: &AppConfig,
) -> ComposingResponse {
    if external.is_empty() {
        return noop(state, config);
    }
    let Phase::Continuous { raw, .. } = &state.phase else {
        return noop(state, config);
    };
    let mut combined = derived_display(raw, config);
    combined.push_str(&external);
    exit_to_idle(
        state,
        vec![
            commit_text_replacing_preedit(combined),
            reset_autocomplete(),
            reset_autocomplete_context(),
            next_word_clear_for_new_composing(),
        ],
    )
}

/// `Phase::Continuous` segment commit. `consumed_bytes >= pending.len()` is
/// the final-commit branch (exit to Idle); otherwise mid-commit (stay in
/// Continuous). Programmer-error inputs (out-of-range or non-char-boundary
/// `consumed_bytes`) collapse to `noop` rather than panicking.
// 中文: Continuous 下的 commit;consumed_bytes >= pending.len() 為 final commit。
// 中文: 邊界錯誤 (超界 / 非 UTF-8 邊界) 一律降為 noop,不 panic。
fn commit_continuous(
    state: &mut EngineState,
    display_text: String,
    consumed_bytes: usize,
    syllable_count: u8,
    config: &AppConfig,
) -> ComposingResponse {
    let Phase::Continuous { raw, committed } = &state.phase else {
        return noop(state, config);
    };
    if display_text.is_empty()
        || consumed_bytes == 0
        || consumed_bytes > raw.len()
        || !raw.is_char_boundary(consumed_bytes)
    {
        return noop(state, config);
    }
    let pending = raw.clone();
    let mut new_committed = committed.clone();
    let raw_text = pending[..consumed_bytes].to_string();
    let prev_end = new_committed.last().map(|s| s.raw_span.1).unwrap_or(0);
    let raw_span = (prev_end, prev_end + consumed_bytes);
    let new_pending = pending[consumed_bytes..].to_string();
    let segment = CommittedSegment {
        display_text: display_text.clone(),
        raw_text: raw_text.clone(),
        raw_span,
        syllable_count,
    };
    new_committed.push(segment);

    if new_pending.is_empty() {
        // Final commit: drop committed list, exit to Idle.
        return exit_to_idle(
            state,
            vec![
                commit_text_replacing_preedit(display_text.clone()),
                reset_autocomplete(),
                reset_autocomplete_context(),
                next_word_word_selected(display_text, raw_text, true),
            ],
        );
    }

    // Mid-commit: stay in Continuous, refresh marked text to the new pending.
    let pending_display = derived_display(&new_pending, config);
    state.phase = Phase::Continuous {
        raw: new_pending.clone(),
        committed: new_committed,
    };
    state.selected_candidate_index = 0;
    ComposingResponse {
        preedit: Some(Preedit {
            raw_input: new_pending,
            display_text: pending_display.clone(),
        }),
        effect: vec![
            commit_text_replacing_preedit(display_text.clone()),
            update_preedit(pending_display),
            next_word_update_last_selected_word(display_text, raw_text),
            perform_autocomplete(),
        ],
        selected_candidate_index: 0,
        is_composing: true,
        continuous: None,
    }
}

/// Continuous-mode abort. Pending stays out of the document; committed
/// segments stay in the document but the engine drops them from `state` so
/// the next composition does not interact with them. `ClearPreeditWithoutCommit`
/// drops the marked text only.
// 中文: Continuous 中途 abort;committed segments 已在文件中、不還原;只清 marked。
fn reset_continuous(state: &mut EngineState, config: &AppConfig) -> ComposingResponse {
    if !matches!(state.phase, Phase::Continuous { .. }) {
        return noop(state, config);
    }
    exit_to_idle(
        state,
        vec![
            clear_preedit_without_commit(),
            reset_autocomplete(),
            next_word_clear_for_new_composing(),
        ],
    )
}

/// `Append { ch }` under `Phase::Continuous`. Appends to pending; committed
/// segments are untouched. Empty `ch` collapses to noop (mirrors how
/// Composing's empty `Append` produces a degenerate buffer state — kept
/// guarded here rather than echoed forward).
// 中文: Continuous 下的 Append:把 ch 黏到 pending 尾,committed 不動。
fn append_continuous(state: &mut EngineState, ch: String, config: &AppConfig) -> ComposingResponse {
    let Phase::Continuous { raw, committed } = &state.phase else {
        return noop(state, config);
    };
    if ch.is_empty() {
        return noop(state, config);
    }
    let mut new_pending = raw.clone();
    new_pending.push_str(&ch);
    let pending_display = derived_display(&new_pending, config);
    state.phase = Phase::Continuous {
        raw: new_pending.clone(),
        committed: committed.clone(),
    };
    state.selected_candidate_index = 0;
    continuous_step_response(new_pending, pending_display, 0)
}

// ---- Effect constructors ------------------------------------------

fn update_preedit(display: String) -> Effect {
    Effect {
        kind: Some(effect::Kind::UpdatePreedit(UpdatePreedit { display })),
    }
}

fn clear_preedit_without_commit() -> Effect {
    Effect {
        kind: Some(effect::Kind::ClearPreeditWithoutCommit(
            ClearPreeditWithoutCommit {},
        )),
    }
}

fn commit_text_replacing_preedit(text: String) -> Effect {
    Effect {
        kind: Some(effect::Kind::CommitTextReplacingPreedit(
            CommitTextReplacingPreedit { text },
        )),
    }
}

fn delete_backward_from_document() -> Effect {
    Effect {
        kind: Some(effect::Kind::DeleteBackwardFromDocument(
            DeleteBackwardFromDocument {},
        )),
    }
}

fn reset_autocomplete() -> Effect {
    Effect {
        kind: Some(effect::Kind::ResetAutocomplete(ResetAutocomplete {})),
    }
}

fn perform_autocomplete() -> Effect {
    Effect {
        kind: Some(effect::Kind::PerformAutocomplete(PerformAutocomplete {})),
    }
}

fn reset_autocomplete_context() -> Effect {
    Effect {
        kind: Some(effect::Kind::ResetAutocompleteContext(
            ResetAutocompleteContext {},
        )),
    }
}

fn next_word_update_last_selected_word(text: String, roman: String) -> Effect {
    Effect {
        kind: Some(effect::Kind::NextWordUpdateLastSelectedWord(
            NextWordUpdateLastSelectedWord { text, roman },
        )),
    }
}

fn next_word_word_selected(text: String, roman: String, trigger_prediction: bool) -> Effect {
    Effect {
        kind: Some(effect::Kind::NextWordWordSelected(NextWordWordSelected {
            text,
            roman,
            trigger_prediction,
        })),
    }
}

fn next_word_clear_for_new_composing() -> Effect {
    Effect {
        kind: Some(effect::Kind::NextWordClearForNewComposing(
            NextWordClearForNewComposing {},
        )),
    }
}
