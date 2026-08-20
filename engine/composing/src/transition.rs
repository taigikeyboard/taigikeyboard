//! Pure transition function: `(state, intent, config) → ComposingResponse`.
//! Pure — no logging, no FFI, no platform types. The dispatcher in
//! `dispatch.rs` runs this and returns the response to callers.
//!
//! Effect ordering matters; iOS/Android downstream wrappers consume effects
//! in proto-list order. `prost` preserves order on `repeated Effect` fields.
//!
//! `Phase::Continuous` (v3.5.8) follows **Model B** (mainstream-aligned —
//! librime / khiin-rs / MOE / azooKey; see
//! `docs/engine/continuous-input-ranking.md` §10): nailed segments are **NOT**
//! in the host document. The whole composition — `Σ nailed[i].display_text`
//! followed by the derived display of the pending `raw` tail — occupies a
//! single marked / preedit region until a hard finalize (Enter / final-commit
//! / external-suggestion commit), which writes the whole composition to the
//! document in one `CommitTextReplacingPreedit`. A candidate tap *nails* a
//! segment inside the composition (no document write). `ComposingResponse.
//! preedit` therefore carries the **whole composition** during Continuous;
//! tests inspect `nailed` via `Engine::snapshot_state`.

// 中文: 純粹的狀態轉移函式,不做 log/FFI/平台型別轉換。
// 中文: Effect 順序攸關平台端消化,prost 對 repeated Effect 會保留順序。
// 中文: Phase::Continuous 採 Model B(對齊主流,§10)— nailed 段**未**寫入文件;
// 中文: 整段組字 (Σ nailed.display_text + pending raw 衍生形) 留在單一 marked region,
// 中文: 直到 hard finalize 才一次把整段寫入文件。preedit 欄位反映整段組字。

use crate::api::{combined_display, nailed_prefix, EngineState, Intent, NailedSegment, Phase};
use crate::derived::{derived_display, strip_tps_separator_markers};
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
            // §21: a leading `--` 輕聲 marker typed from Idle is a document
            // literal, not composing input (see helper). Composing-phase Start
            // (non-production) keeps the plain enter-composing behavior.
            Phase::Idle => enter_composing_or_insert_leading_hyphens(state, text, config),
            Phase::Composing { .. } => enter_composing(state, text, config),
        },
        Intent::Append { ch } => match &state.phase {
            Phase::Idle => enter_composing_or_insert_leading_hyphens(state, ch, config),
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
        Intent::CommitRaw => commit_raw(state, config),
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
            canonical_text,
            association_tl,
            consumed_bytes,
            syllable_count,
        } => commit_continuous(
            state,
            display_text,
            canonical_text,
            association_tl,
            consumed_bytes,
            syllable_count,
            config,
        ),
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

