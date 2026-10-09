//! Transition function: `(state, intent, config) → ComposingResponse`.
//! No logging, no FFI, no platform types. The dispatcher in `requests.rs`
//! runs this and returns the response to callers. It reads the installed
//! lexicon in two places — the compound oracle of the nailed join
//! (`api::nailed_prefix`) and, when the request asks for it, the Hanji
//! conversion of a TPS tail (`crate::conversion`) — so the preedit depends on
//! the lexicon as well as on `(state, intent, config)`, and, through a
//! conversion walk, on the user's learned counts the request supplies
//! ([`Frequency`]).
//!
//! Effect ordering matters; iOS/Android downstream wrappers consume effects
//! in proto-list order. `prost` preserves order on `repeated Effect` fields.
//!
//! `Phase::Continuous` (v3.5.8) follows **Model B** (mainstream-aligned —
//! librime / khiin-rs / MOE / azooKey; see
//! `docs/engine/continuous-commit-and-display.md` §10): nailed segments are **NOT**
//! in the host document. The whole composition — `Σ nailed[i].display_text`
//! followed by the derived display of the pending `raw` tail — occupies a
//! single marked / preedit region until a hard finalize (Enter / final-commit
//! / external-suggestion commit), which writes the whole composition to the
//! document in one `CommitTextReplacingPreedit`. A candidate tap *nails* a
//! segment inside the composition (no document write). `ComposingResponse.
//! preedit` therefore carries the **whole composition** during Continuous;
//! tests inspect `nailed` via `Engine::snapshot_state`. Under a Hanji
//! conversion the preedit shows the tail converted ([`phase_preedit`]): a
//! pick keeps composing ([`nail_pick_keeping_composition`]), and the desktop
//! commits with `CommitAsShown` / `CommitAsTyped`; `CommitRaw` and the legacy
//! pick write what they always wrote.

use crate::api::{
    combined_display, join_nailed_prefix_and_tail, nailed_prefix, Applied, CaretDirection,
    CommitScript, ConversionFrequency, EngineState, Intent, NailedSegment, Phase, Usage,
};
use crate::commit_text::{commit_resolution, resolve_commit_text};
use crate::conversion::{next_raw_span, PreviousTail};
use crate::derived::{
    buffer_input_mode, derived_display, display_caret_utf16, learning_text,
    strip_tps_separator_markers,
};
use lexicon::LearnedEntry;
use protos::engine::composing_response::Preedit;
use protos::engine::effect;
use protos::engine::AppConfig;
use protos::engine::{
    ClearCandidates, ClearPreeditWithoutCommit, CommitOutcome, CommitTextReplacingPreedit,
    CommittedWord, ComposingResponse, Effect, NextWordClearForNewComposing,
    NextWordUpdateLastSelectedWord, NextWordWordSelected, RefreshCandidates, ResetCandidateContext,
    UpdatePreedit,
};

/// Learned phrases (§50) — the longest composition the final commit turns
/// into one learned `(Hanji, canonical-TL)` pair, in syllables. ChiaKey caps
/// its in-buffer word capture at 6 characters; a Taigi phrase past six
/// syllables is a clause, not a word.
const MAX_LEARNED_PHRASE_SYLLABLES: usize = 6;

/// The user's learned counts a Hanji conversion walk ranks with; `None` walks
/// neutral (`crate::conversion::convert`).
type Frequency<'a> = Option<&'a dyn ConversionFrequency>;

/// Apply `intent` against `state`, mutate, return the proto response and
/// the phrase a final commit taught (§50).
pub(crate) fn apply(
    state: &mut EngineState,
    intent: Intent,
    config: &AppConfig,
    frequency: Frequency<'_>,
) -> Applied {
    let response = match intent {
        Intent::Start { text } => match &state.phase {
            Phase::Continuous { .. } => start_under_continuous(state, text, config, frequency),
            // §21: a leading `--` neutral-tone marker typed from Idle is a document
            // literal, not composing input (see helper).
            Phase::Idle => {
                begin_composition_or_insert_leading_hyphens(state, text, config, frequency)
            }
        },
        Intent::Append { ch } => match &state.phase {
            Phase::Idle => {
                begin_composition_or_insert_leading_hyphens(state, ch, config, frequency)
            }
            Phase::Continuous {
                raw, caret, nailed, ..
            } => append_continuous(
                state,
                raw.clone(),
                *caret,
                nailed.clone(),
                &ch,
                config,
                frequency,
            ),
        },
        Intent::AppendHyphen => {
            return apply(
                state,
                Intent::Append {
                    ch: "-".to_string(),
                },
                config,
                frequency,
            )
        }
        Intent::ReplaceLast { replacement } => replace_last(state, replacement, config, frequency),
        Intent::DeleteBackward => delete_backward(state, config, frequency),
        Intent::CommitRaw => commit_raw(state, config),
        Intent::CommitPreeditThenInsertExternal { text } => match &state.phase {
            Phase::Continuous { .. } => {
                commit_preedit_then_insert_external_under_continuous(state, text, config)
            }
            Phase::Idle => insert_external_when_idle(state, text, config),
        },
        Intent::Reset => reset(state, config),
        // FetchAtPos is a read-only query that needs lexicon state; the
        // dispatcher short-circuits before reaching `apply`. Reaching
        // here means a caller bypassed dispatch (test path or future
        // refactor) — return a snapshot rather than panic so the
        // invariant "transition is total" holds (Codex post-impl
        // continuous_phase findings #2/#3 pattern).
        Intent::FetchAtPos { .. } => snapshot(state, config),
        Intent::CommitContinuous {
            canonical_text,
            association_tl,
            hanji,
            consumed_bytes,
            syllable_count,
            script,
            roman,
        } => {
            let pick = SegmentPick {
                canonical_text,
                association_tl,
                hanji,
                consumed_bytes,
                syllable_count,
            };
            return commit_continuous(state, pick, script, &roman, config, frequency);
        }
        Intent::TelexKey { key } => telex_key(state, &key, config, frequency),
        Intent::TpsKey { key } => tps_key(state, &key, config, frequency),
        Intent::MoveCaret { direction } => move_caret(state, direction, config, frequency),
        Intent::CommitAsShown => return commit_as_shown(state, config),
        Intent::CommitAsTyped => commit_as_typed(state, config),
    };
    response.into()
}

