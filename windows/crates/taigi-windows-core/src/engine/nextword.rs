//! Next-word slice of the engine bridge: the learning intents the desktop
//! sends, which is three of the engine's nine. Port of
//! `RustEngineBridge+NextWord.swift`.
//!
//! The desktop learns but does not predict, so the whole read half —
//! `FilterPredictions`, `BoostCandidates`, `NextWordQueryState`,
//! `SetIsShowing` — has no caller and is not wrapped. `Backspace` and
//! `ContextTimeoutFired` are absent too: recording is already fenced by a
//! strict 10-second window inside the engine (`decide.rs:306-312`), so with
//! no predictions on screen a fired timeout changes nothing observable.

use protos::engine::{
    next_word_effect, next_word_request, next_word_response, request, response, DecisionInput,
    NextWordEffect as WireEffect, NextWordRequest, ResetFull, UpdateLastSelectedWord, WordSelected,
};

use super::bridge::{nextword_config, record_failure, roundtrip};
use crate::settings::EngineSettings;

/// One learned bigram. `previous_tl` / `next_tl` are canonical TL — the
/// identity axis of Core Principle #7.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssociationPair {
    pub previous: String,
    pub previous_tl: String,
    pub next: String,
    pub next_tl: String,
}

/// What one learning intent asked the platform to write. Only the recording
/// effects are represented; the engine's other four are about a prediction
/// UI the desktop does not have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NextWordEffect {
    RecordAssociation(AssociationPair),
    RecordCompoundAssociations(Vec<AssociationPair>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NextWordOutcome {
    pub effects: Vec<NextWordEffect>,
}

/// The user committed `text`, read as `roman`. `trigger_prediction` is forced
/// `false` rather than forwarded from the composing effect: the flag only
/// decides whether the engine appends a `QueryPredictions` effect, and the
/// desktop has nothing to answer such a query with (`decide.rs:130-180`).
pub fn word_selected(
    text: &str,
    roman: &str,
    now_ms: i64,
    settings: &EngineSettings,
    generation: u64,
) -> Option<NextWordOutcome> {
    decide(
        next_word_request::Method::WordSelected(WordSelected {
            text: text.to_owned(),
            roman: roman.to_owned(),
            // Gates Enter-commits-raw-romanization paths, which reach the
            // engine as ordinary commits on desktop.
            require_roman_mode: false,
            trigger_prediction: false,
            input: Some(DecisionInput { now_ms }),
        }),
        "nextwordWordSelected",
        settings,
        generation,
    )
}

/// A continuous composition nailed a segment mid-commit: the context moves
/// on, nothing is finalized into the document yet. Records the compound
/// bigrams only and does not bump the engine's generation (`decide.rs:253-290`).
pub fn update_last_selected_word(
    text: &str,
    roman: &str,
    now_ms: i64,
    settings: &EngineSettings,
    generation: u64,
) -> Option<NextWordOutcome> {
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
pub fn reset_full(
    now_ms: i64,
    settings: &EngineSettings,
    generation: u64,
) -> Option<NextWordOutcome> {
    decide(
        next_word_request::Method::ResetFull(ResetFull {
            input: Some(DecisionInput { now_ms }),
        }),
        "nextwordResetFull",
        settings,
        generation,
    )
}

fn decide(
    method: next_word_request::Method,
    op: &str,
    settings: &EngineSettings,
    generation: u64,
) -> Option<NextWordOutcome> {
    let payload = request::Payload::Nextword(NextWordRequest {
        method: Some(method),
    });
    let response = match roundtrip(payload, op, generation, Some(nextword_config(settings)))? {
        response::Payload::Nextword(response) => response,
        other => {
            record_failure(op, &format!("expected a nextword payload, got {other:?}"));
            return None;
        }
    };
    match response.result {
        Some(next_word_response::Result::Decide(result)) => Some(NextWordOutcome {
            effects: result.effects.iter().filter_map(decode_effect).collect(),
        }),
        _ => {
            record_failure(op, "expected a decide result");
            None
        }
    }
}

/// `None` for an effect this platform has nothing to do with. Exhaustive on
/// purpose: a next-word effect added to the engine later has to be classified
/// here rather than silently ignored.
fn decode_effect(effect: &WireEffect) -> Option<NextWordEffect> {
    match effect.kind.as_ref()? {
        next_word_effect::Kind::RecordAssociation(payload) => payload
            .pair
            .as_ref()
            .map(|pair| NextWordEffect::RecordAssociation(decode_pair(pair))),
        next_word_effect::Kind::RecordCompoundAssociations(payload) => {
            Some(NextWordEffect::RecordCompoundAssociations(
                payload.pairs.iter().map(decode_pair).collect(),
            ))
        }
        // The desktop runs no context timer — see this file's header.
        next_word_effect::Kind::RescheduleContextTimeout(_)
        | next_word_effect::Kind::CancelContextTimeout(_) => None,
        // Neither is reachable: queries need `trigger_prediction`, always
        // false here, and a UI clear needs `is_showing`, never set true.
        next_word_effect::Kind::QueryPredictions(_)
        | next_word_effect::Kind::ClearPredictionsUi(_) => None,
    }
}

fn decode_pair(pair: &protos::engine::AssociationPair) -> AssociationPair {
    AssociationPair {
        previous: pair.prev.clone(),
        previous_tl: pair.prev_tl.clone(),
        next: pair.next.clone(),
        next_tl: pair.next_tl.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protos::engine::{CancelContextTimeout, RecordAssociation};

    #[test]
    fn only_recording_effects_survive_decoding() {
        let record = WireEffect {
            kind: Some(next_word_effect::Kind::RecordAssociation(
                RecordAssociation {
                    pair: Some(protos::engine::AssociationPair {
                        prev: "台".into(),
                        prev_tl: "tâi".into(),
                        next: "語".into(),
                        next_tl: "gí".into(),
                    }),
                },
            )),
        };
        let timer = WireEffect {
            kind: Some(next_word_effect::Kind::CancelContextTimeout(
                CancelContextTimeout {},
            )),
        };
        assert_eq!(
            decode_effect(&record),
            Some(NextWordEffect::RecordAssociation(AssociationPair {
                previous: "台".into(),
                previous_tl: "tâi".into(),
                next: "語".into(),
                next_tl: "gí".into(),
            }))
        );
        assert_eq!(decode_effect(&timer), None);
        assert_eq!(decode_effect(&WireEffect { kind: None }), None);
    }
}
