//! The engine writing its own user data (user-data-engine-roadmap P3c):
//! once the platform opens the stores, `RecordUsage` counts a pick and
//! touches a learned phrase, a final commit of hanji picks is learned, and
//! the bigrams the next-word engine decides are recorded by the engine
//! itself — never handed to the platform.
//! Its own process: the user-data handle is process-wide.
#![cfg(feature = "user-data")]

mod common;

use common::{open_user_data, tl_config};
use std::time::{Duration, Instant};

use protos::engine::{
    composing_request, next_word_request, next_word_response, request, response, user_data_request,
    CommitContinuous, ComposingRequest, ComposingResponse, DecisionInput, EnterContinuous,
    NextWordRequest, RecordUsage, Response, Start, UserDataRequest, WordSelected,
};
use userdata::{JournalMode, UserDataPaths, UserDataStores};

/// The composing requests share one generation, so none resets the buffer.
fn composing(method: composing_request::Method) -> ComposingResponse {
    let response = roundtrip(
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

fn commit_continuous(
    hanji: &str,
    tl: &str,
    consumed_bytes: u32,
    syllable_count: u32,
) -> composing_request::Method {
    composing_request::Method::CommitContinuous(CommitContinuous {
        display_text: hanji.to_owned(),
        canonical_text: hanji.to_owned(),
        association_tl: tl.to_owned(),
        hanji: Some(hanji.to_owned()),
        consumed_bytes,
        syllable_count,
    })
}

fn roundtrip(generation: u64, payload: request::Payload) -> Response {
    common::roundtrip(tl_config(false), generation, payload)
}

fn user_data(method: user_data_request::Method) -> Response {
    roundtrip(
        0,
        request::Payload::UserData(UserDataRequest {
            method: Some(method),
        }),
    )
}

fn record_usage(usage: RecordUsage) -> Response {
    user_data(user_data_request::Method::RecordUsage(usage))
}

/// One commit handed to the next-word engine; answers its decision.
fn word_selected(text: &str, roman: &str, now_ms: i64) {
    let response = roundtrip(
        0,
        request::Payload::Nextword(NextWordRequest {
            method: Some(next_word_request::Method::WordSelected(WordSelected {
                text: text.to_owned(),
                roman: roman.to_owned(),
                require_roman_mode: false,
                trigger_prediction: false,
                input: Some(DecisionInput { now_ms }),
            })),
        }),
    );
    match response.payload {
        Some(response::Payload::Nextword(nextword)) => match nextword.result {
            Some(next_word_response::Result::Decide(_)) => {}
            other => panic!("expected a decision, got {other:?}"),
        },
        other => panic!("expected a nextword payload, got {other:?}"),
    }
}

/// Polls `read` until it holds: the engine's writes are queued, and this
/// process reads them back through its own connection.
fn eventually(mut read: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if read() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn the_engine_writes_what_the_platforms_wrote() {
    let directory = tempfile::tempdir().unwrap();
    let paths = UserDataPaths::in_directory(directory.path());

    // Before the open: the bigram the decision records has nowhere to go.
    word_selected("台", "tâi", 1_000);
    word_selected("灣", "uân", 2_000);
    assert!(!matches!(
        record_usage(RecordUsage {
            display_text: "台灣".to_owned(),
            ..RecordUsage::default()
        })
        .payload,
        Some(response::Payload::UserData(_))
    ));

    // A learned phrase already on disk, to be touched.
    {
        let stores = UserDataStores::at(paths.clone(), JournalMode::Delete);
        stores.open_blocking();
        stores
            .learned_phrases
            .learn_phrase("做進出口", "tsò tsìn-tshut-kháu");
        stores.learned_phrases.all_rows(); // flush the queued write
    }
    open_user_data(&paths);
    let reader = UserDataStores::at(paths.clone(), JournalMode::Delete);
    reader.open_blocking();

    // A pick counts.
    assert!(matches!(
        record_usage(RecordUsage {
            display_text: "台灣".to_owned(),
            canonical_tl: "tâi-uân".to_owned(),
            hanji: Some("台灣".to_owned()),
            frequency_recording_disabled: false,
        })
        .payload,
        Some(response::Payload::UserData(_))
    ));
    assert!(eventually(|| reader
        .frequency
        .rows_for_words(&["台灣".to_owned()])
        .is_some_and(|rows| rows.len() == 1 && rows[0].tl == "tâi-uân")));

    // With the desktop's recording setting off, no count — but a learned
    // phrase picked whole is still touched (learning data, always on).
    record_usage(RecordUsage {
        display_text: "做進出口".to_owned(),
        canonical_tl: "tsò tsìn-tshut-kháu".to_owned(),
        hanji: Some("做進出口".to_owned()),
        frequency_recording_disabled: true,
    });
    assert!(eventually(|| reader
        .learned_phrases
        .all_rows()
        .is_some_and(|rows| rows
            .iter()
            .any(|row| row.learn_count == 2))));
    // A later count landing proves the queue drained past the uncounted one.
    record_usage(RecordUsage {
        display_text: "台北".to_owned(),
        canonical_tl: "tâi-pak".to_owned(),
        ..RecordUsage::default()
    });
    assert!(eventually(|| reader
        .frequency
        .rows_for_words(&["台北".to_owned()])
        .is_some_and(|rows| rows.len() == 1)));
    assert!(reader
        .frequency
        .rows_for_words(&["做進出口".to_owned()])
        .is_some_and(|rows| rows.is_empty()));

    // A final commit of hanji picks is learned into the engine's store (§50).
    composing(composing_request::Method::Start(Start {
        text: "kikhilai".to_owned(),
    }));
    composing(composing_request::Method::EnterContinuous(
        EnterContinuous {},
    ));
    composing(commit_continuous("記", "kì", 2, 1));
    let committed = composing(commit_continuous("起來", "khí-lâi", 6, 2));
    assert!(
        !committed.is_composing,
        "the second pick is the final commit"
    );
    assert!(eventually(|| reader
        .learned_phrases
        .all_rows()
        .is_some_and(|rows| rows
            .iter()
            .any(|row| row.hanzi == "記起來"))));

    // The engine records the bigram it decides on.
    word_selected("食", "tsia̍h", 10_000);
    word_selected("飯", "pn̄g", 11_000);
    assert!(eventually(|| reader.association.all_rows().is_some_and(
        |rows| rows
            .iter()
            .any(|row| row.pair.previous == "食" && row.pair.next == "飯")
    )));
    assert!(
        reader.association.all_rows().is_some_and(|rows| !rows
            .iter()
            .any(|row| row.pair.previous == "台" && row.pair.next == "灣")),
        "the bigram decided before the open is not kept"
    );
}
