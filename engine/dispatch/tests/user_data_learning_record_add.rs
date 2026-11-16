//! Learning rows added to the custom dictionary over the wire
//! (learning-records-page-roadmap P7 / P8 / P10): a learned phrase is added
//! and forgotten, a one-syllable phrase is added; a word-frequency row (a
//! dictionary word already) and a phrase with no Hanji are never offered and
//! refused with `FAIL_INVARIANT` and no payload.
//! Its own process: the user-data handle is process-wide.
#![cfg(feature = "user-data")]

mod common;

use common::{open_user_data, user_data};

use protos::engine::{
    response, user_data_request, user_data_response, AddLearningRecordToCustomDictionary,
    CustomDictionaryRefusal, ErrorCode, LearningRecord, LearningRecordKind, ListLearningRecords,
    RecordUsage, Response, SearchCustomEntries,
};
use userdata::{JournalMode, UserDataPaths, UserDataStores};

fn answer(response: Response) -> user_data_response::Result {
    assert_eq!(response.error, ErrorCode::Ok as i32, "{response:?}");
    match response.payload {
        Some(response::Payload::UserData(user_data)) => user_data.result.expect("a result"),
        other => panic!("expected a user-data payload, got {other:?}"),
    }
}

/// Every row of `kind`; the pick is queued, the list waits behind it.
fn listed(kind: LearningRecordKind) -> Vec<LearningRecord> {
    match answer(user_data(user_data_request::Method::ListLearningRecords(
        ListLearningRecords {
            kind: kind as i32,
            limit: 50,
            ..ListLearningRecords::default()
        },
    ))) {
        user_data_response::Result::LearningRecords(records) => records.records,
        other => panic!("expected records, got {other:?}"),
    }
}

fn add_to_custom_dictionary(record: LearningRecord) -> Response {
    user_data(
        user_data_request::Method::AddLearningRecordToCustomDictionary(
            AddLearningRecordToCustomDictionary {
                record: Some(record),
                ..AddLearningRecordToCustomDictionary::default()
            },
        ),
    )
}

fn added(response: Response) {
    match answer(response) {
        user_data_response::Result::LearningRecordAddedToCustomDictionary(added) => {
            assert_eq!(added.refusal(), CustomDictionaryRefusal::None);
        }
        other => panic!("expected an add, got {other:?}"),
    }
}

fn record_usage(word: &str, tl: &str) {
    answer(user_data(user_data_request::Method::RecordUsage(
        RecordUsage {
            display_text: word.into(),
            canonical_tl: tl.into(),
        },
    )));
}

fn custom_words(query: &str) -> Vec<String> {
    match answer(user_data(user_data_request::Method::SearchCustomEntries(
        SearchCustomEntries {
            query: query.into(),
            input_mode: "tl".into(),
            limit: 5,
        },
    ))) {
        user_data_response::Result::CustomEntryMatches(matches) => matches
            .entries
            .into_iter()
            .map(|entry| entry.hanji)
            .collect(),
        other => panic!("expected matches, got {other:?}"),
    }
}

fn refused(record: LearningRecord) {
    assert!(!record.can_add_to_custom_dictionary, "never offered");
    let response = add_to_custom_dictionary(record);
    assert_eq!(response.error, ErrorCode::FailInvariant as i32);
    assert!(response.payload.is_none());
}

#[test]
fn a_learned_phrase_is_added_and_goes_and_a_counted_word_is_refused() {
    let directory = tempfile::tempdir().unwrap();
    // Phrases the keyboard learned from segment-by-segment commits.
    {
        let stores = UserDataStores::at(
            UserDataPaths::in_directory(directory.path()),
            JournalMode::Delete,
        );
        stores.open_blocking();
        for (word, tl) in [("台灣", "tâi-uân"), ("是", "sī"), ("guá sī", "guá sī")] {
            stores.learned_phrases.learn_phrase(word, tl);
        }
        stores.learned_phrases.all_rows(); // flush the queued writes
    }
    open_user_data(directory.path());
    // Picked again: the word is counted.
    record_usage("台灣", "tâi-uân");
    let [counted] = listed(LearningRecordKind::Frequency).try_into().unwrap();
    let phrase = |text: &str| {
        listed(LearningRecordKind::LearnedPhrase)
            .into_iter()
            .find(|row| row.text == text)
            .unwrap()
    };

    // A counted word is a dictionary word already: refused, and still listed.
    refused(counted.clone());
    assert!(custom_words("taiuan").is_empty());
    assert_eq!(listed(LearningRecordKind::Frequency), [counted]);

    // No Hanji: never offered, refused when sent anyway.
    refused(phrase("guá sī"));

    // One syllable: the user's to file, like any custom word.
    assert!(phrase("是").can_add_to_custom_dictionary);
    added(add_to_custom_dictionary(phrase("是")));
    assert_eq!(custom_words("si"), ["是"]);

    // Added, then forgotten: the custom word offers it whole from now on.
    let taiuan = phrase("台灣");
    assert!(taiuan.can_add_to_custom_dictionary);
    added(add_to_custom_dictionary(taiuan));
    assert_eq!(custom_words("taiuan"), ["台灣"]);
    let left: Vec<String> = listed(LearningRecordKind::LearnedPhrase)
        .into_iter()
        .map(|row| row.text)
        .collect();
    assert_eq!(left, ["guá sī"]);
}
