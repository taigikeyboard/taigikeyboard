//! The engine writing its own user data (user-data-engine-roadmap P3c):
//! once the platform opens the stores, `RecordUsage` counts a pick and
//! touches a learned phrase, a final commit of hanji picks is learned, and
//! the bigrams the next-word engine decides are recorded by the engine
//! itself — never handed to the platform.
//! Its own process: the user-data handle is process-wide.
#![cfg(feature = "user-data")]

mod common;

use common::{open_user_data, tl_config, user_data};
use std::time::{Duration, Instant};

use protos::engine::{
    composing_request, next_word_request, next_word_response, request, response, user_data_request,
    CommitContinuous, CommitOutcome, CommitScript, ComposingRequest, ComposingResponse,
    DecisionInput, NextWordRequest, RecordUsage, Response, Start, WordSelected,
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
        script: CommitScript::Roman as i32,
        roman: hanji.to_owned(),
        canonical_text: hanji.to_owned(),
        association_tl: tl.to_owned(),
        hanji: Some(hanji.to_owned()),
        consumed_bytes,
        syllable_count,
    })
}

/// An R5 pick: the engine resolves its document text and counts it itself.
fn commit_resolved(
    roman: &str,
    hanji: Option<&str>,
    tl: &str,
    consumed_bytes: u32,
) -> composing_request::Method {
    composing_request::Method::CommitContinuous(CommitContinuous {
        canonical_text: hanji.unwrap_or(roman).to_owned(),
        association_tl: tl.to_owned(),
        hanji: hanji.map(str::to_owned),
        consumed_bytes,
        syllable_count: 1,
        script: CommitScript::Lead as i32,
        roman: roman.to_owned(),
    })
}

/// Composes `raw` and picks each of `picks` in turn; asserts the last one
/// finalized the composition.
fn compose_resolved(raw: &str, picks: &[composing_request::Method]) {
    composing(composing_request::Method::Start(Start {
        text: raw.to_owned(),
    }));
    let mut last = None;
    for pick in picks {
        last = composing(pick.clone()).commit;
    }
    assert_eq!(
        last.map(|commit| commit.outcome),
        Some(CommitOutcome::Finalized as i32)
    );
}

/// The frequency row of `word`, as `(tl, count)`.
fn frequency_row(reader: &UserDataStores, word: &str) -> Option<(String, i64)> {
    reader
        .frequency
        .rows_for_words(&[word.to_owned()])?
        .into_iter()
        .next()
        .map(|row| (row.tl, row.count))
}

fn learn_count(reader: &UserDataStores, hanji: &str) -> Option<i64> {
    reader
        .learned_phrases
        .all_rows()?
        .into_iter()
        .find(|row| row.hanji == hanji)
        .map(|row| row.learn_count)
}

