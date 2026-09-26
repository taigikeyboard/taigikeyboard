//! The user-data stores, opened once per process on the platform's word
//! (`UserDataRequest.open`) and reset on its word (`.reset`). What they hold
//! is the engine's (`.claude/rules/rust-migration-policy.md` §6); where the
//! files live is the platform's. Plan: `docs/architecture/user-data-engine-roadmap.md`.

use std::collections::HashSet;
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::OnceLock;

use composing::api::ComposingError;
use nextword::NextWordError;
use protos::engine::{
    composing_request, effect, next_word_effect, next_word_request, next_word_response, response,
    user_data_request, user_data_response, AppConfig, ComposingRequest, ComposingResponse,
    CustomCsvExported, CustomCsvImported, CustomDictEntry, CustomDictionaryEntry,
    CustomDictionaryRefusal, CustomEntries, CustomEntryDeleted, CustomEntrySaved, ErrorCode,
    FetchAtPos, FrequencyEntry, ImportCustomCsv, LearnedEntry, ListCustomEntries, NextWordRequest,
    NextWordResponse, OpenUserData, RawNextWordPrediction, RecordUsage, ResetUserData, Response,
    SaveCustomEntry, Source, UsageRecorded, UserDataJournal, UserDataOpened, UserDataRequest,
    UserDataReset, UserDataResponse,
};
use userdata::{
    AssociationPair, CustomDictionaryCSV, CustomDictionaryCSVError, CustomDictionaryError,
    CustomDictionaryRow, CustomDictionarySource, CustomDictionaryStore, CustomEntry, FollowingRow,
    FrequencyRow, JournalMode, LearnedPhrase, LearnedPhraseSource, UserDataPaths, UserDataStores,
};

/// Why a user-data request did nothing.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum UserDataError {
    /// The request itself is wrong: no method, a relative or empty path, a
    /// second open at other paths or journal, a reset of nothing, a reset
    /// before open.
    Invalid(&'static str),
    /// A store could not do what was asked — its message, logged.
    Store(String),
}

/// Answers one user-data request — the `user-data` arm of `crate::run`.
pub(crate) fn respond(id: u32, generation: u64, request: &UserDataRequest) -> Response {
    let (error, payload) = match UserDataHandle::instance().handle(request) {
        Ok(answer) => (ErrorCode::Ok, Some(response::Payload::UserData(answer))),
        Err(UserDataError::Invalid(reason)) => {
            log::warn!("user-data request refused (id={id}): {reason}");
            (ErrorCode::FailInvariant, None)
        }
        Err(UserDataError::Store(message)) => {
            log::error!("user-data store failed (id={id}): {message}");
            (ErrorCode::FailIo, None)
        }
    };
    Response {
        id,
        error: error as i32,
        generation,
        payload,
    }
}

/// Where and how the stores were opened, and the stores.
struct Opened {
    paths: UserDataPaths,
    journal: JournalMode,
    stores: UserDataStores,
}

/// The stores this process opened. Set once, then read without a lock: from
/// roadmap P3b every keystroke reads through [`UserDataHandle::stores`], and
/// must never wait behind an open (its takeover can take seconds).
pub(crate) struct UserDataHandle {
    opened: OnceLock<Opened>,
}

impl UserDataHandle {
    pub(crate) const fn new() -> Self {
        Self {
            opened: OnceLock::new(),
        }
    }

