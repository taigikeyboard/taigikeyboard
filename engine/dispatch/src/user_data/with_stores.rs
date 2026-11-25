//! The `user-data` build's half of `super`: the user-data request's wire
//! answer, the composing / next-word requests answered with the stores the
//! platform opened, and a dictionary search's custom entries. The stores,
//! their one open per process and the requests' logic are `userdata`'s
//! (`UserDataHandle`); this module maps its answer to the wire's error code
//! and joins the stores to `composing` / `nextword` / `lexicon`, which only
//! `dispatch` sees together.

use composing::api::ComposingError;
use composing::{ConversionFrequency, UserRows};
use nextword::NextWordError;
use protos::engine::{
    composing_request, next_word_request, response, AppConfig, ComposingRequest, ComposingResponse,
    CustomDictionaryEntry, DictionarySearchRequest, ErrorCode, NextWordRequest, NextWordResponse,
    RawNextWordPrediction, Response, Source, UserDataRequest,
};
use ranking::{FrequencyData, FrequencyMap};
use userdata::{
    AssociationPair, CustomDictionaryCSV, CustomDictionaryRow, CustomDictionaryStore, FollowingRow,
    FrequencyRow, LearnedPhraseRow, LearnedPhraseStore, RequestError, UserDataHandle,
    UserDataStores,
};

use crate::fetch::UserRowSource;

// The limit a platform may check before the read is the codec's own.
const _: () = assert!(crate::CUSTOM_CSV_MAX_FILE_BYTES == CustomDictionaryCSV::MAX_FILE_SIZE_BYTES);

/// Answers one user-data request — the `UserData` arm of `crate::run`.
pub(crate) fn respond(id: u32, request: &UserDataRequest) -> Response {
    let (error, payload) = match UserDataHandle::instance().handle(request) {
        Ok(answer) => (ErrorCode::Ok, Some(response::Payload::UserData(answer))),
        Err(RequestError::Invalid(reason)) => {
            log::warn!("user-data request refused (id={id}): {reason}");
            (ErrorCode::FailInvariant, None)
        }
        Err(RequestError::Store(message)) => {
            log::error!("user-data store failed (id={id}): {message}");
            (ErrorCode::FailIo, None)
        }
    };
    Response {
        id,
        error: error as i32,
        payload,
    }
}

/// A composing request, answered with the open stores. `FetchAtPos` takes
/// the one fetch path (`crate::fetch`) over the stores' rows. Every other
/// request goes to composing, a Hanji conversion walk ranking with the
/// user's counts ([`StoredRows`]); the phrase a final commit taught (§50)
/// is written to `learned_phrases.db` here (P3c), then the pick a
/// `CommitContinuous` counts (R5) — after the request's own walk, so a pick
/// ranks the walks that follow it.
pub(super) fn handle_composing(
    stores: &UserDataStores,
    request: &ComposingRequest,
    config: &AppConfig,
    generation: u64,
) -> Result<ComposingResponse, ComposingError> {
    let rows = StoredRows { stores, config };
    if let Some(composing_request::Method::FetchAtPos(sent)) = request.method.as_ref() {
        return Ok(crate::fetch::fetch_at_pos(
            Some(&rows),
            sent,
            config,
            generation,
        ));
    }
    let applied = composing::EngineHandle::instance().handle_ranked(
        request,
        config,
        generation,
        Some(&rows),
    )?;
    if let Some(learned) = applied.learned {
        stores
            .learned_phrases
            .learn_phrase(&learned.hanji, &learned.canonical_tl);
    }
    if let Some(usage) = applied.usage {
        stores.record_usage(
            &usage.display_text,
            &usage.canonical_tl,
            usage.hanji.as_deref(),
            usage.abbreviation,
        );
    }
    Ok(applied.response)
}

/// The user data a fetch and a Hanji conversion walk rank with: the open
/// `user_frequency.db` at the wall clock (the composing engine keeps none),
/// the custom words (unless `config` turns the dictionary off) and learned
/// phrases for the listed buffer or filed under the walk's edges, and the
/// learned continuations of the previous word.
pub(super) struct StoredRows<'a> {
    pub(super) stores: &'a UserDataStores,
    pub(super) config: &'a AppConfig,
}

