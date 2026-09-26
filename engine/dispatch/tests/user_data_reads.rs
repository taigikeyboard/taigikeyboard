//! The engine reading its own user data (user-data-engine-roadmap P3b):
//! once the platform opens the stores, a `FetchAtPos` answers exactly what
//! composing answers for the same rows handed to it directly, and a
//! `PredictNext` ranks the bigram the store learned — and nothing else
//! changes. Its own process: the user-data handle is process-wide.
#![cfg(feature = "user-data")]

mod common;

use common::{open_user_data, tl_config};
use std::path::PathBuf;

use composing::{Intent, UserRows};
use lexicon::{EngineHandle as LexiconHandle, LexiconPaths};
use protos::engine::{
    composing_request, next_word_request, next_word_response, request, response, Append,
    ComposingRequest, ContinuousResponse, DictionaryToggles, EnginePrediction, EnterContinuous,
    FetchAtPos, NextWordRequest, PredictNext, Response,
};
use ranking::{FrequencyData, FrequencyMap};
use userdata::{
    AssociationPair, CustomDictionaryRow, CustomDictionarySource, JournalMode, LearnedPhraseSource,
    UserDataPaths, UserDataStores,
};

const COMPOSING_GENERATION: u64 = 1;
const NOW_MS: i64 = 1_800_000_000_000;

fn production_artifact(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../dictionaries")
        .join(name)
        .to_str()
        .expect("artifact path UTF-8")
        .to_owned()
}

