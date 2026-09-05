//! Decode `NextWordRequest` → `Intent`, apply against `Engine`, encode the
//! `NextWordResponse`. Generation-mismatch envelope reset lives one layer
//! up in `EngineHandle::handle`.

use crate::api::{Engine, Intent, NextWordError};
use crate::booster;
use protos::engine::{
    next_word_request, next_word_response, AppConfig, BoostResult, NextWordRequest,
    NextWordResponse,
};

/// Decode the proto request method into a typed `Intent`. Returns
/// `MissingMethod` when `oneof method` is empty, `MissingDecisionInput`
/// when an intent's nested `DecisionInput` is missing.
fn decode_intent(req: &NextWordRequest) -> Result<DecodedRequest, NextWordError> {
    use next_word_request::Method;
    let method = req.method.clone().ok_or(NextWordError::MissingMethod)?;
    Ok(match method {
        Method::WordSelected(m) => {
            let now_ms = m
                .input
                .as_ref()
                .map(|i| i.now_ms)
                .ok_or(NextWordError::MissingDecisionInput)?;
            DecodedRequest::Decide(Intent::WordSelected {
                text: m.text,
                roman: m.roman,
                require_roman_mode: m.require_roman_mode,
                trigger_prediction: m.trigger_prediction,
                now_ms,
            })
        }
        Method::Backspace(m) => {
            let now_ms = m
                .input
                .as_ref()
                .map(|i| i.now_ms)
                .ok_or(NextWordError::MissingDecisionInput)?;
            DecodedRequest::Decide(Intent::Backspace {
                last_char: m.last_char,
                now_ms,
            })
        }
        Method::ContextTimeoutFired(m) => {
            let now_ms = m
                .input
                .as_ref()
                .map(|i| i.now_ms)
                .ok_or(NextWordError::MissingDecisionInput)?;
            DecodedRequest::Decide(Intent::ContextTimeoutFired { now_ms })
        }
        Method::ClearForNewComposing(m) => {
            let now_ms = m
                .input
                .as_ref()
                .map(|i| i.now_ms)
                .ok_or(NextWordError::MissingDecisionInput)?;
            DecodedRequest::Decide(Intent::ClearForNewComposing { now_ms })
        }
        Method::ResetFull(m) => {
            let now_ms = m
                .input
                .as_ref()
                .map(|i| i.now_ms)
                .ok_or(NextWordError::MissingDecisionInput)?;
            DecodedRequest::Decide(Intent::ResetFull { now_ms })
        }
        Method::UpdateLastSelectedWord(m) => {
            let now_ms = m
                .input
                .as_ref()
                .map(|i| i.now_ms)
                .ok_or(NextWordError::MissingDecisionInput)?;
            DecodedRequest::Decide(Intent::UpdateLastSelectedWord {
                text: m.text,
                roman: m.roman,
                now_ms,
            })
        }
        Method::SetIsShowing(m) => DecodedRequest::Decide(Intent::SetIsShowing {
            is_showing: m.is_showing,
        }),
        Method::FilterPredictions(m) => DecodedRequest::Filter {
            raw: m.raw,
            query_generation: m.query_generation,
            now_ms: m.now_ms,
            limit: m.limit,
        },
        Method::BoostCandidates(m) => DecodedRequest::Boost {
            words: m.words,
            predicted_first_chars: m.predicted_first_chars,
        },
        Method::QueryState(_) => DecodedRequest::QueryState,
    })
}

enum DecodedRequest {
    Decide(Intent),
    Filter {
        raw: Vec<protos::engine::RawNextWordPrediction>,
        query_generation: u64,
        now_ms: i64,
        limit: i32,
    },
    Boost {
        words: Vec<String>,
        predicted_first_chars: Vec<String>,
    },
    QueryState,
}

/// Pure dispatch entry: decode the proto request and route to the matching
/// engine method. `BoostCandidates` is stateless — runs without entering
/// the decide / filter paths. `QueryState` is also stateless (read-only
/// snapshot). Other methods route through the engine state machine.
pub fn handle(
    req: &NextWordRequest,
    engine: &mut Engine,
    config: &AppConfig,
) -> Result<NextWordResponse, NextWordError> {
    let decoded = decode_intent(req)?;
    let result = match decoded {
        DecodedRequest::Decide(intent) => {
            let decide = engine.apply(intent, config)?;
            next_word_response::Result::Decide(decide)
        }
        DecodedRequest::Filter {
            raw,
            query_generation,
            now_ms,
            limit,
        } => {
            let filter = engine.filter(raw, query_generation, now_ms, limit, config)?;
            next_word_response::Result::Filter(filter)
        }
        DecodedRequest::Boost {
            words,
            predicted_first_chars,
        } => {
            let words = booster::boost_words(words, predicted_first_chars);
            next_word_response::Result::Boost(BoostResult { words })
        }
        DecodedRequest::QueryState => next_word_response::Result::StateSnapshot(engine.snapshot()),
    };
    Ok(NextWordResponse {
        result: Some(result),
    })
}
