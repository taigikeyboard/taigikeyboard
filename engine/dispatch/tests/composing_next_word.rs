//! A composing request carrying the shell's next-word envelope gets its
//! next-word handshakes applied in the same call (roadmap E2): each handshake
//! is replaced in place by the engine's decision, and the pairs it decides are
//! recorded. Without the envelope the handshakes reach the shell unchanged.
//! Its own process: the user-data handle is process-wide.
#![cfg(feature = "user-data")]

mod common;

use common::{open_user_data, tl_config};
use protos::engine::{
    composing_request, effect, next_word_effect, CommitRaw, ComposingResponse, NextWordContext,
    Reset, Start,
};
use userdata::{JournalMode, UserDataPaths, UserDataStores};

const COMPOSING_GENERATION: u64 = 7;
const NEXT_WORD_GENERATION: u64 = 3;

fn composing(
    method: composing_request::Method,
    next_word_context: Option<NextWordContext>,
) -> ComposingResponse {
    common::composing(
        tl_config(false),
        COMPOSING_GENERATION,
        method,
        next_word_context,
    )
}

fn context(now_ms: i64) -> Option<NextWordContext> {
    Some(NextWordContext {
        generation: NEXT_WORD_GENERATION,
        now_ms,
    })
}

/// Types `raw` and commits it with Enter.
fn type_and_commit(raw: &str, next_word_context: Option<NextWordContext>) -> ComposingResponse {
    composing(
        composing_request::Method::Start(Start { text: raw.into() }),
        next_word_context,
    );
    composing(
        composing_request::Method::CommitRaw(CommitRaw {}),
        next_word_context,
    )
}

fn kind_names(response: &ComposingResponse) -> Vec<&'static str> {
    response
        .effect
        .iter()
        .map(|effect| match effect.kind.as_ref() {
            Some(effect::Kind::CommitTextReplacingPreedit(_)) => "commit",
            Some(effect::Kind::ClearPreeditWithoutCommit(_)) => "clear_preedit",
            Some(effect::Kind::ClearCandidates(_)) => "clear_candidates",
            Some(effect::Kind::ResetCandidateContext(_)) => "reset_candidate_context",
            Some(effect::Kind::NextWordWordSelected(_)) => "word_selected",
            Some(effect::Kind::NextWordClearForNewComposing(_)) => "clear_for_new_composing",
            Some(effect::Kind::NextWordDecided(_)) => "decided",
            other => panic!("unexpected effect {other:?}"),
        })
        .collect()
}

fn association_count(reader: &UserDataStores) -> usize {
    reader
        .association
        .all_rows()
        .expect("association rows")
        .len()
}

#[test]
fn handshakes_are_applied_in_place_only_with_the_envelope() {
    let directory = tempfile::tempdir().unwrap();
    open_user_data(directory.path());
    let reader = UserDataStores::at(
        UserDataPaths::in_directory(directory.path()),
        JournalMode::Delete,
    );
    reader.open_blocking();

    // Control: without the envelope the shell gets the handshake and the
    // engine learns nothing from two quick commits.
    let first = type_and_commit("tsit", None);
    assert_eq!(
        kind_names(&first),
        [
            "commit",
            "clear_candidates",
            "reset_candidate_context",
            "word_selected"
        ]
    );
    assert!(first.commit.as_ref().unwrap().taught_next_word);
    type_and_commit("e", None);
    assert_eq!(association_count(&reader), 0);

    // With the envelope: the handshake's place holds the decision, which asks
    // for predictions after the word (trigger_prediction on Enter).
    let now_ms = 1_000_000;
    let first = type_and_commit("tsit", context(now_ms));
    assert_eq!(
        kind_names(&first),
        [
            "commit",
            "clear_candidates",
            "reset_candidate_context",
            "decided"
        ]
    );
    let Some(effect::Kind::NextWordDecided(decided)) = first.effect[3].kind.as_ref() else {
        unreachable!()
    };
    assert!(decided.effects.iter().any(|effect| matches!(
        effect.kind,
        Some(next_word_effect::Kind::QueryPredictions(_))
    )));
    // trace: second commit 1 s later is inside the 10 s window
    // (decide.rs should_record_association) → pair tsit → e recorded.
    type_and_commit("e", context(now_ms + 1_000));
    let rows = common::wait_for_association_rows(&reader);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].pair.previous, "tsit");
    assert_eq!(rows[0].pair.next, "e");

    // An abort: the trio's handshake becomes a decision, in third place.
    composing(
        composing_request::Method::Start(Start { text: "a".into() }),
        context(now_ms + 2_000),
    );
    let aborted = composing(
        composing_request::Method::Reset(Reset {}),
        context(now_ms + 2_000),
    );
    assert_eq!(
        kind_names(&aborted),
        ["clear_preedit", "clear_candidates", "decided"]
    );

    // The context's clock decides the window: the same pair committed 30 s
    // apart (decide.rs `ASSOCIATION_TIMEOUT_MS` = 10 s) is not counted again.
    type_and_commit("tsit", context(now_ms + 30_000));
    type_and_commit("e", context(now_ms + 60_000));
    let rows = reader.association.all_rows().unwrap();
    assert_eq!((rows.len(), rows[0].count), (1, 1), "{rows:?}");
}
