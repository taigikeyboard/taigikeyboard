//! Next-word slice of the engine bridge: the learning intents the desktop
//! sends, which is three of the engine's eight.
//!
//! The desktop learns but does not predict, so the whole read half —
//! `FilterPredictions`, `SetPredictionsVisible` — has no caller and is not wrapped. `Backspace` and
//! `ContextTimeoutFired` are absent too: recording is already fenced by a
//! strict 10-second window inside the engine (`decide.rs` `should_record_association`), so with
//! no predictions on screen a fired timeout changes nothing observable.
//!
//! The desktop acts on none of the answer's effects: the engine records the
//! bigrams it decides on itself (user-data-engine-roadmap P9b), and the rest
//! are about a prediction UI the desktop does not have.

use protos::engine::{
    next_word_request, next_word_response, request, response, CommittedWord, DecisionInput,
    NextWordRequest, ResetAll, UpdateLastSelectedWord, WordSelected,
};

use super::bridge::{app_config, record_failure, roundtrip};
use crate::settings::EngineSettings;

/// The user committed `text`, read as `roman`. `trigger_prediction` is forced
/// `false` rather than forwarded from the composing effect: the flag only
/// decides whether the engine appends a `QueryPredictions` effect, and the
/// desktop has nothing to answer such a query with (`decide.rs` `decide_word_selected`).
pub fn word_selected(
    text: &str,
    roman: &str,
    preceding: &[CommittedWord],
    now_ms: i64,
    settings: &EngineSettings,
    generation: u64,
) {
    decide(
        next_word_request::Method::WordSelected(WordSelected {
            text: text.to_owned(),
            roman: roman.to_owned(),
            // Gates Enter-commits-raw-romanization paths, which reach the
            // engine as ordinary commits on desktop.
            require_roman_mode: false,
            trigger_prediction: false,
            input: Some(DecisionInput { now_ms }),
            preceding: preceding.to_vec(),
        }),
        "nextwordWordSelected",
        settings,
        generation,
    )
}

/// A continuous composition nailed a segment mid-commit: nothing is
/// finalized into the document yet, so the engine learns nothing and keeps
/// the committed context — the final commit's `preceding` carries the
/// segment (behavioral-invariants §40).
pub fn update_last_selected_word(
    text: &str,
    roman: &str,
    now_ms: i64,
    settings: &EngineSettings,
    generation: u64,
) {
    decide(
        next_word_request::Method::UpdateLastSelectedWord(UpdateLastSelectedWord {
            text: text.to_owned(),
            roman: roman.to_owned(),
            input: Some(DecisionInput { now_ms }),
        }),
        "nextwordUpdateLastSelectedWord",
        settings,
        generation,
    )
}

/// Forgets the current context outright. Sent when the composition session
/// changes hands, so the last word typed in one application cannot be
/// learned as the predecessor of the first word typed in the next.
pub fn reset_all(now_ms: i64, settings: &EngineSettings, generation: u64) {
    decide(
        next_word_request::Method::ResetAll(ResetAll {
            input: Some(DecisionInput { now_ms }),
        }),
        "nextwordResetAll",
        settings,
        generation,
    )
}

fn decide(method: next_word_request::Method, op: &str, settings: &EngineSettings, generation: u64) {
    let payload = request::Payload::Nextword(NextWordRequest {
        method: Some(method),
    });
    let Some(response) = roundtrip(payload, op, generation, Some(app_config(settings))) else {
        return;
    };
    match response {
        response::Payload::Nextword(response) => {
            if !matches!(response.result, Some(next_word_response::Result::Decide(_))) {
                record_failure(op, "expected a decide result");
            }
        }
        other => record_failure(op, &format!("expected a nextword payload, got {other:?}")),
    }
}
