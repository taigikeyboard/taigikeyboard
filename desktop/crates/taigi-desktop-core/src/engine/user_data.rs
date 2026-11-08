//! User-data slice of the engine bridge: the engine owns the four stores
//! (`docs/architecture/user-data-engine-roadmap.md`), the desktop names the
//! files, and edits the custom dictionary through the same ops the other
//! platforms' settings pages use; the engine counts the picks itself (R5,
//! `engine::commit_continuous`). Answered only by a shell built with
//! `dispatch/user-data` (roadmap U11); anywhere else the engine refuses and
//! these answer `false` (`open`) or `Err(UserDataError::EngineUnavailable)`
//! (the page ops). macOS keeps a Swift twin:
//! `RustEngineBridge+UserData.swift` / `UserDataClient.swift`.

use std::path::Path;

use protos::engine::{
    request, response, user_data_request, user_data_response, AddLearningRecordToCustomDictionary,
    DeleteCustomEntry, DeleteLearningRecord, ExportCustomCsv, ImportCustomCsv, ListCustomEntries,
    ListLearningRecords, OpenUserData, ResetUserData, SaveCustomEntry, SearchCustomEntries,
    SetLearningRecordCount, UserDataJournal, UserDataRequest,
};
pub use protos::engine::{
    CustomCsvImported, CustomDictionaryEntry, CustomDictionaryRefusal, CustomEntries,
    LearningRecord, LearningRecordKind, LearningRecordOrder,
};

use super::bridge::{record_failure, roundtrip};
use crate::platform::DesktopPlatform;
use crate::settings::InputMode;

/// Opens the engine's stores over `directory` — the desktop's one-directory
/// layout, under `platform`'s journal (U3, [`journal`]). Cheap enough for a
/// key path: the engine puts the stores in use at once (a pick reported
/// meanwhile queues behind the open) and finishes opening — the first
/// takeover's re-derivation included — on a thread of its own. `true` once the engine acknowledged
/// the open; it does not promise that every store is usable yet.
pub fn open(directory: &Path, platform: DesktopPlatform) -> bool {
    open_with(directory, journal(platform), true)
}

/// Opens the engine's stores over `directory` and finishes before it
/// returns, under a rollback journal: the web, which has no thread to finish
/// on and whose files sit in SQLite's memory VFS, with no shared memory for
/// WAL. `true` once the engine acknowledged the open.
pub fn open_on_this_thread(directory: &Path) -> bool {
    open_with(directory, UserDataJournal::Delete, false)
}

fn open_with(directory: &Path, journal: UserDataJournal, in_background: bool) -> bool {
    let Some(answer) = user_data(
        user_data_request::Method::Open(OpenUserData {
            directory: directory.display().to_string(),
            journal: journal as i32,
            in_background,
            ..OpenUserData::default()
        }),
        "userDataOpen",
    ) else {
        return false;
    };
    if matches!(answer, user_data_response::Result::Opened(_)) {
        true
    } else {
        record_failure("userDataOpen", "response carried no open result");
        false
    }
}

/// The journal `platform`'s stores have always used: write-ahead logged on
/// Windows and Linux, rollback on macOS (inventory S8).
/// The engine refuses a second open at another journal
/// (`engine/userdata/src/requests.rs`), so a process never mixes the two.
fn journal(platform: DesktopPlatform) -> UserDataJournal {
    match platform {
        DesktopPlatform::Windows | DesktopPlatform::Linux => UserDataJournal::Wal,
        DesktopPlatform::MacOS => UserDataJournal::Delete,
    }
}

// ---- The settings window's pages (roadmap P4; macOS twin: `UserDataClient.swift`) ----

/// Why a page's request did nothing. The description is the alert's
/// diagnostic line, English on purpose (`PageMessage::failure`).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum UserDataError {
    /// The round-trip failed or the engine refused the request; the bridge
    /// logged why (a store's own error is in the engine log, `user-data
    /// store failed`).
    #[error("the engine did not answer {0}")]
    EngineUnavailable(&'static str),
    /// Something the user can be told — a full dictionary, a file that is
    /// not UTF-8 — in the engine's own words.
    #[error("{detail}")]
    Refused {
        refusal: CustomDictionaryRefusal,
        detail: String,
    },
    /// A reset that emptied some stores and not others, one line per store
    /// that failed (`user_frequency: <error>`).
    #[error("{}", .0.join("\n"))]
    NotEmptied(Vec<String>),
}

