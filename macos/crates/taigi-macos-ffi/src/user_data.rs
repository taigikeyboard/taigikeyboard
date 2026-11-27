//! UserData operations, non-key-path rules, and export/backup file naming
//! served through `desktop_shell.proto` from `taigi-desktop-core`
//! (`engine::user_data`, `settings::custom_dictionary`,
//! `settings::learning_records`, `settings::listing`).
//!
//! Answered in `Shell::serve` before taking `Shell::session` (and without
//! gating on `Configure` or mutating `Shell::settings`) so pre-open callers
//! and file-size / missing-file refusals behave identically to `desktop-core`.
//! Does not take the session lock; the engine's own store locks order
//! concurrent user-data and composing accesses.

use std::path::Path;

use taigi_desktop_core::engine::user_data::{self, UserDataError};
use taigi_desktop_core::settings::custom_dictionary as custom_dict_settings;
use taigi_desktop_core::settings::learning_records::{
    self, BACKUP_OPEN_EXTENSIONS, KINDS, MAX_COUNT, ORDERS,
};
use taigi_desktop_core::settings::listing::PAGE_SIZE;
use taigi_desktop_core::settings::{InputMode, SettingChoice};

use crate::proto::{
    desktop_request, desktop_response, user_data_error, user_data_reply, user_data_request,
    BackupExported, BackupImported, CustomCsvExported, CustomCsvImported, CustomDictionaryEntry,
    CustomDictionaryRefusal, CustomEntries, LearningRecord, LearningRecordDeleted,
    LearningRecordKind, LearningRecordKindRule, LearningRecordOrder, LearningRecordOrderRule,
    LearningRecordSaved, LearningRecords, UserDataAck, UserDataError as ProtoUserDataError,
    UserDataExportFileNameReply, UserDataExportFileNameRequest, UserDataExportKind,
    UserDataNotEmptied, UserDataRefusal, UserDataReply, UserDataRequest, UserDataRulesReply,
};
use crate::runtime::{Refusal, DESKTOP_PLATFORM};

/// Handles `UserData*` requests; `None` when `request` belongs to another
/// handler (`key_rules`, `dictionary_search`, or `Shell::dispatch`).
pub(crate) fn answer(
    request: &desktop_request::Request,
) -> Option<Result<desktop_response::Reply, Refusal>> {
    match request {
        desktop_request::Request::UserData(req) => {
            Some(serve_user_data(req).map(desktop_response::Reply::UserData))
        }
        desktop_request::Request::UserDataRules(_) => {
            Some(Ok(desktop_response::Reply::UserDataRules(rules())))
        }
        desktop_request::Request::UserDataExportFileName(req) => {
            Some(export_file_name(req).map(desktop_response::Reply::UserDataExportFileName))
        }
        _ => None,
    }
}

