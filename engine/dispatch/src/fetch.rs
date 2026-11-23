//! The one `FetchAtPos` path (and the developer view of it): the listed
//! buffer's user rows and previous-word context, a neutral fetch, then a
//! fetch re-ranked by the learned counts of the candidates it offered —
//! roadmap P3b, brainstorm R5, §56. Without user data (the stores not open
//! yet, or a build without them) the bundled context ranks one pass alone.

use composing::explain::mechanism;
use composing::requests::{fetch_at_pos_intent, query_listing};
use composing::{ConversionFrequency, Engine, EngineHandle, UserRows};
use protos::engine::{AppConfig, ComposingResponse, FetchAtPos};
use ranking::{ContextRanks, FrequencyMap, CONTEXT_RANK_USER};

use crate::context::{bundled_ranks, context_word, CONTEXT_ROWS};

/// The user data a fetch ranks with — the open stores
/// (`user_data::with_stores`). A trait because the `user-data` feature gates
/// the store crate: a build without it has no store type to name.
pub(crate) trait UserRowSource: ConversionFrequency {
    /// The custom-dictionary rows and learned phrases for the listed raw
    /// buffer — no frequency rows.
    fn buffer_rows(&self, raw: &str, config: &AppConfig) -> UserRows;
    /// The user's learned continuations of `(word, word_tl)`, best evidence
    /// first.
    fn continuations(&self, word: &str, word_tl: &str, limit: usize) -> Vec<(String, String)>;
}

/// Answers a `FetchAtPos`. The main thread keeps typing while a worker
/// fetches: every pass answers from one copy of the engine as of
/// `generation`, so rows read for one buffer never rank another. A stale
/// generation answers the idle snapshot. The pass answered records its rows
/// (`EngineHandle::remember_listing`) before the response returns.
pub(crate) fn fetch_at_pos(
    source: Option<&dyn UserRowSource>,
    sent: &FetchAtPos,
    config: &AppConfig,
    generation: u64,
) -> ComposingResponse {
    let handle = EngineHandle::instance();
    let Some(engine) = handle.engine_at(generation) else {
        return Engine::idle_snapshot(config);
    };
    let (mut rows, context) = ranking_inputs(source, &engine, config, sent.now_ms);
    let (response, listing) = match source {
        None => query_listing(&fetch_at_pos_intent(sent, rows, context), &engine, config),
        Some(source) => {
            let neutral = query_listing(
                &fetch_at_pos_intent(sent, rows.clone(), context.clone()),
                &engine,
                config,
            );
            let frequency = candidate_frequency(source, &neutral.0);
            if frequency.is_empty() {
                neutral
            } else {
                rows.frequency = frequency;
                query_listing(&fetch_at_pos_intent(sent, rows, context), &engine, config)
            }
        }
    };
    handle.remember_listing(generation, listing, config);
    response
}

/// The open composition's cut and candidate order, ranked with the rows the
/// last pass of [`fetch_at_pos`] uses; `None` when nothing is being composed.
pub(crate) fn explain(
    source: Option<&dyn UserRowSource>,
    sent: &FetchAtPos,
    config: &AppConfig,
    generation: u64,
) -> Option<composing::explain::Mechanism> {
    let engine = EngineHandle::instance().engine_at(generation)?;
    let (mut rows, context) = ranking_inputs(source, &engine, config, sent.now_ms);
    if let Some(source) = source {
        let neutral = query_listing(
            &fetch_at_pos_intent(sent, rows.clone(), context.clone()),
            &engine,
            config,
        )
        .0;
        rows.frequency = candidate_frequency(source, &neutral);
    }
    mechanism(
        &engine,
        sent.now_ms,
        &rows,
        &context,
        sent.literal_roman_candidate_disabled,
        config,
    )
}

/// The listed buffer's user rows and the continuations of the word it
/// follows: the user's learned bigrams (`user_association.db`) over the
/// bundled ones.
fn ranking_inputs(
    source: Option<&dyn UserRowSource>,
    engine: &Engine,
    config: &AppConfig,
    now_ms: i64,
) -> (UserRows, ContextRanks) {
    let snapshot = engine.pending_snapshot(config);
    let previous = context_word(&snapshot, now_ms);
    let mut context = bundled_ranks(previous.as_ref());
    let Some(source) = source else {
        return (UserRows::default(), context);
    };
    let rows = source.buffer_rows(&snapshot.listed_raw, config);
    if let Some((word, word_tl)) = previous {
        for (next, next_tl) in source.continuations(&word, &word_tl, CONTEXT_ROWS) {
            context.insert(next, next_tl, CONTEXT_RANK_USER);
        }
    }
    (rows, context)
}

/// The learned counts for the candidates `neutral` offers, each word asked
/// once; empty when there is nothing to re-rank with.
fn candidate_frequency(source: &dyn UserRowSource, neutral: &ComposingResponse) -> FrequencyMap {
    let mut seen = std::collections::HashSet::new();
    let words: Vec<String> = neutral
        .continuous
        .iter()
        .flat_map(|continuous| &continuous.candidates)
        .map(|candidate| candidate.display_text.as_str())
        .filter(|word| seen.insert(*word))
        .map(str::to_owned)
        .collect();
    if words.is_empty() {
        return FrequencyMap::new();
    }
    source.rows_for_words(&words)
}