impl ConversionFrequency for StoredRows<'_> {
    fn rows_for_edge_keys(&self, edge_keys: &[String], mode: phonetics::InputMode) -> UserRows {
        let query_keys: Vec<_> = edge_keys
            .iter()
            .map(|body| phonetics::api::toneless_edge_key(body, mode))
            .collect();
        let custom = if self.config.custom_dictionary_disabled {
            Vec::new()
        } else {
            self.stores
                .custom_dictionary
                .rows_with_keys(&query_keys, CustomDictionaryStore::KEYSTROKE_LIMIT)
                .into_iter()
                .map(custom_entry)
                .collect()
        };
        let learned = self
            .stores
            .learned_phrases
            .rows_matching_any(&query_keys, LearnedPhraseStore::KEYSTROKE_LIMIT)
            .into_iter()
            .map(learned_entry)
            .collect();
        UserRows {
            custom,
            learned,
            ..UserRows::default()
        }
    }

    fn rows_for_words(&self, words: &[String]) -> FrequencyMap {
        self.stores
            .frequency
            .rows_for_words(words)
            .into_iter()
            .flatten()
            .map(frequency_entry)
            .collect()
    }

    fn now_ms(&self) -> i64 {
        userdata::unix_millis_now()
    }
}

impl UserRowSource for StoredRows<'_> {
    /// Keyed the way the platforms keyed them.
    fn buffer_rows(&self, raw: &str, config: &AppConfig) -> UserRows {
        let Some(key) = (!raw.is_empty())
            .then(|| phonetics::api::derive_custom_query_key(raw, &config.input_mode))
            .flatten()
        else {
            return UserRows::default();
        };
        let custom = if config.custom_dictionary_disabled {
            Vec::new()
        } else {
            self.stores
                .custom_dictionary
                .keystroke_rows(&key, CustomDictionaryStore::KEYSTROKE_LIMIT)
                .into_iter()
                .map(custom_entry)
                .collect()
        };
        let learned = self
            .stores
            .learned_phrases
            .rows_matching(&key, LearnedPhraseStore::KEYSTROKE_LIMIT)
            .into_iter()
            .map(learned_entry)
            .collect();
        UserRows {
            custom,
            learned,
            ..UserRows::default()
        }
    }

    /// The §24 Hanji-keyed recall of `user_association.db`.
    fn continuations(&self, word: &str, word_tl: &str, limit: usize) -> Vec<(String, String)> {
        self.stores
            .association
            .rows_following(word, word_tl, limit)
            .unwrap_or_default()
            .into_iter()
            .map(|row| (row.next, row.next_tl))
            .collect()
    }
}

/// A stored row as the `(word, canonical TL)`-keyed entry ranking reads:
/// the reading picks (every pick but the abbreviation ones) apart from the
/// abbreviation picks. A reading pick's recency is the row's `last_used`,
/// which an abbreviation pick also touches.
fn frequency_entry(row: FrequencyRow) -> (String, String, FrequencyData) {
    let data = FrequencyData {
        count: ranked_count(row.count - row.abbrev_count.max(0)),
        last_used_ms: row.last_used_ms,
        abbrev_count: ranked_count(row.abbrev_count),
        abbrev_last_used_ms: row.abbrev_last_used_ms,
    };
    (row.word, row.tl, data)
}

/// A stored count as ranking reads it: saturated at `i32::MAX`; a negative
/// one (never written) reads as 0.
fn ranked_count(count: i64) -> i32 {
    i32::try_from(count.max(0)).unwrap_or(i32::MAX)
}

/// A next-word request, answered with the open stores: `PredictNext` ranks
/// the rows the store holds for the word, and the bigrams a decision records
/// are written here — one decision's pairs in one transaction (P3b / P3c).
pub(super) fn handle_nextword(
    stores: &UserDataStores,
    request: NextWordRequest,
    config: &AppConfig,
    generation: u64,
) -> Result<NextWordResponse, NextWordError> {
    let user_rows = following_rows(stores, &request);
    let request = crate::predict::expand_predict_next(request, user_rows);
    let nextword::Handled {
        response,
        associations,
    } = nextword::EngineHandle::instance().handle_recording(&request, config, generation)?;
    let pairs: Vec<AssociationPair> = associations.into_iter().map(association_pair).collect();
    stores.association.record(&pairs);
    Ok(response)
}

