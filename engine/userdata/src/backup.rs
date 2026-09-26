//! The `.taigi` backup file — version 2, as iOS and Android have written
//! it: the custom dictionary, the frequency counts and the learned bigrams
//! in one JSON document; learned phrases never travel (§50). One codec for
//! every platform (user-data-engine-roadmap P4b); the file picker and share
//! sheet stay with the platform.
//!
//! Reading is lenient, as Android's was: a missing field takes its default,
//! an unknown one is ignored, only a version below 1 is refused — so an old
//! backup, or one from the other phone, still restores. Writing is iOS's
//! shape: sorted keys, pretty-printed, `lastUsed` empty, ids and times of
//! custom rows left out. Every struct below declares its fields in the
//! serialized keys' alphabetical order, which IS the sorted-keys output —
//! whatever `serde_json` features the build unifies.

use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::timestamp::format_rfc3339_utc;
use crate::{AssociationPair, AssociationRow, CustomDictionaryRow, UserDataStores};

/// The version this codec writes.
pub const BACKUP_VERSION: i64 = 2;

#[derive(Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Backup {
    app_version: String,
    custom_dictionary: Vec<CustomEntry>,
    exported_at: String,
    platform: String,
    user_association: Vec<AssociationEntry>,
    user_frequency: Vec<FrequencyEntry>,
    version: i64,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct CustomEntry {
    hanzi: String,
    /// The unreleased 2026-09-20 shape tagged learned phrases `origin = 1`;
    /// read to skip them, never written.
    #[serde(skip_serializing_if = "Option::is_none")]
    origin: Option<i64>,
    roman: String,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct FrequencyEntry {
    count: i64,
    last_used: String,
    /// Absent in a backup written before the `(word, tl)` pair key: the
    /// legacy `""` bucket.
    tl: Option<String>,
    word: String,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct AssociationEntry {
    count: i64,
    last_used: String,
    next_tl: String,
    next_word: String,
    prev_tl: Option<String>,
    prev_word: String,
}

/// Why a backup could not be restored.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum BackupError {
    #[error("not a .taigi backup: {0}")]
    Unreadable(String),
    #[error("unsupported backup version {0}")]
    UnsupportedVersion(i64),
    #[error("{0}")]
    Store(String),
}

fn store_error(error: impl Display) -> BackupError {
    BackupError::Store(error.to_string())
}

/// What a restore merged, per store.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BackupImported {
    pub custom_dictionary: usize,
    pub frequency: usize,
    pub association: usize,
}

/// The stores as a `.taigi` file. `platform` / `app_version` name the
/// writer (`ios`, `android`, `macos`, …); `now_seconds` stamps `exportedAt`.
pub fn export_backup(
    stores: &UserDataStores,
    platform: &str,
    app_version: &str,
    now_seconds: u64,
) -> Result<Vec<u8>, BackupError> {
    let custom = stores.custom_dictionary.all_rows().map_err(store_error)?;
    let frequency = stores
        .frequency
        .all_rows()
        .ok_or_else(|| store_error("frequency store not readable"))?;
    let association = stores
        .association
        .all_rows()
        .ok_or_else(|| store_error("association store not readable"))?;
    let backup = Backup {
        app_version: app_version.to_owned(),
        custom_dictionary: custom
            .into_iter()
            .map(|row| CustomEntry {
                hanzi: row.hanzi,
                origin: None,
                roman: row.roman,
            })
            .collect(),
        exported_at: format_rfc3339_utc(now_seconds),
        platform: platform.to_owned(),
        user_association: association
            .into_iter()
            .map(|row| AssociationEntry {
                count: row.count,
                last_used: String::new(),
                next_tl: row.pair.next_tl,
                next_word: row.pair.next,
                prev_tl: Some(row.pair.previous_tl),
                prev_word: row.pair.previous,
            })
            .collect(),
        user_frequency: frequency
            .into_iter()
            .map(|row| FrequencyEntry {
                count: row.count,
                last_used: String::new(),
                tl: Some(row.tl),
                word: row.word,
            })
            .collect(),
        version: BACKUP_VERSION,
    };
    serde_json::to_vec_pretty(&backup).map_err(store_error)
}

/// Restores a `.taigi` file into the stores, merging with what is there:
/// custom words already stored are skipped and the import fills whatever
/// room the dictionary has left; counts keep the larger of the two and count
/// as used now; bigram readings are normalized POJ → TL (older and
/// cross-platform backups can carry POJ). A custom row missing its
/// romanization or its Hanji is skipped, as Android's restore did; a missing
/// count reads as 1, as Android's did.
pub fn import_backup(stores: &UserDataStores, bytes: &[u8]) -> Result<BackupImported, BackupError> {
    let backup: Backup = serde_json::from_slice(bytes)
        .map_err(|error| BackupError::Unreadable(error.to_string()))?;
    if backup.version < 1 {
        return Err(BackupError::UnsupportedVersion(backup.version));
    }

    let custom: Vec<CustomDictionaryRow> = backup
        .custom_dictionary
        .iter()
        .filter(|entry| entry.origin != Some(1))
        .filter(|entry| !entry.roman.is_empty() && !entry.hanzi.is_empty())
        .map(|entry| CustomDictionaryRow::new(&entry.roman, &entry.hanzi))
        .collect();
    let custom_dictionary = stores
        .custom_dictionary
        .import_until_full(&custom)
        .map_err(store_error)?
        .imported;

    let frequency = stores
        .frequency
        .import_merge(
            backup
                .user_frequency
                .into_iter()
                .map(|entry| (entry.word, entry.tl.unwrap_or_default(), entry.count.max(1)))
                .collect(),
        )
        .map_err(store_error)?;

    let association = stores
        .association
        .import_merge(
            backup
                .user_association
                .into_iter()
                .map(|entry| AssociationRow {
                    pair: AssociationPair {
                        previous: entry.prev_word,
                        previous_tl: phonetics::api::poj_display_to_tl_display(
                            &entry.prev_tl.unwrap_or_default(),
                        ),
                        next: entry.next_word,
                        next_tl: phonetics::api::poj_display_to_tl_display(&entry.next_tl),
                    },
                    count: entry.count.max(1),
                })
                .collect(),
        )
        .map_err(store_error)?;

    Ok(BackupImported {
        custom_dictionary,
        frequency,
        association,
    })
}
