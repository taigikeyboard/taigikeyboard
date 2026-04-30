//! Pure transition function: `(state, intent, config) → ComposingResponse`.
//! Pure — no logging, no FFI, no platform types. The dispatcher in
//! `dispatch.rs` runs this and returns the response to callers.
//!
//! Effect ordering matters; iOS/Android downstream wrappers consume effects
//! in proto-list order. `prost` preserves order on `repeated Effect` fields.

use crate::api::{EngineState, Intent, Phase};
use crate::derived::derived_display;
use protos::engine::composing_response::Preedit;
use protos::engine::effect;
use protos::engine::AppConfig;
use protos::engine::{
    ClearPreeditWithoutCommit, CommitTextReplacingPreedit, ComposingResponse,
    DeleteBackwardFromDocument, Effect, PerformAutocomplete, ResetAutocomplete,
    ResetAutocompleteContext, UpdatePreedit,
};

/// Apply `intent` against `state`, mutate, return the proto response.
pub(crate) fn apply(
    state: &mut EngineState,
    intent: Intent,
    config: &AppConfig,
) -> ComposingResponse {
    match intent {
        Intent::Start { text } => enter_composing(state, text, config),
        Intent::Append { ch } => match &state.phase {
            Phase::Idle => enter_composing(state, ch, config),
            Phase::Composing { raw } => {
                let mut next = raw.clone();
                next.push_str(&ch);
                enter_composing(state, next, config)
            }
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
        Intent::CommitDerived => commit_derived(state, config),
        Intent::CommitRaw => commit_raw(state, config),
        Intent::SelectSuggestion { text } => select_suggestion(state, text, config),
        Intent::CommitPreeditThenInsertExternal { text } => {
            commit_preedit_then_insert_external(state, text, config)
        }
        Intent::Reset => reset(state, config),
        Intent::SetSelectedCandidateIndex { index } => {
            set_selected_candidate_index(state, index, config)
        }
        Intent::QueryState => snapshot(state, config),
    }
}

// ---- Helpers ------------------------------------------------------

/// Drop the last `char` from `s` in a single UTF-8 walk via `Chars::as_str`.
/// Returns "" if `s` is empty.
fn drop_last_char(s: &str) -> String {
    let mut it = s.chars();
    it.next_back();
    it.as_str().to_string()
}

/// Build a `composing` step response (typing / replace-last / delete-backward
/// non-empty branches). All three update the preedit + request a fresh
/// autocomplete query against the new buffer.
fn step_response(raw: String, display: String, selected_index: i32) -> ComposingResponse {
    ComposingResponse {
        preedit: Some(Preedit {
            raw_input: raw,
            display_text: display.clone(),
        }),
        effect: vec![update_preedit(display), perform_autocomplete()],
        selected_candidate_index: selected_index,
        is_composing: true,
    }
}

/// Enter or update the composing phase. A fresh composition step resets
/// `selected_candidate_index` to 0.
fn enter_composing(state: &mut EngineState, raw: String, config: &AppConfig) -> ComposingResponse {
    state.phase = Phase::Composing { raw: raw.clone() };
    state.selected_candidate_index = 0;
    let display = derived_display(&raw, config);
    step_response(raw, display, 0)
}

/// TPS auto-correct. Preserves `selected_candidate_index` (correction on top
/// of an in-progress selection).
fn replace_last(
    state: &mut EngineState,
    replacement: String,
    config: &AppConfig,
) -> ComposingResponse {
    let Phase::Composing { raw } = &state.phase else {
        return noop(state, config);
    };
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

fn delete_backward(state: &mut EngineState, config: &AppConfig) -> ComposingResponse {
    let Phase::Composing { raw } = &state.phase else {
        return noop(state, config);
    };
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

fn reset(state: &mut EngineState, config: &AppConfig) -> ComposingResponse {
    if matches!(state.phase, Phase::Idle) {
        return noop(state, config);
    }
    exit_to_idle(
        state,
        vec![clear_preedit_without_commit(), reset_autocomplete()],
    )
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
    };
    ComposingResponse {
        preedit: Some(Preedit {
            raw_input: raw,
            display_text: display,
        }),
        effect: Vec::new(),
        selected_candidate_index: state.selected_candidate_index,
        is_composing,
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
    }
}

fn noop(state: &EngineState, config: &AppConfig) -> ComposingResponse {
    snapshot(state, config)
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