fn serve_user_data(request: &UserDataRequest) -> Result<UserDataReply, Refusal> {
    use user_data_reply::Result as ReplyResult;
    use user_data_request::Method;

    let Some(method) = &request.method else {
        return Err(Refusal::Missing("user_data.method"));
    };
    match method {
        Method::ListCustomEntries(req) => {
            let input_mode = if req.input_mode.is_empty() {
                InputMode::DEFAULT
            } else {
                InputMode::from_raw(&req.input_mode).ok_or(Refusal::Invalid("input_mode"))?
            };
            let limit = usize::try_from(req.limit).unwrap_or(usize::MAX);
            let offset = usize::try_from(req.offset).unwrap_or(usize::MAX);
            Ok(reply_from(
                user_data::list_custom_entries(&req.filter, limit, offset, input_mode),
                |listed| {
                    ReplyResult::CustomEntries(CustomEntries {
                        entries: listed
                            .entries
                            .into_iter()
                            .map(to_proto_custom_entry)
                            .collect(),
                        total: listed.total,
                        matching_total: listed.matching_total,
                        offset: listed.offset,
                    })
                },
            ))
        }
        Method::SaveCustomEntry(req) => Ok(reply_from(
            user_data::save_custom_entry(&req.id, &req.roman, &req.hanji),
            |saved| ReplyResult::CustomEntrySaved(to_proto_custom_entry(saved)),
        )),
        Method::DeleteCustomEntry(req) => {
            Ok(reply_from(user_data::delete_custom_entry(&req.id), |()| {
                ReplyResult::Ack(UserDataAck {})
            }))
        }
        Method::DeleteAllCustomEntries(_) => {
            Ok(reply_from(user_data::delete_all_custom_entries(), |()| {
                ReplyResult::Ack(UserDataAck {})
            }))
        }
        Method::ExportCustomCsv(_) => Ok(reply_from(user_data::export_custom_csv(), |csv| {
            ReplyResult::CustomCsvExported(CustomCsvExported { csv })
        })),
        Method::ImportCustomCsvFile(req) => {
            if req.path.is_empty() {
                return Err(Refusal::Missing("path"));
            }
            Ok(reply_from(
                user_data::import_custom_csv_file(Path::new(&req.path)),
                |imported| {
                    ReplyResult::CustomCsvImported(CustomCsvImported {
                        imported: imported.imported,
                        skipped: imported.skipped,
                    })
                },
            ))
        }
        Method::ExportBackup(req) => Ok(reply_from(
            user_data::export_backup(DESKTOP_PLATFORM, &req.app_version),
            |backup| ReplyResult::BackupExported(BackupExported { backup }),
        )),
        Method::ImportBackupFile(req) => {
            if req.path.is_empty() {
                return Err(Refusal::Missing("path"));
            }
            Ok(reply_from(
                user_data::import_backup_file(Path::new(&req.path)),
                |imported| {
                    ReplyResult::BackupImported(BackupImported {
                        custom_dictionary: imported.custom_dictionary,
                        frequency: imported.frequency,
                        association: imported.association,
                        learned_phrases: imported.learned_phrases,
                    })
                },
            ))
        }
        Method::ClearLearningRecords(_) => {
            Ok(reply_from(user_data::clear_learning_records(), |()| {
                ReplyResult::Ack(UserDataAck {})
            }))
        }
        Method::ListLearningRecords(req) => {
            let kind = to_engine_kind(
                LearningRecordKind::try_from(req.kind).map_err(|_| Refusal::Invalid("kind"))?,
            );
            let order = to_engine_order(
                LearningRecordOrder::try_from(req.order).map_err(|_| Refusal::Invalid("order"))?,
            );
            let limit = usize::try_from(req.limit).unwrap_or(usize::MAX);
            let offset = usize::try_from(req.offset).unwrap_or(usize::MAX);
            Ok(reply_from(
                user_data::list_learning_records(kind, order, &req.filter, limit, offset),
                |listed| {
                    ReplyResult::LearningRecords(LearningRecords {
                        records: listed
                            .records
                            .into_iter()
                            .map(to_proto_learning_record)
                            .collect(),
                        total: listed.total,
                        matching_total: listed.matching_total,
                        offset: listed.offset,
                    })
                },
            ))
        }
        Method::SetLearningRecordCount(req) => {
            let Some(record) = req.record.as_ref() else {
                return Err(Refusal::Missing("record"));
            };
            let record = from_proto_learning_record(record)?;
            Ok(reply_from(
                user_data::set_learning_record_count(record, req.count),
                |saved| {
                    ReplyResult::LearningRecordSaved(LearningRecordSaved {
                        record: saved.map(to_proto_learning_record),
                    })
                },
            ))
        }
        Method::DeleteLearningRecord(req) => {
            let Some(record) = req.record.as_ref() else {
                return Err(Refusal::Missing("record"));
            };
            let record = from_proto_learning_record(record)?;
            Ok(reply_from(
                user_data::delete_learning_record(record),
                |removed| ReplyResult::LearningRecordDeleted(LearningRecordDeleted { removed }),
            ))
        }
        Method::AddLearningRecordToCustomDictionary(req) => {
            let Some(record) = req.record.as_ref() else {
                return Err(Refusal::Missing("record"));
            };
            let record = from_proto_learning_record(record)?;
            Ok(reply_from(
                user_data::add_learning_record_to_custom_dictionary(record, &req.roman, &req.hanji),
                |()| ReplyResult::Ack(UserDataAck {}),
            ))
        }
    }
}

fn reply_from<T>(
    result: Result<T, UserDataError>,
    ok: impl FnOnce(T) -> user_data_reply::Result,
) -> UserDataReply {
    match result {
        Ok(value) => UserDataReply {
            error: None,
            result: Some(ok(value)),
        },
        Err(error) => UserDataReply {
            error: Some(to_proto_error(error)),
            result: None,
        },
    }
}

