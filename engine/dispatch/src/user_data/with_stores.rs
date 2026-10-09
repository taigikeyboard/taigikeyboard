//! The `user-data` build's half of `super`: the user-data request's wire
//! answer, and the composing / next-word requests answered with the stores
//! the platform opened. The stores, their one open per process and the
//! requests' logic are `userdata`'s (`UserDataHandle`); this module maps its
//! answer to the wire's error code and joins the stores to `composing` /
//! `nextword`, which only `dispatch` sees together.

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use composing::api::ComposingError;
use composing::{ConversionFrequency, PendingSnapshot, UserRows};
use nextword::NextWordError;
use protos::engine::{
    composing_request, next_word_request, response, AppConfig, ComposingRequest, ComposingResponse,
    ErrorCode, NextWordRequest, NextWordResponse, RawNextWordPrediction, Response, Source,
    UserDataRequest,
};
use ranking::{ContextRanks, FrequencyData, FrequencyMap, CONTEXT_RANK_USER};
use userdata::{
    AssociationPair, CustomDictionaryCSV, CustomDictionaryRow, CustomDictionaryStore, FollowingRow,
    FrequencyRow, LearnedPhraseRow, LearnedPhraseStore, RequestError, UserDataHandle,
    UserDataStores,
};

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

/// A composing request, answered with the open stores. `FetchAtPos` runs
/// two passes in-process (roadmap P3b, brainstorm R5): the custom and
/// learned rows for the listed buffer, a neutral fetch that discovers the
/// candidates, their frequency rows, and a re-ranked fetch — both passes
/// answered by one copy of the engine as of the request, so the rows read
/// for its buffer and list start rank that buffer and list. Every other
/// request goes to composing, a Hanji conversion walk ranking with the
/// user's counts ([`StoredFrequency`]); the phrase a final commit taught
/// (§50) is written to `learned_phrases.db` here (P3c), then the pick a
/// `CommitContinuous` counts (R5) — after the request's own walk, so a pick
/// ranks the walks that follow it.
pub(super) fn handle_composing(
    stores: &UserDataStores,
    request: &ComposingRequest,
    config: &AppConfig,
    generation: u64,
) -> Result<ComposingResponse, ComposingError> {
    let composing = composing::EngineHandle::instance();
    let Some(composing_request::Method::FetchAtPos(sent)) = request.method.as_ref() else {
        let frequency = StoredFrequency { stores };
        let applied = composing.handle_ranked(request, config, generation, Some(&frequency))?;
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
            );
        }
        return Ok(applied.response);
    };
    // The main thread keeps typing while a worker fetches: the passes answer
    // from one copy of the engine, so rows read for one buffer never rank
    // another. A stale generation answers the idle snapshot.
    let Some(engine) = composing.engine_at(generation) else {
        return Ok(composing::Engine::idle_snapshot(config));
    };
    let snapshot = engine.pending_snapshot(config);
    let mut rows = buffer_rows(
        stores,
        &snapshot.listed_raw,
        config,
        sent.custom_dictionary_disabled,
    );
    let context = context_ranks(stores, &snapshot, sent.now_ms);
    let fetch = |rows, context| {
        composing::requests::query(
            &composing::requests::fetch_at_pos_intent(sent, rows, context),
            &engine,
            config,
        )
    };
    let neutral = fetch(rows.clone(), context.clone());
    let Some(frequency) = frequency_map(stores, &neutral) else {
        return Ok(neutral);
    };
    rows.frequency = frequency;
    Ok(fetch(rows, context))
}

/// The learned counts a Hanji conversion walk ranks with: the open
/// `user_frequency.db`, at the wall clock (the composing engine keeps none).
struct StoredFrequency<'a> {
    stores: &'a UserDataStores,
}

impl ConversionFrequency for StoredFrequency<'_> {
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
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| {
                i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
            })
    }
}

/// The continuations of the word the pending tail follows (§56): the
/// user's learned bigrams (`user_association.db`, the §24 Hanji-keyed
/// recall, best evidence first) over the bundled ones.
fn context_ranks(stores: &UserDataStores, snapshot: &PendingSnapshot, now_ms: i64) -> ContextRanks {
    let previous = crate::context::context_word(snapshot, now_ms);
    let mut ranks = crate::context::bundled_ranks(previous.as_ref());
    if let Some((word, word_tl)) = previous {
        let rows = stores
            .association
            .rows_following(&word, &word_tl, crate::context::CONTEXT_ROWS)
            .unwrap_or_default();
        for row in rows {
            ranks.insert(row.next, row.next_tl, CONTEXT_RANK_USER);
        }
    }
    ranks
}

/// The custom-dictionary rows (unless the user turned the dictionary off)
/// and the learned phrases for `raw`, keyed the way the platforms keyed
/// them — no frequency rows yet.
fn buffer_rows(
    stores: &UserDataStores,
    raw: &str,
    config: &AppConfig,
    custom_dictionary_disabled: bool,
) -> UserRows {
    let Some(key) = (!raw.is_empty())
        .then(|| phonetics::api::derive_custom_query_key(raw, &config.input_mode))
        .flatten()
    else {
        return UserRows::default();
    };
    let custom = if custom_dictionary_disabled {
        Vec::new()
    } else {
        stores
            .custom_dictionary
            .rows_matching(&key, CustomDictionaryStore::KEYSTROKE_LIMIT)
            .into_iter()
            .map(custom_entry)
            .collect()
    };
    let learned = stores
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

/// The learned counts for the candidates `neutral` offers, deduped by the
/// key the engine ranks on; `None` when there is nothing to re-rank with.
fn frequency_map(stores: &UserDataStores, neutral: &ComposingResponse) -> Option<FrequencyMap> {
    let mut seen = HashSet::new();
    let words: Vec<String> = neutral
        .continuous
        .iter()
        .flat_map(|continuous| &continuous.candidates)
        .map(|candidate| candidate.display_text.as_str())
        .filter(|word| seen.insert(*word))
        .map(str::to_owned)
        .collect();
    if words.is_empty() {
        return None;
    }
    let rows = stores.frequency.rows_for_words(&words)?;
    if rows.is_empty() {
        return None;
    }
    Some(rows.into_iter().map(frequency_entry).collect())
}

/// A stored row as the `(word, canonical TL)`-keyed entry ranking reads.
fn frequency_entry(row: FrequencyRow) -> (String, String, FrequencyData) {
    let data = FrequencyData {
        count: ranked_count(row.count),
        last_used_ms: row.last_used_ms,
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

// The row → engine / wire forms.

/// An empty stored hanji is a romanization-only entry.
fn custom_entry(row: CustomDictionaryRow) -> lexicon::CustomEntry {
    lexicon::CustomEntry {
        roman: row.roman,
        hanji: (!row.hanji.is_empty()).then_some(row.hanji),
    }
}

fn learned_entry(phrase: LearnedPhraseRow) -> lexicon::LearnedEntry {
    lexicon::LearnedEntry {
        hanji: phrase.hanji,
        canonical_tl: phrase.canonical_tl,
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

        let rows = buffer_rows(&stores, "taiuan", &config, false);
        assert_eq!(rows.custom.len(), CustomDictionaryStore::KEYSTROKE_LIMIT);
        assert_eq!(rows.custom.len(), 20);
        assert_eq!(rows.learned.len(), 5);

        let disabled = buffer_rows(&stores, "taiuan", &config, true);
        assert!(disabled.custom.is_empty(), "the user turned it off");
        assert_eq!(
            disabled.learned.len(),
            5,
            "learning data is not the dictionary"
        );
    }
}