/// `Intent::TelexKey` — edit the chunk before the caret through
/// `telex::apply_telex_key` (the key acts on the syllable being typed, which
/// is whatever ends that chunk) and keep the rest of the pending tail; a
/// `None` edit is a no-op. Under Continuous the nailed segments stay
/// untouched and the preedit re-renders the whole composition; selection
/// resets as a fresh typing step does.
fn telex_key(
    state: &mut EngineState,
    key: &str,
    config: &AppConfig,
    frequency: Frequency<'_>,
) -> ComposingResponse {
    let mode = phonetics::api::composing_mode(config);
    match &state.phase {
        Phase::Idle => match crate::telex::apply_telex_key("", key, mode) {
            Some(text) => begin_composition(state, text, config, frequency),
            None => noop(state, config),
        },
        Phase::Continuous {
            raw, caret, nailed, ..
        } => match telex_before_caret(raw, *caret, key, mode) {
            Some((next, caret)) => {
                step_continuous(state, next, caret, nailed.clone(), config, frequency)
            }
            None => noop(state, config),
        },
    }
}

/// `Intent::MoveCaret` — step the caret one char inside the pending tail
/// (under a shown Hanji conversion, one word over a converted word), or jump
/// it to the tail's start / end. The
/// buffer is untouched, so the answer is the snapshot plus one
/// `UpdatePreedit` carrying the new caret and nothing else: no
/// `RefreshCandidates`, so candidates, highlight and page stay. A move the
/// caret cannot make (a step past an edge — the caret never enters a nailed
/// segment — or a jump to the edge it sits at) is a plain snapshot —
/// except a step left from the start of a tail the request converts, which
/// re-opens the last nailed segment ([`unnail_last`]) and answers as
/// Backspace's un-nail does. A tail the request converts and the phase holds
/// no conversion for is walked first; a new conversion is answered with the
/// `UpdatePreedit` even when the step meets an edge.
fn move_caret(
    state: &mut EngineState,
    direction: Option<CaretDirection>,
    config: &AppConfig,
    frequency: Frequency<'_>,
) -> ComposingResponse {
    if let Phase::Continuous {
        raw, caret, nailed, ..
    } = &state.phase
    {
        let reopens = direction == Some(CaretDirection::Left)
            && *caret == 0
            && !nailed.is_empty()
            && crate::conversion::is_requested_for_composition(nailed, raw, config);
        if reopens {
            let (nailed, tail) = (nailed.clone(), raw.clone());
            return unnail_last(
                state,
                nailed,
                tail,
                UnnailedCaret::BeforeIt,
                config,
                frequency,
            );
        }
    }
    // A tail the request converts that holds no conversion for it (another
    // source filter, or the switch was off) is walked first, so the step
    // goes over its words and never the other way.
    let mut is_newly_converted = false;
    if let Phase::Continuous {
        raw,
        caret,
        nailed,
        conversion,
    } = &state.phase
    {
        let is_shown = conversion
            .as_ref()
            .is_some_and(|conversion| conversion.is_for(config));
        if !is_shown && crate::conversion::is_requested(raw, config) {
            let (raw, caret, nailed) = (raw.clone(), *caret, nailed.clone());
            set_continuous_walked_afresh(state, raw, caret, nailed, config, frequency);
            is_newly_converted = matches!(
                state.phase,
                Phase::Continuous {
                    conversion: Some(_),
                    ..
                }
            );
        }
    }
    let moved = match &state.phase {
        Phase::Idle => None,
        Phase::Continuous {
            raw,
            caret,
            nailed,
            conversion,
        } => direction
            .and_then(|direction| match direction {
                CaretDirection::Start | CaretDirection::End => step_caret(raw, *caret, direction),
                CaretDirection::Left | CaretDirection::Right => conversion
                    .as_ref()
                    .filter(|conversion| conversion.is_for(config))
                    .and_then(|conversion| conversion.step_over_word(*caret, direction))
                    .or_else(|| step_caret(raw, *caret, direction)),
            })
            .map(|next| (raw.clone(), next, nailed.clone())),
    };
    let has_moved = moved.is_some();
    if let Some((raw, caret, nailed)) = moved {
        set_continuous(state, raw, caret, nailed, config, frequency);
    }
    let mut resp = snapshot(state, config);
    if has_moved || is_newly_converted {
        if let Some(preedit) = &resp.preedit {
            resp.effect.push(update_preedit(preedit));
        }
    }
    resp
}

// ---- Helpers ------------------------------------------------------

// Pending-tail edits around the caret. `caret` is a char boundary in
// `0..=raw.len()`; each returns the new buffer and the new caret.

fn insert_at_caret(raw: &str, caret: usize, text: &str) -> (String, usize) {
    let mut next = String::with_capacity(raw.len() + text.len());
    next.push_str(&raw[..caret]);
    next.push_str(text);
    next.push_str(&raw[caret..]);
    (next, caret + text.len())
}

/// `None` when nothing precedes the caret.
fn delete_before_caret(raw: &str, caret: usize) -> Option<(String, usize)> {
    let removed = raw[..caret].chars().next_back()?;
    let start = caret - removed.len_utf8();
    let mut next = String::with_capacity(raw.len());
    next.push_str(&raw[..start]);
    next.push_str(&raw[caret..]);
    Some((next, start))
}

fn replace_before_caret(raw: &str, caret: usize, replacement: &str) -> Option<(String, usize)> {
    let (next, caret) = delete_before_caret(raw, caret)?;
    Some(insert_at_caret(&next, caret, replacement))
}

/// Telex key on the chunk before the caret; the tail after it rides along.
fn telex_before_caret(
    raw: &str,
    caret: usize,
    key: &str,
    mode: phonetics::api::InputMode,
) -> Option<(String, usize)> {
    let prefix = crate::telex::apply_telex_key(&raw[..caret], key, mode)?;
    let next_caret = prefix.len();
    let mut next = prefix;
    next.push_str(&raw[caret..]);
    Some((next, next_caret))
}

/// The TPS Space key's marker in the raw buffer (§31).
const TPS_SEPARATOR: &str = " ";

/// TPS key on the chunk before the caret; the tail after it rides along.
/// `None` when the key changes nothing: an empty key, or a separator where
/// nothing precedes the caret or where a tone mark or a separator sits on
/// either side of it (the syllable is already closed, or the separator would
/// part a syllable from its own tone mark).
fn tps_key_before_caret(raw: &str, caret: usize, key: &str) -> Option<(String, usize)> {
    let prefix = &raw[..caret];
    if key == TPS_SEPARATOR {
        let closes_syllable = |c: char| c == ' ' || phonetics::is_tps_tone_mark(c);
        if closes_syllable(prefix.chars().next_back()?) {
            return None;
        }
        if raw[caret..].chars().next().is_some_and(closes_syllable) {
            return None;
        }
        return Some(insert_at_caret(raw, caret, key));
    }
    if key.is_empty() {
        return None;
    }
    let (adjusted, replace_last) = phonetics::tps_input_adjust(key, prefix);
    match replace_last {
        Some(replacement) => {
            // The adjuster replaces the character it read, so one precedes the caret.
            let (raw, caret) = replace_before_caret(raw, caret, &replacement)?;
            Some(insert_at_caret(&raw, caret, &adjusted))
        }
        None => Some(insert_at_caret(raw, caret, &adjusted)),
    }
}

