//! Literal rendering must not merge stored tone-boundary identities.
//! Own process because the dispatch user-data handle is process-wide.
#![cfg(feature = "user-data")]

mod common;

use protos::engine::{
    composing_request, AppConfig, CommitContinuous, CommitScript, ComposingResponse, FetchAtPos,
    NextWordContext, Start,
};
use std::time::{Duration, Instant};
use userdata::{JournalMode, UserDataPaths, UserDataStores};

/// One composing request under the next-word envelope at `now_ms`.
fn composing(
    config: &AppConfig,
    method: composing_request::Method,
    now_ms: i64,
) -> ComposingResponse {
    let context = NextWordContext {
        generation: 1,
        now_ms,
    };
    common::composing(config.clone(), 7, method, Some(context))
}

#[test]
fn literal_picks_reuse_existing_keys_and_keep_tone_boundaries_distinct() {
    let cases = [
        // raw, rendered/committed, pre-existing text key, pre-existing TL key
        ("a1i3", "aì", "a1i3", "a1i3"),
        ("ai3", "ài", "ài", "ài"),
        ("a1i1", "ai", "a1i1", "a1i1"),
        ("ai1", "ai", "ai1", "ai"),
        ("a4i3", "aì", "a4i3", "a4i3"),
    ];
    let directory = tempfile::tempdir().unwrap();
    let reader = UserDataStores::at(
        UserDataPaths::in_directory(directory.path()),
        JournalMode::Delete,
    );
    reader.open_blocking();
    for (_, _, text, tl) in cases {
        reader.frequency.record(text, tl);
    }
    // Flush seed writes before opening the engine's connection.
    assert_eq!(reader.frequency.all_rows().unwrap().len(), cases.len());
    common::open_user_data(directory.path());

    // Each pick lands 1 s after the last, inside the association window, so
    // every consecutive pair of picks is learned under both picks' identities.
    let mut now_ms = 1_000_000;
    let mut previous: Option<(&str, &str)> = None;
    let mut expected_pairs = Vec::new();
    for mode in ["tl", "poj"] {
        let config = AppConfig {
            input_mode: mode.into(),
            ..common::tl_config(false)
        };
        for (raw, rendered, text, tl) in cases {
            let preview = composing(
                &config,
                composing_request::Method::Start(Start { text: raw.into() }),
                now_ms,
            );
            assert_eq!(preview.preedit.unwrap().display_text, rendered);
            let fetched = composing(
                &config,
                composing_request::Method::FetchAtPos(FetchAtPos::default()),
                now_ms,
            );
            let candidate = &fetched.continuous.unwrap().candidates[0];
            assert_eq!(candidate.display_text, text, "{mode}: {raw}");
            assert_eq!(candidate.canonical_tl, tl, "{mode}: {raw}");
            assert_eq!(candidate.roman, rendered);
            let committed = composing(
                &config,
                composing_request::Method::CommitContinuous(CommitContinuous {
                    canonical_text: candidate.display_text.clone(),
                    association_tl: candidate.canonical_tl.clone(),
                    roman: candidate.roman.clone(),
                    hanji: None,
                    script: CommitScript::Roman as i32,
                    consumed_bytes: raw.len() as u32,
                    syllable_count: candidate.syllable_count,
                }),
                now_ms,
            );
            let commit = committed.commit.unwrap();
            assert_eq!(commit.document_text, rendered);
            assert!(commit.taught_next_word, "{mode}: {raw}");
            if let Some((previous_text, previous_tl)) = previous {
                expected_pairs.push((previous_text, previous_tl, text, tl));
            }
            previous = Some((text, tl));
            now_ms += 1_000;
        }
    }

    // Each pick is learned under its stable identity, never the rendering.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let rows = reader.association.all_rows().unwrap();
        let learned = |(previous, previous_tl, next, next_tl): &(&str, &str, &str, &str)| {
            rows.iter().any(|row| {
                row.pair.previous == *previous
                    && row.pair.previous_tl == *previous_tl
                    && row.pair.next == *next
                    && row.pair.next_tl == *next_tl
            })
        };
        if expected_pairs.iter().all(learned) {
            // Writes land in order, so the last pair present means every pick
            // landed: no pick may add a row under its rendering.
            let mut distinct = expected_pairs.clone();
            distinct.sort();
            distinct.dedup();
            assert_eq!(rows.len(), distinct.len(), "{rows:?}");
            break;
        }
        assert!(
            Instant::now() < deadline,
            "picks were not learned under their identities: {rows:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }

    // Engine writes are queued; wait until all ten picks reach the old rows.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let rows = reader.frequency.all_rows().unwrap();
        if cases.iter().all(|(_, _, text, tl)| {
            rows.iter()
                .any(|row| row.word == *text && row.tl == *tl && row.count == 3)
        }) {
            assert_eq!(rows.len(), cases.len(), "no forked or merged keys");
            break;
        }
        assert!(
            Instant::now() < deadline,
            "old keys were not reused: {rows:?}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
