//! The Learning Records page's requests over the wire
//! (learning-records-page-roadmap P1): a pick the engine counted is listed,
//! its count set, the row deleted; a request the engine refuses answers
//! `FAIL_INVARIANT` with no payload.
//! Its own process: the user-data handle is process-wide.
#![cfg(feature = "user-data")]

mod common;

use common::{open_user_data, user_data};

use protos::engine::{
    response, user_data_request, user_data_response, DeleteLearningRecord, ErrorCode,
    LearningRecord, LearningRecordKind, LearningRecords, ListLearningRecords, RecordUsage,
    Response, SetLearningRecordCount,
};

fn answer(response: Response) -> user_data_response::Result {
    assert_eq!(response.error, ErrorCode::Ok as i32, "{response:?}");
    match response.payload {
        Some(response::Payload::UserData(user_data)) => user_data.result.expect("a result"),
        other => panic!("expected a user-data payload, got {other:?}"),
    }
}

fn list(limit: u32) -> Response {
    user_data(user_data_request::Method::ListLearningRecords(
        ListLearningRecords {
            kind: LearningRecordKind::Frequency as i32,
            limit,
            ..ListLearningRecords::default()
        },
    ))
}

fn listed() -> LearningRecords {
    match answer(list(50)) {
        user_data_response::Result::LearningRecords(records) => records,
        other => panic!("expected records, got {other:?}"),
    }
}

/// The pick is queued; the list waits behind it on the same writer.
fn only_record() -> LearningRecord {
    let mut records = listed().records;
    assert_eq!(records.len(), 1, "{records:?}");
    records.remove(0)
}

#[test]
fn a_counted_pick_is_listed_edited_and_deleted_over_the_wire() {
    let directory = tempfile::tempdir().unwrap();
    open_user_data(directory.path());
    answer(user_data(user_data_request::Method::RecordUsage(
        RecordUsage {
            display_text: "台灣".into(),
            canonical_tl: "tâi-uân".into(),
        },
    )));

    let record = only_record();
    assert_eq!(
        (record.text.as_str(), record.tl.as_str(), record.count),
        ("台灣", "tâi-uân", 1)
    );

    match answer(user_data(
        user_data_request::Method::SetLearningRecordCount(SetLearningRecordCount {
            record: Some(record.clone()),
            count: 25,
        }),
    )) {
        user_data_response::Result::LearningRecordSaved(saved) => {
            assert_eq!(saved.record.expect("the stored row").count, 25);
        }
        other => panic!("expected a save, got {other:?}"),
    }
    assert_eq!(only_record().count, 25);

    // An unpaged list is refused: no payload, the invariant code.
    let refused = list(0);
    assert_eq!(refused.error, ErrorCode::FailInvariant as i32);
    assert!(refused.payload.is_none());

    match answer(user_data(user_data_request::Method::DeleteLearningRecord(
        DeleteLearningRecord {
            record: Some(record),
        },
    ))) {
        user_data_response::Result::LearningRecordDeleted(deleted) => assert!(deleted.removed),
        other => panic!("expected a delete, got {other:?}"),
    }
    assert_eq!(listed().total, 0);
}
