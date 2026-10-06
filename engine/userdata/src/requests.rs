//! The user-data requests after the open — the pages' (list, save, delete,
//! search, CSV and backup import / export, reset) and the key path's
//! `RecordUsage` — answered from the stores [`UserDataHandle`] opened.
//! `engine/dispatch` routes `UserDataRequest` here and maps [`RequestError`]
//! to the wire's error code.

use std::fmt::Display;

use protos::engine::{
    user_data_request, user_data_response, BackupExported, BackupImported, BackupRefusal,
    CustomCsvExported, CustomCsvImported, CustomDictionaryEntry, CustomDictionaryRefusal,
    CustomEntries, CustomEntryDeleted, CustomEntryMatches, CustomEntrySaved, ImportBackup,
    ImportCustomCsv, LearningRecord, LearningRecordAddedToCustomDictionary, LearningRecordDeleted,
    LearningRecordKind, LearningRecordOrder, LearningRecordSaved, LearningRecords,
    ListCustomEntries, ListLearningRecords, RecordUsage, ResetUserData, SaveCustomEntry,
    SearchCustomEntries, UserDataReset,
};

use crate::paging::{last_page_offset, saturated_u32};
use crate::{
    learning_records, BackupError, CustomDictionaryCSV, CustomDictionaryCSVError,
    CustomDictionaryError, CustomDictionaryRow, CustomDictionaryStore, UserDataHandle,
    UserDataStores,
};

/// Why a user-data request did nothing.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum RequestError {
    /// The request itself is wrong: no method, a relative or empty path, a
    /// second open at other paths or journal, a reset of nothing, a reset
    /// before open.
    #[error("{0}")]
    Invalid(&'static str),
    /// A store could not do what was asked — its message, logged.
    #[error("{0}")]
    Store(String),
}

impl UserDataHandle {
    /// A page's request — everything but the open and the key path's
    /// `RecordUsage` — answered from stores that have finished opening.
    pub(crate) fn handle_page(
        stores: &UserDataStores,
        method: &user_data_request::Method,
    ) -> Result<user_data_response::Result, RequestError> {
        use user_data_request::Method;
        use user_data_response::Result as Answer;
        Ok(match method {
            Method::Reset(reset) => Answer::Reset(Self::reset(stores, reset)?),
            Method::ListCustomEntries(list) => {
                Answer::CustomEntries(Self::list_custom_entries(stores, list)?)
            }
            Method::SaveCustomEntry(save) => {
                Answer::CustomEntrySaved(Self::save_custom_entry(stores, save)?)
            }
            Method::DeleteCustomEntry(delete) => {
                let removed = stores
                    .custom_dictionary
                    .delete(&delete.id)
                    .map_err(store_error)?;
                Answer::CustomEntryDeleted(CustomEntryDeleted { removed })
            }
            Method::ImportCustomCsv(import) => {
                Answer::CustomCsvImported(Self::import_custom_csv(stores, import)?)
            }
            Method::ExportCustomCsv(_) => {
                Answer::CustomCsvExported(Self::export_custom_csv(stores)?)
            }
            Method::ExportBackup(export) => {
                let backup = crate::export_backup(
                    stores,
                    &export.platform,
                    &export.app_version,
                    crate::unix_seconds_now(),
                )
                .map_err(store_error)?;
                Answer::BackupExported(BackupExported { backup })
            }
            Method::ImportBackup(import) => {
                Answer::BackupImported(Self::import_backup(stores, import)?)
            }
            Method::SearchCustomEntries(search) => {
                Answer::CustomEntryMatches(Self::search_custom_entries(stores, search)?)
            }
            Method::ListLearningRecords(list) => {
                Answer::LearningRecords(Self::list_learning_records(stores, list)?)
            }
            Method::SetLearningRecordCount(set) => {
                let (kind, record) = named_learning_record(set.record.as_ref())?;
                let record = learning_records::set_count(stores, kind, record, set.count)
                    .map_err(store_error)?;
                Answer::LearningRecordSaved(LearningRecordSaved { record })
            }
            Method::DeleteLearningRecord(delete) => {
                let (kind, record) = named_learning_record(delete.record.as_ref())?;
                let removed =
                    learning_records::delete(stores, kind, record).map_err(store_error)?;
                Answer::LearningRecordDeleted(LearningRecordDeleted { removed })
            }
            Method::AddLearningRecordToCustomDictionary(add) => {
                let (kind, record) = named_learning_record(add.record.as_ref())?;
                Answer::LearningRecordAddedToCustomDictionary(Self::add_to_custom_dictionary(
                    stores, kind, record,
                )?)
            }
            // Answered by `handle` before a page request is looked at.
            Method::Open(_) | Method::RecordUsage(_) => {
                return Err(RequestError::Invalid("not a page request"));
            }
        })
    }

    /// The dictionary search's lookup: the query's key, prefix-matched as
    /// the keyboard matches it. A query that derives no key matches nothing.
    fn search_custom_entries(
        stores: &UserDataStores,
        search: &SearchCustomEntries,
    ) -> Result<CustomEntryMatches, RequestError> {
        let entries = phonetics::api::derive_custom_query_key(&search.query, &search.input_mode)
            .map(|key| {
                stores
                    .custom_dictionary
                    .rows_matching(&key, search.limit as usize)
            })
            .unwrap_or_default();
        Ok(CustomEntryMatches {
            entries: entries.iter().map(custom_dictionary_entry).collect(),
        })
    }