/// The rows `user_association.db` holds after a `PredictNext` word; none for
/// any other request. Over-fetches twice the prediction limit, as the
/// platforms did, so the `(hanji, tl)` merge never leaves fewer than `limit`
/// survivors.
fn following_rows(
    stores: &UserDataStores,
    request: &NextWordRequest,
) -> Vec<RawNextWordPrediction> {
    let Some(next_word_request::Method::PredictNext(predict)) = request.method.as_ref() else {
        return Vec::new();
    };
    let limit = nextword::api::effective_prediction_limit(predict.limit) * 2;
    stores
        .association
        .rows_following(&predict.word, &predict.roman, limit)
        .unwrap_or_default()
        .iter()
        .map(user_prediction)
        .collect()
}

/// The custom entries a dictionary search finds (waits for a background
/// open to settle). None before the platform opens its user data.
pub(super) fn custom_entries_matching(
    search: &DictionarySearchRequest,
) -> Vec<CustomDictionaryEntry> {
    match UserDataHandle::instance().custom_entries_matching(
        &search.query,
        &search.input_mode,
        search.limit as usize,
    ) {
        Ok(entries) => entries,
        // The platform has not opened its user data yet.
        Err(RequestError::Invalid(reason)) => {
            log::debug!("dictionary search without custom entries: {reason}");
            Vec::new()
        }
        Err(RequestError::Store(message)) => {
            log::warn!("dictionary search: custom entries failed: {message}");
            Vec::new()
        }
    }
}

// The row → engine / wire forms.

/// An empty stored hanji is a romanization-only entry.
fn custom_entry(row: CustomDictionaryRow) -> lexicon::CustomEntry {
    lexicon::CustomEntry {
        roman: row.roman,
        hanji: (!row.hanji.is_empty()).then_some(row.hanji),
    }
}

/// A phrase learned before readings dropped their case may be stored
/// `Kóng`; it ranks by the `kóng` the rest of the engine keys it under. Case
/// only: the stored reading is already canonical TL, and a second fold would
/// move a dictionary tone mark (`hùi`).
fn learned_entry(phrase: LearnedPhraseRow) -> lexicon::LearnedEntry {
    lexicon::LearnedEntry {
        hanji: phrase.hanji,
        canonical_tl: phrase.canonical_tl.to_lowercase(),
    }
}

fn association_pair(pair: nextword::Association) -> AssociationPair {
    AssociationPair {
        previous: pair.previous,
        previous_tl: pair.previous_tl,
        next: pair.next,
        next_tl: pair.next_tl,
    }
}

