//! The sentence walk meeting the user's data inside the buffer, not only at
//! its start: with the stores open, a `FetchAtPos` reads the counts of every
//! word an edge of the walk could write, and the custom words and learned
//! phrases filed under any edge — so a pick or an entry counts wherever its
//! word falls in the sentence. Its own process: the user-data handle is
//! process-wide.
#![cfg(feature = "user-data")]

mod common;

use common::{open_user_data, production_lexicon_ready, tl_config};

use protos::engine::{
    composing_request, request, response, AppConfig, CandidateMessage, ComposingRequest,
    FetchAtPos, Reset, Start,
};
use userdata::{CustomDictionaryRow, JournalMode, UserDataPaths, UserDataStores};

const GENERATION: u64 = 3;
const NOW_MS: i64 = 1_800_000_000_000;

fn composing(config: &AppConfig, method: composing_request::Method) -> response::Payload {
    common::roundtrip(
        config.clone(),
        GENERATION,
        request::Payload::Composing(ComposingRequest {
            next_word_context: None,
            method: Some(method),
        }),
    )
    .payload
    .expect("a composing answer")
}

/// The list `config` fetches for a fresh composition of `raw`.
fn listed(config: &AppConfig, raw: &str) -> Vec<CandidateMessage> {
    composing(config, composing_request::Method::Reset(Reset {}));
    composing(
        config,
        composing_request::Method::Start(Start {
            text: raw.to_owned(),
        }),
    );
    match composing(
        config,
        composing_request::Method::FetchAtPos(FetchAtPos {
            now_ms: NOW_MS,
            ..FetchAtPos::default()
        }),
    ) {
        response::Payload::Composing(composing) => {
            composing.continuous.expect("candidates").candidates
        }
        other => panic!("composing answered {other:?}"),
    }
}

/// The whole-buffer sentence's Hanji: the first row past the §34 literal
/// that covers the buffer.
fn sentence(config: &AppConfig, raw: &str) -> Option<String> {
    listed(config, raw)
        .into_iter()
        .skip(1)
        .find(|candidate| candidate.consumed_span_end as usize == raw.len())
        .and_then(|candidate| candidate.hanji)
}

#[test]
fn the_sentence_reads_the_users_data_inside_the_buffer() {
    if !production_lexicon_ready() {
        return;
    }
    let han_lo = AppConfig {
        han_lo_mixed: true,
        ..tl_config(true)
    };
    let neutral_tsiah = sentence(&han_lo, "guatsiah");
    let neutral_uhootsinghu = sentence(&han_lo, "uhootsinghu");
    let neutral_mtsaiiann = sentence(&tl_config(true), "guamtsaiiann");

    let directory = tempfile::tempdir().unwrap();
    let stores = UserDataStores::at(
        UserDataPaths::in_directory(directory.path()),
        JournalMode::Delete,
    );
    stores.open_blocking();
    // `tsia̍h` picked as a romanization, on its own.
    for _ in 0..3 {
        stores.frequency.record("tsia̍h", "tsia̍h");
    }
    // 予 picked over the LKK recommendation `hōo`, on its own.
    for _ in 0..5 {
        stores.frequency.record("予", "hōo");
    }
    // `m̄知影` filed under its own reading, never under `guamtsaiiann`.
    stores
        .custom_dictionary
        .upsert(&CustomDictionaryRow::new("m̄-tsai-iánn", "m̄知影"))
        .unwrap();
    // Flush the queued writes before the engine opens the same files.
    stores.frequency.all_rows();
    drop(stores);
    open_user_data(directory.path());

    // The pick counts inside the sentence (review #1): `tsia̍h` is no
    // candidate of the neutral list for `guatsiah`, yet its count is read.
    assert_ne!(neutral_tsiah.as_deref(), Some("我tsia̍h"));
    assert_eq!(sentence(&han_lo, "guatsiah").as_deref(), Some("我tsia̍h"));
    // … and in the middle of one: 予 beats the LKK `hōo` there too.
    assert_ne!(neutral_uhootsinghu.as_deref(), Some("有予政府"));
    assert_eq!(
        sentence(&han_lo, "uhootsinghu").as_deref(),
        Some("有予政府")
    );
    // The custom word meets its inner edge (review #2) …
    assert_ne!(neutral_mtsaiiann.as_deref(), Some("我m̄知影"));
    assert_eq!(
        sentence(&tl_config(true), "guamtsaiiann").as_deref(),
        Some("我m̄知影")
    );
    // … and is never listed as a word covering the whole buffer.
    assert!(listed(&tl_config(true), "guamtsaiiann")
        .iter()
        .filter(|candidate| candidate.consumed_span_end as usize == "guamtsaiiann".len())
        .all(|candidate| candidate.hanji.as_deref() != Some("m̄知影")));
}
