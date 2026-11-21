//! The FFI request cap (`MAX_REQUEST_BYTES`, enforced by `android-jni`,
//! `swift-ffi` and the macOS FFI before decoding) must admit the largest
//! `.taigi` restore the stores can produce: every store filled to its cap,
//! exported by the engine itself, sent back as one `ImportBackup` (#1032 —
//! a 17 677-word desktop backup of 2.7 MB was refused at the old 2 MB cap).
#![cfg(feature = "user-data")]

use prost::Message;
use protos::engine::{request, user_data_request, ImportBackup, Request, UserDataRequest};
use userdata::{
    export_backup, import_backup, CustomDictionaryStore, JournalMode, LearnedPhraseStore,
    UserAssociationStore, UserDataPaths, UserDataStores, UserFrequencyStore,
};

/// Four-syllable TL readings, distinct per `index` — a long, realistic key.
const SYLLABLES: [&str; 20] = [
    "tsiânn", "khuànn", "thâu", "sîng", "lâng", "kiánn", "tshù", "bīn", "hun", "guā", "sió", "tuā",
    "kháu", "tsuí", "hōo", "phīnn", "tshiū", "ìng", "kuân", "suí",
];

fn reading(index: usize) -> String {
    (0..4)
        .map(|place| SYLLABLES[index / 20usize.pow(place) % 20])
        .collect::<Vec<_>>()
        .join("-")
}

/// Four CJK ideographs, distinct per `index`.
fn hanji(index: usize) -> String {
    (0..4)
        .map(|place| char::from_u32(0x4E00 + (index / 20_000usize.pow(place) % 20_000) as u32))
        .map(Option::unwrap)
        .collect()
}

fn rows<T>(count: usize, row: impl Fn(usize) -> T) -> Vec<T> {
    (0..count).map(row).collect()
}

#[test]
fn a_backup_of_every_store_at_its_cap_fits_one_ffi_request() {
    let custom = CustomDictionaryStore::MAX_ENTRIES;
    let frequency = UserFrequencyStore::shipped_capacity().max_rows() as usize;
    let association = UserAssociationStore::shipped_capacity().max_rows() as usize;
    let learned = LearnedPhraseStore::MAX_ENTRIES;
    let used = "2026-11-21 00:00:00";
    let full = serde_json::json!({
        "version": 4,
        "platform": "windows",
        "appVersion": "3.10.0",
        "exportedAt": "2026-11-21T00:00:00Z",
        "customDictionary": rows(custom, |i| serde_json::json!({
            "roman": reading(i), "hanzi": hanji(i), "createdAt": used, "updatedAt": used,
        })),
        "userFrequency": rows(frequency, |i| serde_json::json!({
            "word": hanji(i), "tl": reading(i), "count": 999, "lastUsed": used,
            "abbrevCount": 999, "abbrevLastUsed": used,
        })),
        "userAssociation": rows(association, |i| serde_json::json!({
            "prevWord": hanji(i), "prevTl": reading(i),
            "nextWord": hanji(i + 1), "nextTl": reading(i + 1),
            "count": 999, "lastUsed": used,
        })),
        "learnedPhrases": rows(learned, |i| serde_json::json!({
            "hanzi": hanji(i), "roman": reading(i), "learnCount": 999, "updatedAt": used,
        })),
    });

    let directory = tempfile::tempdir().unwrap();
    let stores = UserDataStores::at(
        UserDataPaths::in_directory(directory.path()),
        JournalMode::Delete,
    );
    stores.open_blocking();
    let imported = import_backup(&stores, &serde_json::to_vec(&full).unwrap()).unwrap();
    assert_eq!(
        (
            imported.custom_dictionary as usize,
            imported.frequency as usize,
            imported.association as usize,
            imported.learned_phrases as usize,
        ),
        (custom, frequency, association, learned),
        "every store at its cap"
    );

    // What a phone hands its FFI for this restore: the engine's own export.
    let backup = export_backup(&stores, "windows", "3.10.0", 0).unwrap();
    let request = Request {
        id: 1,
        config_snapshot: None,
        generation: 0,
        payload: Some(request::Payload::UserData(UserDataRequest {
            method: Some(user_data_request::Method::ImportBackup(ImportBackup {
                backup,
            })),
        })),
    }
    .encode_to_vec();
    // Half the cap at most, so the room the cap's doc promises stays real.
    assert!(
        request.len() * 2 <= dispatch::MAX_REQUEST_BYTES,
        "a full backup is {} bytes; the FFI cap is {}",
        request.len(),
        dispatch::MAX_REQUEST_BYTES
    );
}