    /// The process's one handle.
    pub(crate) fn instance() -> &'static Self {
        static HANDLE: UserDataHandle = UserDataHandle::new();
        &HANDLE
    }

    pub(crate) fn handle(
        &self,
        request: &UserDataRequest,
    ) -> Result<UserDataResponse, UserDataError> {
        let result = match request.method.as_ref() {
            Some(user_data_request::Method::Open(open)) => {
                user_data_response::Result::Opened(self.open(open)?)
            }
            Some(user_data_request::Method::Reset(reset)) => {
                user_data_response::Result::Reset(self.reset(reset)?)
            }
            Some(user_data_request::Method::RecordUsage(usage)) => {
                self.record_usage(usage)?;
                user_data_response::Result::UsageRecorded(UsageRecorded {})
            }
            Some(user_data_request::Method::ListCustomEntries(list)) => {
                user_data_response::Result::CustomEntries(self.list_custom_entries(list)?)
            }
            Some(user_data_request::Method::SaveCustomEntry(save)) => {
                user_data_response::Result::CustomEntrySaved(self.save_custom_entry(save)?)
            }
            Some(user_data_request::Method::DeleteCustomEntry(delete)) => {
                let removed = self
                    .opened_stores()?
                    .custom_dictionary
                    .delete(&delete.id)
                    .map_err(store_error)?;
                user_data_response::Result::CustomEntryDeleted(CustomEntryDeleted { removed })
            }
            Some(user_data_request::Method::ImportCustomCsv(import)) => {
                user_data_response::Result::CustomCsvImported(self.import_custom_csv(import)?)
            }
            Some(user_data_request::Method::ExportCustomCsv(_)) => {
                user_data_response::Result::CustomCsvExported(self.export_custom_csv()?)
            }
            None => return Err(UserDataError::Invalid("user-data request has no method")),
        };
        Ok(UserDataResponse {
            result: Some(result),
        })
    }

    /// The open stores, for the engine's own reads and writes; `None` until
    /// the platform opens them. Never blocks.
    pub(crate) fn stores(&self) -> Option<&UserDataStores> {
        self.opened.get().map(|opened| &opened.stores)
    }

    fn open(&self, open: &OpenUserData) -> Result<UserDataOpened, UserDataError> {
        let paths = UserDataPaths {
            frequency: absolute(&open.frequency_path)?,
            association: absolute(&open.association_path)?,
            custom_dictionary: absolute(&open.custom_dictionary_path)?,
            learned_phrases: absolute(&open.learned_phrases_path)?,
        };
        let journal = journal(open.journal());
        // A second open waits for the first to finish rather than opening
        // the same files twice.
        let opened = self.opened.get_or_init(|| {
            let stores = UserDataStores::at(paths.clone(), journal);
            stores.open_blocking();
            let ready = readiness(&stores);
            log::info!(
                "user_data.open frequency={} association={} custom_dictionary={} learned_phrases={}",
                ready.frequency_ready,
                ready.association_ready,
                ready.custom_dictionary_ready,
                ready.learned_phrases_ready
            );
            Opened {
                paths: paths.clone(),
                journal,
                stores,
            }
        });
        if opened.paths != paths || opened.journal != journal {
            return Err(UserDataError::Invalid(
                "user data is already open at other paths or with another journal",
            ));
        }
        Ok(readiness(&opened.stores))
    }

    /// The open stores, or the refusal a request before the open gets.
    fn opened_stores(&self) -> Result<&UserDataStores, UserDataError> {
        self.stores()
            .ok_or(UserDataError::Invalid("user data is not open yet"))
    }

    fn list_custom_entries(
        &self,
        list: &ListCustomEntries,
    ) -> Result<CustomEntries, UserDataError> {
        let dictionary = &self.opened_stores()?.custom_dictionary;
        let entries = dictionary
            .rows(&list.filter, list.limit as usize, list.offset as usize)
            .map_err(store_error)?;
        let total = dictionary.count().map_err(store_error)?;
        let matching_total = if list.filter.trim().is_empty() {
            total
        } else {
            dictionary
                .count_matching(&list.filter)
                .map_err(store_error)?
        };
        Ok(CustomEntries {
            entries: entries.iter().map(custom_dictionary_entry).collect(),
            total: u32::try_from(total).unwrap_or(u32::MAX),
            matching_total: u32::try_from(matching_total).unwrap_or(u32::MAX),
        })
    }

    /// A new word (no `id`) or an edit. What the user can be told is a
    /// refusal in the answer; only a store failure is an error.
    fn save_custom_entry(&self, save: &SaveCustomEntry) -> Result<CustomEntrySaved, UserDataError> {
        let dictionary = &self.opened_stores()?.custom_dictionary;
        let refused = |refusal: CustomDictionaryRefusal| CustomEntrySaved {
            refusal: refusal as i32,
            entry: None,
        };
        let roman = save.roman.trim();
        if roman.is_empty() {
            return Ok(refused(CustomDictionaryRefusal::EmptyRoman));
        }
        let hanzi = save.hanzi.trim();
        let row = match save.id.as_deref() {
            Some(id) if !id.is_empty() => CustomDictionaryRow::with_id(id, roman, hanzi),
            _ => CustomDictionaryRow::new(roman, hanzi),
        };
        if let Err(error) = dictionary.upsert(&row) {
            return match refusal(&error) {
                Some(refusal) => Ok(refused(refusal)),
                None => Err(store_error(error)),
            };
        }
        // Read back: the store stamps the times (an edit keeps `created_at`).
        let stored = dictionary.row(&row.id).map_err(store_error)?;
        Ok(CustomEntrySaved {
            refusal: CustomDictionaryRefusal::None as i32,
            entry: stored.as_ref().map(custom_dictionary_entry),
        })
    }

    fn import_custom_csv(
        &self,
        import: &ImportCustomCsv,
    ) -> Result<CustomCsvImported, UserDataError> {
        let dictionary = &self.opened_stores()?.custom_dictionary;
        let refused = |refusal: CustomDictionaryRefusal| CustomCsvImported {
            refusal: refusal as i32,
            ..CustomCsvImported::default()
        };
        let rows = match CustomDictionaryCSV::decode_bytes(
            &import.csv,
            CustomDictionaryStore::MAX_ENTRIES,
        ) {
            Ok(rows) => rows,
            Err(error) => {
                return match csv_refusal(&error) {
                    Some(refusal) => Ok(refused(refusal)),
                    None => Err(store_error(error)),
                }
            }
        };
        match dictionary.batch_import(&rows) {
            Ok(result) => Ok(CustomCsvImported {
                refusal: CustomDictionaryRefusal::None as i32,
                imported: u32::try_from(result.imported).unwrap_or(u32::MAX),
                skipped: u32::try_from(result.skipped).unwrap_or(u32::MAX),
            }),
            Err(error) => match refusal(&error) {
                Some(refusal) => Ok(refused(refusal)),
                None => Err(store_error(error)),
            },
        }
    }

    fn export_custom_csv(&self) -> Result<CustomCsvExported, UserDataError> {
        let rows = self
            .opened_stores()?
            .custom_dictionary
            .all_rows()
            .map_err(store_error)?;
        Ok(CustomCsvExported {
            csv: CustomDictionaryCSV::encode(&rows).into_bytes(),
        })
    }

    /// One commit counted and, for a Hanji pick, a learned phrase touched —
    /// what each platform's candidate / prediction tap handler did itself.
    fn record_usage(&self, usage: &RecordUsage) -> Result<(), UserDataError> {
        let stores = self.opened_stores()?;
        if usage.display_text.is_empty() {
            return Err(UserDataError::Invalid("usage without a display text"));
        }
        if !usage.frequency_recording_disabled {
            stores
                .frequency
                .record(&usage.display_text, &usage.canonical_tl);
        }
        if let Some(hanji) = &usage.hanji {
            stores
                .learned_phrases
                .touch_phrase(hanji, &usage.canonical_tl);
        }
        Ok(())
    }

    fn reset(&self, reset: &ResetUserData) -> Result<UserDataReset, UserDataError> {
        if !(reset.frequency
            || reset.association
            || reset.custom_dictionary
            || reset.learned_phrases)
        {
            return Err(UserDataError::Invalid("reset selects no store"));
        }
        let stores = self.opened_stores()?;
        let mut removed = UserDataReset::default();
        if reset.frequency {
            removed.frequency_removed = stores.frequency.delete_all().map_err(store_error)?;
        }
        if reset.association {
            removed.association_removed = stores.association.delete_all().map_err(store_error)?;
        }
        if reset.custom_dictionary {
            let count = stores.custom_dictionary.delete_all().map_err(store_error)?;
            removed.custom_dictionary_removed = i64::try_from(count).unwrap_or(i64::MAX);
        }
        if reset.learned_phrases {
            removed.learned_phrases_removed =
                stores.learned_phrases.delete_all().map_err(store_error)?;
        }
        Ok(removed)
    }
}