/// One page of the custom dictionary, newest edit first: `filter` as a
/// substring of the romanization or Hanji, `limit` rows from `offset`
/// (pulled back to the last page that exists).
pub fn list_custom_entries(
    filter: &str,
    limit: usize,
    offset: usize,
) -> Result<CustomEntries, UserDataError> {
    let op = "customDictionaryList";
    match page_request(
        user_data_request::Method::ListCustomEntries(ListCustomEntries {
            filter: filter.to_owned(),
            limit: clamped(limit),
            offset: clamped(offset),
        }),
        op,
    )? {
        user_data_response::Result::CustomEntries(entries) => Ok(entries),
        _ => Err(other_result(op)),
    }
}

/// The page of a user-data list a settings window shows: `page_size` rows
/// from `page` (zero-based), pulled back to the last page that exists when
/// the matches shrank under it (a delete on the last page), so the page
/// asked for is the page shown.
#[derive(Clone, Debug)]
pub struct UserDataPage<Row> {
    pub page: usize,
    pub rows: Vec<Row>,
    /// How many rows `filter` matches — what the pager divides.
    pub match_count: usize,
    /// Every row, whatever the filter — what the store HOLDS.
    pub total_count: usize,
}

impl<Row> UserDataPage<Row> {
    /// The page the engine served: its `offset`, as a page of `page_size`.
    fn served(
        offset: u32,
        page_size: usize,
        rows: Vec<Row>,
        match_count: u32,
        total_count: u32,
    ) -> Self {
        Self {
            page: offset as usize / page_size.max(1),
            rows,
            match_count: match_count as usize,
            total_count: total_count as usize,
        }
    }
}

pub type CustomDictionaryPage = UserDataPage<CustomDictionaryEntry>;
pub type LearningRecordPage = UserDataPage<LearningRecord>;

/// One page as the settings windows page it (`CustomDictionaryPageModel`).
pub fn list_custom_page(
    filter: &str,
    page: usize,
    page_size: usize,
) -> Result<CustomDictionaryPage, UserDataError> {
    let listing = list_custom_entries(filter, page_size, page * page_size)?;
    Ok(CustomDictionaryPage::served(
        listing.offset,
        page_size,
        listing.entries,
        listing.matching_total,
        listing.total,
    ))
}

/// Adds a word (`id` empty) or edits the one with `id`; answers the stored
/// row. A refusal (empty romanization, no search key, a full dictionary)
/// is `UserDataError::Refused`.
pub fn save_custom_entry(
    id: &str,
    roman: &str,
    hanji: &str,
) -> Result<CustomDictionaryEntry, UserDataError> {
    let op = "customDictionarySave";
    match page_request(
        user_data_request::Method::SaveCustomEntry(SaveCustomEntry {
            id: (!id.is_empty()).then(|| id.to_owned()),
            roman: roman.to_owned(),
            hanji: hanji.to_owned(),
        }),
        op,
    )? {
        user_data_response::Result::CustomEntrySaved(saved) => match saved.entry {
            Some(entry) => Ok(entry),
            None => Err(refused(saved.refusal, saved.detail)),
        },
        _ => Err(other_result(op)),
    }
}

/// Removes the row with `id`; a row that was not there is not a failure.
pub fn delete_custom_entry(id: &str) -> Result<(), UserDataError> {
    let op = "customDictionaryDelete";
    match page_request(
        user_data_request::Method::DeleteCustomEntry(DeleteCustomEntry { id: id.to_owned() }),
        op,
    )? {
        user_data_response::Result::CustomEntryDeleted(_) => Ok(()),
        _ => Err(other_result(op)),
    }
}

/// Empties the custom dictionary.
pub fn delete_all_custom_entries() -> Result<(), UserDataError> {
    reset(ResetUserData {
        custom_dictionary: true,
        ..ResetUserData::default()
    })
}

/// Empties the three learning stores (frequency, association, learned
/// phrases); the custom dictionary is left alone. Every store is
/// attempted even when an earlier one fails, and the error names the ones
/// that could not be emptied.
pub fn clear_learning_records() -> Result<(), UserDataError> {
    reset(ResetUserData {
        frequency: true,
        association: true,
        learned_phrases: true,
        ..ResetUserData::default()
    })
}

fn reset(reset: ResetUserData) -> Result<(), UserDataError> {
    let op = "userDataReset";
    match page_request(user_data_request::Method::Reset(reset), op)? {
        user_data_response::Result::Reset(reset) if reset.failures.is_empty() => Ok(()),
        user_data_response::Result::Reset(reset) => Err(UserDataError::NotEmptied(reset.failures)),
        _ => Err(other_result(op)),
    }
}

