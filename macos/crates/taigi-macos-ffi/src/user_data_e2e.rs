use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::thread;

use prost::Message;
use protos::engine::ErrorCode;

use crate::desktop_request_bytes;
use crate::proto::{
    desktop_request, desktop_response, effect, setting_value, user_data_error, user_data_reply,
    user_data_request, ActivateRequest, AddLearningRecordToCustomDictionaryRequest, CancelRequest,
    ClearLearningRecordsRequest, ConfigureRequest, CustomDictionaryRefusal,
    DeleteAllCustomEntriesRequest, DeleteCustomEntryRequest, DeleteLearningRecordRequest,
    DesktopRequest, DesktopResponse, DictionarySearchRequest, Effect, ExportBackupRequest,
    ExportCustomCsvRequest, ImportBackupFileRequest, ImportCustomCsvFileRequest,
    InsertSymbolRequest, KeyEvent, KeyRequest, LearningRecordKind, LearningRecordOrder,
    ListCustomEntriesRequest, ListLearningRecordsRequest, PanelState, PrepareRequest,
    SaveCustomEntryRequest, SessionReply, SetLearningRecordCountRequest, SettingEntry,
    SettingValue, SettingsSnapshot, SymbolSwapRequest, UserDataExportFileNameRequest,
    UserDataExportKind, UserDataReply, UserDataRequest, UserDataRulesRequest,
};

fn repository_dictionaries() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/dictionaries")
}

fn send_bytes(bytes: &[u8]) -> DesktopResponse {
    let raw = desktop_request_bytes(bytes);
    DesktopResponse::decode(raw.as_slice()).expect("every seam answer decodes")
}

fn send_envelope(
    request: desktop_request::Request,
    settings: Option<SettingsSnapshot>,
) -> DesktopResponse {
    let bytes = DesktopRequest {
        request: Some(request),
        settings,
    }
    .encode_to_vec();
    send_bytes(&bytes)
}

fn send_ok(
    request: desktop_request::Request,
    settings: Option<SettingsSnapshot>,
) -> desktop_response::Reply {
    let response = send_envelope(request, settings);
    assert_eq!(response.error, ErrorCode::Ok as i32, "{response:?}");
    response.reply.expect("OK response carries reply")
}

fn user_data_reply(method: user_data_request::Method) -> UserDataReply {
    match send_ok(
        desktop_request::Request::UserData(UserDataRequest {
            method: Some(method),
        }),
        None,
    ) {
        desktop_response::Reply::UserData(reply) => reply,
        other => panic!("expected UserData reply, got {other:?}"),
    }
}

fn user_data_ok(method: user_data_request::Method) -> user_data_reply::Result {
    let reply = user_data_reply(method);
    assert!(reply.error.is_none(), "unexpected error: {:?}", reply.error);
    reply.result.expect("success UserData reply carries result")
}

fn text_setting(name: &str, value: &str) -> SettingEntry {
    SettingEntry {
        name: name.to_owned(),
        value: Some(SettingValue {
            value: Some(setting_value::Value::Text(value.to_owned())),
        }),
    }
}

fn bool_setting(name: &str, value: bool) -> SettingEntry {
    SettingEntry {
        name: name.to_owned(),
        value: Some(SettingValue {
            value: Some(setting_value::Value::Boolean(value)),
        }),
    }
}

const CARRIAGE_RETURN: u32 = 0xD;

fn key_event(characters: &str) -> KeyEvent {
    KeyEvent {
        key_code: None,
        characters: Some(characters.to_owned()),
        characters_ignoring_modifiers: Some(characters.to_owned()),
        modifier_flags: 0,
        special_key: None,
    }
}

fn return_event() -> KeyEvent {
    KeyEvent {
        key_code: None,
        characters: Some("\r".to_owned()),
        characters_ignoring_modifiers: Some("\r".to_owned()),
        modifier_flags: 0,
        special_key: Some(CARRIAGE_RETURN),
    }
}

fn send_session(
    request: desktop_request::Request,
    settings: Option<SettingsSnapshot>,
) -> SessionReply {
    match send_ok(request, settings) {
        desktop_response::Reply::Session(reply) => reply,
        other => panic!("expected Session reply, got {other:?}"),
    }
}

struct ComposingTracker {
    token: u64,
    is_list_on_screen: bool,
    selected_index: Option<u32>,
    slot_indices: HashMap<u32, u32>,
    swap_available: bool,
}

impl ComposingTracker {
    fn new(token: u64) -> Self {
        Self {
            token,
            is_list_on_screen: false,
            selected_index: None,
            slot_indices: HashMap::new(),
            swap_available: false,
        }
    }

