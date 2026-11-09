//! A committed word's canonical TL reading reaches `user_association.db` and
//! the next prediction unchanged — TL special finals `eng` / `ek` included,
//! which a POJ → TL fold would rewrite to `ing` / `ik`.
//! Its own process: the user-data handle is process-wide.
#![cfg(feature = "user-data")]

mod common;

use common::{open_user_data, tl_config};
use protos::engine::{
    next_word_request, next_word_response, request, response, DecisionInput,
    DictionarySourceToggles, EnginePrediction, NextWordRequest, PredictNext, WordSelected,
};
use std::time::{Duration, Instant};
use userdata::{JournalMode, UserDataPaths, UserDataStores};

fn nextword(method: next_word_request::Method) -> next_word_response::Result {
    let response = common::roundtrip(
        tl_config(false),
        0,
        request::Payload::Nextword(NextWordRequest {
            method: Some(method),
        }),
    );
    let Some(response::Payload::Nextword(nextword)) = response.payload else {
        panic!("expected a nextword payload, got {response:?}");
    };
    nextword.result.expect("nextword result")
}

/// Commits `text` / `roman` at `now_ms`; answers the engine's generation.
fn word_selected(text: &str, roman: &str, now_ms: i64) -> u64 {
    let next_word_response::Result::Decide(decided) =
        nextword(next_word_request::Method::WordSelected(WordSelected {
            text: text.to_owned(),
            roman: roman.to_owned(),
            require_roman_mode: false,
            trigger_prediction: false,
            input: Some(DecisionInput { now_ms }),
            preceding: Vec::new(),
        }))
    else {
        panic!("expected a decision");
    };
    decided.current_generation
}

fn predict(word: &str, roman: &str, generation: u64) -> Vec<EnginePrediction> {
    let next_word_response::Result::Filter(filtered) =
        nextword(next_word_request::Method::PredictNext(PredictNext {
            word: word.to_owned(),
            roman: roman.to_owned(),
            toggles: Some(DictionarySourceToggles {
                kautian: true,
                itaigi: true,
                ..DictionarySourceToggles::default()
            }),
            query_generation: generation,
            now_ms: i64::try_from(userdata::unix_seconds_now() * 1000).expect("epoch-ms fits"),
            limit: 30,
        }))
    else {
        panic!("expected a filter result");
    };
    assert!(!filtered.was_stale);
    filtered.predictions
}

/// Every learned association row as `previous/tl→next/tl ×count`, sorted.
fn association_rows(reader: &UserDataStores) -> Vec<String> {
    let mut rows: Vec<String> = reader
        .association
        .all_rows()
        .unwrap()
        .into_iter()
        .map(|row| {
            let pair = row.pair;
            format!(
                "{}/{}→{}/{} ×{}",
                pair.previous, pair.previous_tl, pair.next, pair.next_tl, row.count
            )
        })
        .collect();
    rows.sort();
    rows
}

// trace: dictionary.csv rows 總統/tsóng-thóng, 蔣經國/tsiúnn-keng-kok,
// 德國簫/tek-kok-siau, 台灣/tâi-uân, 人/lâng. Commits 1 s apart share the
// 10 s association window; the 台灣 pair starts a new one.
#[test]
fn tl_special_finals_are_learned_and_predicted_as_committed() {
    if !common::production_lexicon_ready() {
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    open_user_data(directory.path());
    let start = 1_800_000_000_000_i64;
    word_selected("總統", "tsóng-thóng", start);
    word_selected("蔣經國", "tsiúnn-keng-kok", start + 1_000);
    word_selected("德國簫", "tek-kok-siau", start + 2_000);
    word_selected("總統", "tsóng-thóng", start + 60_000);
    word_selected("蔣經國", "tsiúnn-keng-kok", start + 61_000);
    word_selected("台灣", "tâi-uân", start + 120_000);
    let generation = word_selected("人", "lâng", start + 121_000);

    let reader = UserDataStores::at(
        UserDataPaths::in_directory(directory.path()),
        JournalMode::Delete,
    );
    reader.open_blocking();
    // The repeated 總統 → 蔣經國 commit counts the same row; no `king` row.
    let expected = vec![
        "台灣/tâi-uân→人/lâng ×1",
        "總統/tsóng-thóng→蔣經國/tsiúnn-keng-kok ×2",
        "蔣經國/tsiúnn-keng-kok→德國簫/tek-kok-siau ×1",
    ];
    // The engine's writes queue on its own store's worker, which this
    // reader's `all_rows` barrier does not wait for: poll until they land.
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut rows = association_rows(&reader);
    while rows != expected && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
        rows = association_rows(&reader);
    }
    assert_eq!(rows, expected);

    let after_president = predict("總統", "tsóng-thóng", generation);
    let first = &after_president[0];
    assert_eq!(
        (first.hanji.as_str(), first.tl.as_str()),
        ("蔣經國", "tsiúnn-keng-kok")
    );
    let after_taiwan = predict("台灣", "tâi-uân", generation);
    assert_eq!(
        (after_taiwan[0].hanji.as_str(), after_taiwan[0].tl.as_str()),
        ("人", "lâng")
    );
}