/// Imports the `roman,hanji` CSV file the user picked: refused before the
/// read when it is too big to be a word list, read whole, then handed to
/// the engine (which skips the rows already stored and stops at the cap).
/// A file that cannot be read or holds no usable rows is
/// `UserDataError::Refused`.
pub fn import_custom_csv_file(path: &Path) -> Result<CustomCsvImported, UserDataError> {
    let size = std::fs::metadata(path)
        .map_err(|error| unreadable(&error))?
        .len();
    // Refused before the read, so an import never loads a stray gigabyte;
    // the engine refuses the bytes again. `CustomDictionaryCSVError`'s words.
    if size > dispatch::CUSTOM_CSV_MAX_FILE_BYTES {
        return Err(refused(
            CustomDictionaryRefusal::FileTooLarge as i32,
            format!(
                "file is larger than {} MB",
                dispatch::CUSTOM_CSV_MAX_FILE_BYTES / (1024 * 1024)
            ),
        ));
    }
    let csv = std::fs::read(path).map_err(|error| unreadable(&error))?;
    let op = "customDictionaryImportCSV";
    match page_request(
        user_data_request::Method::ImportCustomCsv(ImportCustomCsv { csv }),
        op,
    )? {
        user_data_response::Result::CustomCsvImported(imported)
            if imported.refusal == CustomDictionaryRefusal::None as i32 =>
        {
            Ok(imported)
        }
        user_data_response::Result::CustomCsvImported(imported) => {
            Err(refused(imported.refusal, imported.detail))
        }
        _ => Err(other_result(op)),
    }
}

/// The whole dictionary as `roman,hanji` CSV bytes.
pub fn export_custom_csv() -> Result<Vec<u8>, UserDataError> {
    let op = "customDictionaryExportCSV";
    match page_request(
        user_data_request::Method::ExportCustomCsv(ExportCustomCsv {}),
        op,
    )? {
        user_data_response::Result::CustomCsvExported(exported) => Ok(exported.csv),
        _ => Err(other_result(op)),
    }
}

/// The custom entries `query` finds the way the keyboard finds them: by
/// the search key the query derives under `mode`, prefix-matched. Empty
/// when the query derives no key.
pub fn search_custom_entries(
    query: &str,
    mode: InputMode,
    limit: usize,
) -> Result<Vec<CustomDictionaryEntry>, UserDataError> {
    let op = "customDictionarySearch";
    match page_request(
        user_data_request::Method::SearchCustomEntries(SearchCustomEntries {
            query: query.to_owned(),
            input_mode: mode.wire().to_owned(),
            limit: clamped(limit),
        }),
        op,
    )? {
        user_data_response::Result::CustomEntryMatches(matches) => Ok(matches.entries),
        _ => Err(other_result(op)),
    }
}

/// One page of learned rows of `kind` as the settings windows page them:
/// `filter` as a substring of a row's words or readings, `page_size` rows
/// from `page` (zero-based) in `order`.
pub fn list_learning_page(
    kind: LearningRecordKind,
    order: LearningRecordOrder,
    filter: &str,
    page: usize,
    page_size: usize,
) -> Result<LearningRecordPage, UserDataError> {
    let op = "learningRecordsList";
    let page_size = page_size.max(1);
    match page_request(
        user_data_request::Method::ListLearningRecords(ListLearningRecords {
            kind: kind as i32,
            filter: filter.to_owned(),
            limit: clamped(page_size),
            offset: clamped(page * page_size),
            order: order as i32,
        }),
        op,
    )? {
        user_data_response::Result::LearningRecords(listed) => Ok(LearningRecordPage::served(
            listed.offset,
            page_size,
            listed.records,
            listed.matching_total,
            listed.total,
        )),
        _ => Err(other_result(op)),
    }
}

/// Sets `record`'s count and answers the row as stored now — `None` when
/// the row is gone (deleted, evicted, or its id taken by another word).
pub fn set_learning_record_count(
    record: LearningRecord,
    count: i64,
) -> Result<Option<LearningRecord>, UserDataError> {
    let op = "learningRecordSetCount";
    match page_request(
        user_data_request::Method::SetLearningRecordCount(SetLearningRecordCount {
            record: Some(record),
            count,
        }),
        op,
    )? {
        user_data_response::Result::LearningRecordSaved(saved) => Ok(saved.record),
        _ => Err(other_result(op)),
    }
}

/// Forgets `record`; `false` when it was already gone.
pub fn delete_learning_record(record: LearningRecord) -> Result<bool, UserDataError> {
    let op = "learningRecordDelete";
    match page_request(
        user_data_request::Method::DeleteLearningRecord(DeleteLearningRecord {
            record: Some(record),
        }),
        op,
    )? {
        user_data_response::Result::LearningRecordDeleted(deleted) => Ok(deleted.removed),
        _ => Err(other_result(op)),
    }
}