    fn panel(&self) -> PanelState {
        PanelState {
            is_list_on_screen: self.is_list_on_screen,
            selected_index: self.selected_index,
            slot_indices: self.slot_indices.clone(),
            swap_available: self.swap_available,
        }
    }

    fn apply_effects(&mut self, effects: &[Effect]) {
        for item in effects {
            match item.effect.as_ref() {
                Some(effect::Effect::CandidatesChanged(changed)) => {
                    self.is_list_on_screen = true;
                    self.selected_index = changed.selected_index.or(if changed.cells.is_empty() {
                        None
                    } else {
                        Some(0)
                    });
                    self.slot_indices.clear();
                    for idx in 0..changed.cells.len().min(9) {
                        let slot = u32::try_from(idx).expect("small slot");
                        self.slot_indices.insert(slot, slot);
                    }
                    self.swap_available = false;
                }
                Some(effect::Effect::CandidatesClosed(_)) => {
                    self.is_list_on_screen = false;
                    self.selected_index = None;
                    self.slot_indices.clear();
                }
                Some(effect::Effect::ArmSwap(_)) => {
                    self.swap_available = true;
                }
                Some(
                    effect::Effect::InsertText(_)
                    | effect::Effect::SwapPrecedingSpace(_)
                    | effect::Effect::SetMarkedText(_)
                    | effect::Effect::ClearMarkedText(_),
                ) => {
                    self.swap_available = false;
                }
                _ => {}
            }
        }
    }

    fn send_key(&mut self, characters: &str, settings: Option<SettingsSnapshot>) -> SessionReply {
        self.send_event(key_event(characters), settings)
    }

    fn send_return(&mut self, settings: Option<SettingsSnapshot>) -> SessionReply {
        self.send_event(return_event(), settings)
    }

    fn send_event(&mut self, event: KeyEvent, settings: Option<SettingsSnapshot>) -> SessionReply {
        let reply = send_session(
            desktop_request::Request::Key(KeyRequest {
                token: self.token,
                event: Some(event),
                panel: Some(self.panel()),
            }),
            settings,
        );
        self.apply_effects(&reply.effects);
        reply
    }
}

