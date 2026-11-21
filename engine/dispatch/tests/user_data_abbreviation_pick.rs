//! Whether a pick counts as an abbreviation pick (`user_frequency.abbrev_count`,
//! §46) is decided by the row the list showed, not re-derived from the
//! buffer: `am` lists 阿嬤 `a-má` as a prefix row (`am` starts its toneless
//! reading `ama`), whose abbreviation twin the list drops, so a pick of it
//! is a reading pick and raises the weight that row ranks by.
//! Its own process: the user-data handle is process-wide.
#![cfg(feature = "user-data")]

mod common;

use common::{eventually, open_user_data, production_lexicon_ready, tl_config};
use protos::engine::{
    composing_request, CommitContinuous, CommitOutcome, CommitScript, ComposingResponse,
    FetchAtPos, Start,
};
use userdata::{JournalMode, UserDataPaths, UserDataStores};

/// The composing requests share one generation, so none resets the buffer.
fn composing(method: composing_request::Method) -> ComposingResponse {
    common::composing(tl_config(false), 7, method, None)
}

/// `(count, abbrev_count)` of the `(word, tl)` frequency row.
fn counts(reader: &UserDataStores, word: &str, tl: &str) -> Option<(i64, i64)> {
    reader
        .frequency
        .rows_for_words(&[word.to_owned()])?
        .into_iter()
        .find(|row| row.tl == tl)
        .map(|row| (row.count, row.abbrev_count))
}

// trace (production, `candidate_dump` `am`): 阿嬤 `a-má` end=2 syll=2 — the
// Step 4b prefix row; `derive_custom_search_keys("a-má")` keys tl abbrev
// `am`, so the PR #1057 guess (`is_abbreviation_pick`) filed the pick as an
// abbreviation: abbrev_count 1, reading weight `count - abbrev_count` 0.
#[test]
fn a_pick_of_a_prefix_row_is_counted_as_a_reading_pick() {
    if !production_lexicon_ready() {
        eprintln!("production artifacts absent — run `make dict`; skipping.");
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    open_user_data(directory.path());
    let reader = UserDataStores::at(
        UserDataPaths::in_directory(directory.path()),
        JournalMode::Delete,
    );
    reader.open_blocking();

    fetch_and_pick("am", "阿嬤", "a-má");
    assert!(eventually(|| counts(&reader, "阿嬤", "a-má").is_some()));
    assert_eq!(
        counts(&reader, "阿嬤", "a-má"),
        Some((1, 0)),
        "a reading pick: count rises, abbrev_count does not"
    );

    // Control (production `si`: 所以 `sóo-í` end=2, toneless `sooi`): the
    // list shows it as an abbreviation row, so its pick is one.
    fetch_and_pick("si", "所以", "sóo-í");
    assert!(eventually(|| counts(&reader, "所以", "sóo-í").is_some()));
    assert_eq!(counts(&reader, "所以", "sóo-í"), Some((1, 1)));
}

/// Composes `raw`, fetches its list and picks the whole-buffer row of
/// `(hanji, tl)` from it; asserts the pick finalized.
fn fetch_and_pick(raw: &str, hanji: &str, tl: &str) {
    composing(composing_request::Method::Start(Start {
        text: raw.to_owned(),
    }));
    let listed = composing(composing_request::Method::FetchAtPos(FetchAtPos::default()))
        .continuous
        .expect("a list")
        .candidates;
    let row = listed
        .iter()
        .find(|row| {
            row.hanji.as_deref() == Some(hanji)
                && row.canonical_tl == tl
                && row.consumed_span_end as usize == raw.len()
        })
        .unwrap_or_else(|| panic!("`{raw}` lists {hanji} {tl}: {listed:?}"));
    let committed = composing(composing_request::Method::CommitContinuous(
        CommitContinuous {
            canonical_text: row.display_text.clone(),
            association_tl: row.canonical_tl.clone(),
            hanji: row.hanji.clone(),
            consumed_bytes: row.consumed_span_end,
            syllable_count: row.syllable_count,
            script: CommitScript::Lead as i32,
            roman: row.roman.clone(),
        },
    ));
    assert_eq!(
        committed.commit.map(|commit| commit.outcome),
        Some(CommitOutcome::Finalized as i32)
    );
}