/// A composing request, answered from the engine's own user data once the
/// platform opened it. `FetchAtPos` then runs the two passes every platform
/// ran over the FFI (roadmap P3b, brainstorm R5) — in-process: the custom
/// and learned rows for the pending buffer, a neutral fetch that discovers
/// the candidates, their frequency rows, and a re-ranked fetch. The rows a
/// platform still sends are replaced, never merged (U9). Every other
/// request goes to composing, and the phrases it decides to learn are
/// written here (P3c). Before the open, everything goes straight to composing.
pub(crate) fn handle_composing(
    request: &ComposingRequest,
    config: &AppConfig,
    generation: u64,
) -> Result<ComposingResponse, ComposingError> {
    let composing = composing::EngineHandle::instance();
    let Some(stores) = UserDataHandle::instance().stores() else {
        return composing.handle(request, config, generation);
    };
    let Some(composing_request::Method::FetchAtPos(sent)) = request.method.as_ref() else {
        let mut response = composing.handle(request, config, generation)?;
        persist_learned_phrases(stores, &mut response);
        return Ok(response);
    };
    // The platform's settings for this fetch; its rows are dropped (U9).
    let mut fetch = FetchAtPos {
        now_ms: sent.now_ms,
        enabled_sources_bitmask: sent.enabled_sources_bitmask,
        literal_roman_candidate_disabled: sent.literal_roman_candidate_disabled,
        custom_dictionary_disabled: sent.custom_dictionary_disabled,
        ..FetchAtPos::default()
    };
    // The buffer can grow between reading it and fetching (the main thread
    // keeps typing while a worker fetches): rows chosen for one buffer must
    // not rank another, so a fetch that answers for a different buffer is
    // redone once with that buffer's rows.
    let mut neutral = None;
    for _ in 0..2 {
        // A stale generation answers the idle snapshot inside `handle`.
        let Some(raw) = composing.pending_raw(generation) else {
            return composing.handle(request, config, generation);
        };
        (fetch.custom_entries, fetch.learned_entries) = user_rows(stores, &raw, config, &fetch);
        let answer = composing.handle(&fetch_request(&fetch), config, generation)?;
        let answered_for = answer.preedit.as_ref().map(|p| p.raw_input.as_str());
        let current = answered_for.unwrap_or("") == raw;
        neutral = Some(answer);
        if current {
            break;
        }
    }
    let neutral = neutral.expect("the loop fetches at least once");
    let Some(frequency_entries) = frequency_entries(stores, &neutral) else {
        return Ok(neutral);
    };
    fetch.frequency_entries = frequency_entries;
    // A failed re-rank keeps the neutral answer, as the platforms did.
    Ok(composing
        .handle(&fetch_request(&fetch), config, generation)
        .unwrap_or(neutral))
}