/// Installs the production lexicon; `false` (soft-skip) when the artifacts
/// are absent — as `predict_next.rs`.
fn lexicon_ready() -> bool {
    if !PathBuf::from(production_artifact("association.bin")).exists() {
        eprintln!("user_data_reads: production artifacts absent — run `make dict`; skipping.");
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
}

fn roundtrip(generation: u64, payload: request::Payload) -> Response {
    common::roundtrip(tl_config(true), generation, payload)
}

fn composing(method: composing_request::Method) -> Response {
    roundtrip(
        COMPOSING_GENERATION,
        request::Payload::Composing(ComposingRequest {
            method: Some(method),
        }),
    )
}

fn fetch(fetch: FetchAtPos) -> ContinuousResponse {
    let response = composing(composing_request::Method::FetchAtPos(FetchAtPos {
        now_ms: NOW_MS,
        enabled_sources_bitmask: u32::MAX,
        ..fetch
    }));
    match response.payload {
        Some(response::Payload::Composing(composing)) => {
            composing.continuous.expect("FetchAtPos answers candidates")
        }
        other => panic!("expected a composing payload, got {other:?}"),
    }
}

fn predict(roman: &str) -> Vec<EnginePrediction> {
    let response = roundtrip(
        0,
        request::Payload::Nextword(NextWordRequest {
            method: Some(next_word_request::Method::PredictNext(PredictNext {
                word: "食".to_owned(),
                roman: roman.to_owned(),
                toggles: Some(DictionaryToggles {
                    kautian: true,
                    ..DictionaryToggles::default()
                }),
                query_generation: 0,
                now_ms: NOW_MS,
                limit: 30,
                ..PredictNext::default()
            })),
        }),
    );
    match response.payload {
        Some(response::Payload::Nextword(nextword)) => match nextword.result {
            Some(next_word_response::Result::Filter(filter)) => filter.predictions,
            other => panic!("expected predictions, got {other:?}"),
        },
        other => panic!("expected a nextword payload, got {other:?}"),
    }
}

#[test]
fn engine_reads_answer_what_the_same_rows_answer() {
    if !lexicon_ready() {
        return;
    }
    for character in ["t", "s", "i", "a", "h"] {
        composing(composing_request::Method::Append(Append {
            char: character.to_owned(),
        }));
    }
    composing(composing_request::Method::EnterContinuous(
        EnterContinuous {},
    ));
    let neutral = fetch(FetchAtPos::default());
    let boosted = neutral
        .candidates
        .last()
        .expect("tsiah has candidates")
        .clone();

    // The user's data, written the way the stores write it.
    let directory = tempfile::tempdir().unwrap();
    let paths = UserDataPaths::in_directory(directory.path());
    let stores = UserDataStores::at(paths.clone(), JournalMode::Delete);
    stores.open_blocking();
    for _ in 0..5 {
        stores
            .frequency
            .record(&boosted.display_text, &boosted.canonical_tl);
    }
    stores
        .custom_dictionary
        .upsert(&CustomDictionaryRow::new("tsia̍h-tê", "食茶"))
        .unwrap();
    stores.learned_phrases.learn_phrase("食", "tsia̍h");
    stores.association.record(&[AssociationPair {
        previous: "食".to_owned(),
        previous_tl: "tsia̍h".to_owned(),
        next: "飯".to_owned(),
        next_tl: "pn̄g".to_owned(),
    }]);
    // Flush the queued writes before reading them back.
    stores.frequency.all_rows();
    stores.learned_phrases.all_rows();
    stores.association.all_rows();

    // The same rows, handed to composing directly.
    let key = userdata::derive_custom_query_key("tsiah", "tl").unwrap();
    let frequency: FrequencyMap = stores
        .frequency
        .rows_for_words(std::slice::from_ref(&boosted.display_text))
        .unwrap()
        .into_iter()
        .map(|row| {
            let data = FrequencyData {
                count: i32::try_from(row.count).unwrap(),
                last_used_ms: row.last_used_ms,
            };
            (row.word, row.tl, data)
        })
        .collect();
    let rows = UserRows {
        frequency,
        custom: CustomDictionarySource::rows_matching(
            &stores.custom_dictionary,
            &key.family,
            &key.form,
            &key.key,
        )
        .into_iter()
        .map(|row| lexicon::CustomEntry {
            roman: row.roman,
            hanji: (!row.hanzi.is_empty()).then_some(row.hanzi),
        })
        .collect(),
        learned: LearnedPhraseSource::rows_matching(
            &stores.learned_phrases,
            &key.family,
            &key.form,
            &key.key,
        )
        .into_iter()
        .map(|phrase| lexicon::LearnedEntry {
            hanji: phrase.hanzi,
            canonical_tl: phrase.canonical_tl,
        })
        .collect(),
    };
    assert!(rows.frequency != FrequencyMap::new());
    assert!(!rows.custom.is_empty());
    assert!(!rows.learned.is_empty());
    let direct_candidates = composing::EngineHandle::instance()
        .query(
            &Intent::FetchAtPos {
                now_ms: NOW_MS,
                enabled_sources_bitmask: u32::MAX,
                literal_roman_candidate_disabled: false,
                user_rows: rows,
            },
            &tl_config(true),
            COMPOSING_GENERATION,
        )
        .continuous
        .expect("FetchAtPos answers candidates");
    let bundled_only = predict("tsia̍h");
    assert_ne!(
        direct_candidates, neutral,
        "the rows change the answer, or the test proves nothing"
    );
    drop(stores);

    // The engine owns the data: the requests carry no rows.
    open_user_data(&paths);

    assert_eq!(fetch(FetchAtPos::default()), direct_candidates);
    // 食 → 飯 / pn̄g, the one bigram the store holds, is boosted; every
    // other prediction stays as the bundled rows ranked it.
    let learned = predict("tsia̍h");
    let is_learned = |p: &&EnginePrediction| p.hanzi == "飯" && p.tl == "pn̄g";
    let score_of = |predictions: &[EnginePrediction]| {
        predictions
            .iter()
            .find(is_learned)
            .expect("飯 / pn̄g predicted")
            .score
    };
    assert!(score_of(&learned) > score_of(&bundled_only));
    let others = |predictions: &[EnginePrediction]| {
        predictions
            .iter()
            .filter(|p| !is_learned(p))
            .cloned()
            .collect::<Vec<_>>()
    };
    assert_eq!(others(&learned), others(&bundled_only));
    // The custom-dictionary setting, off: the engine reads no custom rows.
    let without_custom = fetch(FetchAtPos {
        custom_dictionary_disabled: true,
        ..FetchAtPos::default()
    });
    assert!(without_custom
        .candidates
        .iter()
        .all(|candidate| candidate.hanji.as_deref() != Some("食茶")));
}
