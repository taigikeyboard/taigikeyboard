// PredictNext through `process_request` against the production lexicon: bundled rows and source toggles.

use std::path::PathBuf;
use std::sync::OnceLock;

use lexicon::{EngineHandle as LexiconHandle, LexiconPaths};
use prost::Message;
use protos::engine::{
    next_word_request::Method, next_word_response, request, response, AppConfig, DictionaryToggles,
    EnginePrediction, NextWordRequest, Platform, PredictNext, RawNextWordPrediction, Request,
    Response, Source,
};

fn production_artifact(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../dictionaries")
        .join(name);
    path.to_str().expect("artifact path UTF-8").to_owned()
}

/// Installs the production lexicon once; `false` (callers soft-skip) when
/// the artifacts are absent — mirrors `composing/tests/cross_mode_parity.rs`.
fn lexicon_ready() -> bool {
    static READY: OnceLock<bool> = OnceLock::new();
    *READY.get_or_init(|| {
        if !PathBuf::from(production_artifact("association.bin")).exists() {
            eprintln!("predict_next: production artifacts absent — run `make dict`; skipping.");
            return false;
        }
        let paths = LexiconPaths::validated(
            &production_artifact("dictionary.fst"),
            &production_artifact("dictionary.bin"),
            &production_artifact("association.bin"),
            &production_artifact("syllables.fst"),
            0,
        )
        .expect("validate production LexiconPaths");
        LexiconHandle::install(paths).is_ok()
    })
}

fn all_sources(enabled: bool) -> DictionaryToggles {
    DictionaryToggles {
        kautian: enabled,
        taigitv: enabled,
        itaigi: enabled,
        sitbut: enabled,
        taihoa: enabled,
        taijit: enabled,
        kungge: enabled,
        stti: enabled,
        khpoo: enabled,
        ..DictionaryToggles::default()
    }
}

fn learned_row(hanzi: &str) -> RawNextWordPrediction {
    RawNextWordPrediction {
        hanzi: hanzi.to_owned(),
        tl: String::new(),
        count: 1,
        last_used_ms: 0,
        source: Source::User as i32,
    }
}

/// One PredictNext round trip. Envelope generation 0 matches the fresh
/// nextword handle, so `query_generation: 0` is never stale.
fn predict(word: &str, toggles: DictionaryToggles) -> Vec<EnginePrediction> {
    let request = Request {
        id: 1,
        config_snapshot: Some(AppConfig {
            platform_id: Platform::Ios as i32,
            input_mode: "tl".to_owned(),
            is_translate_swapped: true,
            ..AppConfig::default()
        }),
        generation: 0,
        payload: Some(request::Payload::Nextword(NextWordRequest {
            method: Some(Method::PredictNext(PredictNext {
                word: word.to_owned(),
                user_rows: vec![learned_row("𫝛")],
                toggles: Some(toggles),
                query_generation: 0,
                now_ms: 0,
                limit: 30,
            })),
        })),
    };
    let response_bytes = dispatch::process_request(&request.encode_to_vec());
    let response = Response::decode(response_bytes.as_slice()).expect("response decodes");
    let Some(response::Payload::Nextword(nextword)) = response.payload else {
        panic!("expected Nextword payload, got {response:?}");
    };
    let Some(next_word_response::Result::Filter(filter)) = nextword.result else {
        panic!("expected FilterResult, got {nextword:?}");
    };
    assert!(!filter.was_stale);
    filter.predictions
}

fn hanzi_of(predictions: &[EnginePrediction]) -> Vec<&str> {
    predictions.iter().map(|p| p.hanzi.as_str()).collect()
}

// 台 → 灣 is the top bundled pair (association.bin, count 2137); the key is
// the last character, so 臺台 predicts from 台.
#[test]
fn bundled_rows_join_learned_rows_when_sources_enabled() {
    if !lexicon_ready() {
        return;
    }
    let predictions = predict("臺台", all_sources(true));
    let hanzi = hanzi_of(&predictions);
    assert!(
        hanzi.contains(&"灣"),
        "bundled 台→灣 expected, got {hanzi:?}"
    );
    assert!(hanzi.contains(&"𫝛"), "learned row expected, got {hanzi:?}");
    assert!(predictions.len() <= 30);
}

// INVARIANT_NEXTWORD_LOOKUP_KEY_LAST_GRAPHEME: 𣍐 (U+2334D) is one key, so
// 𣍐使 in the dictionary predicts 使 after a word ending in 𣍐.
#[test]
fn supplementary_plane_hanji_is_one_lookup_key() {
    if !lexicon_ready() {
        return;
    }
    let predictions = predict("袂𣍐", all_sources(true));
    let hanzi = hanzi_of(&predictions);
    assert!(
        hanzi.contains(&"使"),
        "bundled 𣍐→使 expected, got {hanzi:?}"
    );
}

#[test]
fn disabled_sources_leave_only_learned_rows() {
    if !lexicon_ready() {
        return;
    }
    let predictions = predict("台", all_sources(false));
    assert_eq!(hanzi_of(&predictions), vec!["𫝛"]);
}