/// Writes the phrases the engine decided to learn (§50) into
/// `learned_phrases.db` and takes their effects out of the response: "the
/// engine decides, the platform persists" became "the engine persists" once
/// the platform opened the stores (roadmap P3c) — a platform must not learn
/// them a second time.
fn persist_learned_phrases(stores: &UserDataStores, response: &mut ComposingResponse) {
    response.effect.retain(|effect| match &effect.kind {
        Some(effect::Kind::PhraseLearned(learned)) => {
            stores
                .learned_phrases
                .learn_phrase(&learned.hanji, &learned.canonical_tl);
            false
        }
        _ => true,
    });
}

/// Writes the bigrams the next-word engine decided to record into
/// `user_association.db` — a compound word's pairs in one transaction, as
/// the desktop did — and takes those effects out of the response (P3c).
fn persist_associations(mut response: NextWordResponse) -> NextWordResponse {
    let Some(stores) = UserDataHandle::instance().stores() else {
        return response;
    };
    let Some(next_word_response::Result::Decide(decide)) = response.result.as_mut() else {
        return response;
    };
    decide.effects.retain(|effect| {
        let pairs: Vec<AssociationPair> = match &effect.kind {
            Some(next_word_effect::Kind::RecordAssociation(record)) => {
                record.pair.iter().map(association_pair).collect()
            }
            Some(next_word_effect::Kind::RecordCompoundAssociations(record)) => {
                record.pairs.iter().map(association_pair).collect()
            }
            _ => return true,
        };
        stores.association.record(&pairs);
        false
    });
    response
}

