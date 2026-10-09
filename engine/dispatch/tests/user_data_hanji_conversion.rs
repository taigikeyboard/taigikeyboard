//! A Hanji conversion learning from the user's picks
//! (`docs/architecture/desktop-tps-hanji-conversion-roadmap.md` H5, H7): with
//! the stores open, a pick from the list of the word before the caret is
//! counted, and the walks after it rank with that count.
//! Its own process: the user-data handle is process-wide.
#![cfg(feature = "user-data")]

mod common;

use std::time::{Duration, Instant};

use common::{open_user_data, production_lexicon_ready};
use protos::engine::{
    composing_request, request, response, AppConfig, CommitAsShown, CommitContinuous,
    CommitOutcome, CommitScript, ComposingRequest, ComposingResponse, FetchAtPos, HanjiConversion,
    Reset, Start,
};

/// The desktop TPS config, asking for the conversion.
fn tps_converting() -> AppConfig {
    AppConfig {
        input_mode: "tps".to_owned(),
        hanji_conversion: Some(HanjiConversion { toggles: None }),
        ..AppConfig::default()
    }
}

/// The composing requests share one generation, so none resets the buffer.
fn composing(method: composing_request::Method) -> ComposingResponse {
    let response = common::roundtrip(
        tps_converting(),
        7,
        request::Payload::Composing(ComposingRequest {
            method: Some(method),
        }),
    );
    match response.payload {
        Some(response::Payload::Composing(composing)) => composing,
        other => panic!("composing answered {other:?}"),
    }
}

fn shown(response: &ComposingResponse) -> String {
    response.preedit.clone().unwrap_or_default().display_text
}

/// A fresh composition of `raw`, as the preedit shows it.
fn compose(raw: &str) -> String {
    composing(composing_request::Method::Reset(Reset {}));
    shown(&composing(composing_request::Method::Start(Start {
        text: raw.to_owned(),
    })))
}

// trace: "ㄒㄧˋ" (sí, closed by its mark) converts to the walker's word; the
// list of the word before the caret holds its homophones over (0, 8). Picking
// another one counts it (the engine records the usage itself), commit as
// shown writes it, and once the queued count lands a new walk shows it — the
// user weight leads the edge's pick.
#[test]
fn a_picked_word_ranks_the_walks_after_it() {
    if !production_lexicon_ready() {
        eprintln!("production artifacts absent — run `make dict`; skipping.");
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    open_user_data(directory.path());

    let raw = "ㄒㄧˋ";
    let neutral = compose(raw);
    let listed = composing(composing_request::Method::FetchAtPos(FetchAtPos::default()))
        .continuous
        .expect("a list")
        .candidates;
    let other = listed
        .into_iter()
        .find(|row| {
            row.consumed_span_end as usize == raw.len()
                && row.hanji.as_deref().is_some_and(|hanji| hanji != neutral)
        })
        .expect("a homophone of the walker's word");
    let picked = other.hanji.clone().unwrap();

    let response = composing(composing_request::Method::CommitContinuous(
        CommitContinuous {
            consumed_bytes: other.consumed_span_end,
            syllable_count: other.syllable_count,
            canonical_text: other.display_text.clone(),
            association_tl: other.canonical_tl.clone(),
            hanji: other.hanji.clone(),
            script: CommitScript::Lead as i32,
            roman: other.roman.clone(),
        },
    ));
    assert_eq!(
        response.commit.as_ref().map(|commit| commit.outcome),
        Some(CommitOutcome::Nailed as i32)
    );
    assert_eq!(shown(&response), picked);
    composing(composing_request::Method::CommitAsShown(CommitAsShown {}));

    let deadline = Instant::now() + Duration::from_secs(5);
    while compose(raw) != picked {
        assert!(
            Instant::now() < deadline,
            "the walk never showed the picked {picked} over {neutral}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
