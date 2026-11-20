// The store-less `FetchAtPos` path through `process_request` (and the
// developer view of it) against the production lexicon: the stores never
// open in this process, so the bundled context ranks one pass alone — the
// path a desktop shell without `user-data` and every adapter before its
// open take. Its stores-open twin is `composing_context.rs`.

mod common;

use common::{roundtrip, tl_config};
use protos::engine::{
    composing_request, next_word_request, request, response, ComposingRequest, DecisionInput,
    FetchAtPos, NextWordRequest, Reset, ResetAll, Start, WordSelected,
};

const NOW_MS: i64 = 1_800_000_000_000;

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

fn nextword(method: next_word_request::Method) {
    roundtrip(
        tl_config(true),
        0,
        request::Payload::Nextword(NextWordRequest {
            method: Some(method),
        }),
    );
}

fn start(generation: u64, raw: &str) {
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
}

/// The hanji a `FetchAtPos` at `generation` lists, in display order.
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
        .map(|continuous| continuous.candidates)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|candidate| candidate.hanji)
        .collect()
}

/// The hanji of the developer view's candidate rows, in order.
fn explained_hanji(generation: u64) -> Vec<String> {
    let json = dispatch::explain_composition(&tl_config(true), generation, NOW_MS + 1_000, false)
        .expect("a composition is open");
    let view: serde_json::Value = serde_json::from_str(&json).unwrap();
    view["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|row| row["hanji"].as_str().map(str::to_owned))
        .collect()
}

// One process, one test: the composing and next-word singletons are shared.
#[test]
fn the_bundled_context_ranks_alone_before_the_stores_open() {
    if !common::production_lexicon_ready() {
        return;
    }
    nextword(next_word_request::Method::ResetAll(ResetAll {
        input: Some(DecisionInput { now_ms: NOW_MS }),
    }));
    nextword(next_word_request::Method::WordSelected(WordSelected {
        text: "真".to_owned(),
        roman: "tsin".to_owned(),
        require_roman_mode: false,
        trigger_prediction: false,
        input: Some(DecisionInput { now_ms: NOW_MS }),
        preceding: Vec::new(),
    }));
    start(1, "tse");

    // trace: 真 committed 1 s ago; its bundled continuation 濟 (association.bin)
    // leads. 姊 is only a user-learned continuation (`composing_context.rs`),
    // and no store is open, so it keeps its context-free place.
    let fetched = fetched_hanji(1);
    assert_eq!(
        fetched.first().map(String::as_str),
        Some("濟"),
        "{fetched:?}"
    );
    assert_ne!(
        fetched.get(1).map(String::as_str),
        Some("姊"),
        "{fetched:?}"
    );

    // The developer view ranks with the same inputs.
    let explained = explained_hanji(1);
    assert_eq!(explained.first(), fetched.first(), "{explained:?}");

    // A stale generation answers the idle snapshot: no candidates, no view.
    assert!(fetched_hanji(0).is_empty());
    assert!(dispatch::explain_composition(&tl_config(true), 0, NOW_MS, false).is_none());
}