fn roundtrip(generation: u64, payload: request::Payload) -> Response {
    common::roundtrip(tl_config(false), generation, payload)
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
                preceding: Vec::new(),
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
    // An engine-resolved pick has nowhere to be counted either.
    compose_resolved("tsa", &[commit_resolved("tsá", Some("早"), "tsá", 3)]);

    // A learned phrase already on disk, to be touched.
    {
        let stores = UserDataStores::at(paths.clone(), JournalMode::Delete);
        stores.open_blocking();
        stores
            .learned_phrases
            .learn_phrase("做進出口", "tsò tsìn-tshut-kháu");
        stores.learned_phrases.all_rows(); // flush the queued write
    }
    open_user_data(directory.path());
    let reader = UserDataStores::at(paths.clone(), JournalMode::Delete);
    reader.open_blocking();

    // A pick counts.
    assert!(matches!(
        record_usage(RecordUsage {
            display_text: "台灣".to_owned(),
            canonical_tl: "tâi-uân".to_owned(),
        })
        .payload,
        Some(response::Payload::UserData(_))
    ));
    assert!(eventually(|| reader
        .frequency
        .rows_for_words(&["台灣".to_owned()])
        .is_some_and(|rows| rows.len() == 1 && rows[0].tl == "tâi-uân")));

    // A platform pick of a learned phrase counts it but does not touch it
    // (§50): the touch belongs to the commits the engine resolves itself.
    record_usage(RecordUsage {
        display_text: "做進出口".to_owned(),
        canonical_tl: "tsò tsìn-tshut-kháu".to_owned(),
    });
    // A later count landing proves the queue drained past the pick.
    record_usage(RecordUsage {
        display_text: "台北".to_owned(),
        canonical_tl: "tâi-pak".to_owned(),
    });
    assert!(eventually(|| reader
        .frequency
        .rows_for_words(&["台北".to_owned()])
        .is_some_and(|rows| rows.len() == 1)));
    assert!(
        reader
            .frequency
            .rows_for_words(&["做進出口".to_owned()])
            .is_some_and(|rows| rows.len() == 1),
        "every pick counts"
    );
    assert!(
        reader
            .learned_phrases
            .all_rows()
            .is_some_and(|rows| rows.iter().all(|row| row.learn_count == 1)),
        "a platform pick names no Hanji, so no phrase is touched"
    );

    // A final commit of hanji picks is learned into the engine's store (§50).
    composing(composing_request::Method::Start(Start {
        text: "kikhilai".to_owned(),
    }));
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
            .any(|row| row.hanji == "記起來"))));

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

    engine_resolved_picks_are_counted_by_the_engine(&reader);
}

/// R5: a pick whose document text the engine resolved is counted by the
/// engine — once per pick, under the `(display, canonical TL)` pair, with a
/// learned phrase touched only by a Hanji pick.
fn engine_resolved_picks_are_counted_by_the_engine(reader: &UserDataStores) {
    // trace: 食 (`tsiah`, 5 bytes) nails, 飯 (`png`, 3 bytes) finalizes.
    compose_resolved(
        "tsiahpng",
        &[
            commit_resolved("tsia̍h", Some("食"), "tsia̍h", 5),
            commit_resolved("pn̄g", Some("飯"), "pn̄g", 3),
        ],
    );
    assert!(eventually(|| frequency_row(reader, "飯").is_some()));
    assert_eq!(
        frequency_row(reader, "食"),
        Some(("tsia̍h".to_owned(), 1)),
        "the nail counts once"
    );
    assert_eq!(frequency_row(reader, "飯"), Some(("pn̄g".to_owned(), 1)));

    // The phrase learned earlier (`做進出口`, touched once by `RecordUsage`
    // above): a Hanji-less pick of its reading counts the romanization and
    // touches nothing; a pick of the phrase itself touches it.
    let touched_before = learn_count(reader, "做進出口").expect("seeded phrase");
    let reading = "tsò tsìn-tshut-kháu";
    compose_resolved(
        "tsotsintshutkhau",
        &[commit_resolved(reading, None, reading, 16)],
    );
    compose_resolved(
        "tsotsintshutkhau",
        &[commit_resolved(reading, Some("做進出口"), reading, 16)],
    );
    assert!(eventually(
        || learn_count(reader, "做進出口") == Some(touched_before + 1)
    ));
    assert_eq!(
        frequency_row(reader, reading),
        Some((reading.to_owned(), 1))
    );

    // A final count landing proves the frequency queue drained past every
    // pick above: the pick made before the open was never counted, and the
    // learned-phrase picks (記 / 起來) were, once each.
    compose_resolved("tsiah", &[commit_resolved("tsia̍h", Some("食"), "tsia̍h", 5)]);
    assert!(eventually(
        || frequency_row(reader, "食") == Some(("tsia̍h".to_owned(), 2))
    ));
    assert_eq!(frequency_row(reader, "早"), None);
    assert_eq!(frequency_row(reader, "記"), Some(("kì".to_owned(), 1)));
    assert_eq!(
        frequency_row(reader, "起來"),
        Some(("khí-lâi".to_owned(), 1))
    );
    assert_eq!(learn_count(reader, "做進出口"), Some(touched_before + 1));
}