/// The custom-dictionary rows (unless the user turned the dictionary off)
/// and the learned phrases for `raw`, keyed the way the platforms keyed them.
fn user_rows(
    stores: &UserDataStores,
    raw: &str,
    config: &AppConfig,
    fetch: &FetchAtPos,
) -> (Vec<CustomDictEntry>, Vec<LearnedEntry>) {
    let Some(key) = (!raw.is_empty())
        .then(|| userdata::derive_custom_query_key(raw, &config.input_mode))
        .flatten()
    else {
        return (Vec::new(), Vec::new());
    };
    let custom = if fetch.custom_dictionary_disabled {
        Vec::new()
    } else {
        CustomDictionarySource::rows_matching(
            &stores.custom_dictionary,
            &key.family,
            &key.form,
            &key.key,
        )
        .iter()
        .map(custom_dict_entry)
        .collect()
    };
    let learned = LearnedPhraseSource::rows_matching(
        &stores.learned_phrases,
        &key.family,
        &key.form,
        &key.key,
    )
    .iter()
    .map(learned_entry)
    .collect();
    (custom, learned)
}

/// The learned counts for the candidates `neutral` offers, deduped by the
/// key the engine ranks on; `None` when there is nothing to re-rank with.
fn frequency_entries(
    stores: &UserDataStores,
    neutral: &ComposingResponse,
) -> Option<Vec<FrequencyEntry>> {
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
    (!rows.is_empty()).then(|| rows.iter().map(frequency_entry).collect())
}

fn fetch_request(fetch: &FetchAtPos) -> ComposingRequest {
    ComposingRequest {
        method: Some(composing_request::Method::FetchAtPos(fetch.clone())),
    }
}

/// A next-word request, with the engine's own user data once the platform
/// opened it: `PredictNext` reads its user rows from the store, and the
/// associations the decision records are written here (P3b / P3c).
pub(crate) fn handle_nextword(
    request: NextWordRequest,
    config: &AppConfig,
    generation: u64,
) -> Result<NextWordResponse, NextWordError> {
    let request = crate::predict::expand_predict_next(with_user_rows(request));
    nextword::EngineHandle::instance()
        .handle(&request, config, generation)
        .map(persist_associations)
}

/// A next-word request with its user rows read from the engine's own
/// `user_association.db` once the platform opened it — replacing, never
/// merging, whatever `PredictNext.user_rows` a platform still sends (U9).
/// Over-fetches twice the prediction limit, as the platforms did, so the
/// `(hanzi, tl)` merge never leaves fewer than `limit` survivors.
fn with_user_rows(mut request: NextWordRequest) -> NextWordRequest {
    let Some(stores) = UserDataHandle::instance().stores() else {
        return request;
    };
    if let Some(next_word_request::Method::PredictNext(predict)) = request.method.as_mut() {
        let limit = nextword::api::effective_prediction_limit(predict.limit) * 2;
        predict.user_rows = stores
            .association
            .rows_following(&predict.word, &predict.roman, limit)
            .unwrap_or_default()
            .iter()
            .map(user_prediction)
            .collect();
    }
    request
}

// The row → wire forms, as the platforms marshalled them.

/// A count past `u32` saturates; a negative one (never written) reads as 0.
fn frequency_entry(row: &FrequencyRow) -> FrequencyEntry {
    FrequencyEntry {
        display_text_key: row.word.clone(),
        count: u32::try_from(row.count.max(0)).unwrap_or(u32::MAX),
        last_used_ms: row.last_used_ms,
        canonical_tl: row.tl.clone(),
    }
}

/// An empty stored hanzi is a romanization-only entry: an ABSENT wire field.
fn custom_dict_entry(row: &CustomEntry) -> CustomDictEntry {
    CustomDictEntry {
        roman: row.roman.clone(),
        hanji: (!row.hanzi.is_empty()).then(|| row.hanzi.clone()),
    }
}

fn learned_entry(phrase: &LearnedPhrase) -> LearnedEntry {
    LearnedEntry {
        hanji: phrase.hanzi.clone(),
        canonical_tl: phrase.canonical_tl.clone(),
    }
}

