//! Literal rendering must not merge stored tone-boundary identities.
//! Own process because the dispatch user-data handle is process-wide.
#![cfg(feature = "user-data")]

mod common;

use protos::engine::{
    composing_request, effect, request, response, AppConfig, CommitContinuous, CommitScript,
    ComposingRequest, ComposingResponse, FetchAtPos, Start,
};
use std::time::{Duration, Instant};
use userdata::{JournalMode, UserDataPaths, UserDataStores};

fn composing(config: &AppConfig, method: composing_request::Method) -> ComposingResponse {
    let response = common::roundtrip(
        config.clone(),
        7,
        request::Payload::Composing(ComposingRequest {
            next_word_context: None,
            method: Some(method),
        }),
    );
    match response.payload {
        Some(response::Payload::Composing(composing)) => composing,
        other => panic!("composing answered {other:?}"),
    }
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

    for mode in ["tl", "poj"] {
        let config = AppConfig {
            input_mode: mode.into(),
            ..common::tl_config(false)
        };
        for (raw, rendered, text, tl) in cases {
            let preview = composing(
                &config,
                composing_request::Method::Start(Start { text: raw.into() }),
            );
            assert_eq!(preview.preedit.unwrap().display_text, rendered);
            let fetched = composing(
                &config,
                composing_request::Method::FetchAtPos(FetchAtPos::default()),
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
            );
            assert_eq!(committed.commit.unwrap().document_text, rendered);
            assert!(
                committed.effect.iter().any(|effect| matches!(
                    &effect.kind,
                    Some(effect::Kind::NextWordWordSelected(word))
                        if word.text == text && word.roman == tl
                )),
                "{mode}: {raw} must learn its stable identity"
            );
        }
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