    fn list_custom_entries(
        stores: &UserDataStores,
        list: &ListCustomEntries,
    ) -> Result<CustomEntries, RequestError> {
        let dictionary = &stores.custom_dictionary;
        let total = dictionary.count().map_err(store_error)?;
        let matching_total = if list.filter.trim().is_empty() {
            total
        } else {
            dictionary
                .count_matching(&list.filter)
                .map_err(store_error)?
        };
        // 0 = every match: a platform listing the whole dictionary asks once.
        let limit = match list.limit {
            0 => usize::MAX,
            limit => limit as usize,
        };
        let offset = last_page_offset(matching_total, limit, list.offset as usize);
        let entries = dictionary
            .rows(&list.filter, limit, offset)
            .map_err(store_error)?;
        Ok(CustomEntries {
            entries: entries.iter().map(custom_dictionary_entry).collect(),
            total: saturated_u32(total),
            matching_total: saturated_u32(matching_total),
            offset: saturated_u32(offset),
        })
    }

    /// A page of one learning store. Paged always: a store holds up to
    /// 50 000 rows, which never travel in one answer.
    fn list_learning_records(
        stores: &UserDataStores,
        list: &ListLearningRecords,
    ) -> Result<LearningRecords, RequestError> {
        let kind = learning_record_kind(list.kind)?;
        let order = LearningRecordOrder::try_from(list.order)
            .map_err(|_| RequestError::Invalid("unknown learning record order"))?;
        if list.limit == 0 {
            return Err(RequestError::Invalid("learning records are listed by page"));
        }
        learning_records::list(stores, kind, &list.filter, order, list.limit, list.offset)
            .map_err(store_error)
    }

    /// A new word (no `id`) or an edit. What the user can be told is a
    /// refusal in the answer; only a store failure is an error.
    fn save_custom_entry(
        stores: &UserDataStores,
        save: &SaveCustomEntry,
    ) -> Result<CustomEntrySaved, RequestError> {
        let dictionary = &stores.custom_dictionary;
        let refused = |refusal: CustomDictionaryRefusal, detail: String| CustomEntrySaved {
            refusal: refusal as i32,
            entry: None,
            detail,
        };
        let roman = save.roman.trim();
        if roman.is_empty() {
            return Ok(refused(
                CustomDictionaryRefusal::EmptyRoman,
                "an entry needs a romanization".into(),
            ));
        }
        let hanji = save.hanji.trim();
        let row = match save.id.as_deref() {
            Some(id) if !id.is_empty() => CustomDictionaryRow::with_id(id, roman, hanji),
            _ => CustomDictionaryRow::new(roman, hanji),
        };
        if let Err(error) = dictionary.upsert(&row) {
            return match refusal(&error) {
                Some(refusal) => Ok(refused(refusal, error.to_string())),
                None => Err(store_error(error)),
            };
        }
        // Read back: the store stamps the times (an edit keeps `created_at`).
        let stored = dictionary.row(&row.id).map_err(store_error)?;
        Ok(CustomEntrySaved {
            refusal: CustomDictionaryRefusal::None as i32,
            entry: stored.as_ref().map(custom_dictionary_entry),
            detail: String::new(),
        })
    }

    /// A row's word made a custom word: added unless it is stored already.
    /// A learned phrase is then forgotten — the custom word offers it whole
    /// from now on; a frequency row stays, still weighting its word. Two
    /// files, so no one transaction: the add comes first and a refused or
    /// failed add keeps the row; a forget that fails after it is a store
    /// error, and the retry finds the word stored and forgets the phrase.
    fn add_to_custom_dictionary(
        stores: &UserDataStores,
        kind: LearningRecordKind,
        record: &LearningRecord,
    ) -> Result<LearningRecordAddedToCustomDictionary, RequestError> {
        // Decided again from the row's own text and TL: the request's
        // `can_add_to_custom_dictionary` is the page's copy, not a permission.
        if !learning_records::can_add_to_custom_dictionary(kind, &record.text, &record.tl) {
            return Err(RequestError::Invalid(
                "only a learned phrase or frequency row of two syllables or more with Hanji is added",
            ));
        }
        let row = CustomDictionaryRow::new(record.tl.trim(), record.text.trim());
        if let Err(error) = stores.custom_dictionary.add_unless_stored(&row) {
            return match refusal(&error) {
                Some(refusal) => Ok(LearningRecordAddedToCustomDictionary {
                    refusal: refusal as i32,
                    detail: error.to_string(),
                }),
                None => Err(store_error(error)),
            };
        }
        // `false` — the phrase is gone already (evicted, deleted, its id
        // reused): the word is in the dictionary, which is what was asked.
        if kind == LearningRecordKind::LearnedPhrase {
            learning_records::delete(stores, kind, record).map_err(store_error)?;
        }
        Ok(LearningRecordAddedToCustomDictionary {
            refusal: CustomDictionaryRefusal::None as i32,
            detail: String::new(),
        })
    }