fn to_proto_error(error: UserDataError) -> ProtoUserDataError {
    let kind = match error {
        UserDataError::EngineUnavailable(op) => {
            user_data_error::Kind::EngineUnavailableOp(op.to_owned())
        }
        UserDataError::Refused { refusal, detail } => {
            user_data_error::Kind::Refused(UserDataRefusal {
                refusal: to_proto_refusal(refusal) as i32,
                detail,
            })
        }
        UserDataError::NotEmptied(failures) => {
            user_data_error::Kind::NotEmptied(UserDataNotEmptied { failures })
        }
    };
    ProtoUserDataError { kind: Some(kind) }
}

fn to_proto_refusal(refusal: user_data::CustomDictionaryRefusal) -> CustomDictionaryRefusal {
    match refusal {
        user_data::CustomDictionaryRefusal::None => CustomDictionaryRefusal::None,
        user_data::CustomDictionaryRefusal::Full => CustomDictionaryRefusal::Full,
        user_data::CustomDictionaryRefusal::EmptyRoman => CustomDictionaryRefusal::EmptyRoman,
        user_data::CustomDictionaryRefusal::Unsearchable => CustomDictionaryRefusal::Unsearchable,
        user_data::CustomDictionaryRefusal::FileTooLarge => CustomDictionaryRefusal::FileTooLarge,
        user_data::CustomDictionaryRefusal::NotUtf8 => CustomDictionaryRefusal::NotUtf8,
        user_data::CustomDictionaryRefusal::NoUsableRows => CustomDictionaryRefusal::NoUsableRows,
    }
}

fn to_proto_custom_entry(entry: user_data::CustomDictionaryEntry) -> CustomDictionaryEntry {
    CustomDictionaryEntry {
        id: entry.id,
        roman: entry.roman,
        hanji: entry.hanji,
        display_roman: entry.display_roman,
    }
}

fn to_proto_kind(kind: user_data::LearningRecordKind) -> LearningRecordKind {
    match kind {
        user_data::LearningRecordKind::Frequency => LearningRecordKind::Frequency,
        user_data::LearningRecordKind::LearnedPhrase => LearningRecordKind::LearnedPhrase,
    }
}

fn to_engine_kind(kind: LearningRecordKind) -> user_data::LearningRecordKind {
    match kind {
        LearningRecordKind::Frequency => user_data::LearningRecordKind::Frequency,
        LearningRecordKind::LearnedPhrase => user_data::LearningRecordKind::LearnedPhrase,
    }
}

fn to_proto_order(order: user_data::LearningRecordOrder) -> LearningRecordOrder {
    match order {
        user_data::LearningRecordOrder::MostUsed => LearningRecordOrder::MostUsed,
        user_data::LearningRecordOrder::MostRecent => LearningRecordOrder::MostRecent,
    }
}

fn to_engine_order(order: LearningRecordOrder) -> user_data::LearningRecordOrder {
    match order {
        LearningRecordOrder::MostUsed => user_data::LearningRecordOrder::MostUsed,
        LearningRecordOrder::MostRecent => user_data::LearningRecordOrder::MostRecent,
    }
}

fn to_proto_learning_record(record: user_data::LearningRecord) -> LearningRecord {
    let kind = to_proto_kind(record.kind());
    LearningRecord {
        kind: kind as i32,
        id: record.id,
        text: record.text,
        tl: record.tl,
        count: record.count,
        last_used_ms: record.last_used_ms,
        can_add_to_custom_dictionary: record.can_add_to_custom_dictionary,
    }
}

fn from_proto_learning_record(
    record: &LearningRecord,
) -> Result<user_data::LearningRecord, Refusal> {
    let kind =
        LearningRecordKind::try_from(record.kind).map_err(|_| Refusal::Invalid("record.kind"))?;
    Ok(user_data::LearningRecord {
        kind: to_engine_kind(kind) as i32,
        id: record.id,
        text: record.text.clone(),
        tl: record.tl.clone(),
        count: record.count,
        last_used_ms: record.last_used_ms,
        can_add_to_custom_dictionary: record.can_add_to_custom_dictionary,
    })
}