/// §21 INVARIANT_KHINSIANN_LEADING_MARKER_LITERAL — a leading ASCII-hyphen run
/// typed from `Phase::Idle` (no syllable content yet) is the 輕聲 (neutral-tone)
/// marker `--` (e.g. `--ah` 矣). It is a **document literal**, not composing
/// input: insert the run verbatim and — if a syllable remainder follows in the
/// same text — enter composing with the remainder. The underlined preedit then
/// covers only the convertible syllable, matching the candidate strip and the
/// reference IME (MOE).
///
/// Internal hyphens (typed AFTER syllable content, e.g. the 連字 in `tai-bak`)
/// never reach this fn — the buffer is already `Phase::Composing`, so they stay
/// composing-boundary delimiters via the `Append`/`Composing` arm.
///
/// Production keystrokes arrive one char at a time, so the common case is
/// `text == "-"` (remainder empty → pure literal insert, stay Idle). The
/// split also covers a multi-char `Start { text: "--ah" }` from the engine API
/// / tests so no old-model entry survives.
// 中文: 開頭 hyphen run(輕聲標記 --ah,在 Idle 尚無音節內容時)視為文件 literal,直接插入;
// 中文: 若同一段後面有音節餘字則以餘字進 Composing。底線只蓋可轉換音節,對齊候選詞與 MOE。
// 中文: 內部 hyphen(音節之後,如 tai-bak 連字)走 Append/Composing arm,維持斷詞分隔,不到此函式。
fn enter_composing_or_insert_leading_hyphens(
    state: &mut EngineState,
    text: String,
    config: &AppConfig,
) -> ComposingResponse {
    let hyphen_len = text.bytes().take_while(|&b| b == b'-').count();
    if hyphen_len == 0 {
        return enter_composing(state, text, config);
    }
    let (run, remainder) = text.split_at(hyphen_len);
    if remainder.is_empty() {
        // All hyphens — insert the literal run, stay Idle (no preedit).
        return exit_to_idle(state, vec![commit_text_replacing_preedit(run.to_string())]);
    }
    // Leading run + syllable remainder (multi-char `Start` / engine-API /
    // test path ONLY — the platform sends one char per keystroke, so a
    // production leading `-` always arrives as `Start{"-"}` with an empty
    // remainder above). Insert the literal run first, then compose the
    // remainder. This emits a mixed commit+composing transition; callers MUST
    // feed leading-hyphen input char-by-char, not as one multi-char `Start`:
    // a mixed transition can desync Android's `onUpdateSelection` clear-hook
    // (commit fires before the preedit lands). Engine/proto level is correct
    // and tested; the contract is char-by-char at the platform boundary.
    let mut resp = enter_composing(state, remainder.to_string(), config);
    resp.effect
        .insert(0, commit_text_replacing_preedit(run.to_string()));
    resp
}

/// TPS auto-correct. Preserves `selected_candidate_index` (correction on top
/// of an in-progress selection). Under `Phase::Continuous` it edits the
/// pending tail only — nailed segments are untouched — but the preedit
/// re-renders the whole composition (Model B).
// 中文: TPS 自動修正:替換尾端字元,保留目前候選索引 (修正疊在已選擇之上)。
// 中文: Continuous phase 下只動 pending 尾,nailed 不受影響,但 preedit 重渲染整段組字 (Model B)。
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
        Phase::Continuous { raw, nailed } => {
            if raw.is_empty() {
                return noop(state, config);
            }
            let mut new_pending = drop_last_char(raw);
            new_pending.push_str(&replacement);
            // Empty pending + empty nailed = degenerate Continuous state
            // (Codex post-impl finding #2). Exit to Idle and clear nextword.
            if new_pending.is_empty() && nailed.is_empty() {
                return exit_to_idle(
                    state,
                    vec![
                        clear_preedit_without_commit(),
                        reset_autocomplete(),
                        next_word_clear_for_new_composing(),
                    ],
                );
            }
            let combined = combined_display(nailed, &new_pending, config);
            let preserved_index = state.selected_candidate_index;
            state.phase = Phase::Continuous {
                raw: new_pending.clone(),
                nailed: nailed.clone(),
            };
            continuous_step_response(new_pending, combined, preserved_index)
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
        Phase::Continuous { raw, nailed } => {
            delete_backward_continuous(state, raw.clone(), nailed.clone(), config)
        }
        Phase::Idle => noop(state, config),
    }
}

