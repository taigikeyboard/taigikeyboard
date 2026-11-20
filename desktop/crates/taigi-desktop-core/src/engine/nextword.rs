//! Next-word slice of the engine bridge: the two intents the desktop sends
//! itself, out of the engine's eight.
//!
//! A composition's commits are not among them: every composing request
//! carries `NextWordContext`, and the engine applies the commit's next-word
//! handshake in that same call (roadmap E2). The desktop learns but does not
//! predict, so the whole read half — `FilterPredictions`,
//! `SetPredictionsVisible` — has no caller and is not wrapped. `Backspace` and
//! `ContextTimeoutFired` are absent too: recording is already fenced by a
//! strict 10-second window inside the engine (`decide.rs`
//! `should_record_association`), so with no predictions on screen a fired
//! timeout changes nothing observable.
//!
//! The desktop acts on none of the answer's effects: the engine records the
//! bigrams it decides on itself (user-data-engine-roadmap P9b), and the rest
//! are about a prediction UI the desktop does not have.

use protos::engine::{
    next_word_request, next_word_response, request, response, DecisionInput, NextWordRequest,
    ResetAll, WordSelected,
};

use super::bridge::{app_config, record_failure, roundtrip};
use super::composing::RequestStamp;
use crate::settings::EngineSettings;

/// `character` reached the document outside any composition: a commit with
/// no reading and no prediction, so the engine can end the context on
/// sentence-end punctuation (`decide.rs` sentence-end rule).
pub fn character_typed(character: &str, settings: &EngineSettings, stamp: RequestStamp) {
    decide(
        next_word_request::Method::WordSelected(WordSelected {
            text: character.to_owned(),
            roman: String::new(),
            require_roman_mode: false,
            // The desktop has nothing to answer a prediction query with.
            trigger_prediction: false,
            input: Some(DecisionInput {
                now_ms: stamp.now_ms,
            }),
            preceding: Vec::new(),
        }),
        "nextwordWordSelected",
        settings,
        stamp.generation,
    )
}

/// Forgets the current context outright. Sent when the composition session
/// changes hands, so the last word typed in one application cannot be
/// learned as the predecessor of the first word typed in the next.
pub fn reset_all(settings: &EngineSettings, stamp: RequestStamp) {
    decide(
        next_word_request::Method::ResetAll(ResetAll {
            input: Some(DecisionInput {
                now_ms: stamp.now_ms,
            }),
        }),
        "nextwordResetAll",
        settings,
        stamp.generation,
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