pub(crate) fn rules() -> UserDataRulesReply {
    UserDataRulesReply {
        page_size: u32::try_from(PAGE_SIZE).unwrap_or(u32::MAX),
        custom_csv_max_file_bytes: dispatch::CUSTOM_CSV_MAX_FILE_BYTES,
        learning_record_max_count: MAX_COUNT,
        learning_record_kinds: KINDS
            .into_iter()
            .map(|(kind, label_key)| LearningRecordKindRule {
                kind: to_proto_kind(kind) as i32,
                stored_spelling: kind.raw().to_owned(),
                label_key: label_key.as_str().to_owned(),
                offers_add_to_custom_dictionary: learning_records::offers_add_to_custom_dictionary(
                    kind,
                ),
                count_note_key: learning_records::count_note(kind)
                    .map(|key| key.as_str().to_owned()),
            })
            .collect(),
        learning_record_orders: ORDERS
            .into_iter()
            .map(|order| LearningRecordOrderRule {
                order: to_proto_order(order) as i32,
                stored_spelling: order.raw().to_owned(),
                label_key: learning_records::order_label(order).as_str().to_owned(),
            })
            .collect(),
        backup_open_extensions: BACKUP_OPEN_EXTENSIONS
            .iter()
            .map(|ext| (*ext).to_owned())
            .collect(),
    }
}