/// Makes `record`'s word a custom word, filed as `roman` / `hanji` (the
/// word as the user edited it; the engine trims both) — a learned phrase is
/// then forgotten, a frequency row stays; a word the dictionary holds
/// already is not added twice. A refusal (no romanization, no search key, a
/// full dictionary) is `UserDataError::Refused` and keeps the row.
pub fn add_learning_record_to_custom_dictionary(
    record: LearningRecord,
    roman: &str,
    hanji: &str,
) -> Result<(), UserDataError> {
    let op = "learningRecordAddToCustomDictionary";
    match page_request(
        user_data_request::Method::AddLearningRecordToCustomDictionary(
            AddLearningRecordToCustomDictionary {
                record: Some(record),
                roman: Some(roman.to_owned()),
                hanji: Some(hanji.to_owned()),
            },
        ),
        op,
    )? {
        user_data_response::Result::LearningRecordAddedToCustomDictionary(added)
            if added.refusal() == CustomDictionaryRefusal::None =>
        {
            Ok(())
        }
        user_data_response::Result::LearningRecordAddedToCustomDictionary(added) => {
            Err(refused(added.refusal, added.detail))
        }
        _ => Err(other_result(op)),
    }
}

fn page_request(
    method: user_data_request::Method,
    op: &'static str,
) -> Result<user_data_response::Result, UserDataError> {
    user_data(method, op).ok_or(UserDataError::EngineUnavailable(op))
}

fn other_result(op: &'static str) -> UserDataError {
    record_failure(op, "response carried a different user-data result");
    UserDataError::EngineUnavailable(op)
}

fn refused(refusal: i32, detail: String) -> UserDataError {
    UserDataError::Refused {
        refusal: CustomDictionaryRefusal::try_from(refusal)
            .unwrap_or(CustomDictionaryRefusal::None),
        detail,
    }
}

/// The engine's former file reader's words (`could not read the file: …`),
/// so the alert reads as before.
fn unreadable(error: &std::io::Error) -> UserDataError {
    refused(
        CustomDictionaryRefusal::None as i32,
        format!("could not read the file: {error}"),
    )
}

fn clamped(count: usize) -> u32 {
    u32::try_from(count).unwrap_or(u32::MAX)
}

fn user_data(method: user_data_request::Method, op: &str) -> Option<user_data_response::Result> {
    let payload = request::Payload::UserData(UserDataRequest {
        method: Some(method),
    });
    match roundtrip(payload, op, 0, None)? {
        response::Payload::UserData(answer) => match answer.result {
            Some(result) => Some(result),
            None => {
                record_failure(op, "response carried no user-data result");
                None
            }
        },
        _ => {
            record_failure(op, "response carried no user-data payload");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_desktop_opens_its_stores_under_its_own_journal() {
        // trace: WAL on Windows and Linux (U3); DELETE on the Mac
        // (inventory S8).
        assert_eq!(journal(DesktopPlatform::Windows), UserDataJournal::Wal);
        assert_eq!(journal(DesktopPlatform::Linux), UserDataJournal::Wal);
        assert_eq!(journal(DesktopPlatform::MacOS), UserDataJournal::Delete);
    }

    /// trace: the import refuses a missing file with the codec's own words
    /// (`could not read the file: …`) before any engine round-trip.
    #[test]
    fn an_import_of_a_missing_file_is_refused_in_the_reader_words() {
        let directory = tempfile::tempdir().unwrap();
        let error = import_custom_csv_file(&directory.path().join("absent.csv")).unwrap_err();
        match error {
            UserDataError::Refused { refusal, detail } => {
                assert_eq!(refusal, CustomDictionaryRefusal::None);
                assert!(detail.starts_with("could not read the file: "), "{detail}");
            }
            other => panic!("expected a read refusal, got {other:?}"),
        }
    }

    /// trace: a file over the limit is refused on its size, unread
    /// (`FileTooLarge`), with the same text the engine gives.
    #[test]
    fn an_import_over_the_size_limit_is_refused_before_the_read() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("huge.csv");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(dispatch::CUSTOM_CSV_MAX_FILE_BYTES + 1)
            .unwrap();
        let error = import_custom_csv_file(&path).unwrap_err();
        assert_eq!(
            error,
            UserDataError::Refused {
                refusal: CustomDictionaryRefusal::FileTooLarge,
                detail: "file is larger than 5 MB".to_owned(),
            }
        );
    }

    /// trace: the error texts are the macOS `UserDataClientError`
    /// descriptions.
    #[test]
    fn errors_read_as_the_alert_lines() {
        assert_eq!(
            UserDataError::EngineUnavailable("customDictionaryList").to_string(),
            "the engine did not answer customDictionaryList"
        );
        assert_eq!(
            UserDataError::NotEmptied(vec![
                "user_frequency: x".into(),
                "learned_phrases: y".into()
            ])
            .to_string(),
            "user_frequency: x\nlearned_phrases: y"
        );
    }
}