#[test]
fn user_data_and_symbol_swap_end_to_end() {
    if std::env::var_os("TAIGI_MACOS_FFI_E2E_CHILD").is_none() {
        let exe = std::env::current_exe().expect("test executable path");
        let output = std::process::Command::new(exe)
            .env("TAIGI_MACOS_FFI_E2E_CHILD", "1")
            .args([
                "--exact",
                "user_data_e2e::user_data_and_symbol_swap_end_to_end",
                "--nocapture",
            ])
            .output()
            .expect("spawn isolated E2E child process");
        assert!(
            output.status.success(),
            "isolated E2E child failed (status {:?})\n--- stdout ---\n{}\n--- stderr ---\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
        return;
    }

    let data_dir = tempfile::tempdir().expect("temp data dir");
    let scratch_dir = tempfile::tempdir().expect("temp scratch dir");

    phase_pre_configure_and_ffi_contract();

    // Configure + Prepare once for this isolated test binary.
    assert_eq!(
        send_envelope(
            desktop_request::Request::Configure(ConfigureRequest {
                data_directory: Some(data_dir.path().display().to_string()),
                dictionaries_directory: Some(repository_dictionaries().display().to_string()),
                dictionary_stamp: 1,
                system_locale: "zh-Hant-TW".to_owned(),
            }),
            None,
        )
        .error,
        ErrorCode::Ok as i32
    );
    match send_ok(desktop_request::Request::Prepare(PrepareRequest {}), None) {
        desktop_response::Reply::Prepare(prepare) => {
            let stats = prepare.lexicon.expect("installed lexicon stats");
            assert!(stats.dictionary_record_count > 0);
        }
        other => panic!("expected Prepare reply, got {other:?}"),
    }

    phase_sqlite_delete_journal(data_dir.path());
    phase_custom_dictionary_lifecycle(scratch_dir.path());
    phase_learning_records_and_backup(scratch_dir.path());
    phase_symbol_swap();
    phase_concurrent_user_data_and_composing();
}

fn phase_pre_configure_and_ffi_contract() {
    // Rules and export filenames work before Configure.
    match send_ok(
        desktop_request::Request::UserDataRules(UserDataRulesRequest {}),
        None,
    ) {
        desktop_response::Reply::UserDataRules(rules) => {
            assert_eq!(rules.page_size, 10);
            assert_eq!(rules.custom_csv_max_file_bytes, 5 * 1024 * 1024);
            assert_eq!(rules.learning_record_max_count, 1_000_000);
            assert_eq!(rules.backup_open_extensions, vec!["taigi", "json"]);
            assert_eq!(rules.learning_record_kinds.len(), 2);
            assert_eq!(rules.learning_record_orders.len(), 2);
        }
        other => panic!("expected UserDataRules, got {other:?}"),
    }

    match send_ok(
        desktop_request::Request::UserDataExportFileName(UserDataExportFileNameRequest {
            kind: UserDataExportKind::CustomCsv as i32,
            local_date: "2026-11-27".to_owned(),
        }),
        None,
    ) {
        desktop_response::Reply::UserDataExportFileName(reply) => {
            assert_eq!(reply.file_name, "taigi_custom_dictionary_2026-11-27.csv");
        }
        other => panic!("expected UserDataExportFileName, got {other:?}"),
    }

    // Before Prepare, user-data store calls surface EngineUnavailableOp.
    let pre_open = user_data_reply(user_data_request::Method::ListCustomEntries(
        ListCustomEntriesRequest {
            filter: String::new(),
            limit: 10,
            offset: 0,
            input_mode: "tl".to_owned(),
        },
    ));
    assert!(matches!(
        pre_open.error.and_then(|err| err.kind),
        Some(user_data_error::Kind::EngineUnavailableOp(op)) if op == "customDictionaryList"
    ));

    // Malformed protobuf -> FAIL_PARSE; missing variant / missing required subfield -> FAIL_INVARIANT.
    assert_eq!(
        send_bytes(&[0xFF, 0xFF, 0xFF]).error,
        ErrorCode::FailParse as i32
    );
    assert_eq!(
        send_bytes(&DesktopRequest::default().encode_to_vec()).error,
        ErrorCode::FailInvariant as i32
    );
    assert_eq!(
        send_envelope(
            desktop_request::Request::UserData(UserDataRequest { method: None }),
            None,
        )
        .error,
        ErrorCode::FailInvariant as i32
    );
    assert_eq!(
        send_envelope(
            desktop_request::Request::UserData(UserDataRequest {
                method: Some(user_data_request::Method::SetLearningRecordCount(
                    SetLearningRecordCountRequest {
                        record: None,
                        count: 1,
                    },
                )),
            }),
            None,
        )
        .error,
        ErrorCode::FailInvariant as i32
    );
}

fn phase_sqlite_delete_journal(data_dir: &Path) {
    // Wait for background open by running a read-list request.
    let _ = user_data_ok(user_data_request::Method::ListCustomEntries(
        ListCustomEntriesRequest {
            filter: String::new(),
            limit: 10,
            offset: 0,
            input_mode: "tl".to_owned(),
        },
    ));
    for store in [
        "user_frequency.db",
        "user_association.db",
        "custom_dictionary.db",
        "learned_phrases.db",
    ] {
        let db = data_dir.join(store);
        let header = std::fs::read(&db).unwrap_or_else(|e| panic!("{store}: {e}"));
        assert_eq!(
            &header[18..20],
            &[1, 1],
            "{store} must use DELETE rollback journal"
        );
        assert!(!data_dir.join(format!("{store}-wal")).exists());
        assert!(!data_dir.join(format!("{store}-shm")).exists());
    }
}

fn phase_custom_dictionary_lifecycle(scratch_dir: &Path) {
    // 1. Save with empty id -> engine mints UUID.
    let saved = match user_data_ok(user_data_request::Method::SaveCustomEntry(
        SaveCustomEntryRequest {
            id: String::new(),
            roman: "khór".to_owned(),
            hanji: "苦".to_owned(),
        },
    )) {
        user_data_reply::Result::CustomEntrySaved(entry) => entry,
        other => panic!("expected CustomEntrySaved, got {other:?}"),
    };
    assert!(!saved.id.is_empty());

    // 2. Edit keeping the same id.
    let edited = match user_data_ok(user_data_request::Method::SaveCustomEntry(
        SaveCustomEntryRequest {
            id: saved.id.clone(),
            roman: "khór".to_owned(),
            hanji: "苦瓜".to_owned(),
        },
    )) {
        user_data_reply::Result::CustomEntrySaved(entry) => entry,
        other => panic!("expected CustomEntrySaved, got {other:?}"),
    };
    assert_eq!(edited.id, saved.id);
    assert_eq!(edited.hanji, "苦瓜");

    // 3. List across TL, POJ, TPS (including `or` -> `ㄜ` per TPS_OR_MAPS_TO_ER).
    for (mode, expected) in [("tl", "khór"), ("poj", "khór"), ("tps", "ㄎㄜˋ")] {
        let listed = match user_data_ok(user_data_request::Method::ListCustomEntries(
            ListCustomEntriesRequest {
                filter: "kh".to_owned(),
                limit: 10,
                offset: 0,
                input_mode: mode.to_owned(),
            },
        )) {
            user_data_reply::Result::CustomEntries(entries) => entries,
            other => panic!("expected CustomEntries, got {other:?}"),
        };
        assert_eq!(listed.matching_total, 1);
        assert_eq!(listed.entries[0].display_roman, expected);
    }

    // 4. Manage Dictionaries search finds the custom entry.
    match send_ok(
        desktop_request::Request::DictionarySearch(DictionarySearchRequest {
            query: "khor".to_owned(),
        }),
        Some(SettingsSnapshot {
            entries: vec![text_setting("inputMode", "tl")],
        }),
    ) {
        desktop_response::Reply::DictionarySearch(search) => {
            let hit = search.hits.first().expect("custom entry hit");
            assert!(hit.key.starts_with("custom:"), "{hit:?}");
            assert_eq!(hit.hanji.as_deref(), Some("苦瓜"));
        }
        other => panic!("expected DictionarySearch, got {other:?}"),
    }

    // 5. Pagination & pullback + delete.
    let mut extra_ids = Vec::new();
    for i in 1..=11 {
        let entry = match user_data_ok(user_data_request::Method::SaveCustomEntry(
            SaveCustomEntryRequest {
                id: String::new(),
                roman: format!("tshì-{i}"),
                hanji: format!("試{i}"),
            },
        )) {
            user_data_reply::Result::CustomEntrySaved(entry) => entry,
            other => panic!("expected CustomEntrySaved, got {other:?}"),
        };
        extra_ids.push(entry.id);
    }
    let page2 = match user_data_ok(user_data_request::Method::ListCustomEntries(
        ListCustomEntriesRequest {
            filter: String::new(),
            limit: 10,
            offset: 20, // beyond end -> pulled back to offset 10
            input_mode: "tl".to_owned(),
        },
    )) {
        user_data_reply::Result::CustomEntries(entries) => entries,
        other => panic!("expected CustomEntries, got {other:?}"),
    };
    assert_eq!(page2.total, 12);
    assert_eq!(page2.offset, 10);
    assert_eq!(page2.entries.len(), 2);

    let _ = user_data_ok(user_data_request::Method::DeleteCustomEntry(
        DeleteCustomEntryRequest {
            id: extra_ids.pop().expect("id"),
        },
    ));

    // 6. Export CSV -> DeleteAll -> Import CSV roundtrip + deduplicating re-import.
    let csv_bytes = match user_data_ok(user_data_request::Method::ExportCustomCsv(
        ExportCustomCsvRequest {},
    )) {
        user_data_reply::Result::CustomCsvExported(exported) => exported.csv,
        other => panic!("expected CustomCsvExported, got {other:?}"),
    };
    let csv_text = String::from_utf8(csv_bytes.clone()).expect("UTF-8 CSV");
    assert!(csv_text.contains("khór,苦瓜"), "{csv_text}");

    let _ = user_data_ok(user_data_request::Method::DeleteAllCustomEntries(
        DeleteAllCustomEntriesRequest {},
    ));
    let csv_path = scratch_dir.join("export.csv");
    std::fs::write(&csv_path, &csv_bytes).expect("write CSV");

    let imported = match user_data_ok(user_data_request::Method::ImportCustomCsvFile(
        ImportCustomCsvFileRequest {
            path: csv_path.display().to_string(),
        },
    )) {
        user_data_reply::Result::CustomCsvImported(res) => res,
        other => panic!("expected CustomCsvImported, got {other:?}"),
    };
    assert_eq!(imported.imported, 11);
    assert_eq!(imported.skipped, 0);

    let reimported = match user_data_ok(user_data_request::Method::ImportCustomCsvFile(
        ImportCustomCsvFileRequest {
            path: csv_path.display().to_string(),
        },
    )) {
        user_data_reply::Result::CustomCsvImported(res) => res,
        other => panic!("expected CustomCsvImported, got {other:?}"),
    };
    assert_eq!(reimported.imported, 0);
    assert_eq!(reimported.skipped, 11);

    // 7. Refusals.
    assert!(matches!(
        user_data_reply(user_data_request::Method::SaveCustomEntry(
            SaveCustomEntryRequest {
                id: String::new(),
                roman: "   ".to_owned(),
                hanji: "空".to_owned(),
            },
        ))
        .error
        .and_then(|err| err.kind),
        Some(user_data_error::Kind::Refused(refused))
            if refused.refusal == CustomDictionaryRefusal::EmptyRoman as i32
    ));
    assert!(matches!(
        user_data_reply(user_data_request::Method::SaveCustomEntry(
            SaveCustomEntryRequest {
                id: String::new(),
                roman: "---".to_owned(),
                hanji: "橫線".to_owned(),
            },
        ))
        .error
        .and_then(|err| err.kind),
        Some(user_data_error::Kind::Refused(refused))
            if refused.refusal == CustomDictionaryRefusal::Unsearchable as i32
    ));

    let bad_utf8_path = scratch_dir.join("bad_utf8.csv");
    std::fs::write(&bad_utf8_path, [0xFF, 0xFE, 0xFD]).expect("write bad UTF-8");
    assert!(matches!(
        user_data_reply(user_data_request::Method::ImportCustomCsvFile(
            ImportCustomCsvFileRequest {
                path: bad_utf8_path.display().to_string(),
            },
        ))
        .error
        .and_then(|err| err.kind),
        Some(user_data_error::Kind::Refused(refused))
            if refused.refusal == CustomDictionaryRefusal::NotUtf8 as i32
    ));

    let no_rows_path = scratch_dir.join("empty_rows.csv");
    std::fs::write(&no_rows_path, "not-two-columns\n").expect("write malformed csv");
    assert!(matches!(
        user_data_reply(user_data_request::Method::ImportCustomCsvFile(
            ImportCustomCsvFileRequest {
                path: no_rows_path.display().to_string(),
            },
        ))
        .error
        .and_then(|err| err.kind),
        Some(user_data_error::Kind::Refused(refused))
            if refused.refusal == CustomDictionaryRefusal::NoUsableRows as i32
    ));

    let big_csv_path = scratch_dir.join("big.csv");
    std::fs::File::create(&big_csv_path)
        .expect("create big file")
        .set_len(dispatch::CUSTOM_CSV_MAX_FILE_BYTES + 1)
        .expect("set_len");
    assert!(matches!(
        user_data_reply(user_data_request::Method::ImportCustomCsvFile(
            ImportCustomCsvFileRequest {
                path: big_csv_path.display().to_string(),
            },
        ))
        .error
        .and_then(|err| err.kind),
        Some(user_data_error::Kind::Refused(refused))
            if refused.refusal == CustomDictionaryRefusal::FileTooLarge as i32
    ));

    let missing_csv_path = scratch_dir.join("missing.csv");
    assert!(matches!(
        user_data_reply(user_data_request::Method::ImportCustomCsvFile(
            ImportCustomCsvFileRequest {
                path: missing_csv_path.display().to_string(),
            },
        ))
        .error
        .and_then(|err| err.kind),
        Some(user_data_error::Kind::Refused(refused))
            if refused.refusal == CustomDictionaryRefusal::None as i32
                && refused.detail.starts_with("could not read the file: ")
    ));
}

fn phase_learning_records_and_backup(scratch_dir: &Path) {
    let settings = SettingsSnapshot {
        entries: vec![
            text_setting("inputMode", "tl"),
            bool_setting("isTranslateSwapped", true),
            bool_setting("literalRomanCandidateEnabled", false),
            bool_setting("autoSpaceEnabled", false),
        ],
    };
    let mut tracker = ComposingTracker::new(1001);
    let activated = send_session(
        desktop_request::Request::Activate(ActivateRequest {
            token: tracker.token,
        }),
        Some(settings.clone()),
    );
    assert!(!activated.ignored);

    // 1. Compose "taigi" and commit cell 0 with Return -> records Frequency row.
    for ch in ['t', 'a', 'i', 'g', 'i'] {
        let reply = tracker.send_key(&ch.to_string(), None);
        assert!(reply.handled && reply.is_composing);
    }
    let committed = tracker.send_return(None);
    assert!(!committed.is_composing);

    // 2. Compose "guahoo" -> pick cell 1 ("我") so "hoo" stays composing, then
    //    finalize the tail with Return -> records a LearnedPhrase.
    for ch in ['g', 'u', 'a', 'h', 'o', 'o'] {
        let reply = tracker.send_key(&ch.to_string(), None);
        assert!(reply.handled && reply.is_composing);
    }
    tracker.selected_index = Some(1);
    let pick_prefix = tracker.send_return(None);
    assert!(
        pick_prefix.is_composing,
        "expected cell 1 of guahoo to leave tail composing: {pick_prefix:?}"
    );
    let tail_commit = tracker.send_return(None);
    assert!(!tail_commit.is_composing);

    // Wait/list Frequency and LearnedPhrase rows.
    let freq_listed = match user_data_ok(user_data_request::Method::ListLearningRecords(
        ListLearningRecordsRequest {
            kind: LearningRecordKind::Frequency as i32,
            order: LearningRecordOrder::MostUsed as i32,
            filter: String::new(),
            limit: 10,
            offset: 0,
        },
    )) {
        user_data_reply::Result::LearningRecords(list) => list,
        other => panic!("expected LearningRecords, got {other:?}"),
    };
    assert!(!freq_listed.records.is_empty(), "expected frequency rows");

    let recent_freq = match user_data_ok(user_data_request::Method::ListLearningRecords(
        ListLearningRecordsRequest {
            kind: LearningRecordKind::Frequency as i32,
            order: LearningRecordOrder::MostRecent as i32,
            filter: String::new(),
            limit: 10,
            offset: 0,
        },
    )) {
        user_data_reply::Result::LearningRecords(list) => list,
        other => panic!("expected LearningRecords, got {other:?}"),
    };
    assert_eq!(recent_freq.total, freq_listed.total);

    // 3. SetLearningRecordCount clamping above MAX_COUNT (1_000_000) and stale mismatch.
    let freq_row = freq_listed.records[0].clone();
    let updated = match user_data_ok(user_data_request::Method::SetLearningRecordCount(
        SetLearningRecordCountRequest {
            record: Some(freq_row.clone()),
            count: 5_000_000,
        },
    )) {
        user_data_reply::Result::LearningRecordSaved(saved) => saved.record.expect("updated row"),
        other => panic!("expected LearningRecordSaved, got {other:?}"),
    };
    assert_eq!(updated.count, 1_000_000);

    let mut stale = updated.clone();
    stale.text = "STALE_TEXT".to_owned();
    match user_data_ok(user_data_request::Method::SetLearningRecordCount(
        SetLearningRecordCountRequest {
            record: Some(stale),
            count: 5,
        },
    )) {
        user_data_reply::Result::LearningRecordSaved(saved) => assert!(saved.record.is_none()),
        other => panic!("expected LearningRecordSaved, got {other:?}"),
    }

    // 4. Attempting to add a Frequency row to Custom Dictionary is refused by the engine.
    let add_freq = user_data_reply(
        user_data_request::Method::AddLearningRecordToCustomDictionary(
            AddLearningRecordToCustomDictionaryRequest {
                record: Some(updated.clone()),
                roman: updated.tl.clone(),
                hanji: updated.text.clone(),
            },
        ),
    );
    assert!(add_freq.error.is_some());

    // 5. Verify LearnedPhrase row exists, promotes to Custom Dictionary, and forgets phrase.
    let phrases = match user_data_ok(user_data_request::Method::ListLearningRecords(
        ListLearningRecordsRequest {
            kind: LearningRecordKind::LearnedPhrase as i32,
            order: LearningRecordOrder::MostRecent as i32,
            filter: String::new(),
            limit: 10,
            offset: 0,
        },
    )) {
        user_data_reply::Result::LearningRecords(list) => list,
        other => panic!("expected LearningRecords, got {other:?}"),
    };
    let phrase = phrases
        .records
        .iter()
        .find(|row| row.can_add_to_custom_dictionary)
        .cloned()
        .expect("expected a promotable LearnedPhrase row");
    {
        let _ = user_data_ok(
            user_data_request::Method::AddLearningRecordToCustomDictionary(
                AddLearningRecordToCustomDictionaryRequest {
                    record: Some(phrase.clone()),
                    roman: phrase.tl.clone(),
                    hanji: phrase.text.clone(),
                },
            ),
        );
        let phrases_after = match user_data_ok(user_data_request::Method::ListLearningRecords(
            ListLearningRecordsRequest {
                kind: LearningRecordKind::LearnedPhrase as i32,
                order: LearningRecordOrder::MostRecent as i32,
                filter: String::new(),
                limit: 10,
                offset: 0,
            },
        )) {
            user_data_reply::Result::LearningRecords(list) => list,
            other => panic!("expected LearningRecords, got {other:?}"),
        };
        assert!(!phrases_after.records.iter().any(|row| row.id == phrase.id));
    }

    // 6. Export .taigi backup -> Clear all -> Import backup -> restored!
    let backup_bytes = match user_data_ok(user_data_request::Method::ExportBackup(
        ExportBackupRequest {
            app_version: "9.8.7".to_owned(),
        },
    )) {
        user_data_reply::Result::BackupExported(exported) => exported.backup,
        other => panic!("expected BackupExported, got {other:?}"),
    };
    let backup_json = String::from_utf8(backup_bytes.clone()).expect("UTF-8 backup JSON");
    assert!(backup_json.contains("\"macos\""), "{backup_json}");
    assert!(backup_json.contains("\"9.8.7\""), "{backup_json}");

    let backup_path = scratch_dir.join("test.taigi");
    std::fs::write(&backup_path, &backup_bytes).expect("write backup");

    let deleted = match user_data_ok(user_data_request::Method::DeleteLearningRecord(
        DeleteLearningRecordRequest {
            record: Some(updated.clone()),
        },
    )) {
        user_data_reply::Result::LearningRecordDeleted(del) => del,
        other => panic!("expected LearningRecordDeleted, got {other:?}"),
    };
    assert!(deleted.removed);
    let deleted_again = match user_data_ok(user_data_request::Method::DeleteLearningRecord(
        DeleteLearningRecordRequest {
            record: Some(updated.clone()),
        },
    )) {
        user_data_reply::Result::LearningRecordDeleted(del) => del,
        other => panic!("expected LearningRecordDeleted, got {other:?}"),
    };
    assert!(!deleted_again.removed);

    let _ = user_data_ok(user_data_request::Method::DeleteAllCustomEntries(
        DeleteAllCustomEntriesRequest {},
    ));
    let _ = user_data_ok(user_data_request::Method::ClearLearningRecords(
        ClearLearningRecordsRequest {},
    ));
    let empty_freq = match user_data_ok(user_data_request::Method::ListLearningRecords(
        ListLearningRecordsRequest {
            kind: LearningRecordKind::Frequency as i32,
            order: LearningRecordOrder::MostUsed as i32,
            filter: String::new(),
            limit: 10,
            offset: 0,
        },
    )) {
        user_data_reply::Result::LearningRecords(list) => list,
        other => panic!("expected LearningRecords, got {other:?}"),
    };
    assert_eq!(empty_freq.total, 0);

    let imported_backup = match user_data_ok(user_data_request::Method::ImportBackupFile(
        ImportBackupFileRequest {
            path: backup_path.display().to_string(),
        },
    )) {
        user_data_reply::Result::BackupImported(imported) => imported,
        other => panic!("expected BackupImported, got {other:?}"),
    };
    assert!(imported_backup.custom_dictionary > 0);
    assert!(imported_backup.frequency > 0);

    let restored_freq = match user_data_ok(user_data_request::Method::ListLearningRecords(
        ListLearningRecordsRequest {
            kind: LearningRecordKind::Frequency as i32,
            order: LearningRecordOrder::MostUsed as i32,
            filter: String::new(),
            limit: 10,
            offset: 0,
        },
    )) {
        user_data_reply::Result::LearningRecords(list) => list,
        other => panic!("expected LearningRecords, got {other:?}"),
    };
    assert!(restored_freq
        .records
        .iter()
        .any(|row| row.text == updated.text && row.count == 1_000_000));

    // Backup refusals: corrupt JSON, version 0, missing file.
    let bad_backup = scratch_dir.join("corrupt.taigi");
    std::fs::write(&bad_backup, "not json").expect("write corrupt backup");
    assert!(matches!(
        user_data_reply(user_data_request::Method::ImportBackupFile(
            ImportBackupFileRequest {
                path: bad_backup.display().to_string(),
            },
        ))
        .error
        .and_then(|err| err.kind),
        Some(user_data_error::Kind::Refused(refused))
            if refused.detail == "not a .taigi backup"
    ));

    let old_version_backup = scratch_dir.join("v0.taigi");
    std::fs::write(&old_version_backup, r#"{"version":0}"#).expect("write v0 backup");
    assert!(matches!(
        user_data_reply(user_data_request::Method::ImportBackupFile(
            ImportBackupFileRequest {
                path: old_version_backup.display().to_string(),
            },
        ))
        .error
        .and_then(|err| err.kind),
        Some(user_data_error::Kind::Refused(refused))
            if refused.detail == "unsupported backup version"
    ));

    let missing_backup = scratch_dir.join("missing.taigi");
    assert!(matches!(
        user_data_reply(user_data_request::Method::ImportBackupFile(
            ImportBackupFileRequest {
                path: missing_backup.display().to_string(),
            },
        ))
        .error
        .and_then(|err| err.kind),
        Some(user_data_error::Kind::Refused(refused))
            if refused.detail.starts_with("could not read the file: ")
    ));
}

fn phase_symbol_swap() {
    let settings = SettingsSnapshot {
        entries: vec![
            text_setting("inputMode", "tl"),
            bool_setting("isTranslateSwapped", false),
            bool_setting("literalRomanCandidateEnabled", true),
            bool_setting("autoSpaceEnabled", true),
            bool_setting("outputBothScripts", false),
        ],
    };
    let mut tracker = ComposingTracker::new(2002);
    let _ = send_session(
        desktop_request::Request::Activate(ActivateRequest {
            token: tracker.token,
        }),
        Some(settings.clone()),
    );

    for ch in ['t', 'a', 'i', 'g', 'i'] {
        let _ = tracker.send_key(&ch.to_string(), None);
    }
    let commit = tracker.send_return(None);
    assert!(!commit.is_composing);
    assert!(
        tracker.swap_available,
        "roman-first commit with autoSpaceEnabled must arm swap: {:?}",
        commit.effects
    );

    let ask = |symbol: &str, snap: SettingsSnapshot| -> bool {
        match send_ok(
            desktop_request::Request::SymbolSwap(SymbolSwapRequest {
                symbol: symbol.to_owned(),
            }),
            Some(snap),
        ) {
            desktop_response::Reply::SymbolSwap(reply) => reply.may_swap,
            other => panic!("expected SymbolSwap reply, got {other:?}"),
        }
    };

    assert!(!ask("（", settings.clone()));
    assert!(ask("，", settings.clone()));

    let inserted = send_session(
        desktop_request::Request::InsertSymbol(InsertSymbolRequest {
            token: tracker.token,
            symbol: "，".to_owned(),
            panel: Some(tracker.panel()),
        }),
        None,
    );
    tracker.apply_effects(&inserted.effects);
    assert!(
        inserted
            .effects
            .iter()
            .any(|e| matches!(e.effect.as_ref(), Some(effect::Effect::SwapPrecedingSpace(s)) if s.replacement == "，")),
        "attaching symbol with swap_available=true must swap trailing space: {:?}",
        inserted.effects
    );

    // Bracket-annotation toggle keeps SymbolSwap enabled when side-by-side even with Auto-Space off.
    assert!(ask(
        "，",
        SettingsSnapshot {
            entries: vec![
                bool_setting("autoSpaceEnabled", false),
                bool_setting("outputBothScripts", true),
                text_setting("candidateDisplayMode", "sideBySide"),
            ],
        }
    ));
    assert!(!ask(
        "，",
        SettingsSnapshot {
            entries: vec![
                bool_setting("autoSpaceEnabled", false),
                bool_setting("outputBothScripts", true),
                text_setting("candidateDisplayMode", "romanOnly"),
            ],
        }
    ));
}

fn phase_concurrent_user_data_and_composing() {
    let handles: Vec<_> = (0..4)
        .map(|worker| {
            thread::spawn(move || {
                for iteration in 0..20 {
                    let entries = match user_data_ok(user_data_request::Method::ListCustomEntries(
                        ListCustomEntriesRequest {
                            filter: String::new(),
                            limit: 10,
                            offset: 0,
                            input_mode: if iteration % 2 == 0 { "tl" } else { "poj" }.to_owned(),
                        },
                    )) {
                        user_data_reply::Result::CustomEntries(listed) => listed,
                        other => panic!("worker {worker}: unexpected {other:?}"),
                    };
                    assert!(entries.total > 0);
                    let _ = user_data_ok(user_data_request::Method::ListLearningRecords(
                        ListLearningRecordsRequest {
                            kind: LearningRecordKind::Frequency as i32,
                            order: LearningRecordOrder::MostUsed as i32,
                            filter: String::new(),
                            limit: 10,
                            offset: 0,
                        },
                    ));
                    assert!(matches!(
                        send_ok(
                            desktop_request::Request::UserDataRules(UserDataRulesRequest {}),
                            None
                        ),
                        desktop_response::Reply::UserDataRules(_)
                    ));
                }
            })
        })
        .collect();

    let mut tracker = ComposingTracker::new(3003);
    let _ = send_session(
        desktop_request::Request::Activate(ActivateRequest {
            token: tracker.token,
        }),
        None,
    );
    for _ in 0..5 {
        for ch in ['k', 'h', 'o', 'r'] {
            let reply = tracker.send_key(&ch.to_string(), None);
            assert!(reply.handled);
        }
        let _ = send_session(
            desktop_request::Request::Cancel(CancelRequest {
                token: tracker.token,
                panel: Some(tracker.panel()),
            }),
            None,
        );
        tracker = ComposingTracker::new(tracker.token);
    }

    for handle in handles {
        handle.join().expect("worker thread completed cleanly");
    }
}
