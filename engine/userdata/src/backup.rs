//! The `.taigi` backup file: the custom dictionary, the frequency counts, the
//! learned bigrams and the learned phrases (§50) in one JSON document — what
//! typing reads from the user's stores, so a restore on any platform, phone
//! or desktop, types as the device that wrote the file did (#1032). One codec
//! for every platform (user-data-engine-roadmap P4b); the file picker and
//! share sheet stay with the platform.
//!
//! Versions share one shape. 1 and 2 (iOS and Android) may carry POJ bigram
//! readings, so those are folded POJ → TL; from 3 the readings are the ones
//! the store holds and restore unchanged — a fold would misread a TL `eng` /
//! `ek` final as POJ (蔣經國 `tsiúnn-keng-kok` → `king`). 4 adds the learned
//! phrases, each learned row's last use (`lastUsed` / `updatedAt`, which the
//! walker's decay and the stores' eviction read), each custom word's
//! `createdAt` / `updatedAt` (the keystroke order of homophone custom words)
//! and each frequency row's abbreviation picks (`abbrevCount` /
//! `abbrevLastUsed`, written only for a row that has any; frequency rows keep
//! reading and abbreviation picks apart, `UserFrequencyStore::import_merge`);
//! a file before 4 restores those times as used now and no abbreviation
//! picks. An older app reads a newer file as its
//! own version, ignoring what it does not know.
//!
//! Reading is lenient, as Android's was: a missing field takes its default,
//! an unknown one is ignored, only a version below 1 is refused — so an old
//! backup, or one from the other phone, still restores. Writing is iOS's
//! shape: sorted keys, pretty-printed, times in the stores' own
//! `yyyy-MM-dd HH:mm:ss` UTC text, custom ids left out (a restore mints its
//! own). Every struct below declares its fields in the serialized keys'
//! alphabetical order, which IS the sorted-keys output — whatever
//! `serde_json` features the build unifies.

use std::fmt::Display;

use serde::{Deserialize, Serialize};

use crate::timestamp::{format_rfc3339_utc, format_utc_timestamp};
use crate::{AssociationPair, CustomDictionaryRow, FrequencyImportRow, UserDataStores};

/// The version this codec writes.
pub const BACKUP_VERSION: i64 = 4;

/// The first version whose bigram readings restore unchanged.
const VERBATIM_READINGS_VERSION: i64 = 3;