pub(crate) fn export_file_name(
    request: &UserDataExportFileNameRequest,
) -> Result<UserDataExportFileNameReply, Refusal> {
    if request.local_date.trim().is_empty() {
        return Err(Refusal::Missing("local_date"));
    }
    let file_name = match UserDataExportKind::try_from(request.kind) {
        Ok(UserDataExportKind::CustomCsv) => {
            custom_dict_settings::export_file_name(&request.local_date)
        }
        Ok(UserDataExportKind::Backup) => learning_records::backup_file_name(&request.local_date),
        Ok(UserDataExportKind::Unspecified) => return Err(Refusal::Missing("kind")),
        Err(_) => return Err(Refusal::Invalid("kind")),
    };
    Ok(UserDataExportFileNameReply { file_name })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proto::{
        AddLearningRecordToCustomDictionaryRequest, DeleteLearningRecordRequest,
        ImportBackupFileRequest, ImportCustomCsvFileRequest, ListLearningRecordsRequest,
        SetLearningRecordCountRequest,
    };

    #[test]
    fn custom_dictionary_refusal_maps_every_variant() {
        let pairs = [
            (
                user_data::CustomDictionaryRefusal::None,
                CustomDictionaryRefusal::None,
            ),
            (
                user_data::CustomDictionaryRefusal::Full,
                CustomDictionaryRefusal::Full,
            ),
            (
                user_data::CustomDictionaryRefusal::EmptyRoman,
                CustomDictionaryRefusal::EmptyRoman,
            ),
            (
                user_data::CustomDictionaryRefusal::Unsearchable,
                CustomDictionaryRefusal::Unsearchable,
            ),
            (
                user_data::CustomDictionaryRefusal::FileTooLarge,
                CustomDictionaryRefusal::FileTooLarge,
            ),
            (
                user_data::CustomDictionaryRefusal::NotUtf8,
                CustomDictionaryRefusal::NotUtf8,
            ),
            (
                user_data::CustomDictionaryRefusal::NoUsableRows,
                CustomDictionaryRefusal::NoUsableRows,
            ),
        ];
        for (engine, proto) in pairs {
            assert_eq!(to_proto_refusal(engine), proto);
        }
    }

    #[test]
    fn user_data_error_maps_all_variants() {
        assert_eq!(
            to_proto_error(UserDataError::EngineUnavailable("customDictionaryList")),
            ProtoUserDataError {
                kind: Some(user_data_error::Kind::EngineUnavailableOp(
                    "customDictionaryList".to_owned()
                )),
            }
        );
        assert_eq!(
            to_proto_error(UserDataError::Refused {
                refusal: user_data::CustomDictionaryRefusal::NotUtf8,
                detail: "not UTF-8".to_owned(),
            }),
            ProtoUserDataError {
                kind: Some(user_data_error::Kind::Refused(UserDataRefusal {
                    refusal: CustomDictionaryRefusal::NotUtf8 as i32,
                    detail: "not UTF-8".to_owned(),
                })),
            }
        );
        assert_eq!(
            to_proto_error(UserDataError::NotEmptied(vec![
                "user_frequency: locked".to_owned()
            ])),
            ProtoUserDataError {
                kind: Some(user_data_error::Kind::NotEmptied(UserDataNotEmptied {
                    failures: vec!["user_frequency: locked".to_owned()],
                })),
            }
        );
    }

    #[test]
    fn custom_entry_and_learning_record_roundtrip_all_fields() {
        let entry = user_data::CustomDictionaryEntry {
            id: "entry-1".to_owned(),
            roman: "taigi".to_owned(),
            hanji: "台語".to_owned(),
            display_roman: "tâi-gí".to_owned(),
        };
        let proto_entry = to_proto_custom_entry(entry);
        assert_eq!(proto_entry.id, "entry-1");
        assert_eq!(proto_entry.roman, "taigi");
        assert_eq!(proto_entry.hanji, "台語");
        assert_eq!(proto_entry.display_roman, "tâi-gí");

        for kind in [
            user_data::LearningRecordKind::Frequency,
            user_data::LearningRecordKind::LearnedPhrase,
        ] {
            let original = user_data::LearningRecord {
                kind: kind as i32,
                id: 42,
                text: "台語".to_owned(),
                tl: "tai5-gi2".to_owned(),
                count: 9,
                last_used_ms: 1_700_000_000_000,
                can_add_to_custom_dictionary: kind == user_data::LearningRecordKind::LearnedPhrase,
            };
            let proto = to_proto_learning_record(original.clone());
            let roundtripped = from_proto_learning_record(&proto).expect("valid record");
            assert_eq!(roundtripped, original);
        }

        for order in [
            user_data::LearningRecordOrder::MostUsed,
            user_data::LearningRecordOrder::MostRecent,
        ] {
            assert_eq!(to_engine_order(to_proto_order(order)), order);
        }
    }

    #[test]
    fn rules_match_desktop_core_constants() {
        let reply = rules();
        assert_eq!(reply.page_size, 10);
        assert_eq!(reply.custom_csv_max_file_bytes, 5 * 1024 * 1024);
        assert_eq!(reply.learning_record_max_count, 1_000_000);
        assert_eq!(reply.backup_open_extensions, vec!["taigi", "json"]);

        assert_eq!(reply.learning_record_kinds.len(), 2);
        assert_eq!(
            reply.learning_record_kinds[0],
            LearningRecordKindRule {
                kind: LearningRecordKind::Frequency as i32,
                stored_spelling: "frequency".to_owned(),
                label_key: "i18n_dictionary_learningRecordsFrequency".to_owned(),
                offers_add_to_custom_dictionary: false,
                count_note_key: Some("i18n_dictionary_learningRecordsCountCapInfo".to_owned()),
            }
        );
        assert_eq!(
            reply.learning_record_kinds[1],
            LearningRecordKindRule {
                kind: LearningRecordKind::LearnedPhrase as i32,
                stored_spelling: "learnedPhrase".to_owned(),
                label_key: "i18n_dictionary_learningRecordsPhrases".to_owned(),
                offers_add_to_custom_dictionary: true,
                count_note_key: None,
            }
        );

        assert_eq!(reply.learning_record_orders.len(), 2);
        assert_eq!(
            reply.learning_record_orders[0],
            LearningRecordOrderRule {
                order: LearningRecordOrder::MostUsed as i32,
                stored_spelling: "mostUsed".to_owned(),
                label_key: "i18n_dictionary_learningRecordsOrderMostUsed".to_owned(),
            }
        );
        assert_eq!(
            reply.learning_record_orders[1],
            LearningRecordOrderRule {
                order: LearningRecordOrder::MostRecent as i32,
                stored_spelling: "mostRecent".to_owned(),
                label_key: "i18n_dictionary_learningRecordsOrderMostRecent".to_owned(),
            }
        );
    }

    #[test]
    fn export_file_name_formats_both_kinds_and_refuses_missing_fields() {
        assert_eq!(
            export_file_name(&UserDataExportFileNameRequest {
                kind: UserDataExportKind::CustomCsv as i32,
                local_date: "2026-11-27".to_owned(),
            }),
            Ok(UserDataExportFileNameReply {
                file_name: "taigi_custom_dictionary_2026-11-27.csv".to_owned(),
            })
        );
        assert_eq!(
            export_file_name(&UserDataExportFileNameRequest {
                kind: UserDataExportKind::Backup as i32,
                local_date: "2026-11-27".to_owned(),
            }),
            Ok(UserDataExportFileNameReply {
                file_name: "taigi_backup_2026-11-27.taigi".to_owned(),
            })
        );
        assert_eq!(
            export_file_name(&UserDataExportFileNameRequest {
                kind: UserDataExportKind::Unspecified as i32,
                local_date: "2026-11-27".to_owned(),
            }),
            Err(Refusal::Missing("kind"))
        );
        assert_eq!(
            export_file_name(&UserDataExportFileNameRequest {
                kind: UserDataExportKind::Backup as i32,
                local_date: "   ".to_owned(),
            }),
            Err(Refusal::Missing("local_date"))
        );
    }

    #[test]
    fn missing_fields_and_unknown_enums_are_refused() {
        assert_eq!(
            serve_user_data(&UserDataRequest { method: None }),
            Err(Refusal::Missing("user_data.method"))
        );
        assert_eq!(
            serve_user_data(&UserDataRequest {
                method: Some(user_data_request::Method::ImportCustomCsvFile(
                    ImportCustomCsvFileRequest {
                        path: String::new(),
                    },
                )),
            }),
            Err(Refusal::Missing("path"))
        );
        assert_eq!(
            serve_user_data(&UserDataRequest {
                method: Some(user_data_request::Method::ImportBackupFile(
                    ImportBackupFileRequest {
                        path: String::new(),
                    },
                )),
            }),
            Err(Refusal::Missing("path"))
        );
        assert_eq!(
            serve_user_data(&UserDataRequest {
                method: Some(user_data_request::Method::ListLearningRecords(
                    ListLearningRecordsRequest {
                        kind: 99,
                        order: LearningRecordOrder::MostUsed as i32,
                        filter: String::new(),
                        limit: 10,
                        offset: 0,
                    },
                )),
            }),
            Err(Refusal::Invalid("kind"))
        );
        assert_eq!(
            serve_user_data(&UserDataRequest {
                method: Some(user_data_request::Method::ListLearningRecords(
                    ListLearningRecordsRequest {
                        kind: LearningRecordKind::Frequency as i32,
                        order: 99,
                        filter: String::new(),
                        limit: 10,
                        offset: 0,
                    },
                )),
            }),
            Err(Refusal::Invalid("order"))
        );
        assert_eq!(
            serve_user_data(&UserDataRequest {
                method: Some(user_data_request::Method::SetLearningRecordCount(
                    SetLearningRecordCountRequest {
                        record: None,
                        count: 1,
                    },
                )),
            }),
            Err(Refusal::Missing("record"))
        );
        assert_eq!(
            serve_user_data(&UserDataRequest {
                method: Some(user_data_request::Method::DeleteLearningRecord(
                    DeleteLearningRecordRequest { record: None },
                )),
            }),
            Err(Refusal::Missing("record"))
        );
        assert_eq!(
            serve_user_data(&UserDataRequest {
                method: Some(
                    user_data_request::Method::AddLearningRecordToCustomDictionary(
                        AddLearningRecordToCustomDictionaryRequest {
                            record: None,
                            roman: "taigi".to_owned(),
                            hanji: "台語".to_owned(),
                        },
                    ),
                ),
            }),
            Err(Refusal::Missing("record"))
        );

        let bad_record = LearningRecord {
            kind: 99,
            ..LearningRecord::default()
        };
        assert_eq!(
            from_proto_learning_record(&bad_record),
            Err(Refusal::Invalid("record.kind"))
        );
    }

    #[test]
    fn csv_import_refuses_missing_and_oversized_files_without_engine() {
        let dir = tempfile::tempdir().expect("temp dir");
        let missing = dir.path().join("missing.csv");
        let reply = serve_user_data(&UserDataRequest {
            method: Some(user_data_request::Method::ImportCustomCsvFile(
                ImportCustomCsvFileRequest {
                    path: missing.display().to_string(),
                },
            )),
        })
        .expect("missing file maps to UserDataReply.error");
        assert!(matches!(
            reply.error,
            Some(ProtoUserDataError {
                kind: Some(user_data_error::Kind::Refused(UserDataRefusal {
                    refusal: 0,
                    ..
                }))
            })
        ));

        let big = dir.path().join("huge.csv");
        let file = std::fs::File::create(&big).expect("create big file");
        file.set_len(dispatch::CUSTOM_CSV_MAX_FILE_BYTES + 1)
            .expect("sparse len");
        let reply = serve_user_data(&UserDataRequest {
            method: Some(user_data_request::Method::ImportCustomCsvFile(
                ImportCustomCsvFileRequest {
                    path: big.display().to_string(),
                },
            )),
        })
        .expect("oversized file maps to UserDataReply.error");
        assert!(matches!(
            reply.error,
            Some(ProtoUserDataError {
                kind: Some(user_data_error::Kind::Refused(UserDataRefusal {
                    refusal,
                    ref detail,
                }))
            }) if refusal == CustomDictionaryRefusal::FileTooLarge as i32
                && detail == "file is larger than 5 MB"
        ));
    }
}