fn custom_dictionary_entry(row: &CustomDictionaryRow) -> CustomDictionaryEntry {
    CustomDictionaryEntry {
        id: row.id.clone(),
        roman: row.roman.clone(),
        hanzi: row.hanzi.clone(),
        created_at: row.created_at.clone(),
        updated_at: row.updated_at.clone(),
    }
}

fn association_pair(pair: &protos::engine::AssociationPair) -> AssociationPair {
    AssociationPair {
        previous: pair.prev.clone(),
        previous_tl: pair.prev_tl.clone(),
        next: pair.next.clone(),
        next_tl: pair.next_tl.clone(),
    }
}

fn user_prediction(row: &FollowingRow) -> RawNextWordPrediction {
    RawNextWordPrediction {
        hanzi: row.next.clone(),
        tl: row.next_tl.clone(),
        count: row.count,
        last_used_ms: row.last_used_ms,
        source: Source::User as i32,
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

/// The CSV outcomes the user can be told about; `None` for a read failure
/// (only a file path can fail to read, never the bytes a platform hands in).
fn csv_refusal(error: &CustomDictionaryCSVError) -> Option<CustomDictionaryRefusal> {
    match error {
        CustomDictionaryCSVError::FileTooLarge { .. } => {
            Some(CustomDictionaryRefusal::FileTooLarge)
        }
        CustomDictionaryCSVError::NotUtf8 => Some(CustomDictionaryRefusal::NotUtf8),
        CustomDictionaryCSVError::NoUsableRows => Some(CustomDictionaryRefusal::NoUsableRows),
        CustomDictionaryCSVError::TooManyRows { .. } => Some(CustomDictionaryRefusal::Full),
        CustomDictionaryCSVError::Read(_) => None,
    }
}

fn store_error(error: impl Display) -> UserDataError {
    UserDataError::Store(error.to_string())
}

fn absolute(path: &str) -> Result<PathBuf, UserDataError> {
    let path = PathBuf::from(path);
    if path.is_absolute() {
        Ok(path)
    } else {
        Err(UserDataError::Invalid(
            "user-data paths must be absolute and non-empty",
        ))
    }
}

fn journal(journal: UserDataJournal) -> JournalMode {
    match journal {
        UserDataJournal::Wal => JournalMode::Wal,
        UserDataJournal::Delete => JournalMode::Delete,
    }
}

fn readiness(stores: &UserDataStores) -> UserDataOpened {
    UserDataOpened {
        frequency_ready: stores.frequency.is_ready(),
        association_ready: stores.association.is_ready(),
        custom_dictionary_ready: stores.custom_dictionary.is_ready(),
        learned_phrases_ready: stores.learned_phrases.is_ready(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protos::engine::{DeleteCustomEntry, ExportCustomCsv};

    fn open_request(directory: &std::path::Path) -> UserDataRequest {
        let paths = UserDataPaths::in_directory(directory);
        UserDataRequest {
            method: Some(user_data_request::Method::Open(OpenUserData {
                frequency_path: paths.frequency.display().to_string(),
                association_path: paths.association.display().to_string(),
                custom_dictionary_path: paths.custom_dictionary.display().to_string(),
                learned_phrases_path: paths.learned_phrases.display().to_string(),
                journal: UserDataJournal::Delete as i32,
            })),
        }
    }

    fn reset_request(reset: ResetUserData) -> UserDataRequest {
        UserDataRequest {
            method: Some(user_data_request::Method::Reset(reset)),
        }
    }

    fn opened(response: UserDataResponse) -> UserDataOpened {
        match response.result {
            Some(user_data_response::Result::Opened(opened)) => opened,
            other => panic!("expected Opened, got {other:?}"),
        }
    }

    #[test]
    fn open_readies_every_store_and_seeds_the_custom_dictionary() {
        let directory = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();

        let answer = opened(handle.handle(&open_request(directory.path())).unwrap());

        assert!(answer.frequency_ready);
        assert!(answer.association_ready);
        assert!(answer.custom_dictionary_ready);
        assert!(answer.learned_phrases_ready);
        let stores = handle.stores().unwrap();
        assert_eq!(
            stores.custom_dictionary.count().unwrap(),
            2,
            "seeded before the answer"
        );
    }

    #[test]
    fn a_repeat_open_at_the_same_paths_answers_and_other_paths_are_refused() {
        let directory = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();
        handle.handle(&open_request(directory.path())).unwrap();

        assert!(handle.handle(&open_request(directory.path())).is_ok());
        assert_eq!(
            handle.handle(&open_request(elsewhere.path())).unwrap_err(),
            UserDataError::Invalid(
                "user data is already open at other paths or with another journal"
            )
        );
    }

    #[test]
    fn a_repeat_open_with_another_journal_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();
        handle.handle(&open_request(directory.path())).unwrap();
        let mut wal = open_request(directory.path());
        if let Some(user_data_request::Method::Open(open)) = wal.method.as_mut() {
            open.journal = UserDataJournal::Wal as i32;
        }

        assert!(matches!(
            handle.handle(&wal),
            Err(UserDataError::Invalid(_))
        ));
    }

    #[test]
    fn a_relative_path_is_refused_before_anything_opens() {
        let handle = UserDataHandle::new();
        let mut request = open_request(std::path::Path::new("/tmp"));
        if let Some(user_data_request::Method::Open(open)) = request.method.as_mut() {
            open.frequency_path = "user_frequency.db".into();
        }

        assert!(matches!(
            handle.handle(&request),
            Err(UserDataError::Invalid(_))
        ));
        assert!(handle.stores().is_none());
    }

    #[test]
    fn reset_empties_only_the_selected_stores() {
        let directory = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();
        handle.handle(&open_request(directory.path())).unwrap();
        let stores = handle.stores().unwrap();
        stores.frequency.record("台", "tâi");

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
            }
            other => panic!("expected Reset, got {other:?}"),
        }
        assert_eq!(
            stores.custom_dictionary.count().unwrap(),
            2,
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
            UserDataError::Invalid("user data is not open yet")
        );
        assert_eq!(
            handle
                .handle(&reset_request(ResetUserData::default()))
                .unwrap_err(),
            UserDataError::Invalid("reset selects no store")
        );
    }

    #[test]
    fn a_learned_phrase_effect_is_written_and_left_out_of_the_response() {
        use protos::engine::{Effect, PhraseLearned, ResetAutocomplete};

        let directory = tempfile::tempdir().unwrap();
        let handle = UserDataHandle::new();
        handle.handle(&open_request(directory.path())).unwrap();
        let stores = handle.stores().unwrap();
        let mut response = ComposingResponse {
            effect: vec![
                Effect {
                    kind: Some(effect::Kind::PhraseLearned(PhraseLearned {
                        hanji: "做進出口".into(),
                        canonical_tl: "tsò tsìn-tshut-kháu".into(),
                    })),
                },
                Effect {
                    kind: Some(effect::Kind::ResetAutocomplete(ResetAutocomplete {})),
                },
            ],
            ..ComposingResponse::default()
        };

        persist_learned_phrases(stores, &mut response);

        assert_eq!(response.effect.len(), 1, "the document effect stays");
        assert!(matches!(
            response.effect[0].kind,
            Some(effect::Kind::ResetAutocomplete(_))
        ));
        let learned = stores.learned_phrases.all_rows().unwrap();
        assert_eq!(learned.len(), 1);
        assert_eq!(learned[0].hanzi, "做進出口");
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
        hanzi: &str,
    ) -> CustomEntrySaved {
        match call(
            handle,
            user_data_request::Method::SaveCustomEntry(SaveCustomEntry {
                id: id.map(str::to_owned),
                roman: roman.into(),
                hanzi: hanzi.into(),
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
        assert_eq!(edited.entry.unwrap().hanzi, "臺灣");
        let filtered = list(&handle, "臺");
        assert_eq!(filtered.entries.len(), 1);
        assert_eq!(filtered.matching_total, 1);
        assert_eq!(filtered.total, seeded + 1, "an edit is not a second word");

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
        assert_eq!(
            import(&handle, b"just one column\n").refusal(),
            CustomDictionaryRefusal::NoUsableRows
        );
        let huge = vec![b'a'; 5 * 1024 * 1024 + 1];
        assert_eq!(
            import(&handle, &huge).refusal(),
            CustomDictionaryRefusal::FileTooLarge
        );
    }
}