/// Where the caret goes in `raw`: one char for a step, the tail's edge for a
/// jump. `None` when it would not move — at the edge a step would cross, or
/// already at the edge a jump goes to.
fn step_caret(raw: &str, caret: usize, direction: CaretDirection) -> Option<usize> {
    match direction {
        CaretDirection::Left => raw[..caret]
            .chars()
            .next_back()
            .map(|c| caret - c.len_utf8()),
        CaretDirection::Right => raw[caret..].chars().next().map(|c| caret + c.len_utf8()),
        CaretDirection::Start => (caret > 0).then_some(0),
        CaretDirection::End => (caret < raw.len()).then_some(raw.len()),
    }
}

/// Build a mid-composition step response (typing / replace-last /
/// delete-backward non-empty branches): update the preedit + request a fresh
/// autocomplete query against the new buffer. `preedit` is the **whole
/// composition** ([`phase_preedit`], Model B); its `raw_input` stays the
/// still-editable pending tail.
fn step_response(preedit: Preedit) -> ComposingResponse {
    let effects = vec![update_preedit(&preedit), refresh_candidates()];
    ComposingResponse {
        preedit: Some(preedit),
        effect: effects,
        is_composing: true,
        continuous: None,
        commit: None,
    }
}

/// The wire form of `phase`'s composition: the pending `raw`, the whole
/// display — the nailed prefix, then the pending tail: its Hanji conversion
/// when the phase holds one this request asks for, else its derived form —
/// and the caret projected into it: the prefix's UTF-16 length plus the
/// caret's offset inside the tail (word by word under a conversion, else
/// [`display_caret_utf16`]).
fn phase_preedit(phase: &Phase, config: &AppConfig) -> Preedit {
    let Phase::Continuous {
        raw,
        caret,
        nailed,
        conversion,
    } = phase
    else {
        return Preedit::default();
    };
    let converted_tail = conversion
        .as_ref()
        .and_then(|conversion| conversion.tail_display(raw, *caret, config));
    let (tail, converted_caret) = match converted_tail {
        Some((tail, caret_utf16)) => (tail, Some(caret_utf16)),
        None => (derived_display(raw, config), None),
    };
    let (display, tail_start) = join_nailed_prefix_and_tail(nailed, &tail, config);
    let (prefix, tail) = display.split_at(tail_start);
    let caret_in_tail =
        converted_caret.unwrap_or_else(|| display_caret_utf16(raw, tail, *caret, config));
    let caret_utf16 = prefix.encode_utf16().count() + caret_in_tail;
    Preedit {
        raw_input: raw.clone(),
        display_text: display,
        caret_utf16: caret_utf16 as u32,
    }
}

/// Replace the phase with a Continuous one over `raw`, converting its tail
/// when `config` asks for it. The tail being replaced lends its conversion
/// (`crate::conversion::convert`), which may move the caret out of a word a
/// new walk made. With [`set_continuous_walked_afresh`], the one place a
/// Continuous phase is built, so no path leaves a conversion of another
/// buffer behind.
fn set_continuous(
    state: &mut EngineState,
    raw: String,
    caret: usize,
    nailed: Vec<NailedSegment>,
    config: &AppConfig,
    frequency: Frequency<'_>,
) {
    let previous = match &state.phase {
        Phase::Continuous {
            raw,
            caret,
            conversion: Some(conversion),
            ..
        } => Some(PreviousTail {
            raw,
            caret: *caret,
            conversion,
        }),
        _ => None,
    };
    let (conversion, caret) = crate::conversion::convert(previous, &raw, caret, config, frequency);
    state.phase = Phase::Continuous {
        raw,
        caret,
        nailed,
        conversion,
    };
}

/// [`set_continuous`] for a tail no word of the replaced one carries over
/// to: a nailed segment's raw text put back in front of it.
fn set_continuous_walked_afresh(
    state: &mut EngineState,
    raw: String,
    caret: usize,
    nailed: Vec<NailedSegment>,
    config: &AppConfig,
    frequency: Frequency<'_>,
) {
    // With the phase dropped first, nothing lends a conversion.
    state.phase = Phase::Idle;
    set_continuous(state, raw, caret, nailed, config, frequency);
}

/// Begin a composition from Idle: the buffer becomes `raw` with nothing
/// nailed and the caret at its end. Empty `raw` stays Idle (a Continuous
/// phase is never empty in both `raw` and `nailed`).
fn begin_composition(
    state: &mut EngineState,
    raw: String,
    config: &AppConfig,
    frequency: Frequency<'_>,
) -> ComposingResponse {
    if raw.is_empty() {
        return noop(state, config);
    }
    let caret = raw.len();
    step_continuous(state, raw, caret, Vec::new(), config, frequency)
}

/// One `Phase::Continuous` step on the pending tail: `nailed` is untouched,
/// the whole composition re-renders (Model B) and a fresh fetch is requested.
fn step_continuous(
    state: &mut EngineState,
    pending: String,
    caret: usize,
    nailed: Vec<NailedSegment>,
    config: &AppConfig,
    frequency: Frequency<'_>,
) -> ComposingResponse {
    set_continuous(state, pending, caret, nailed, config, frequency);
    step_response(phase_preedit(&state.phase, config))
}

