// Expands a nextword `PredictNext` into `FilterPredictions` by running the bundled lexicon lookup.

use protos::engine::{
    next_word_request::Method, FilterPredictions, NextWordRequest, PredictNext,
    RawNextWordPrediction, Source,
};

/// Bundled rows fetched per requested prediction: slack so the `(hanji, tl)`
/// merge with user rows never leaves fewer than `limit` survivors.
const BUNDLED_OVERFETCH_FACTOR: usize = 2;

/// Rewrites `PredictNext` into the `FilterPredictions` it stands for, the
/// bundled rows followed by `user_rows` (the engine's own, in store order);
/// every other method passes through untouched. Runs before the nextword
/// handle takes its locks, so the lexicon read never nests inside them.
pub(crate) fn expand_predict_next(
    request: NextWordRequest,
    user_rows: Vec<RawNextWordPrediction>,
) -> NextWordRequest {
    match request.method {
        Some(Method::PredictNext(predict)) => NextWordRequest {
            method: Some(Method::FilterPredictions(filter_request(
                predict, user_rows,
            ))),
        },
        _ => request,
    }
}

fn filter_request(
    predict: PredictNext,
    user_rows: Vec<RawNextWordPrediction>,
) -> FilterPredictions {
    let raw = if predict.word.is_empty() {
        // An empty word predicts nothing — the platforms returned no rows at
        // all, learned ones included, before this op existed.
        Vec::new()
    } else {
        let mut raw = bundled_rows(
            &predict.word,
            &predict.roman,
            predict.toggles,
            predict.limit,
        );
        raw.extend(user_rows);
        raw
    };
    FilterPredictions {
        raw,
        query_generation: predict.query_generation,
        now_ms: predict.now_ms,
        limit: predict.limit,
    }
}

/// Bundled `association.bin` rows for the committed word — the lexicon picks
/// the key (word key `hanji\u{1}tl`, backing off to the last character; §24
/// `INVARIANT_NEXTWORD_WORD_KEY_BACKOFF`). A lookup failure (lexicon not
/// installed yet) yields no rows, so learned rows still surface.
fn bundled_rows(
    word: &str,
    roman: &str,
    toggles: Option<protos::engine::DictionarySourceToggles>,
    limit: i32,
) -> Vec<RawNextWordPrediction> {
    let bundled_limit = nextword::api::effective_prediction_limit(limit) * BUNDLED_OVERFETCH_FACTOR;
    let bitmask = lexicon::api::association_bitmask(&toggles.unwrap_or_default());
    crate::context::bundled_continuations(word, roman, bundled_limit, bitmask)
        .into_iter()
        .map(|entry| RawNextWordPrediction {
            hanji: entry.candidate_word,
            tl: entry.candidate_tl,
            count: i64::from(entry.count),
            last_used_ms: 0,
            source: Source::Dict as i32,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;
    use protos::engine::{next_word_response, request, response, Request, Response};

    fn user_row(hanji: &str, count: i64) -> RawNextWordPrediction {
        RawNextWordPrediction {
            hanji: hanji.to_owned(),
            tl: String::new(),
            count,
            last_used_ms: 1,
            source: Source::User as i32,
        }
    }

    fn predict_next(word: &str) -> PredictNext {
        PredictNext {
            word: word.to_owned(),
            toggles: None,
            query_generation: 3,
            now_ms: 99,
            limit: 30,
            ..PredictNext::default()
        }
    }

    fn expanded_filter(
        predict: PredictNext,
        user_rows: Vec<RawNextWordPrediction>,
    ) -> FilterPredictions {
        let expanded = expand_predict_next(
            NextWordRequest {
                method: Some(Method::PredictNext(predict)),
            },
            user_rows,
        );
        let Some(Method::FilterPredictions(filter)) = expanded.method else {
            panic!("PredictNext must expand to FilterPredictions, got {expanded:?}");
        };
        filter
    }

    #[test]
    fn empty_word_filters_no_rows_even_with_user_rows() {
        let filter = expanded_filter(predict_next(""), vec![user_row("好", 5)]);
        assert!(filter.raw.is_empty());
        assert_eq!(filter.query_generation, 3);
        assert_eq!(filter.now_ms, 99);
        assert_eq!(filter.limit, 30);
    }

    // No lexicon is installed in this crate's tests, so the bundled lookup
    // fails the way it does before install — the learned rows must survive
    // in their SQL order.
    #[test]
    fn bundled_lookup_failure_keeps_user_rows_in_order() {
        let rows = vec![user_row("好", 5), user_row("食", 9)];
        let filter = expanded_filter(predict_next("早安"), rows.clone());
        assert_eq!(filter.raw, rows);
    }

    #[test]
    fn other_methods_pass_through_unchanged() {
        let filter_request = NextWordRequest {
            method: Some(Method::FilterPredictions(FilterPredictions::default())),
        };
        assert_eq!(
            expand_predict_next(filter_request.clone(), Vec::new()),
            filter_request
        );
    }

    // Through the FFI entry point: the expanded request reaches the nextword
    // handle, whose stale-generation check still applies.
    #[test]
    fn predict_next_via_process_request_drops_stale_generation() {
        let request = Request {
            id: 5,
            config_snapshot: None,
            generation: 424_242,
            payload: Some(request::Payload::Nextword(NextWordRequest {
                method: Some(Method::PredictNext(PredictNext {
                    query_generation: u64::MAX,
                    ..predict_next("早安")
                })),
            })),
        };
        let response_bytes = crate::process_request(&request.encode_to_vec());
        let response = Response::decode(response_bytes.as_slice()).expect("response decodes");
        let Some(response::Payload::Nextword(nextword)) = response.payload else {
            panic!("expected Nextword payload, got {response:?}");
        };
        let Some(next_word_response::Result::Filter(filter)) = nextword.result else {
            panic!("expected FilterResult, got {nextword:?}");
        };
        assert!(filter.was_stale);
        assert!(filter.predictions.is_empty());
    }
}
