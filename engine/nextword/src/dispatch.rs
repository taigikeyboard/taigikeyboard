//! Decode `NextWordRequest` → `Intent`, apply against `Engine`, encode the
//! `NextWordResponse`. Generation-mismatch envelope reset lives one layer
//! up in `EngineHandle::handle`.

use crate::api::{Engine, Handled, Intent, NextWordError};
use protos::engine::{
    next_word_request, next_word_response, AppConfig, NextWordRequest, NextWordResponse,
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
        Method::PredictNext(_) => return Err(NextWordError::UnexpandedPredictNext),
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
}

/// Pure dispatch entry: decode the proto request and route to the matching
/// engine method (decide intents or the post-query filter).
pub fn handle(
    req: &NextWordRequest,
    engine: &mut Engine,
    config: &AppConfig,
) -> Result<Handled, NextWordError> {
    let decoded = decode_intent(req)?;
    let (result, associations) = match decoded {
        DecodedRequest::Decide(intent) => {
            let decided = engine.apply(intent, config)?;
            (
                next_word_response::Result::Decide(decided.result),
                decided.associations,
            )
        }
        DecodedRequest::Filter {
            raw,
            query_generation,
            now_ms,
            limit,
        } => {
            let filter = engine.filter(raw, query_generation, now_ms, limit, config)?;
            (next_word_response::Result::Filter(filter), Vec::new())
        }
    };
    Ok(Handled {
        response: NextWordResponse {
            result: Some(result),
        },
        associations,
    })
}