    fn import_custom_csv(
        stores: &UserDataStores,
        import: &ImportCustomCsv,
    ) -> Result<CustomCsvImported, RequestError> {
        let dictionary = &stores.custom_dictionary;
        let refused = |refusal: CustomDictionaryRefusal, detail: String| CustomCsvImported {
            refusal: refusal as i32,
            detail,
            ..CustomCsvImported::default()
        };
        let rows = match CustomDictionaryCSV::decode_bytes(
            &import.csv,
            CustomDictionaryStore::MAX_ENTRIES,
        ) {
            Ok(rows) => rows,
            Err(error) => return Ok(refused(csv_refusal(&error), error.to_string())),
        };
        match dictionary.batch_import(&rows) {
            Ok(result) => Ok(CustomCsvImported {
                refusal: CustomDictionaryRefusal::None as i32,
                imported: u32::try_from(result.imported).unwrap_or(u32::MAX),
                skipped: u32::try_from(result.skipped).unwrap_or(u32::MAX),
                detail: String::new(),
            }),
            Err(error) => match refusal(&error) {
                Some(refusal) => Ok(refused(refusal, error.to_string())),
                None => Err(store_error(error)),
            },
        }
    }

    fn import_backup(
        stores: &UserDataStores,
        import: &ImportBackup,
    ) -> Result<BackupImported, RequestError> {
        let refused = |refusal: BackupRefusal| BackupImported {
            refusal: refusal as i32,
            ..BackupImported::default()
        };
        match crate::import_backup(stores, &import.backup) {
            Ok(merged) => Ok(BackupImported {
                refusal: BackupRefusal::None as i32,
                custom_dictionary: u32::try_from(merged.custom_dictionary).unwrap_or(u32::MAX),
                frequency: u32::try_from(merged.frequency).unwrap_or(u32::MAX),
                association: u32::try_from(merged.association).unwrap_or(u32::MAX),
            }),
            Err(BackupError::Unreadable(_)) => Ok(refused(BackupRefusal::Unreadable)),
            Err(BackupError::UnsupportedVersion(_)) => {
                Ok(refused(BackupRefusal::UnsupportedVersion))
            }
            Err(error @ BackupError::Store(_)) => Err(store_error(error)),
        }
    }

    fn export_custom_csv(stores: &UserDataStores) -> Result<CustomCsvExported, RequestError> {
        let rows = stores.custom_dictionary.all_rows().map_err(store_error)?;
        Ok(CustomCsvExported {
            csv: CustomDictionaryCSV::encode(&rows).into_bytes(),
        })
    }

    /// One commit counted and, for a Hanji pick, a learned phrase touched —
    /// what each platform's candidate / prediction tap handler did itself.
    pub(crate) fn record_usage(&self, usage: &RecordUsage) -> Result<(), RequestError> {
        let stores = self.opened_stores()?;
        if usage.display_text.is_empty() {
            return Err(RequestError::Invalid("usage without a display text"));
        }
        stores.record_usage(
            &usage.display_text,
            &usage.canonical_tl,
            usage.hanji.as_deref(),
        );
        Ok(())
    }

    fn reset(
        stores: &UserDataStores,
        reset: &ResetUserData,
    ) -> Result<UserDataReset, RequestError> {
        if !(reset.frequency
            || reset.association
            || reset.custom_dictionary
            || reset.learned_phrases)
        {
            return Err(RequestError::Invalid("reset selects no store"));
        }
        let mut removed = UserDataReset::default();
        if reset.frequency {
            removed.frequency_removed = emptied(
                "user_frequency",
                stores.frequency.delete_all(),
                &mut removed.failures,
            );
        }
        if reset.association {
            removed.association_removed = emptied(
                "user_association",
                stores.association.delete_all(),
                &mut removed.failures,
            );
        }
        if reset.custom_dictionary {
            let count = emptied(
                "custom_dictionary",
                stores.custom_dictionary.delete_all(),
                &mut removed.failures,
            );
            removed.custom_dictionary_removed = i64::try_from(count).unwrap_or(i64::MAX);
        }
        if reset.learned_phrases {
            removed.learned_phrases_removed = emptied(
                "learned_phrases",
                stores.learned_phrases.delete_all(),
                &mut removed.failures,
            );
        }
        Ok(removed)
    }
}

fn learning_record_kind(raw: i32) -> Result<LearningRecordKind, RequestError> {
    LearningRecordKind::try_from(raw)
        .map_err(|_| RequestError::Invalid("unknown learning record kind"))
}

/// The record a learning-record mutation names, with its kind.
fn named_learning_record(
    record: Option<&LearningRecord>,
) -> Result<(LearningRecordKind, &LearningRecord), RequestError> {
    let record = record.ok_or(RequestError::Invalid("no learning record named"))?;
    Ok((learning_record_kind(record.kind)?, record))
}

fn custom_dictionary_entry(row: &CustomDictionaryRow) -> CustomDictionaryEntry {
    CustomDictionaryEntry {
        id: row.id.clone(),
        roman: row.roman.clone(),
        hanji: row.hanji.clone(),
        created_at: row.created_at.clone(),
        updated_at: row.updated_at.clone(),
    }
}

/// The custom-dictionary write outcomes the user can be told about; `None`
/// for a store failure.
fn refusal(error: &CustomDictionaryError) -> Option<CustomDictionaryRefusal> {
    match error {
        CustomDictionaryError::CapacityReached { .. } => Some(CustomDictionaryRefusal::Full),
        CustomDictionaryError::SearchKeyDerivationFailed { .. } => {
            Some(CustomDictionaryRefusal::Unsearchable)
        }
        CustomDictionaryError::Database(_) => None,
    }
}

