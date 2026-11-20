// INVARIANT_CONTINUOUS_CONTEXT_RERANK (behavioral-invariants §56) through
// `process_request` against the production lexicon: the word the pending
// tail follows — the last committed word inside its association window, or
// the composition's last nailed segment — re-ranks the continuous candidates;
// a user-learned continuation outranks a bundled one.
#![cfg(feature = "user-data")]

mod common;

use protos::engine::CommitScript;
use std::sync::OnceLock;

use common::{open_user_data, roundtrip, tl_config};
use protos::engine::{
    composing_request, next_word_request, request, response, CommitContinuous, ComposingRequest,
    DecisionInput, FetchAtPos, NextWordRequest, Reset, ResetAll, Start, WordSelected,
};
use userdata::{AssociationPair, JournalMode, UserDataPaths, UserDataStores};

const NOW_MS: i64 = 1_800_000_000_000;

/// Opens the engine's user data once, with one learned bigram 真 → 姊/tsé.
fn user_data_open() {
    static DIRECTORY: OnceLock<tempfile::TempDir> = OnceLock::new();
    DIRECTORY.get_or_init(|| {
        let directory = tempfile::tempdir().unwrap();
        let paths = UserDataPaths::in_directory(directory.path());
        let stores = UserDataStores::at(paths.clone(), JournalMode::Delete);
        stores.open_blocking();
        stores.association.record(&[AssociationPair {
            previous: "真".to_owned(),
            previous_tl: "tsin".to_owned(),
            next: "姊".to_owned(),
            next_tl: "tsé".to_owned(),
        }]);
        stores.association.all_rows(); // flush the queued writes
        drop(stores);
        open_user_data(directory.path());
        directory
    });
}

fn nextword(method: next_word_request::Method) {
    let response = roundtrip(
        tl_config(true),
        0,
        request::Payload::Nextword(NextWordRequest {
            method: Some(method),
        }),
    );
    assert!(matches!(
        response.payload,
        Some(response::Payload::Nextword(_))
    ));
}

fn composing(generation: u64, method: composing_request::Method) -> response::Payload {
    roundtrip(
        tl_config(true),
        generation,
        request::Payload::Composing(ComposingRequest {
            next_word_context: None,
            method: Some(method),
        }),
    )
    .payload
    .expect("composing answers")
}

/// The hanji of a fresh `raw` composition's candidates, in display order.
fn hanji_after_typing(generation: u64, raw: &str) -> Vec<String> {
    composing(
        generation,
        composing_request::Method::Reset(Reset::default()),
    );
    composing(
        generation,
        composing_request::Method::Start(Start {
            text: raw.to_owned(),
        }),
    );
    fetched_hanji(generation)
}

fn fetched_hanji(generation: u64) -> Vec<String> {
    let payload = composing(
        generation,
        composing_request::Method::FetchAtPos(FetchAtPos {
            now_ms: NOW_MS + 1_000,
            ..FetchAtPos::default()
        }),
    );
    let response::Payload::Composing(composing) = payload else {
        panic!("expected a composing payload, got {payload:?}");
    };
    composing
        .continuous
        .expect("FetchAtPos answers candidates")
        .candidates
        .into_iter()
        .filter_map(|candidate| candidate.hanji)
        .collect()
}

fn commit(text: &str, roman: &str, now_ms: i64) {
    nextword(next_word_request::Method::WordSelected(WordSelected {
        text: text.to_owned(),
        roman: roman.to_owned(),
        require_roman_mode: false,
        trigger_prediction: false,
        input: Some(DecisionInput { now_ms }),
        preceding: Vec::new(),
    }));
}

// One process, one test: the composing and next-word singletons are shared.
#[test]
fn the_previous_word_reranks_the_candidates() {
    if !common::production_lexicon_ready() {
        return;
    }
    user_data_open();
    let mut generation = 1;

    // No committed word: the context-free order, neither continuation first.
    nextword(next_word_request::Method::ResetAll(ResetAll {
        input: Some(DecisionInput { now_ms: NOW_MS }),
    }));
    let baseline = hanji_after_typing(generation, "tse");
    assert!(
        !matches!(baseline.first().map(String::as_str), Some("濟" | "姊")),
        "{baseline:?}"
    );

    // 真 committed 1 s ago: its learned continuation 姊 leads, then the
    // bundled 濟, then the rest in their context-free order.
    commit("真", "tsin", NOW_MS);
    generation += 1;
    let ranked = hanji_after_typing(generation, "tse");
    assert_eq!(
        ranked
            .iter()
            .take(2)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["姊", "濟"],
        "{ranked:?}"
    );
    // The developer view ranks with the same stored rows.
    let explained: serde_json::Value = serde_json::from_str(
        &dispatch::explain_composition(&tl_config(true), generation, NOW_MS + 1_000, false)
            .expect("a composition is open"),
    )
    .unwrap();
    let first_hanji = explained["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|row| row["hanji"].as_str());
    assert_eq!(first_hanji, Some("姊"), "{explained}");

    // Outside the association window the committed word is no context.
    commit("真", "tsin", NOW_MS - 10_000);
    generation += 1;
    assert_eq!(hanji_after_typing(generation, "tse"), baseline);

    // A sentence end / full reset clears it.
    commit("真", "tsin", NOW_MS);
    nextword(next_word_request::Method::ResetAll(ResetAll {
        input: Some(DecisionInput { now_ms: NOW_MS }),
    }));
    generation += 1;
    assert_eq!(hanji_after_typing(generation, "tse"), baseline);

    // Inside a composition the pending tail follows the last nailed segment,
    // whatever the next-word context says.
    generation += 1;
    composing(
        generation,
        composing_request::Method::Reset(Reset::default()),
    );
    composing(
        generation,
        composing_request::Method::Start(Start {
            text: "tsintse".to_owned(),
        }),
    );
    composing(
        generation,
        composing_request::Method::CommitContinuous(CommitContinuous {
            script: CommitScript::Roman as i32,
            roman: "真".to_owned(),
            canonical_text: "真".to_owned(),
            association_tl: "tsin".to_owned(),
            hanji: Some("真".to_owned()),
            consumed_bytes: 4,
            syllable_count: 1,
        }),
    );
    let nailed = fetched_hanji(generation);
    assert_eq!(
        nailed
            .iter()
            .take(2)
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["姊", "濟"],
        "{nailed:?}"
    );
}