fn user_prediction(row: &FollowingRow) -> RawNextWordPrediction {
    RawNextWordPrediction {
        hanji: row.next.clone(),
        tl: row.next_tl.clone(),
        count: row.count,
        last_used_ms: row.last_used_ms,
        source: Source::User as i32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use userdata::{CustomDictionaryRow, CustomDictionaryStore, JournalMode};

    fn buffer_rows(stores: &UserDataStores, raw: &str, config: &AppConfig) -> UserRows {
        StoredRows { stores, config }.buffer_rows(raw, config)
    }

    #[test]
    fn a_stored_count_saturates_for_ranking() {
        assert_eq!(ranked_count(7), 7);
        assert_eq!(ranked_count(i64::from(i32::MAX) + 1), i32::MAX);
        assert_eq!(ranked_count(-3), 0);
    }

    #[test]
    fn buffer_rows_hand_the_keystroke_path_at_most_its_limits() {
        // trace: 21 custom rows `tâi-uân` / 台灣0..20 and 6 learned phrases
        // `tâi-uân` / 台灣0..5 all key `taiuan` (tl); raw "taiuan" prefix-
        // matches the custom rows (KEYSTROKE_LIMIT 20) and equals the learned
        // key (KEYSTROKE_LIMIT 5).
        let directory = tempfile::tempdir().unwrap();
        let stores = UserDataStores::at(
            userdata::UserDataPaths::in_directory(directory.path()),
            JournalMode::Delete,
        );
        stores.open_blocking();
        for index in 0..21 {
            stores
                .custom_dictionary
                .upsert(&CustomDictionaryRow::new(
                    "tâi-uân",
                    &format!("台灣{index}"),
                ))
                .unwrap();
            if index < 6 {
                stores
                    .learned_phrases
                    .learn_phrase(&format!("台灣{index}"), "tâi-uân");
            }
        }
        // Flushes the queued learns.
        assert_eq!(stores.learned_phrases.all_rows().unwrap().len(), 6);
        let config = AppConfig {
            input_mode: "tl".into(),
            ..AppConfig::default()
        };

        let rows = buffer_rows(&stores, "taiuan", &config);
        assert_eq!(rows.custom.len(), CustomDictionaryStore::KEYSTROKE_LIMIT);
        assert_eq!(rows.custom.len(), 20);
        assert_eq!(rows.learned.len(), 5);

        let disabled_config = AppConfig {
            custom_dictionary_disabled: true,
            ..config.clone()
        };
        let disabled = buffer_rows(&stores, "taiuan", &disabled_config);
        assert!(disabled.custom.is_empty(), "the user turned it off");
        assert_eq!(
            disabled.learned.len(),
            5,
            "learning data is not the dictionary"
        );
    }

    #[test]
    fn a_conversion_walk_reads_the_rows_filed_under_its_edges() {
        // trace: 陳三 `Tân Sann` files `tps:notone` `ㄉㄢㄙㆩ`; 刀 `to` files
        // `ㄉㄜ` and its variant `ㄉㄛ`. A walk's edge bodies match whole — a
        // custom word anywhere in the walked text, not only one the whole
        // text prefixes — and the dictionary setting off reads no custom
        // rows; learned phrases are not the dictionary.
        let directory = tempfile::tempdir().unwrap();
        let stores = UserDataStores::at(
            userdata::UserDataPaths::in_directory(directory.path()),
            JournalMode::Delete,
        );
        stores.open_blocking();
        for (roman, hanji) in [("Tân Sann", "陳三"), ("to", "刀")] {
            stores
                .custom_dictionary
                .upsert(&CustomDictionaryRow::new(roman, hanji))
                .unwrap();
        }
        stores.learned_phrases.learn_phrase("陳三", "tân-sann");
        assert_eq!(stores.learned_phrases.all_rows().unwrap().len(), 1);
        let config = AppConfig {
            input_mode: "tps".into(),
            ..AppConfig::default()
        };
        let rows = |config: &AppConfig, keys: &[&str]| {
            let keys: Vec<String> = keys.iter().map(|key| (*key).to_owned()).collect();
            StoredRows {
                stores: &stores,
                config,
            }
            .rows_for_edge_keys(&keys, phonetics::InputMode::Tps)
        };
        let hanji = |rows: &UserRows| -> Vec<String> {
            rows.custom
                .iter()
                .filter_map(|entry| entry.hanji.clone())
                .collect()
        };

        let found = rows(&config, &["ㄍㄚ", "ㄉㄢㄙㆩ", "ㄉㄢ"]);
        assert_eq!(hanji(&found), vec!["陳三"]);
        assert_eq!(found.learned.len(), 1);
        assert_eq!(hanji(&rows(&config, &["ㄉㄛ"])), vec!["刀"]);
        assert_eq!(
            hanji(&rows(&config, &["ㄉㄜ", "ㄉㄛ"])),
            vec!["刀"],
            "one entry once"
        );

        let disabled = AppConfig {
            custom_dictionary_disabled: true,
            ..config.clone()
        };
        let without_custom = rows(&disabled, &["ㄉㄢㄙㆩ"]);
        assert!(without_custom.custom.is_empty());
        assert_eq!(without_custom.learned.len(), 1);
    }

    #[test]
    fn buffer_rows_hand_over_an_abbreviation_only_when_typed_whole() {
        // trace: 陳建中 `Tân Kiàn-tiong` keys tl notone `tankiantiong`, tl
        // abbrev `tkt`. The store pre-filter only; the reach rule itself is
        // the lexicon's (`custom_reach`).
        let directory = tempfile::tempdir().unwrap();
        let stores = UserDataStores::at(
            userdata::UserDataPaths::in_directory(directory.path()),
            JournalMode::Delete,
        );
        stores.open_blocking();
        stores
            .custom_dictionary
            .upsert(&CustomDictionaryRow::new("Tân Kiàn-tiong", "陳建中"))
            .unwrap();
        let config = AppConfig {
            input_mode: "tl".into(),
            ..AppConfig::default()
        };
        let custom_count = |raw: &str| buffer_rows(&stores, raw, &config).custom.len();
        assert_eq!(custom_count("tkt"), 1);
        assert_eq!(custom_count("tk"), 0);
        assert_eq!(custom_count("tankiant"), 1);
    }
}
