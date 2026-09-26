// Expands a nextword `PredictNext` into `FilterPredictions` by running the bundled lexicon lookup.

use protos::engine::{
    next_word_request::Method, AssocLookupRequest, DictionaryFiltersRequest, FilterPredictions,
    NextWordRequest, PredictNext, RawNextWordPrediction, Source,
};

/// Bundled rows fetched per requested prediction: slack so the `(hanzi, tl)`
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
    let raw = match predict.word.chars().last() {
        // An empty word predicts nothing — the platforms returned no rows at
        // all, learned ones included, before this op existed.
        None => Vec::new(),
        Some(last_character) => {
            let mut raw = bundled_rows(last_character, predict.toggles, predict.limit);
            raw.extend(user_rows);
            raw
        }
    };
    FilterPredictions {
        raw,
        query_generation: predict.query_generation,
        now_ms: predict.now_ms,
        limit: predict.limit,
    }
}

/// Bundled `association.bin` rows keyed on the committed word's last
/// character. A lookup failure (lexicon not installed yet) yields no rows,
/// so learned rows still surface.
fn bundled_rows(
    last_character: char,
    toggles: Option<protos::engine::DictionaryToggles>,
    limit: i32,
) -> Vec<RawNextWordPrediction> {
    let bundled_limit = nextword::api::effective_prediction_limit(limit) * BUNDLED_OVERFETCH_FACTOR;
    let bitmask = match lexicon::api::dictionary_filters(DictionaryFiltersRequest { toggles }) {
        Ok(filters) => filters.assoc_lookup_bitmask,
        Err(err) => {
            log::warn!("nextword.predict.filters_failed: {err}");
            return Vec::new();
        }
    };
    let lookup = lexicon::api::assoc_lookup(AssocLookupRequest {
        previous_word: last_character.to_string(),
        limit: u32::try_from(bundled_limit).unwrap_or(u32::MAX),
        enabled_sources_bitmask: bitmask,
    });
    match lookup {
        Ok(response) => response
            .entries
            .into_iter()
            .map(|entry| RawNextWordPrediction {
                hanzi: entry.candidate_word,
                tl: entry.candidate_tl,
                count: i64::from(entry.count),
                last_used_ms: 0,
                source: Source::Dict as i32,
            })
            .collect(),
        Err(err) => {
            log::debug!("nextword.predict.bundled_lookup_skipped: {err}");
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use prost::Message;
    use protos::engine::{
        next_word_response, request, response, AppConfig, Platform, Request, Response,
    };

    fn user_row(hanzi: &str, count: i64) -> RawNextWordPrediction {
        RawNextWordPrediction {
            hanzi: hanzi.to_owned(),
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
            config_snapshot: Some(AppConfig {
                platform_id: Platform::Ios as i32,
                ..AppConfig::default()
            }),
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
