//! A pair whose second word was written in the other script is learned under
//! the words' identity — `(display text, canonical TL)` — when the engine
//! applies the handshakes in the composing call (roadmap E2). Moved here from
//! the desktop manager suite, which no longer sees the handshakes.
//! Its own process: the user-data handle is process-wide.
#![cfg(feature = "user-data")]

mod common;

use common::{open_user_data, tl_config};
use protos::engine::{
    composing_request, CandidateMessage, CommitContinuous, CommitScript, ComposingResponse,
    FetchAtPos, NextWordContext, Start,
};
use userdata::{JournalMode, UserDataPaths, UserDataStores};

fn composing(method: composing_request::Method, now_ms: i64) -> ComposingResponse {
    let context = NextWordContext {
        generation: 5,
        now_ms,
    };
    common::composing(tl_config(true), 11, method, Some(context))
}

/// Types `raw`, picks the candidate whose Hanji is `hanji`, writing `script`.
fn pick(raw: &str, hanji: &str, script: CommitScript, now_ms: i64) -> CandidateMessage {
    composing(
        composing_request::Method::Start(Start { text: raw.into() }),
        now_ms,
    );
    let fetched = composing(
        composing_request::Method::FetchAtPos(FetchAtPos::default()),
        now_ms,
    );
    let candidate = fetched
        .continuous
        .expect("continuous candidates")
        .candidates
        .into_iter()
        .find(|candidate| candidate.hanji.as_deref() == Some(hanji))
        .unwrap_or_else(|| panic!("{hanji} for {raw}"));
    composing(
        composing_request::Method::CommitContinuous(CommitContinuous {
            canonical_text: candidate.display_text.clone(),
            association_tl: candidate.canonical_tl.clone(),
            roman: candidate.roman.clone(),
            hanji: candidate.hanji.clone(),
            script: script as i32,
            consumed_bytes: raw.len() as u32,
            syllable_count: candidate.syllable_count,
        }),
        now_ms,
    );
    candidate
}

#[test]
fn a_pair_written_in_two_scripts_is_learned_under_its_identity() {
    if !common::production_lexicon_ready() {
        eprintln!("skipped: production dictionaries absent (run `make dict`)");
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    open_user_data(directory.path());
    let reader = UserDataStores::at(
        UserDataPaths::in_directory(directory.path()),
        JournalMode::Delete,
    );
    reader.open_blocking();

    let now_ms = 2_000_000;
    let tai = pick("tai5", "台", CommitScript::Lead, now_ms);
    // trace: Hanji first (`tl_config(true)`) — Other writes the romanization
    // `gí`, yet the handshake carries the candidate's display text and
    // canonical TL.
    let gi = pick("gi2", "語", CommitScript::Other, now_ms + 500);

    let rows = common::wait_for_association_rows(&reader);
    assert_eq!(rows.len(), 1, "{rows:?}");
    let pair = &rows[0].pair;
    assert_eq!(
        (&pair.previous, &pair.previous_tl, &pair.next, &pair.next_tl),
        (
            &tai.display_text,
            &tai.canonical_tl,
            &gi.display_text,
            &gi.canonical_tl
        )
    );
}