/// What the user is told about a CSV the codec refused — every outcome is
/// theirs to hear: the platform hands in bytes, so nothing can fail to read.
fn csv_refusal(error: &CustomDictionaryCSVError) -> CustomDictionaryRefusal {
    match error {
        CustomDictionaryCSVError::FileTooLarge { .. } => CustomDictionaryRefusal::FileTooLarge,
        CustomDictionaryCSVError::NotUtf8 => CustomDictionaryRefusal::NotUtf8,
        CustomDictionaryCSVError::NoUsableRows => CustomDictionaryRefusal::NoUsableRows,
        CustomDictionaryCSVError::TooManyRows { .. } => CustomDictionaryRefusal::Full,
    }
}

/// One store emptied by a reset, or its failure noted for the answer and
/// the log — a store that cannot be emptied is no reason to leave the
/// others full.
fn emptied<T: Default>(
    store: &str,
    result: Result<T, impl Display>,
    failures: &mut Vec<String>,
) -> T {
    result.unwrap_or_else(|error| {
        log::error!("user_data.reset_failed store={store} error={error}");
        failures.push(format!("{store}: {error}"));
        T::default()
    })
}

fn store_error(error: impl Display) -> RequestError {
    RequestError::Store(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handle::tests::{open_request, opened};
    use protos::engine::{
        DeleteCustomEntry, ExportCustomCsv, OpenUserData, UserDataJournal, UserDataRequest,
    };

    #[test]
    fn a_custom_dictionary_error_maps_to_its_refusal() {
        // trace: `refusal` — CapacityReached → Full,
        // SearchKeyDerivationFailed → Unsearchable, Database → None.
        assert_eq!(
            refusal(&CustomDictionaryError::CapacityReached { limit: 1 }),
            Some(CustomDictionaryRefusal::Full)
        );
        assert_eq!(
            refusal(&CustomDictionaryError::SearchKeyDerivationFailed { roman: "x".into() }),
            Some(CustomDictionaryRefusal::Unsearchable)
        );
        assert_eq!(
            refusal(&CustomDictionaryError::Database(
                crate::UserDataDatabaseError::NotOpen("custom_dictionary".into())
            )),
            None
        );
    }

    #[test]
    fn a_csv_error_maps_to_its_refusal() {
        // trace: `csv_refusal` — FileTooLarge → FileTooLarge, NotUtf8 →
        // NotUtf8, NoUsableRows → NoUsableRows, TooManyRows → Full.
        assert_eq!(
            csv_refusal(&CustomDictionaryCSVError::FileTooLarge { limit_bytes: 1 }),
            CustomDictionaryRefusal::FileTooLarge
        );
        assert_eq!(
            csv_refusal(&CustomDictionaryCSVError::NotUtf8),
            CustomDictionaryRefusal::NotUtf8
        );
        assert_eq!(
            csv_refusal(&CustomDictionaryCSVError::NoUsableRows),
            CustomDictionaryRefusal::NoUsableRows
        );
        assert_eq!(
            csv_refusal(&CustomDictionaryCSVError::TooManyRows { limit: 1 }),
            CustomDictionaryRefusal::Full
        );
    }

    #[test]
    fn a_usage_without_a_display_text_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();
        handle.handle(&open_request(directory.path())).unwrap();

        assert_eq!(
            handle
                .handle(&UserDataRequest {
                    method: Some(user_data_request::Method::RecordUsage(RecordUsage {
                        canonical_tl: "tâi-uân".into(),
                        ..RecordUsage::default()
                    })),
                })
                .unwrap_err(),
            RequestError::Invalid("usage without a display text")
        );
    }

    fn reset_request(reset: ResetUserData) -> UserDataRequest {
        UserDataRequest {
            method: Some(user_data_request::Method::Reset(reset)),
        }
    }

    #[test]
    fn reset_empties_only_the_selected_stores() {
        let directory = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();
        handle.handle(&open_request(directory.path())).unwrap();
        let stores = handle.stores().unwrap();
        stores.frequency.record("台", "tâi");
        save(&handle, None, "tâi-uân", "台灣");

        let response = handle
            .handle(&reset_request(ResetUserData {
                frequency: true,
                ..ResetUserData::default()
            }))
            .unwrap();

        match response.result {
            Some(user_data_response::Result::Reset(reset)) => {
                assert_eq!(reset.frequency_removed, 1);
                assert_eq!(reset.custom_dictionary_removed, 0);
                assert!(reset.failures.is_empty());
            }
            other => panic!("expected Reset, got {other:?}"),
        }
        assert_eq!(
            stores.custom_dictionary.count().unwrap(),
            1,
            "not selected, kept"
        );
    }

    #[test]
    fn a_reset_of_nothing_or_before_open_is_refused() {
        let handle = UserDataHandle::new();
        assert_eq!(
            handle
                .handle(&reset_request(ResetUserData {
                    frequency: true,
                    ..ResetUserData::default()
                }))
                .unwrap_err(),
            RequestError::Invalid("user data is not open yet")
        );
        let directory = tempfile::tempdir().unwrap();
        handle.handle(&open_request(directory.path())).unwrap();
        assert_eq!(
            handle
                .handle(&reset_request(ResetUserData::default()))
                .unwrap_err(),
            RequestError::Invalid("reset selects no store")
        );
    }

    fn call(
        handle: &UserDataHandle,
        method: user_data_request::Method,
    ) -> user_data_response::Result {
        handle
            .handle(&UserDataRequest {
                method: Some(method),
            })
            .unwrap()
            .result
            .unwrap()
    }

    fn list(handle: &UserDataHandle, filter: &str) -> CustomEntries {
        match call(
            handle,
            user_data_request::Method::ListCustomEntries(ListCustomEntries {
                filter: filter.into(),
                limit: 50,
                offset: 0,
            }),
        ) {
            user_data_response::Result::CustomEntries(entries) => entries,
            other => panic!("expected entries, got {other:?}"),
        }
    }

    fn save(
        handle: &UserDataHandle,
        id: Option<&str>,
        roman: &str,
        hanji: &str,
    ) -> CustomEntrySaved {
        match call(
            handle,
            user_data_request::Method::SaveCustomEntry(SaveCustomEntry {
                id: id.map(str::to_owned),
                roman: roman.into(),
                hanji: hanji.into(),
            }),
        ) {
            user_data_response::Result::CustomEntrySaved(saved) => saved,
            other => panic!("expected a save, got {other:?}"),
        }
    }

    fn import(handle: &UserDataHandle, csv: &[u8]) -> CustomCsvImported {
        match call(
            handle,
            user_data_request::Method::ImportCustomCsv(ImportCustomCsv { csv: csv.to_vec() }),
        ) {
            user_data_response::Result::CustomCsvImported(imported) => imported,
            other => panic!("expected an import, got {other:?}"),
        }
    }

    #[test]
    fn the_custom_dictionary_page_adds_edits_filters_and_deletes() {
        let directory = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();
        handle.handle(&open_request(directory.path())).unwrap();
        let seeded = list(&handle, "").total;

        let added = save(&handle, None, " tâi-uân ", "台灣");
        assert_eq!(added.refusal(), CustomDictionaryRefusal::None);
        let entry = added.entry.expect("the stored row");
        assert_eq!(entry.roman, "tâi-uân", "trimmed");
        assert!(!entry.created_at.is_empty());

        let edited = save(&handle, Some(&entry.id), "tâi-uân", "臺灣");
        assert_eq!(edited.entry.unwrap().hanji, "臺灣");
        let filtered = list(&handle, "臺");
        assert_eq!(filtered.entries.len(), 1);
        assert_eq!(filtered.matching_total, 1);
        assert_eq!(filtered.total, seeded + 1, "an edit is not a second word");
        // INVARIANT_USER_DATA_LIST_FILTER_RELOAD_SELECTION (§58): the box
        // reaches the engine as typed and the engine trims it.
        let padded = list(&handle, " 臺 ");
        assert_eq!(padded.entries, filtered.entries);
        assert_eq!(padded.matching_total, filtered.matching_total);

        assert_eq!(
            save(&handle, None, "  ", "空").refusal(),
            CustomDictionaryRefusal::EmptyRoman
        );
        match call(
            &handle,
            user_data_request::Method::DeleteCustomEntry(DeleteCustomEntry { id: entry.id }),
        ) {
            user_data_response::Result::CustomEntryDeleted(deleted) => assert!(deleted.removed),
            other => panic!("expected a delete, got {other:?}"),
        }
        assert_eq!(list(&handle, "").total, seeded);
    }

    fn search(handle: &UserDataHandle, query: &str) -> Vec<String> {
        match call(
            handle,
            user_data_request::Method::SearchCustomEntries(SearchCustomEntries {
                query: query.into(),
                input_mode: "tl".into(),
                limit: 50,
            }),
        ) {
            user_data_response::Result::CustomEntryMatches(matches) => matches
                .entries
                .into_iter()
                .map(|entry| entry.hanji)
                .collect(),
            other => panic!("expected matches, got {other:?}"),
        }
    }

    #[test]
    fn a_malformed_learning_records_request_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();
        handle.handle(&open_request(directory.path())).unwrap();
        let refused = |method: user_data_request::Method| {
            let answer = handle.handle(&UserDataRequest {
                method: Some(method),
            });
            assert!(
                matches!(answer, Err(RequestError::Invalid(_))),
                "{answer:?}"
            );
        };
        let list = |kind: i32, limit: u32, order: i32| {
            user_data_request::Method::ListLearningRecords(ListLearningRecords {
                kind,
                limit,
                order,
                ..ListLearningRecords::default()
            })
        };
        refused(list(0, 0, 0)); // unpaged
        refused(list(9, 10, 0)); // no such kind
        refused(list(0, 10, 9)); // no such order
        refused(user_data_request::Method::SetLearningRecordCount(
            protos::engine::SetLearningRecordCount::default(),
        ));
        refused(user_data_request::Method::DeleteLearningRecord(
            protos::engine::DeleteLearningRecord {
                record: Some(LearningRecord {
                    kind: 9,
                    ..LearningRecord::default()
                }),
            },
        ));
    }

    fn learning_records(handle: &UserDataHandle, kind: LearningRecordKind) -> Vec<LearningRecord> {
        match call(
            handle,
            user_data_request::Method::ListLearningRecords(ListLearningRecords {
                kind: kind as i32,
                limit: 50,
                ..ListLearningRecords::default()
            }),
        ) {
            user_data_response::Result::LearningRecords(page) => page.records,
            other => panic!("expected learning records, got {other:?}"),
        }
    }

    fn learned_phrases(handle: &UserDataHandle) -> Vec<LearningRecord> {
        learning_records(handle, LearningRecordKind::LearnedPhrase)
    }

    fn frequency_rows(handle: &UserDataHandle) -> Vec<LearningRecord> {
        learning_records(handle, LearningRecordKind::Frequency)
    }

    fn add_to_custom_dictionary(
        handle: &UserDataHandle,
        record: LearningRecord,
    ) -> Result<LearningRecordAddedToCustomDictionary, RequestError> {
        let request = UserDataRequest {
            method: Some(
                user_data_request::Method::AddLearningRecordToCustomDictionary(
                    protos::engine::AddLearningRecordToCustomDictionary {
                        record: Some(record),
                    },
                ),
            ),
        };
        handle.handle(&request).map(|answer| match answer.result {
            Some(user_data_response::Result::LearningRecordAddedToCustomDictionary(added)) => added,
            other => panic!("expected an add, got {other:?}"),
        })
    }

    /// An open handle whose one learned phrase is 食飯 / tsia̍h-pn̄g.
    fn handle_with_a_learned_phrase(directory: &tempfile::TempDir) -> UserDataHandle {
        let handle = UserDataHandle::new();
        handle.handle(&open_request(directory.path())).unwrap();
        let stores = handle.stores().unwrap();
        stores.learned_phrases.learn_phrase("食飯", "tsia̍h-pn̄g");
        stores.learned_phrases.database.wait_for_queued_writes();
        handle
    }

    /// An open handle whose frequency store counted each `(word, tl)` once.
    fn handle_with_frequency_rows(
        directory: &tempfile::TempDir,
        rows: &[(&str, &str)],
    ) -> UserDataHandle {
        let handle = UserDataHandle::new();
        handle.handle(&open_request(directory.path())).unwrap();
        let stores = handle.stores().unwrap();
        for (word, tl) in rows {
            stores.frequency.record(word, tl);
        }
        stores.frequency.database.wait_for_queued_writes();
        handle
    }

    #[test]
    fn an_added_phrase_is_a_custom_word_and_no_longer_learned() {
        let directory = tempfile::tempdir().unwrap();
        let handle = handle_with_a_learned_phrase(&directory);
        let seeded = list(&handle, "").total;
        let [phrase] = learned_phrases(&handle).try_into().unwrap();
        assert!(phrase.can_add_to_custom_dictionary);

        let added = add_to_custom_dictionary(&handle, phrase.clone()).unwrap();
        assert_eq!(added.refusal(), CustomDictionaryRefusal::None);
        assert!(added.detail.is_empty());
        assert_eq!(search(&handle, "tsiahpng"), ["食飯"]);
        assert_eq!(list(&handle, "").total, seeded + 1);
        assert!(learned_phrases(&handle).is_empty());

        // The page still listed it: the word is stored, the phrase already
        // gone — not a second custom word.
        let again = add_to_custom_dictionary(&handle, phrase).unwrap();
        assert_eq!(again.refusal(), CustomDictionaryRefusal::None);
        assert_eq!(list(&handle, "").total, seeded + 1);
    }

    #[test]
    fn a_phrase_whose_word_is_stored_is_forgotten_without_a_second_custom_word() {
        let directory = tempfile::tempdir().unwrap();
        let handle = handle_with_a_learned_phrase(&directory);
        // trace: canonical_tl_form("chia̍h-pn̄g", Tl) = "tsia̍h-pn̄g" — the POJ
        // spelling of the phrase's reading.
        save(&handle, None, "chia̍h-pn̄g", "食飯");
        let seeded = list(&handle, "").total;
        let [phrase] = learned_phrases(&handle).try_into().unwrap();

        let added = add_to_custom_dictionary(&handle, phrase).unwrap();
        assert_eq!(added.refusal(), CustomDictionaryRefusal::None);
        assert_eq!(list(&handle, "").total, seeded);
        assert!(learned_phrases(&handle).is_empty());
    }

    #[test]
    fn a_forget_that_fails_after_the_add_is_an_error_and_the_retry_finishes_it() {
        let directory = tempfile::tempdir().unwrap();
        let handle = handle_with_a_learned_phrase(&directory);
        let seeded = list(&handle, "").total;
        let [phrase] = learned_phrases(&handle).try_into().unwrap();
        let phrases = &handle.stores().unwrap().learned_phrases.database;
        let set_blocked = |statement: &'static str| {
            phrases
                .perform::<_, crate::UserDataDatabaseError>(move |connection| {
                    Ok(connection.execute_batch(statement)?)
                })
                .unwrap();
        };
        set_blocked(
            "CREATE TRIGGER block_delete BEFORE DELETE ON learned_phrases \
             BEGIN SELECT RAISE(ABORT, 'blocked'); END;",
        );

        assert!(matches!(
            add_to_custom_dictionary(&handle, phrase.clone()),
            Err(RequestError::Store(_))
        ));
        assert_eq!(list(&handle, "").total, seeded + 1, "added first");
        assert_eq!(learned_phrases(&handle), std::slice::from_ref(&phrase));

        set_blocked("DROP TRIGGER block_delete;");
        let retried = add_to_custom_dictionary(&handle, phrase).unwrap();
        assert_eq!(retried.refusal(), CustomDictionaryRefusal::None);
        assert_eq!(list(&handle, "").total, seeded + 1, "not added twice");
        assert!(learned_phrases(&handle).is_empty());
    }

    #[test]
    fn an_added_frequency_row_is_a_custom_word_and_keeps_its_count_and_time() {
        let directory = tempfile::tempdir().unwrap();
        let handle = handle_with_frequency_rows(&directory, &[("食飯", "tsia̍h-pn̄g")]);
        let seeded = list(&handle, "").total;
        let [row] = frequency_rows(&handle).try_into().unwrap();
        assert!(row.can_add_to_custom_dictionary);

        let added = add_to_custom_dictionary(&handle, row.clone()).unwrap();
        assert_eq!(added.refusal(), CustomDictionaryRefusal::None);
        assert_eq!(search(&handle, "tsiahpng"), ["食飯"]);
        assert_eq!(list(&handle, "").total, seeded + 1);
        // Same id, word, count and last-used time: the row still weights 食飯.
        assert_eq!(frequency_rows(&handle), std::slice::from_ref(&row));

        let again = add_to_custom_dictionary(&handle, row.clone()).unwrap();
        assert_eq!(again.refusal(), CustomDictionaryRefusal::None);
        assert_eq!(list(&handle, "").total, seeded + 1, "not added twice");
        assert_eq!(frequency_rows(&handle), [row]);
    }

    #[test]
    fn only_a_hanji_row_of_two_syllables_or_more_is_offered_and_added() {
        let directory = tempfile::tempdir().unwrap();
        // trace: tl_syllables splits on `-` / ` ` and drops the empty piece
        // of `--`: "sī" → 1, "gín--á" → 2 ("gín", "á"), "guá sī" → 2.
        let handle = handle_with_frequency_rows(
            &directory,
            &[
                ("是", "sī"),
                ("guá sī", "guá sī"),
                ("囡仔", "gín--á"),
                ("a好", "a-hó"),
            ],
        );
        let seeded = list(&handle, "").total;
        let offered = |text: &str| {
            frequency_rows(&handle)
                .into_iter()
                .find(|row| row.text == text)
                .unwrap()
        };
        assert!(!offered("是").can_add_to_custom_dictionary, "one syllable");
        assert!(!offered("guá sī").can_add_to_custom_dictionary, "no Hanji");
        assert!(
            offered("囡仔").can_add_to_custom_dictionary,
            "khinsiann `--`"
        );
        assert!(offered("a好").can_add_to_custom_dictionary, "mixed text");

        let refusal = RequestError::Invalid(
            "only a learned phrase or frequency row of two syllables or more with Hanji is added",
        );
        for text in ["是", "guá sī"] {
            // The page's copy of the flag is not a permission.
            let forged = LearningRecord {
                can_add_to_custom_dictionary: true,
                ..offered(text)
            };
            assert_eq!(
                add_to_custom_dictionary(&handle, forged).unwrap_err(),
                refusal
            );
        }
        let association = LearningRecord {
            kind: LearningRecordKind::Association as i32,
            ..offered("囡仔")
        };
        assert_eq!(
            add_to_custom_dictionary(&handle, association).unwrap_err(),
            refusal
        );
        assert_eq!(list(&handle, "").total, seeded);
        assert_eq!(frequency_rows(&handle).len(), 4);
    }

    #[test]
    fn a_set_count_answer_carries_the_flag() {
        let directory = tempfile::tempdir().unwrap();
        let handle = handle_with_frequency_rows(&directory, &[("食飯", "tsia̍h-pn̄g")]);
        let [row] = frequency_rows(&handle).try_into().unwrap();
        match call(
            &handle,
            user_data_request::Method::SetLearningRecordCount(
                protos::engine::SetLearningRecordCount {
                    record: Some(row),
                    count: 7,
                },
            ),
        ) {
            user_data_response::Result::LearningRecordSaved(saved) => {
                let saved = saved.record.unwrap();
                assert_eq!(saved.count, 7);
                assert!(saved.can_add_to_custom_dictionary);
            }
            other => panic!("expected a saved record, got {other:?}"),
        }
    }

    #[test]
    fn a_dictionary_search_matches_by_the_typed_key_not_by_substring() {
        let directory = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();
        handle.handle(&open_request(directory.path())).unwrap();
        save(&handle, None, "tâi-uân", "台灣");

        assert_eq!(search(&handle, "taiuan"), ["台灣"]);
        assert_eq!(search(&handle, "tai"), ["台灣"], "a prefix of the key");
        assert!(search(&handle, "uan").is_empty(), "not a substring match");
        assert!(search(&handle, "").is_empty(), "no key, no matches");
    }

    #[test]
    fn a_page_past_the_end_answers_the_last_page_that_exists() {
        let directory = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();
        handle.handle(&open_request(directory.path())).unwrap();
        // trace: three added = 3 matches; pages of 2 start at 0, 2.
        save(&handle, None, "tâi-uân", "台灣");
        save(&handle, None, "gâu-tsá", "𠢕早");
        save(&handle, None, "tsia̍h-pá--buē", "食飽未");

        let answer = match call(
            &handle,
            user_data_request::Method::ListCustomEntries(ListCustomEntries {
                filter: String::new(),
                limit: 2,
                offset: 10,
            }),
        ) {
            user_data_response::Result::CustomEntries(entries) => entries,
            other => panic!("expected entries, got {other:?}"),
        };
        assert_eq!(answer.offset, 2);
        assert_eq!(answer.entries.len(), 1);
        assert_eq!(answer.matching_total, 3);

        let everything = match call(
            &handle,
            user_data_request::Method::ListCustomEntries(ListCustomEntries {
                filter: String::new(),
                limit: 0,
                offset: 0,
            }),
        ) {
            user_data_response::Result::CustomEntries(entries) => entries,
            other => panic!("expected entries, got {other:?}"),
        };
        assert_eq!(everything.entries.len(), 3, "limit 0 lists every match");
    }

    #[test]
    fn a_directory_names_the_same_files_as_the_four_paths() {
        let directory = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();
        handle.handle(&open_request(directory.path())).unwrap();

        let by_directory = UserDataRequest {
            method: Some(user_data_request::Method::Open(OpenUserData {
                directory: directory.path().display().to_string(),
                journal: UserDataJournal::Delete as i32,
                ..OpenUserData::default()
            })),
        };
        assert!(
            handle.handle(&by_directory).is_ok(),
            "a repeat at the same files is answered, not refused as other paths"
        );
        assert!(
            save(&handle, None, "  ", "空").detail == "an entry needs a romanization",
            "a refusal carries its alert line"
        );
    }

    #[test]
    fn a_csv_round_trips_and_an_unusable_file_is_refused_with_its_reason() {
        let directory = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();
        handle.handle(&open_request(directory.path())).unwrap();

        let imported = import(
            &handle,
            "tâi-uân,台灣\n\"say \"\"hi\"\"\",講,好\ntâi-uân,台灣\n".as_bytes(),
        );
        assert_eq!(imported.refusal(), CustomDictionaryRefusal::None);
        assert_eq!(imported.imported, 2, "the file's own duplicate is skipped");
        let exported = match call(
            &handle,
            user_data_request::Method::ExportCustomCsv(ExportCustomCsv {}),
        ) {
            user_data_response::Result::CustomCsvExported(exported) => exported.csv,
            other => panic!("expected an export, got {other:?}"),
        };
        let again = import(&handle, &exported);
        assert_eq!(again.imported, 0, "everything exported is already there");

        assert_eq!(
            import(&handle, &[0xff, 0xfe, 0x00]).refusal(),
            CustomDictionaryRefusal::NotUtf8
        );
        let unusable = import(&handle, b"just one column\n");
        assert_eq!(unusable.refusal(), CustomDictionaryRefusal::NoUsableRows);
        assert_eq!(
            unusable.detail, "no usable rows in the file",
            "the alert's line"
        );
        let huge = vec![b'a'; 5 * 1024 * 1024 + 1];
        assert_eq!(
            import(&handle, &huge).refusal(),
            CustomDictionaryRefusal::FileTooLarge
        );
    }

    #[test]
    fn a_backup_exported_through_the_op_restores_through_it() {
        use protos::engine::ExportBackup;

        let source_directory = tempfile::tempdir().unwrap();
        let source = UserDataHandle::new();
        source
            .handle(&open_request(source_directory.path()))
            .unwrap();
        save(&source, None, "tâi-uân", "台灣");
        let backup = match call(
            &source,
            user_data_request::Method::ExportBackup(ExportBackup {
                platform: "linux".into(),
                app_version: "3.6.10".into(),
            }),
        ) {
            user_data_response::Result::BackupExported(exported) => exported.backup,
            other => panic!("expected a backup, got {other:?}"),
        };

        let target_directory = tempfile::tempdir().unwrap();
        let target = UserDataHandle::new();
        target
            .handle(&open_request(target_directory.path()))
            .unwrap();
        let restore = |bytes: Vec<u8>| match call(
            &target,
            user_data_request::Method::ImportBackup(ImportBackup { backup: bytes }),
        ) {
            user_data_response::Result::BackupImported(imported) => imported,
            other => panic!("expected an import, got {other:?}"),
        };
        let imported = restore(backup);
        assert_eq!(imported.refusal(), BackupRefusal::None);
        assert_eq!(imported.custom_dictionary, 1);
        assert_eq!(list(&target, "台灣").matching_total, 1);
        assert_eq!(restore(b"{".to_vec()).refusal(), BackupRefusal::Unreadable);
        assert_eq!(
            restore(br#"{"version": 0}"#.to_vec()).refusal(),
            BackupRefusal::UnsupportedVersion
        );
    }

    #[test]
    fn a_background_open_answers_at_once_and_loses_nothing_reported_meanwhile() {
        let directory = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();
        let mut request = open_request(directory.path());
        if let Some(user_data_request::Method::Open(open)) = request.method.as_mut() {
            open.in_background = true;
        }

        handle.handle(&request).unwrap();
        // trace: the stores are published before the answer, so this pick
        // queues behind the open on the frequency store's worker.
        call(
            &handle,
            user_data_request::Method::RecordUsage(RecordUsage {
                display_text: "台灣".into(),
                canonical_tl: "tâi-uân".into(),
                ..RecordUsage::default()
            }),
        );

        // A page request waits for the open to finish.
        assert_eq!(
            list(&handle, "").total,
            0,
            "an empty dictionary, read once open"
        );
        let stores = handle.stores().unwrap();
        let rows = stores.frequency.all_rows().unwrap_or_default();
        assert_eq!(rows.len(), 1, "counted once the open landed");
        // The background finish runs once however the next open arrives.
        opened(handle.handle(&open_request(directory.path())).unwrap());
        assert!(handle.stores().unwrap().custom_dictionary.is_ready());
    }
}
