#![allow(dead_code)]
//! Shared process-singleton initialization and synchronization helpers for the
//! long-horizon realistic user-interaction simulator.

pub mod corpus;
pub mod host;
pub mod personas;
pub mod rng;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, OnceLock};

use prost::Message;
use protos::engine::{
    request, response, user_data_request, user_data_response, ExportBackup, LearningRecordKind,
    LearningRecordOrder, OpenUserData, Request, ResetUserData, Response, UserDataJournal,
    UserDataRequest,
};
use taigi_desktop_core::dictionary_artifacts::DictionaryArtifacts;
use taigi_desktop_core::engine::{self, user_data};

struct ProcessEnv {
    _temp_dir: tempfile::TempDir,
    user_data_dir: PathBuf,
    dictionaries_dir: PathBuf,
}

static PROCESS_LOCK: Mutex<()> = Mutex::new(());
static PROCESS_ENV: OnceLock<ProcessEnv> = OnceLock::new();

pub struct LockedProcessEnv<'a> {
    _guard: MutexGuard<'a, ()>,
    pub user_data_dir: &'a Path,
    pub dictionaries_dir: &'a Path,
}

/// Acquires the single-engine process lock and ensures the shipped lexicon and
/// synchronous WAL user-data SQLite stores are initialized.
pub fn acquire_process_env() -> LockedProcessEnv<'static> {
    let guard = PROCESS_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let env = PROCESS_ENV.get_or_init(|| {
        let temp_dir = tempfile::tempdir().expect("process tempdir");
        let user_data_dir = temp_dir.path().join("userdata");
        std::fs::create_dir_all(&user_data_dir).expect("userdata dir");

        let dictionaries_dir = corpus::repo_root().join("assets/dictionaries");
        let artifacts =
            DictionaryArtifacts::locate(&dictionaries_dir).expect("shipped dictionaries present");
        engine::lexicon_install(&artifacts, taigi_linux_core::runtime::dictionary_version())
            .expect("lexicon installs");

        // Open synchronously (`in_background = false`) under `Wal` so `DesktopPlatform::Linux`
        // matches the journal mode and all 4 worker threads are ready immediately.
        let open_req = Request {
            id: 1,
            generation: 0,
            config_snapshot: None,
            payload: Some(request::Payload::UserData(UserDataRequest {
                method: Some(user_data_request::Method::Open(OpenUserData {
                    directory: user_data_dir.display().to_string(),
                    journal: UserDataJournal::Wal as i32,
                    in_background: false,
                    ..OpenUserData::default()
                })),
            })),
        };
        let resp_bytes = dispatch::process_request(&open_req.encode_to_vec());
        let resp = Response::decode(resp_bytes.as_slice()).expect("decode open response");
        assert_eq!(resp.error, 0, "synchronous user-data open failed");

        ProcessEnv {
            _temp_dir: temp_dir,
            user_data_dir,
            dictionaries_dir,
        }
    });
    LockedProcessEnv {
        _guard: guard,
        user_data_dir: &env.user_data_dir,
        dictionaries_dir: &env.dictionaries_dir,
    }
}

/// Resets transient `composing` and `nextword` singletons using a unique
/// high-range generation (`>= 1 << 62`) so out-of-band oracle simulators never
/// collide with an active episode `HostSimulator`'s generation counter.
pub fn reset_transient_engine_state() {
    use std::sync::atomic::{AtomicU64, Ordering};
    static PROBE_GEN: AtomicU64 = AtomicU64::new(1_u64 << 62);
    let gen = PROBE_GEN.fetch_add(1, Ordering::Relaxed);
    let settings = taigi_desktop_core::settings::EngineSettings::default();
    let _ = engine::reset(gen);
    engine::nextword_reset_all(0, &settings, gen);
}

/// Empties all four SQLite stores (`frequency`, `association`, `learned_phrases`,
/// `custom_dictionary`) and resets `nextword` + `composing` singletons.
pub fn reset_all_persistent_state() {
    let reset_req = Request {
        id: 2,
        generation: 0,
        config_snapshot: None,
        payload: Some(request::Payload::UserData(UserDataRequest {
            method: Some(user_data_request::Method::Reset(ResetUserData {
                frequency: true,
                association: true,
                learned_phrases: true,
                custom_dictionary: true,
            })),
        })),
    };
    let bytes = dispatch::process_request(&reset_req.encode_to_vec());
    let resp = Response::decode(bytes.as_slice()).expect("decode reset");
    assert_eq!(resp.error, 0, "user-data reset failed");
    reset_transient_engine_state();
}

/// Barrier across all four store worker threads: executes synchronous `perform`
/// queries on frequency, association, custom_dictionary (`ExportBackup`) and
/// learned_phrases (`list_learning_page`), guaranteeing every prior queued
/// write has committed to SQLite.
pub fn sync_user_data_workers() -> UserDataCounts {
    let backup_bytes = export_backup_bytes();
    let freq = user_data::list_learning_page(
        LearningRecordKind::Frequency,
        LearningRecordOrder::MostUsed,
        "",
        0,
        1,
    )
    .map(|p| p.total_count)
    .unwrap_or(0);
    let phrases = user_data::list_learning_page(
        LearningRecordKind::LearnedPhrase,
        LearningRecordOrder::MostUsed,
        "",
        0,
        1,
    )
    .map(|p| p.total_count)
    .unwrap_or(0);
    let custom = user_data::list_custom_entries("", 1, 0)
        .map(|c| c.total as usize)
        .unwrap_or(0);
    // Count association entries in the exported `.taigi` JSON backup.
    let associations = serde_json::from_slice::<serde_json::Value>(&backup_bytes)
        .ok()
        .and_then(|v| v.get("userAssociation")?.as_array().map(Vec::len))
        .unwrap_or(0);
    UserDataCounts {
        frequency_rows: freq,
        association_rows: associations,
        learned_phrase_rows: phrases,
        custom_rows: custom,
    }
}

pub fn export_backup_bytes() -> Vec<u8> {
    let req = Request {
        id: 3,
        generation: 0,
        config_snapshot: None,
        payload: Some(request::Payload::UserData(UserDataRequest {
            method: Some(user_data_request::Method::ExportBackup(ExportBackup {
                platform: "linux".to_owned(),
                app_version: "3.9.0".to_owned(),
            })),
        })),
    };
    let bytes = dispatch::process_request(&req.encode_to_vec());
    let resp = Response::decode(bytes.as_slice()).expect("decode backup");
    match resp.payload {
        Some(response::Payload::UserData(ud)) => match ud.result {
            Some(user_data_response::Result::BackupExported(exported)) => exported.backup,
            _ => Vec::new(),
        },
        _ => Vec::new(),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UserDataCounts {
    pub frequency_rows: usize,
    pub association_rows: usize,
    pub learned_phrase_rows: usize,
    pub custom_rows: usize,
}
