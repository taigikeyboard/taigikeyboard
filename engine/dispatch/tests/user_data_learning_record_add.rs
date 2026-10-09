//! Learning rows added to the custom dictionary over the wire
//! (learning-records-page-roadmap P7 / P8 / P10): a frequency row stays, a
//! learned phrase is forgotten, a one-syllable word is added, a word with no
//! Hanji is refused with `FAIL_INVARIANT` and no payload.
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

#[test]
fn a_counted_word_is_added_and_stays_and_a_learned_phrase_is_added_and_goes() {
    let directory = tempfile::tempdir().unwrap();
    // A phrase the keyboard learned from a segment-by-segment commit.
    {
        let stores = UserDataStores::at(
            UserDataPaths::in_directory(directory.path()),
            JournalMode::Delete,
        );
        stores.open_blocking();
        stores.learned_phrases.learn_phrase("台灣", "tâi-uân");
        stores.learned_phrases.all_rows(); // flush the queued write
    }
    open_user_data(directory.path());
    // Picked again: the word is counted and the phrase touched.
    record_usage("台灣", "tâi-uân");
    record_usage("是", "sī");
    record_usage("guá", "guá");
    let words = listed(LearningRecordKind::Frequency);
    let word = |text: &str| words.iter().find(|row| row.text == text).unwrap().clone();
    let [phrase] = listed(LearningRecordKind::LearnedPhrase)
        .try_into()
        .unwrap();

    // No Hanji: never offered, refused when sent anyway.
    assert!(!word("guá").can_add_to_custom_dictionary);
    let refused = add_to_custom_dictionary(word("guá"));
    assert_eq!(refused.error, ErrorCode::FailInvariant as i32);
    assert!(refused.payload.is_none());

    // One syllable: the user's to file, like any custom word.
    assert!(word("是").can_add_to_custom_dictionary);
    added(add_to_custom_dictionary(word("是")));
    assert_eq!(custom_words("si"), ["是"]);

    // A frequency row keeps weighting its word: added, and still listed.
    assert!(word("台灣").can_add_to_custom_dictionary);
    added(add_to_custom_dictionary(word("台灣")));
    assert_eq!(custom_words("taiuan"), ["台灣"]);
    assert_eq!(listed(LearningRecordKind::Frequency), words);

    // The word is stored already: the phrase is forgotten, not added twice.
    assert!(phrase.can_add_to_custom_dictionary);
    added(add_to_custom_dictionary(phrase));
    assert!(listed(LearningRecordKind::LearnedPhrase).is_empty());
    assert_eq!(custom_words("taiuan"), ["台灣"]);
}