#[derive(Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct Backup {
    app_version: String,
    custom_dictionary: Vec<CustomEntry>,
    exported_at: String,
    learned_phrases: Vec<LearnedPhraseEntry>,
    platform: String,
    user_association: Vec<AssociationEntry>,
    user_frequency: Vec<FrequencyEntry>,
    version: i64,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct CustomEntry {
    created_at: String,
    hanzi: String,
    /// The unreleased 2026-09-20 shape tagged learned phrases `origin = 1`;
    /// read to skip them, never written.
    #[serde(skip_serializing_if = "Option::is_none")]
    origin: Option<i64>,
    roman: String,
    updated_at: String,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct LearnedPhraseEntry {
    hanzi: String,
    learn_count: i64,
    roman: String,
    updated_at: String,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
struct FrequencyEntry {
    /// The picks of `count` made through a whole-buffer abbreviation; absent
    /// (0) in a file before them and for a row without any.
    #[serde(skip_serializing_if = "is_zero")]
    abbrev_count: i64,
    /// Last abbreviation pick; empty with no abbreviation picks.
    #[serde(skip_serializing_if = "String::is_empty")]
    abbrev_last_used: String,
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

fn is_zero(value: &i64) -> bool {
    *value == 0
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
    pub learned_phrases: usize,
}

/// A store's epoch-ms time as the file writes it; empty when the store had
/// none (0), which a restore reads as now.
fn backup_timestamp(epoch_ms: i64) -> String {
    if epoch_ms < 1000 {
        return String::new();
    }
    format_utc_timestamp((epoch_ms / 1000) as u64)
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
    let learned = stores
        .learned_phrases
        .backup_rows()
        .ok_or_else(|| store_error("learned phrase store not readable"))?;
    let backup = Backup {
        app_version: app_version.to_owned(),
        custom_dictionary: custom
            .into_iter()
            .map(|row| CustomEntry {
                created_at: row.created_at,
                hanzi: row.hanji,
                origin: None,
                roman: row.roman,
                updated_at: row.updated_at,
            })
            .collect(),
        exported_at: format_rfc3339_utc(now_seconds),
        learned_phrases: learned
            .into_iter()
            .map(|(row, updated_at_ms)| LearnedPhraseEntry {
                hanzi: row.hanji,
                learn_count: row.learn_count,
                roman: row.canonical_tl,
                updated_at: backup_timestamp(updated_at_ms),
            })
            .collect(),
        platform: platform.to_owned(),
        user_association: association
            .into_iter()
            .map(|row| AssociationEntry {
                count: row.count,
                last_used: backup_timestamp(row.last_used_ms),
                next_tl: row.pair.next_tl,
                next_word: row.pair.next,
                prev_tl: Some(row.pair.previous_tl),
                prev_word: row.pair.previous,
            })
            .collect(),
        user_frequency: frequency
            .into_iter()
            .map(|row| FrequencyEntry {
                abbrev_count: row.abbrev_count,
                abbrev_last_used: backup_timestamp(row.abbrev_last_used_ms),
                count: row.count,
                last_used: backup_timestamp(row.last_used_ms),
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
/// room the dictionary has left, keeping the file's times; learned rows keep
/// the larger count and the later use (a file before version 4 has no use
/// times: they count as used now) — frequency rows keep reading and
/// abbreviation picks apart ([`UserFrequencyStore::import_merge`](crate::UserFrequencyStore::import_merge));
/// bigram readings of a version-1 / -2 file
/// are folded POJ → TL (the phones could write POJ there). A custom row
/// missing its romanization or its Hanji is skipped, as Android's restore
/// did; a missing count reads as 1, as Android's did.
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
        .map(|entry| CustomDictionaryRow {
            created_at: entry.created_at.clone(),
            updated_at: entry.updated_at.clone(),
            ..CustomDictionaryRow::new(&entry.roman, &entry.hanzi)
        })
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
                .map(|entry| FrequencyImportRow {
                    word: entry.word,
                    tl: entry.tl.unwrap_or_default(),
                    count: entry.count.max(1),
                    last_used: entry.last_used,
                    abbrev_count: entry.abbrev_count,
                    abbrev_last_used: entry.abbrev_last_used,
                })
                .collect(),
        )
        .map_err(store_error)?;

    let folds_readings = backup.version < VERBATIM_READINGS_VERSION;
    let reading = |stored: String| {
        if folds_readings {
            phonetics::api::poj_display_to_tl_display(&stored)
        } else {
            stored
        }
    };
    let association = stores
        .association
        .import_merge(
            backup
                .user_association
                .into_iter()
                .map(|entry| {
                    (
                        AssociationPair {
                            previous: entry.prev_word,
                            previous_tl: reading(entry.prev_tl.unwrap_or_default()),
                            next: entry.next_word,
                            next_tl: reading(entry.next_tl),
                        },
                        entry.count.max(1),
                        entry.last_used,
                    )
                })
                .collect(),
        )
        .map_err(store_error)?;

    let learned_phrases = stores
        .learned_phrases
        .import_merge(
            backup
                .learned_phrases
                .into_iter()
                .map(|entry| {
                    (
                        entry.hanzi,
                        entry.roman,
                        entry.learn_count,
                        entry.updated_at,
                    )
                })
                .collect(),
        )
        .map_err(store_error)?;

    Ok(BackupImported {
        custom_dictionary,
        frequency,
        association,
        learned_phrases,
    })
}