/// `DeleteBackward` under `Phase::Continuous` — **Model B** (Codex risk (v)).
/// Nailed segments are **not** in the document, so backspace never emits
/// `DeleteBackwardFromDocument`: it only re-shapes the single marked region.
/// Three branches:
///   1. pending non-empty → drop last char of pending; if pending now empty
///      AND nailed is also empty, exit to Idle and clear the marked region
///      (no document char is touched — the char only ever lived in the
///      marked region); else stay Continuous and re-render the combined
///      composition.
///   2. pending empty AND nailed non-empty → **unnail** the last segment:
///      pop it, restore its `raw_text` as the new pending tail, roll back
///      NextWord's last-selected, and re-render the combined composition.
///      Authority is `raw_text` (never display-character count — swap / TPS
///      / both-scripts display can desync from raw).
///   3. pending empty AND nailed empty → exit to Idle (degenerate case;
///      shouldn't occur in steady state but guarded).
// 中文: Model B 下的 Continuous backspace:nailed 未在文件,故**不**發
// 中文: DeleteBackwardFromDocument,只重塑單一 marked region。有 pending 砍尾;
// 中文: 無 pending 但有 nailed 則 unnail 最後一段(raw_text 回填為 pending);兩者皆空退 Idle。
fn delete_backward_continuous(
    state: &mut EngineState,
    pending: String,
    nailed: Vec<NailedSegment>,
    config: &AppConfig,
) -> ComposingResponse {
    if !pending.is_empty() {
        let new_pending = drop_last_char(&pending);
        if new_pending.is_empty() && nailed.is_empty() {
            return exit_to_idle(
                state,
                vec![
                    clear_preedit_without_commit(),
                    reset_autocomplete(),
                    next_word_clear_for_new_composing(),
                ],
            );
        }
        let combined = combined_display(&nailed, &new_pending, config);
        state.phase = Phase::Continuous {
            raw: new_pending.clone(),
            nailed,
        };
        state.selected_candidate_index = 0;
        return continuous_step_response(new_pending, combined, 0);
    }

    // pending empty branches
    if nailed.is_empty() {
        return exit_to_idle(
            state,
            vec![
                clear_preedit_without_commit(),
                reset_autocomplete(),
                next_word_clear_for_new_composing(),
            ],
        );
    }

    let mut new_nailed = nailed;
    // JUSTIFICATION: `nailed.is_empty()` was checked at the branch above;
    // popping a non-empty Vec is a programmer-invariant guarantee, not a
    // data path.
    let popped = new_nailed.pop().expect("nailed non-empty checked above");
    // Model B: the popped segment was never in the document — unnailing it
    // just restores its raw text as the editable pending tail. No
    // `DeleteBackwardFromDocument`; the combined preedit re-render replaces
    // the marked region. Authority is `raw_text`, never a display-char
    // count (swap / TPS / both-scripts display can desync from raw).
    let new_pending = popped.raw_text;
    // Codex post-impl finding #1: roll the popped segment back out of
    // nextword's `last_selected_word` so the next final commit can't record
    // a false association from a word no longer being committed.
    // v3.5.8 Phase 9 Bug 1 (Option A): NextWord last-selected correction must
    // use the canonical key, not the (possibly swap-formatted) display
    // string — keeps association learning mode-independent (decision b).
    let nextword_correction = match new_nailed.last() {
        Some(prev) => next_word_update_last_selected_word(
            prev.canonical_text.clone(),
            // R2: re-establish the prior segment's context with its
            // canonical TL (raw-slice fallback), mirroring the commit path.
            association_roman(&prev.association_tl, &prev.raw_text),
        ),
        None => next_word_clear_for_new_composing(),
    };
    let combined = combined_display(&new_nailed, &new_pending, config);
    state.phase = Phase::Continuous {
        raw: new_pending.clone(),
        nailed: new_nailed,
    };
    state.selected_candidate_index = 0;

    ComposingResponse {
        preedit: Some(Preedit {
            raw_input: new_pending,
            display_text: combined.clone(),
        }),
        effect: vec![
            nextword_correction,
            update_preedit(combined),
            perform_autocomplete(),
        ],
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
    match &state.phase {
        Phase::Composing { raw } => commit_raw_composing(state, raw.clone(), config),
        Phase::Continuous { raw, nailed } => {
            commit_raw_continuous(state, raw.clone(), nailed.clone(), config)
        }
        Phase::Idle => noop(state, config),
    }
}

fn commit_raw_composing(
    state: &mut EngineState,
    raw: String,
    config: &AppConfig,
) -> ComposingResponse {
    if raw.is_empty() {
        return noop(state, config);
    }
    // §41 — commit the marker-free literal, not the raw buffer. A TPS
    // buffer's ASCII space is the tone-1 / boundary marker (§31); it lives in
    // `raw` so the barrier machinery and the tone pin can read it, and must
    // not reach the document. The Continuous arm below already commits
    // `combined_display`; this arm has to agree, or the engine's contract
    // would hold only for the phase the platforms happen to be in (both
    // promote to Continuous after every mutation, but the engine cannot
    // depend on that — Codex post-impl BLOCK 2026-08-21).
    // 中文: §41 — 提交去掉記號的字面而非 raw buffer。TPS 的 ASCII 空白是第一調/邊界記號(§31),
    // 中文:   留在 raw 供 barrier 與釘調讀取,不可進到文件。下面 Continuous arm 已提交
    // 中文:   combined_display,此 arm 必須一致 — 否則契約只在平台「剛好所處的 phase」成立
    // 中文:   (兩平台每次變動後都會升 Continuous,但引擎不能倚賴這點)。
    exit_to_idle(
        state,
        vec![
            commit_text_replacing_preedit(strip_tps_separator_markers(&raw)),
            reset_autocomplete(),
            reset_autocomplete_context(),
        ],
    )
}

/// `Intent::CommitRaw` under `Phase::Continuous` (Enter) — v3.5.8 Phase 9
/// Item 3, **Model B (§10)**. Enter commits the **whole composition** —
/// `Σ nailed[i].display_text` + the derived display of the pending `raw`
/// tail — in one `CommitTextReplacingPreedit`, because under Model B the
/// nailed segments were never written to the document (they lived in the
/// marked region). This replaces the single marked region with the literal
/// finalized string. The per-nailed-segment NextWord associations already
/// fired as `NextWordUpdateLastSelectedWord` at nail time; this emits the
/// **single terminal** `NextWordWordSelected` for the last "word": the
/// pending tail when one exists, else the last nailed segment.
///
/// See `docs/engine/continuous-input-ranking.md` §10.3 commit contract
/// (Enter commits the whole composition) and §10.7 "Enter after segments
/// already nailed" row.
// 中文: Phase 9 Item 3,Model B(§10)— Continuous 下 Enter 提交**整段組字**
// 中文: (Σ nailed.display_text + pending 衍生形)一次 CommitTextReplacingPreedit;
// 中文: nailed 在 Model B 從未寫入文件,故此處才一次性寫入。per-segment NextWord
// 中文: 已在 nail 時以 UpdateLastSelectedWord 觸發,此處只發單一 terminal WordSelected。
fn commit_raw_continuous(
    state: &mut EngineState,
    raw: String,
    nailed: Vec<NailedSegment>,
    config: &AppConfig,
) -> ComposingResponse {
    let combined = combined_display(&nailed, &raw, config);
    // Defensive guard: empty composition (no nailed, empty raw) violates the
    // "Continuous is non-empty in at least one of pending / nailed"
    // invariant and is unreachable under normal flow. noop, don't panic.
    // 中文: 防禦性保護;空組字違反 Continuous 不變式,不應出現,出現時 noop 不 panic。
    if combined.is_empty() {
        return noop(state, config);
    }
    // Single terminal NextWord word-selection for the last "word": the
    // pending tail when it exists (roman word, key = its derived form), else
    // the last nailed segment (canonical key — keeps association learning
    // mode-independent, v3.5.8 Phase 9 Bug 1 Option A / decision b). Earlier
    // nailed segments already fired UpdateLastSelectedWord at nail time;
    // they are NOT replayed here (Codex risk (i) — no double-count).
    let terminal_nextword = if !raw.is_empty() {
        // §41 — both fields drop the separator marker. `text` goes through
        // `derived_display`; `roman` is the raw tail, which for TPS still
        // carries the marker, and that string becomes the association's
        // romanization key on both platforms. Learning `ㄍㄠ␣ㄉㄞ` where the
        // committed word is `ㄍㄠㄉㄞ` would key the row on a form no later
        // lookup reconstructs (Codex post-impl BLOCK 2026-08-21).
        // 中文: §41 — 兩個欄位都要去掉分隔記號。text 走 derived_display;roman 是 raw 尾段,
        // 中文:   TPS 下仍帶記號,而該字串會成為兩平台的關聯羅馬字鍵。committed 是 ㄍㄠㄉㄞ
        // 中文:   卻學成 ㄍㄠ␣ㄉㄞ,等於把列鍵在沒有查詢會重建出來的形上。
        let tail_display = derived_display(&raw, config);
        let tail_roman = strip_tps_separator_markers(&raw);
        next_word_word_selected(tail_display, tail_roman, true)
    } else {
        // raw empty → all input is nailed; the last nailed segment is the
        // final word. `nailed` is non-empty here (combined non-empty with
        // empty raw implies a nailed segment exists).
        match nailed.last() {
            Some(last) => next_word_word_selected(
                last.canonical_text.clone(),
                // R2: this segment had a candidate selected at nail time →
                // use its canonical TL (raw-slice fallback). The pending-tail
                // branch above stays raw — there is no candidate there.
                association_roman(&last.association_tl, &last.raw_text),
                true,
            ),
            None => next_word_clear_for_new_composing(),
        }
    };
    exit_to_idle(
        state,
        vec![
            commit_text_replacing_preedit(combined),
            reset_autocomplete(),
            reset_autocomplete_context(),
            terminal_nextword,
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
        // Model B: the composing-buffer surface is the whole composition
        // (Σ nailed.display_text + pending-tail derived form), not the
        // pending tail alone. `raw_input` stays the still-editable tail.
        Phase::Continuous { raw, nailed } => {
            (raw.clone(), combined_display(nailed, raw, config), true)
        }
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
/// TPS replace-last, unnail/pop). **Model B**: `display` is the **whole
/// composition** (`Σ nailed.display_text` + pending-tail derived form) —
/// callers build it via [`combined_display`]. `raw_input` stays the
/// still-editable pending tail.
// 中文: Continuous 編輯中回應。Model B:display 為整段組字 (combined_display),
// 中文: raw_input 仍只反映 pending 尾。
fn continuous_step_response(
    pending: String,
    display: String,
    selected_index: i32,
) -> ComposingResponse {
    ComposingResponse {
        preedit: Some(Preedit {
            raw_input: pending,
            display_text: display.clone(),
        }),
        effect: vec![update_preedit(display), perform_autocomplete()],
        selected_candidate_index: selected_index,
        is_composing: true,
        continuous: None,
    }
}

/// `Phase::Composing { raw }` → `Phase::Continuous { raw, nailed: [] }`.
/// Marked text was already derived from the same `raw` and with no nailed
/// segments the Model B composing surface equals that derived form, so no
/// preedit refresh is necessary; emit zero effects. Idle / already-Continuous
/// / empty-raw Composing → noop (the `Continuous { raw: "", nailed: [] }`
/// state is invalid; entering it from a degenerate empty Composing buffer
/// would violate the "Continuous is non-empty in at least one of pending /
/// nailed" invariant — Codex post-impl finding #2).
// 中文: Composing → Continuous;raw 不變 nailed 起始為空,不發 effect
// 中文: (無 nailed 時 Model B 組字面 = 原 derived 形,marked text 已正確)。
// 中文: Composing.raw 為空時不轉,維持「Continuous 至少 pending 或 nailed 一邊非空」不變式。
fn enter_continuous(state: &mut EngineState, config: &AppConfig) -> ComposingResponse {
    let Phase::Composing { raw } = &state.phase else {
        return noop(state, config);
    };
    if raw.is_empty() {
        return noop(state, config);
    }
    let raw = raw.clone();
    let display = derived_display(&raw, config);
    state.phase = Phase::Continuous {
        raw: raw.clone(),
        nailed: Vec::new(),
    };
    ComposingResponse {
        preedit: Some(Preedit {
            raw_input: raw,
            display_text: display,
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
/// selection. **Model B**: the nailed prefix is in the marked region (not
/// the document), so committing `text` alone would lose it. Commit the
/// whole composition with the pending tail replaced by `text` —
/// `Σ nailed[i].display_text + text` — in one `CommitTextReplacingPreedit`,
/// preserving the net-document parity the pre-Model-B behavior had
/// (nailed-in-doc + text). Then exit Continuous.
// 中文: Continuous 下收到 SelectSuggestion { text }。Model B:nailed 在 marked
// 中文: region(非文件),只 commit text 會丟 nailed;故 commit Σ nailed.display_text + text。
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
    let Phase::Continuous { nailed, .. } = &state.phase else {
        return noop(state, config);
    };
    let mut combined = nailed_prefix(nailed, config);
    combined.push_str(&text);
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

/// `Intent::CommitPreeditThenInsertExternal { text }` under Continuous.
/// Codex post-impl finding #3. **Model B**: the whole composition
/// (`Σ nailed[i].display_text` + pending derived display) plus the external
/// text are committed in one `CommitTextReplacingPreedit` — nailed segments
/// were never in the document, so they must ride the commit here too. Then
/// exits Continuous. Empty `text` collapses to `noop`.
// 中文: Continuous 下:整段組字 (Σ nailed.display_text + pending 衍生形) + external
// 中文: 合成單次 commit,退出(Model B:nailed 未在文件,需一併寫入)。
fn commit_preedit_then_insert_external_under_continuous(
    state: &mut EngineState,
    external: String,
    config: &AppConfig,
) -> ComposingResponse {
    if external.is_empty() {
        return noop(state, config);
    }
    let Phase::Continuous { raw, nailed } = &state.phase else {
        return noop(state, config);
    };
    let mut combined = combined_display(nailed, raw, config);
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
// v3.5.8 Phase 9 Bug 1 (Option A): `display_text` is the swap/TPS/both-
// scripts-formatted string the platform tap handler produced (mirroring the
// legacy lexicon formatter); under **Model B (§10)** it is the segment's
// text **inside the marked region**, not yet in the document. `canonical_text`
// is the canonical dictionary key (`hanji.unwrap_or(roman)`) used for NextWord
// association so learning stays mode-independent (user decision b). Empty
// `canonical_text` (legacy callers) falls back to `display_text`. Model B:
// a mid-commit emits NO `CommitTextReplacingPreedit` — it only re-renders
// the combined marked region; the single literal document write happens at
// final-commit (whole composition) or via Enter (`commit_raw_continuous`).
fn commit_continuous(
    state: &mut EngineState,
    display_text: String,
    canonical_text: String,
    association_tl: String,
    consumed_bytes: usize,
    syllable_count: u8,
    config: &AppConfig,
) -> ComposingResponse {
    let Phase::Continuous { raw, nailed } = &state.phase else {
        return noop(state, config);
    };
    if display_text.is_empty()
        || consumed_bytes == 0
        || consumed_bytes > raw.len()
        || !raw.is_char_boundary(consumed_bytes)
    {
        return noop(state, config);
    }
    let canonical = if canonical_text.is_empty() {
        display_text.clone()
    } else {
        canonical_text
    };
    let pending = raw.clone();
    let mut new_nailed = nailed.clone();
    let raw_text = pending[..consumed_bytes].to_string();
    let prev_end = new_nailed.last().map(|s| s.raw_span.1).unwrap_or(0);
    let raw_span = (prev_end, prev_end + consumed_bytes);
    let new_pending = pending[consumed_bytes..].to_string();
    // R2: the NextWord `roman` arg — canonical TL when the platform sent
    // it, else the raw committed slice (legacy / TPS-OOV fallback). Used
    // for whichever single effect this commit fires below (final OR mid).
    let next_word_roman = association_roman(&association_tl, &raw_text);
    let segment = NailedSegment {
        display_text: display_text.clone(),
        canonical_text: canonical.clone(),
        raw_text: raw_text.clone(),
        association_tl,
        raw_span,
        syllable_count,
    };
    new_nailed.push(segment);

    if new_pending.is_empty() {
        // Final commit (Model B): the whole composition was in the marked
        // region; write all nailed segments' display text to the document
        // in one go, then exit to Idle. Earlier mid-commits emitted
        // UpdateLastSelectedWord per segment; this fires the single
        // terminal WordSelected for the final segment (no replay — Codex
        // risk (i)).
        // Pending is empty here, so the whole composition is just the
        // nailed prefix (combined_display would append derived("") = "").
        let combined = nailed_prefix(&new_nailed, config);
        return exit_to_idle(
            state,
            vec![
                commit_text_replacing_preedit(combined),
                reset_autocomplete(),
                reset_autocomplete_context(),
                next_word_word_selected(canonical, next_word_roman, true),
            ],
        );
    }

    // Mid-commit (Model B): stay in Continuous, NO document write — just
    // re-render the combined marked region (nailed prefix + new pending).
    let combined = combined_display(&new_nailed, &new_pending, config);
    state.phase = Phase::Continuous {
        raw: new_pending.clone(),
        nailed: new_nailed,
    };
    state.selected_candidate_index = 0;
    ComposingResponse {
        preedit: Some(Preedit {
            raw_input: new_pending,
            display_text: combined.clone(),
        }),
        effect: vec![
            update_preedit(combined),
            next_word_update_last_selected_word(canonical, next_word_roman),
            perform_autocomplete(),
        ],
        selected_candidate_index: 0,
        is_composing: true,
        continuous: None,
    }
}

/// Continuous-mode abort — **Model B (Codex risk (ii))**. Nailed segments
/// were never written to the document; the whole composition (nailed +
/// pending) lived in one marked region. `ClearPreeditWithoutCommit` clears
/// that **entire** region, and dropping `state` (exit to Idle) discards all
/// nailed segments. Nothing reaches the document — abort is a clean discard,
/// not "keep nailed, drop pending". No `DeleteBackwardFromDocument`.
// 中文: Continuous 中途 abort。Model B:nailed 從未寫入文件;整段組字在單一
// 中文: marked region,ClearPreeditWithoutCommit 清掉**整段**,退 Idle 丟棄所有 nailed。
// 中文: 不碰文件,是乾淨捨棄而非「保留 nailed 只丟 pending」。
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

/// `Append { ch }` under `Phase::Continuous`. Appends to the pending tail;
/// nailed segments are untouched. The preedit re-renders the **whole
/// composition** (Model B). Empty `ch` collapses to noop (mirrors how
/// Composing's empty `Append` produces a degenerate buffer state — kept
/// guarded here rather than echoed forward).
// 中文: Continuous 下的 Append:把 ch 黏到 pending 尾,nailed 不動,
// 中文: preedit 重渲染整段組字 (Model B)。
fn append_continuous(state: &mut EngineState, ch: String, config: &AppConfig) -> ComposingResponse {
    let Phase::Continuous { raw, nailed } = &state.phase else {
        return noop(state, config);
    };
    if ch.is_empty() {
        return noop(state, config);
    }
    let mut new_pending = raw.clone();
    new_pending.push_str(&ch);
    let combined = combined_display(nailed, &new_pending, config);
    state.phase = Phase::Continuous {
        raw: new_pending.clone(),
        nailed: nailed.clone(),
    };
    state.selected_candidate_index = 0;
    continuous_step_response(new_pending, combined, 0)
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

/// v3.6.1 R2 — resolve the NextWord `roman` arg for a committed
/// continuous segment: the candidate's canonical TL when the platform
/// supplied it (`CommitContinuous.association_tl` / `NailedSegment
/// .association_tl`), else the raw committed slice. The canonical-TL path
/// keeps the learned `prev_tl` / `next_tl` aligned with a normal candidate
/// commit (fixing continuous-vs-normal fragmentation); the raw fallback
/// preserves pre-R2 behavior for legacy callers + TPS-OOV hanji-absent
/// candidates that carry no canonical TL.
// 中文: R2 — 已釘連續 segment 的 NextWord roman 引數:有 canonical TL 用之
// 中文:   (學到的 prev_tl/next_tl 與一般候選 commit 對齊),無則 fallback raw slice。
fn association_roman(association_tl: &str, raw_text: &str) -> String {
    if association_tl.is_empty() {
        raw_text.to_owned()
    } else {
        association_tl.to_owned()
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
