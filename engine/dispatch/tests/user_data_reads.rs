//! The engine reading its own user data (user-data-engine-roadmap P3b):
//! once the platform opens the stores, a `FetchAtPos` / `PredictNext` that
//! carries no rows answers exactly what the platform path answered when it
//! sent the same rows itself. Its own process: the user-data handle is
//! process-wide.
#![cfg(feature = "user-data")]

use std::path::PathBuf;

use lexicon::{EngineHandle as LexiconHandle, LexiconPaths};
use prost::Message;
use protos::engine::{
    composing_request, next_word_request, next_word_response, request, response, user_data_request,
    AppConfig, Append, ComposingRequest, ContinuousResponse, CustomDictEntry, DictionaryToggles,
    EnginePrediction, EnterContinuous, FetchAtPos, FrequencyEntry, LearnedEntry, NextWordRequest,
    OpenUserData, Platform, PredictNext, RawNextWordPrediction, Request, Response, Source,
    UserDataJournal, UserDataRequest,
};
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

fn config() -> AppConfig {
    AppConfig {
        platform_id: Platform::Ios as i32,
        input_mode: "tl".to_owned(),
        is_translate_swapped: true,
        ..AppConfig::default()
    }
}

fn roundtrip(generation: u64, payload: request::Payload) -> Response {
    let request = Request {
        id: 1,
        config_snapshot: Some(config()),
        generation,
        payload: Some(payload),
    };
    Response::decode(dispatch::process_request(&request.encode_to_vec()).as_slice())
        .expect("response decodes")
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

fn predict(roman: &str, user_rows: Vec<RawNextWordPrediction>) -> Vec<EnginePrediction> {
    let response = roundtrip(
        0,
        request::Payload::Nextword(NextWordRequest {
            method: Some(next_word_request::Method::PredictNext(PredictNext {
                word: "食".to_owned(),
                roman: roman.to_owned(),
                user_rows,
                toggles: Some(DictionaryToggles {
                    kautian: true,
                    ..DictionaryToggles::default()
                }),
                query_generation: 0,
                now_ms: NOW_MS,
                limit: 30,
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
fn engine_reads_answer_what_the_platform_rows_answered() {
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

    // What the platform sent before the engine owned the data.
    let key = userdata::derive_custom_query_key("tsiah", "tl").unwrap();
    let platform_fetch = FetchAtPos {
        frequency_entries: stores
            .frequency
            .rows_for_words(std::slice::from_ref(&boosted.display_text))
            .unwrap()
            .into_iter()
            .map(|row| FrequencyEntry {
                display_text_key: row.word,
                count: u32::try_from(row.count).unwrap(),
                last_used_ms: row.last_used_ms,
                canonical_tl: row.tl,
            })
            .collect(),
        custom_entries: CustomDictionarySource::rows_matching(
            &stores.custom_dictionary,
            &key.family,
            &key.form,
            &key.key,
        )
        .into_iter()
        .map(|row| CustomDictEntry {
            roman: row.roman,
            hanji: (!row.hanzi.is_empty()).then_some(row.hanzi),
        })
        .collect(),
        learned_entries: LearnedPhraseSource::rows_matching(
            &stores.learned_phrases,
            &key.family,
            &key.form,
            &key.key,
        )
        .into_iter()
        .map(|phrase| LearnedEntry {
            hanji: phrase.hanzi,
            canonical_tl: phrase.canonical_tl,
        })
        .collect(),
        ..FetchAtPos::default()
    };
    assert!(!platform_fetch.frequency_entries.is_empty());
    assert!(!platform_fetch.custom_entries.is_empty());
    assert!(!platform_fetch.learned_entries.is_empty());
    let platform_rows: Vec<RawNextWordPrediction> = stores
        .association
        .rows_following("食", "tsia̍h", 60)
        .unwrap()
        .into_iter()
        .map(|row| RawNextWordPrediction {
            hanzi: row.next,
            tl: row.next_tl,
            count: row.count,
            last_used_ms: row.last_used_ms,
            source: Source::User as i32,
        })
        .collect();
    let platform_candidates = fetch(platform_fetch);
    let platform_predictions = predict("tsia̍h", platform_rows);
    assert_ne!(
        platform_candidates, neutral,
        "the rows change the answer, or the test proves nothing"
    );
    drop(stores);

    // The engine owns the data: the same requests, no rows.
    let opened = roundtrip(
        0,
        request::Payload::UserData(UserDataRequest {
            method: Some(user_data_request::Method::Open(OpenUserData {
                frequency_path: paths.frequency.display().to_string(),
                association_path: paths.association.display().to_string(),
                custom_dictionary_path: paths.custom_dictionary.display().to_string(),
                learned_phrases_path: paths.learned_phrases.display().to_string(),
                journal: UserDataJournal::Delete as i32,
            })),
        }),
    );
    assert!(matches!(
        opened.payload,
        Some(response::Payload::UserData(_))
    ));

    assert_eq!(fetch(FetchAtPos::default()), platform_candidates);
    assert_eq!(predict("tsia̍h", Vec::new()), platform_predictions);
    // Rows a platform still sends are replaced, never merged (U9).
    assert_eq!(
        fetch(FetchAtPos {
            custom_entries: vec![CustomDictEntry {
                roman: "tsiah".to_owned(),
                hanji: Some("不該出現".to_owned()),
            }],
            ..FetchAtPos::default()
        }),
        platform_candidates
    );
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