/// §21 INVARIANT_KHINSIANN_LEADING_MARKER_LITERAL — a leading ASCII-hyphen run
/// typed from `Phase::Idle` (no syllable content yet) is the neutral-tone (khinsiann)
/// marker `--` (e.g. `--ah` 矣). It is a **document literal**, not composing
/// input: insert the run verbatim and — if a syllable remainder follows in the
/// same text — begin a composition with the remainder. The underlined preedit then
/// covers only the convertible syllable, matching the candidate strip and the
/// reference IME (MOE).
///
/// Internal hyphens (typed AFTER syllable content, e.g. the hyphen in `tai-bak`)
/// never reach this fn — the buffer is already `Phase::Continuous`, so they stay
/// composing-boundary delimiters via the `Append` Continuous arm.
///
/// Production keystrokes arrive one char at a time, so the common case is
/// `text == "-"` (remainder empty → pure literal insert, stay Idle). The
/// split also covers a multi-char `Start { text: "--ah" }` from the engine API
/// / tests so no old-model entry survives.
fn begin_composition_or_insert_leading_hyphens(
    state: &mut EngineState,
    text: String,
    config: &AppConfig,
    frequency: Frequency<'_>,
) -> ComposingResponse {
    let hyphen_len = text.bytes().take_while(|&b| b == b'-').count();
    if hyphen_len == 0 {
        return begin_composition(state, text, config, frequency);
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
    let mut resp = begin_composition(state, remainder.to_string(), config, frequency);
    resp.effect
        .insert(0, commit_text_replacing_preedit(run.to_string()));
    resp
}

/// `Intent::TpsKey` — see the `TpsKey` proto comment. From Idle a glyph begins
/// the composition as `Append` does (a leading hyphen stays a document
/// literal, §21); under Continuous the nailed segments stay untouched.
fn tps_key(
    state: &mut EngineState,
    key: &str,
    config: &AppConfig,
    frequency: Frequency<'_>,
) -> ComposingResponse {
    match &state.phase {
        Phase::Idle => match tps_key_before_caret("", 0, key) {
            Some((text, _)) => {
                begin_composition_or_insert_leading_hyphens(state, text, config, frequency)
            }
            None => noop(state, config),
        },
        Phase::Continuous {
            raw, caret, nailed, ..
        } => match tps_key_before_caret(raw, *caret, key) {
            Some((next, caret)) => {
                step_continuous(state, next, caret, nailed.clone(), config, frequency)
            }
            None => noop(state, config),
        },
    }
}

/// TPS auto-correct. Under `Phase::Continuous` it edits the
/// pending tail only — nailed segments are untouched — but the preedit
/// re-renders the whole composition (Model B).
fn replace_last(
    state: &mut EngineState,
    replacement: String,
    config: &AppConfig,
    frequency: Frequency<'_>,
) -> ComposingResponse {
    match &state.phase {
        Phase::Continuous {
            raw, caret, nailed, ..
        } => {
            let Some((new_pending, caret)) = replace_before_caret(raw, *caret, &replacement) else {
                return noop(state, config);
            };
            // Empty pending + empty nailed = degenerate Continuous state
            // (Codex post-impl finding #2). Exit to Idle and clear nextword.
            if new_pending.is_empty() && nailed.is_empty() {
                return exit_to_idle(state, abort_continuous_effects());
            }
            step_continuous(state, new_pending, caret, nailed.clone(), config, frequency)
        }
        Phase::Idle => noop(state, config),
    }
}

fn delete_backward(
    state: &mut EngineState,
    config: &AppConfig,
    frequency: Frequency<'_>,
) -> ComposingResponse {
    match &state.phase {
        Phase::Continuous {
            raw, caret, nailed, ..
        } => delete_backward_continuous(
            state,
            raw.clone(),
            *caret,
            nailed.clone(),
            config,
            frequency,
        ),
        Phase::Idle => noop(state, config),
    }
}

/// `DeleteBackward` under `Phase::Continuous` — **Model B** (Codex risk (v)).
/// Nailed segments are **not** in the document, so backspace never emits
/// `DeleteBackwardFromDocument`: it only re-shapes the single marked region.
/// Three branches:
///   1. pending non-empty → drop the char before the caret (nothing before
///      it → no-op, the caret does not fall through into a nailed segment);
///      if pending now empty
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
fn delete_backward_continuous(
    state: &mut EngineState,
    pending: String,
    caret: usize,
    nailed: Vec<NailedSegment>,
    config: &AppConfig,
    frequency: Frequency<'_>,
) -> ComposingResponse {
    if !pending.is_empty() {
        let Some((new_pending, caret)) = delete_before_caret(&pending, caret) else {
            return noop(state, config);
        };
        if new_pending.is_empty() && nailed.is_empty() {
            return exit_to_idle(state, abort_continuous_effects());
        }
        return step_continuous(state, new_pending, caret, nailed, config, frequency);
    }

    // pending empty branches
    if nailed.is_empty() {
        return exit_to_idle(state, abort_continuous_effects());
    }
    unnail_last(
        state,
        nailed,
        String::new(),
        UnnailedCaret::AfterIt,
        config,
        frequency,
    )
}

/// Where an un-nailed segment's raw text leaves the caret.
enum UnnailedCaret {
    /// Backspace: the next one takes the segment's last glyph.
    AfterIt,
    /// A step left from the start of a converted tail re-opens the segment
    /// (H3): the caret lands before its glyphs.
    BeforeIt,
}

/// Pop the last of the non-empty `nailed` and make its raw text, followed
/// by `tail`, the pending tail, walked afresh — a pick re-opened from in
/// front of the tail is dropped, as librime re-opens a selected segment. The
/// usage the pick recorded stays recorded.
fn unnail_last(
    state: &mut EngineState,
    mut nailed: Vec<NailedSegment>,
    tail: String,
    caret: UnnailedCaret,
    config: &AppConfig,
    frequency: Frequency<'_>,
) -> ComposingResponse {
    // JUSTIFICATION: both callers checked `nailed` is non-empty; popping a
    // non-empty Vec is a programmer-invariant guarantee, not a data path.
    let popped = nailed
        .pop()
        .expect("nailed non-empty checked by the caller");
    // Model B: the popped segment was never in the document — unnailing it
    // just restores its raw text as the editable pending tail. No
    // `DeleteBackwardFromDocument`; the combined preedit re-render replaces
    // the marked region. Authority is `raw_text`, never a display-char
    // count (swap / TPS / both-scripts display can desync from raw).
    let caret = match caret {
        UnnailedCaret::AfterIt => popped.raw_text.len(),
        UnnailedCaret::BeforeIt => 0,
    };
    let new_pending = popped.raw_text + &tail;
    // Unnail handshake. NextWord learns nothing from it and keeps its
    // committed context (behavioral-invariants §40) — the popped segment can
    // no longer reach NextWord at all, because only the final commit's
    // `preceding` carries nailed segments. Kept for platforms that read the
    // effect stream; canonical key per v3.5.8 Phase 9 Bug 1 (Option A).
    // A segment nailed as shown is no word the user selected (B4): the
    // handshake names only a picked one.
    let nextword_correction = match nailed.last() {
        Some(prev) if prev.is_picked => next_word_update_last_selected_word(
            prev.canonical_text.clone(),
            // R2: canonical TL (raw-slice fallback), as the commit path.
            association_roman(&prev.association_tl, &prev.raw_text),
        ),
        _ => next_word_clear_for_new_composing(),
    };
    set_continuous_walked_afresh(state, new_pending, caret, nailed, config, frequency);

    let preedit = phase_preedit(&state.phase, config);
    let effects = vec![
        nextword_correction,
        update_preedit(&preedit),
        refresh_candidates(),
    ];
    ComposingResponse {
        preedit: Some(preedit),
        effect: effects,
        is_composing: true,
        continuous: None,
        commit: None,
    }
}

fn commit_raw(state: &mut EngineState, config: &AppConfig) -> ComposingResponse {
    match &state.phase {
        Phase::Continuous { raw, nailed, .. } => {
            commit_raw_continuous(state, raw.clone(), nailed.clone(), config)
        }
        Phase::Idle => noop(state, config),
    }
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
/// See `docs/engine/continuous-commit-and-display.md` §10.3 commit contract
/// (Enter commits the whole composition) and §10.7 "Enter after segments
/// already nailed" row.
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
    if combined.is_empty() {
        return noop(state, config);
    }
    // Single terminal NextWord word-selection for the last "word": the
    // pending tail when it exists (roman word, key = its derived form), else
    // the last nailed segment (canonical key — keeps association learning
    // mode-independent, v3.5.8 Phase 9 Bug 1 Option A / decision b). Every
    // nailed segment before it rides along as `preceding` — the only place
    // NextWord learns a nailed segment (behavioral-invariants §40).
    let terminal_nextword = if !raw.is_empty() {
        // §41 — both fields drop the separator marker. `text` is the tail's
        // learning text (§34: the literal candidate's key, not its permissive
        // rendering); `roman` is the raw tail, which for TPS still
        // carries the marker, and that string becomes the association's
        // romanization key on both platforms. Learning `ㄍㄠ␣ㄉㄞ` where the
        // committed word is `ㄍㄠㄉㄞ` would key the row on a form no later
        // lookup reconstructs (Codex post-impl BLOCK 2026-08-21).
        // NextWord learns `roman` as sent, so the tail is put in canonical
        // TL form here: POJ folds to TL, TL keeps its `eng` / `ek` finals,
        // TPS and English pass through.
        let tail_text = learning_text(&raw, config);
        let tail_roman = phonetics::api::canonical_tl_form(
            &strip_tps_separator_markers(&raw),
            buffer_input_mode(&raw, config),
        );
        next_word_terminal(tail_text, tail_roman, &nailed)
    } else {
        // raw empty → all input is nailed; the last nailed segment is the
        // final word. `nailed` is non-empty here (combined non-empty with
        // empty raw implies a nailed segment exists).
        next_word_nailed_terminal(&nailed)
    };
    finalize(state, combined, terminal_nextword)
}

/// `Intent::CommitPreeditThenInsertExternal` from Idle — a plain insert
/// (emoji / paste with nothing composed). `CommitTextReplacingPreedit` is
/// no-op-on-empty-preedit safe on every platform.
fn insert_external_when_idle(
    state: &mut EngineState,
    external: String,
    config: &AppConfig,
) -> ComposingResponse {
    if external.is_empty() {
        return noop(state, config);
    }
    exit_to_idle(state, vec![commit_text_replacing_preedit(external)])
}

/// User-initiated reset — **Model B
/// (Codex risk (ii))**. Nailed segments were never written to the document;
/// the whole composition lived in one marked region, so the abort trio
/// clears that **entire** region and dropping the state discards every
/// nailed segment. Nothing reaches the document. Idle → no-op.
fn reset(state: &mut EngineState, config: &AppConfig) -> ComposingResponse {
    match state.phase {
        Phase::Idle => noop(state, config),
        Phase::Continuous { .. } => exit_to_idle(state, abort_continuous_effects()),
    }
}

pub(crate) fn snapshot(state: &EngineState, config: &AppConfig) -> ComposingResponse {
    // Model B: the composing-buffer surface is the whole composition
    // (Σ nailed.display_text + the pending tail as displayed), not the
    // pending tail alone. `raw_input` stays the still-editable tail.
    ComposingResponse {
        preedit: Some(phase_preedit(&state.phase, config)),
        effect: Vec::new(),
        is_composing: matches!(state.phase, Phase::Continuous { .. }),
        continuous: None,
        commit: None,
    }
}

fn exit_to_idle(state: &mut EngineState, effects: Vec<Effect>) -> ComposingResponse {
    state.phase = Phase::Idle;
    ComposingResponse {
        preedit: Some(Preedit::default()),
        effect: effects,
        is_composing: false,
        continuous: None,
        commit: None,
    }
}

fn noop(state: &EngineState, config: &AppConfig) -> ComposingResponse {
    snapshot(state, config)
}

// ---- Continuous-phase helpers --------------------------------------

/// `Intent::Start { text }` arriving while in `Phase::Continuous`. Codex
/// post-impl finding #3: silently dropping `text` would lose user input.
/// Treats the intent as "drop continuous state, then begin a fresh
/// composition with `text`" (no §21 leading-hyphen split here). Emits the
/// abort trio followed by the regular step effects; final state is
/// `Phase::Continuous { raw: text, nailed: [] }` (or Idle if `text` is empty).
fn start_under_continuous(
    state: &mut EngineState,
    text: String,
    config: &AppConfig,
    frequency: Frequency<'_>,
) -> ComposingResponse {
    // Drop continuous state to Idle first.
    state.phase = Phase::Idle;
    let mut effects = abort_continuous_effects();
    let resp = begin_composition(state, text, config, frequency);
    effects.extend(resp.effect);
    ComposingResponse {
        effect: effects,
        ..resp
    }
}

/// `Intent::CommitPreeditThenInsertExternal { text }` under Continuous.
/// Codex post-impl finding #3. **Model B**: the whole composition as the
/// preedit shows it ([`phase_preedit`]: `Σ nailed[i].display_text` + the
/// pending tail — its Hanji conversion when the request asks for one, else
/// its derived display) plus the external text are committed in one
/// `CommitTextReplacingPreedit` — nailed segments were never in the
/// document, so they must ride the commit here too. Then exits Continuous.
/// Empty `text` collapses to `noop`.
fn commit_preedit_then_insert_external_under_continuous(
    state: &mut EngineState,
    external: String,
    config: &AppConfig,
) -> ComposingResponse {
    if external.is_empty() {
        return noop(state, config);
    }
    let mut combined = phase_preedit(&state.phase, config).display_text;
    combined.push_str(&external);
    let mut effects = finalize_effects(combined);
    effects.push(next_word_clear_for_new_composing());
    exit_to_idle(state, effects)
}

/// `Phase::Continuous` segment commit of the resolved `display_text`.
/// `consumed_bytes >= pending.len()` is the final-commit branch (exit to
/// Idle); otherwise mid-commit (stay in Continuous). Programmer-error inputs
/// (out-of-range or non-char-boundary `consumed_bytes`) collapse to `noop`
/// rather than panicking.
// Under **Model B (§10)** `display_text` is the segment's text **inside the
// marked region**, not yet in the document. `canonical_text` is the
// canonical dictionary key (`hanji.unwrap_or(roman)`) used for NextWord
// association so learning stays mode-independent (user decision b). Model B:
// a mid-commit emits NO `CommitTextReplacingPreedit` — it only re-renders
// the combined marked region; the single literal document write happens at
// final-commit (whole composition) or via Enter (`commit_raw_continuous`).
// Returns what the commit did beside the response.
fn nail_segment(
    state: &mut EngineState,
    display_text: String,
    pick: SegmentPick,
    config: &AppConfig,
    frequency: Frequency<'_>,
) -> (Applied, CommitOutcome) {
    let SegmentPick {
        canonical_text: canonical,
        association_tl,
        hanji,
        consumed_bytes,
        syllable_count,
    } = pick;
    let Phase::Continuous { raw, nailed, .. } = &state.phase else {
        return (noop(state, config).into(), CommitOutcome::Ignored);
    };
    if !is_pick_of(raw, 0, consumed_bytes, &display_text) {
        return (noop(state, config).into(), CommitOutcome::Ignored);
    }
    let mut new_nailed = nailed.clone();
    let new_pending = raw[consumed_bytes..].to_string();
    // R2: the NextWord `roman` arg — canonical TL when the platform sent
    // it, else the raw committed slice (legacy / TPS-OOV fallback). Used
    // for whichever single effect this commit fires below (final OR mid).
    let next_word_roman = association_roman(&association_tl, &raw[..consumed_bytes]);
    let segment = picked_segment(
        &new_nailed,
        raw[..consumed_bytes].to_string(),
        display_text,
        canonical.clone(),
        association_tl,
        hanji,
        syllable_count,
    );
    new_nailed.push(segment);

    if new_pending.is_empty() {
        // Final commit (Model B): the whole composition was in the marked
        // region; write all nailed segments' display text to the document
        // in one go, then exit to Idle. The single terminal WordSelected is
        // the final segment, with the earlier nailed segments as `preceding`
        // (behavioral-invariants §40).
        // Pending is empty here, so the whole composition is just the
        // nailed prefix (combined_display would append derived("") = "").
        let combined = nailed_prefix(&new_nailed, config);
        // `nailed` = every segment before the one just pushed.
        let terminal = next_word_terminal(canonical, next_word_roman, nailed);
        // Learned phrases (§50): the whole composition, if it was a
        // sequence of hanji picks, becomes one learned pair.
        let learned = learned_phrase(&new_nailed);
        let applied = Applied {
            response: finalize(state, combined, terminal),
            learned,
            usage: None,
        };
        return (applied, CommitOutcome::Finalized);
    }

    // Mid-commit (Model B): stay in Continuous, NO document write — just
    // re-render the combined marked region (nailed prefix + new pending).
    // Accepting a candidate is a flush of what was typed, so the caret goes
    // to the end of the tail that is left.
    let caret = new_pending.len();
    set_continuous(state, new_pending, caret, new_nailed, config, frequency);
    let response = nailed_response(
        state,
        config,
        canonical,
        next_word_roman,
        refresh_candidates(),
    );
    (response.into(), CommitOutcome::Nailed)
}

/// Whether a pick ending at `consumed_bytes` of the pending `raw`, of a list
/// starting at `start`, can be nailed: something to write, a non-empty span
/// after the start, on a char boundary.
fn is_pick_of(raw: &str, start: usize, consumed_bytes: usize, display_text: &str) -> bool {
    !display_text.is_empty()
        && consumed_bytes > start
        && consumed_bytes <= raw.len()
        && raw.is_char_boundary(consumed_bytes)
}

/// The segment a pick of `raw_text` nails after `nailed`.
fn picked_segment(
    nailed: &[NailedSegment],
    raw_text: String,
    display_text: String,
    canonical_text: String,
    association_tl: String,
    hanji: Option<String>,
    syllable_count: u8,
) -> NailedSegment {
    NailedSegment {
        display_text,
        canonical_text,
        raw_span: next_raw_span(nailed, raw_text.len()),
        raw_text,
        association_tl,
        hanji: hanji.filter(|hanji| !hanji.is_empty()),
        syllable_count,
        is_picked: true,
    }
}

/// The answer of a pick that keeps composing: the preedit, the nail
/// handshake, then `list_effect` — what becomes of the candidate list.
fn nailed_response(
    state: &EngineState,
    config: &AppConfig,
    canonical_text: String,
    next_word_roman: String,
    list_effect: Effect,
) -> ComposingResponse {
    let preedit = phase_preedit(&state.phase, config);
    let effects = vec![
        update_preedit(&preedit),
        next_word_update_last_selected_word(canonical_text, next_word_roman),
        list_effect,
    ];
    ComposingResponse {
        preedit: Some(preedit),
        effect: effects,
        is_composing: true,
        continuous: None,
        commit: None,
    }
}

/// Write `text` out of the composition — the finalize trio, then the
/// NextWord handshake `terminal` — and go idle.
fn finalize(state: &mut EngineState, text: String, terminal: Effect) -> ComposingResponse {
    let mut effects = finalize_effects(text);
    effects.push(terminal);
    exit_to_idle(state, effects)
}

/// A pick under a Hanji conversion the request asks for (H4): the list it
/// came from started at the word before the caret
/// (`crate::conversion::word_list_start`, `start`), so what the preedit shows before that start
/// is nailed as shown and not picked — each word and the glyphs between them,
/// byte for byte, so un-nailing restores what was typed — then the pick is
/// nailed, and the rest of the tail is walked again with the caret at its
/// end. Never finalizes: with nothing left the composition stays, all of it
/// nailed, until a commit. The window closes (`ClearCandidates`, not
/// `RefreshCandidates`). A pick not ending after the start is ignored.
fn nail_pick_keeping_composition(
    state: &mut EngineState,
    display_text: String,
    pick: SegmentPick,
    start: usize,
    config: &AppConfig,
    frequency: Frequency<'_>,
) -> (Applied, CommitOutcome) {
    let SegmentPick {
        canonical_text,
        association_tl,
        hanji,
        consumed_bytes,
        syllable_count,
    } = pick;
    let Phase::Continuous {
        raw,
        nailed,
        conversion,
        ..
    } = &state.phase
    else {
        return (noop(state, config).into(), CommitOutcome::Ignored);
    };
    if !is_pick_of(raw, start, consumed_bytes, &display_text) {
        return (noop(state, config).into(), CommitOutcome::Ignored);
    }
    let mut new_nailed = nailed.clone();
    let shown = conversion
        .iter()
        .flat_map(|conversion| conversion.pieces_before(raw, start));
    for piece in shown {
        let segment = piece.nailed_as_shown(raw, &new_nailed);
        new_nailed.push(segment);
    }
    let raw_text = raw[start..consumed_bytes].to_string();
    let next_word_roman = association_roman(&association_tl, &raw_text);
    let segment = picked_segment(
        &new_nailed,
        raw_text,
        display_text,
        canonical_text.clone(),
        association_tl,
        hanji,
        syllable_count,
    );
    new_nailed.push(segment);
    let rest = raw[consumed_bytes..].to_string();
    let caret = rest.len();
    set_continuous_walked_afresh(state, rest, caret, new_nailed, config, frequency);
    let response = nailed_response(
        state,
        config,
        canonical_text,
        next_word_roman,
        clear_candidates(),
    );
    (response.into(), CommitOutcome::Nailed)
}

/// `Intent::CommitAsShown` — write the composition as the preedit shows it
/// ([`phase_preedit`]: nailed segments, converted words, unconverted glyphs)
/// and go idle. It teaches what a final pick teaches — the §50 phrase and the
/// next-word sequence — only when nothing is pending and the user picked
/// every segment (B4, H7); otherwise it teaches nothing and answers
/// `NextWordClearForNewComposing`, after which the platform forgets its
/// next-word context. Records no usage.
fn commit_as_shown(state: &mut EngineState, config: &AppConfig) -> Applied {
    let Phase::Continuous { raw, nailed, .. } = &state.phase else {
        return noop(state, config).into();
    };
    let text = phase_preedit(&state.phase, config).display_text;
    if text.is_empty() {
        return noop(state, config).into();
    }
    let (terminal, learned) = if raw.is_empty() {
        (next_word_nailed_terminal(nailed), learned_phrase(nailed))
    } else {
        (next_word_clear_for_new_composing(), None)
    };
    Applied {
        response: finalize(state, text, terminal),
        learned,
        usage: None,
    }
}

/// `Intent::CommitAsTyped` — write the glyphs of the whole composition as
/// typed (every nailed segment's raw text, then the pending tail, through
/// the derived display: TPS drops the separators) and go idle. Teaches
/// nothing.
fn commit_as_typed(state: &mut EngineState, config: &AppConfig) -> ComposingResponse {
    let Phase::Continuous { raw, nailed, .. } = &state.phase else {
        return noop(state, config);
    };
    let typed: String = nailed
        .iter()
        .map(|segment| segment.raw_text.as_str())
        .chain([raw.as_str()])
        .collect();
    let text = derived_display(&typed, config);
    if text.is_empty() {
        return noop(state, config);
    }
    finalize(state, text, next_word_clear_for_new_composing())
}

/// The pick a `CommitContinuous` names, apart from what it writes.
struct SegmentPick {
    canonical_text: String,
    association_tl: String,
    hanji: Option<String>,
    consumed_bytes: usize,
    syllable_count: u8,
}

/// `Intent::CommitContinuous` (R5): [`nail_segment`] with the document text
/// the engine resolves ([`resolve_commit_text`]), answering
/// `ComposingResponse.commit` and, for a pick that nailed or finalized, its
/// usage (`Applied.usage`). A pick with no script, no canonical text, or a
/// script it does not have, is ignored: no state change, no usage.
fn commit_continuous(
    state: &mut EngineState,
    pick: SegmentPick,
    script: Option<CommitScript>,
    roman: &str,
    config: &AppConfig,
    frequency: Frequency<'_>,
) -> Applied {
    let hanji = pick.hanji.clone().filter(|hanji| !hanji.is_empty());
    let resolved = script
        .filter(|_| !pick.canonical_text.is_empty())
        .and_then(|script| resolve_commit_text(script, roman, hanji.as_deref(), config));
    let Some(resolved) = resolved else {
        let mut response = noop(state, config);
        response.commit = Some(commit_resolution(CommitOutcome::Ignored, None));
        return response.into();
    };
    let usage = Usage {
        display_text: pick.canonical_text.clone(),
        canonical_tl: pick.association_tl.clone(),
        hanji,
    };
    let word_list_start = match &state.phase {
        Phase::Continuous {
            raw,
            caret,
            nailed,
            conversion,
        } => crate::conversion::word_list_start(nailed, raw, *caret, conversion.as_ref(), config),
        Phase::Idle => None,
    };
    let (mut applied, outcome) = match word_list_start {
        Some(start) => nail_pick_keeping_composition(
            state,
            resolved.text.clone(),
            pick,
            start,
            config,
            frequency,
        ),
        None => nail_segment(state, resolved.text.clone(), pick, config, frequency),
    };
    if outcome != CommitOutcome::Ignored {
        applied.usage = Some(usage);
    }
    applied.response.commit = Some(commit_resolution(outcome, Some(resolved)));
    applied
}

/// `Append { ch }` under `Phase::Continuous`. Appends to the pending tail;
/// nailed segments are untouched. The preedit re-renders the **whole
/// composition** (Model B). Empty `ch` collapses to noop.
fn append_continuous(
    state: &mut EngineState,
    pending: String,
    caret: usize,
    nailed: Vec<NailedSegment>,
    ch: &str,
    config: &AppConfig,
    frequency: Frequency<'_>,
) -> ComposingResponse {
    if ch.is_empty() {
        return noop(state, config);
    }
    let (new_pending, caret) = insert_at_caret(&pending, caret, ch);
    step_continuous(state, new_pending, caret, nailed, config, frequency)
}

// ---- Effect constructors ------------------------------------------

fn update_preedit(preedit: &Preedit) -> Effect {
    Effect {
        kind: Some(effect::Kind::UpdatePreedit(UpdatePreedit {
            display: preedit.display_text.clone(),
            caret_utf16: preedit.caret_utf16,
        })),
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

fn clear_candidates() -> Effect {
    Effect {
        kind: Some(effect::Kind::ClearCandidates(ClearCandidates {})),
    }
}

fn refresh_candidates() -> Effect {
    Effect {
        kind: Some(effect::Kind::RefreshCandidates(RefreshCandidates {})),
    }
}

fn reset_candidate_context() -> Effect {
    Effect {
        kind: Some(effect::Kind::ResetCandidateContext(
            ResetCandidateContext {},
        )),
    }
}

/// The abort trio every "drop Continuous state without writing to the
/// document" branch emits, in this order: clear the whole marked region,
/// reset autocomplete, tear down NextWord's continuous strip. A caller that
/// emits more (`start_under_continuous`) appends after the trio.
fn abort_continuous_effects() -> Vec<Effect> {
    vec![
        clear_preedit_without_commit(),
        clear_candidates(),
        next_word_clear_for_new_composing(),
    ]
}

/// The finalize trio every branch that commits text out of a composition
/// emits, in this order:
/// commit `text` replacing the preedit, reset autocomplete, reset the
/// autocomplete context. A caller that also fires a NextWord effect pushes
/// it after the trio.
fn finalize_effects(text: String) -> Vec<Effect> {
    vec![
        commit_text_replacing_preedit(text),
        clear_candidates(),
        reset_candidate_context(),
    ]
}

/// v3.6.1 R2 — resolve the NextWord `roman` arg for a committed
/// continuous segment: the candidate's canonical TL when the platform
/// supplied it (`CommitContinuous.association_tl` / `NailedSegment
/// .association_tl`), else the raw committed slice. The canonical-TL path
/// keeps the learned `prev_tl` / `next_tl` aligned with a normal candidate
/// commit (fixing continuous-vs-normal fragmentation); the raw fallback
/// preserves pre-R2 behavior for legacy callers + TPS-OOV hanji-absent
/// candidates that carry no canonical TL.
pub(crate) fn association_roman(association_tl: &str, raw_text: &str) -> String {
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

/// The NextWord handshake of a commit whose last word is `text` / `roman`,
/// after the nailed `preceding`: [`next_word_word_selected`] when the user
/// picked every one of them, else `NextWordClearForNewComposing` — a segment
/// nailed as shown is never one end of a next-word pair, and the chain would
/// bridge across it (B4, H7). Every legacy segment is picked.
fn next_word_terminal(text: String, roman: String, preceding: &[NailedSegment]) -> Effect {
    if preceding.iter().all(|segment| segment.is_picked) {
        next_word_word_selected(text, roman, true, preceding)
    } else {
        next_word_clear_for_new_composing()
    }
}

/// [`next_word_terminal`] of a composition that is all nailed: its last
/// segment is the last word — learned only when it was picked too.
fn next_word_nailed_terminal(nailed: &[NailedSegment]) -> Effect {
    match nailed.split_last() {
        Some((last, preceding)) if last.is_picked => next_word_terminal(
            last.canonical_text.clone(),
            // R2: this segment had a candidate selected at nail time → use
            // its canonical TL (raw-slice fallback). A pending tail has no
            // candidate and keys on its raw form.
            association_roman(&last.association_tl, &last.raw_text),
            preceding,
        ),
        _ => next_word_clear_for_new_composing(),
    }
}

/// Final-commit NextWord handshake: the terminal word, with the nailed
/// segments committed before it (`preceding`, document order) so NextWord
/// learns the whole composition as one sequence (behavioral-invariants §40).
fn next_word_word_selected(
    text: String,
    roman: String,
    trigger_prediction: bool,
    preceding: &[NailedSegment],
) -> Effect {
    let preceding = preceding
        .iter()
        .map(|segment| CommittedWord {
            text: segment.canonical_text.clone(),
            roman: association_roman(&segment.association_tl, &segment.raw_text),
        })
        .collect();
    Effect {
        kind: Some(effect::Kind::NextWordWordSelected(NextWordWordSelected {
            text,
            roman,
            trigger_prediction,
            preceding,
        })),
    }
}

/// Learned phrases (§50) — the `(Hanji, canonical-TL)` pair a final
/// continuous commit learns from its nailed segments, or `None` when the
/// composition is not one: fewer than two segments, any segment the user
/// did not pick (B4), any segment without a hanji pick or without a
/// canonical TL, or more than
/// [`MAX_LEARNED_PHRASE_SYLLABLES`] in total. The TL pieces join under
/// the commit's word boundaries ([`crate::api::learned_reading`]): a
/// dictionary compound with `-`, separate words with a space, and the
/// separator the user typed wins — by kind only ([`canonical_separators`];
/// the dictionary cannot cover every phrase and the user manages the
/// separator, USER 2026-09-22). The cap counts
/// the joined TL, not the segments' echoed `syllable_count`: a
/// custom-dictionary pick reports `1` whatever its length
/// (`lexicon::custom_entry_to_candidate`).
fn learned_phrase(nailed: &[NailedSegment]) -> Option<LearnedEntry> {
    if nailed.len() < 2 {
        return None;
    }
    let mut hanji = String::new();
    for seg in nailed {
        if !seg.is_picked {
            return None;
        }
        // `commit_continuous` stored an empty hanji as `None` already.
        let h = seg.hanji.as_deref()?;
        if seg.association_tl.is_empty() {
            return None;
        }
        hanji.push_str(h);
    }
    let canonical_tl = canonical_separators(&crate::api::learned_reading(nailed));
    let syllable_count = phonetics::api::tl_syllables(&canonical_tl).count();
    if syllable_count == 0 || syllable_count > MAX_LEARNED_PHRASE_SYLLABLES {
        return None;
    }
    Some(LearnedEntry {
        hanji,
        canonical_tl,
    })
}

/// The canonical separators a learned TL stores: a `-` run of three or
/// more is the khinsiann `--` (`---` → `--`), and the khinsiann marker
/// binds to the word before it, so the word space in front of a
/// dictionary khinsiann piece drops (`kì --khí-lâi` → `kì--khí-lâi`).
fn canonical_separators(tl: &str) -> String {
    let mut out = String::with_capacity(tl.len());
    let mut run = 0;
    for c in tl.chars() {
        if c == '-' {
            if run == 0 && out.ends_with(' ') {
                out.pop();
            }
            run += 1;
            if run <= 2 {
                out.push(c);
            }
        } else {
            run = 0;
            out.push(c);
        }
    }
    out
}

fn next_word_clear_for_new_composing() -> Effect {
    Effect {
        kind: Some(effect::Kind::NextWordClearForNewComposing(
            NextWordClearForNewComposing {},
        )),
    }
}
