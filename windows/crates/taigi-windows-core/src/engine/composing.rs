//! Composing slice of the engine bridge: the intents the desktop sends and
//! the decoding of what comes back. Port of `RustEngineBridge+Composing.swift`.
//!
//! This is a subset of the engine's sixteen intents, for the reasons that
//! file documents at length: `AppendHyphen` is an alias for `Append("-")`,
//! `ReplaceLast` is TPS-only, `Start` is unnecessary (`Append` enters
//! `Phase::Composing` from Idle), `SelectSuggestion` double-counts the nailed
//! prefix under `Phase::Continuous` (`transition.rs:724`) so the literal
//! commit is `CommitRaw`, and `SetSelectedCandidateIndex` is absent for good —
//! nothing in the engine reads it and candidate navigation is a permanent
//! platform-side concern (`cross-platform-alignment.md` §5.1).
//!
//! Every op answers `None` when the round-trip itself failed, which is a
//! different thing from the engine answering that it is idle.

use protos::engine::{
    composing_request, request, response, Append, CommitContinuous,
    CommitPreeditThenInsertExternal, CommitRaw, ComposingRequest, ComposingResponse,
    CustomDictEntry, DeleteBackward, EnterContinuous, FetchAtPos, FrequencyEntry, Reset, TelexKey,
};

use super::bridge::{app_config, continuous_app_config, record_failure, roundtrip};
use super::transition::{ComposingTransition, ContinuousCandidate, ContinuousFetchResult};
use crate::settings::EngineSettings;

/// One learned frequency row, as the store hands it over.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrequencyRow {
    pub word: String,
    pub tl: String,
    pub count: i64,
    pub last_used_ms: i64,
}

/// One custom-dictionary row, the raw stored columns (`CustomDictEntry`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CustomEntry {
    pub roman: String,
    /// Empty = romanization-only entry; mapped to an ABSENT wire field.
    pub hanzi: String,
}

/// Appends one typed character to the raw buffer.
pub fn append(
    character: &str,
    settings: &EngineSettings,
    generation: u64,
) -> Option<ComposingTransition> {
    dispatch(
        composing_request::Method::Append(Append {
            char: character.to_owned(),
        }),
        "composingAppend",
        generation,
        Some(app_config(settings)),
    )
}

/// Applies one Telex key to the pending syllable's tone — or, for `z`,
/// types the affricate initial the input mode spells (`composing.proto`
/// `TelexKey`, `engine/composing/src/telex.rs`). Carries the app config
/// like `append`, because `z` resolves by `input_mode`. Port of
/// `composingTelexKey` (`RustEngineBridge+Composing.swift`).
pub fn telex_key(
    key: &str,
    settings: &EngineSettings,
    generation: u64,
) -> Option<ComposingTransition> {
    dispatch(
        composing_request::Method::TelexKey(TelexKey {
            key: key.to_owned(),
        }),
        "composingTelexKey",
        generation,
        Some(app_config(settings)),
    )
}

/// Drops the last character of the raw buffer.
pub fn delete_backward(settings: &EngineSettings, generation: u64) -> Option<ComposingTransition> {
    dispatch(
        composing_request::Method::DeleteBackward(DeleteBackward {}),
        "composingDeleteBackward",
        generation,
        Some(app_config(settings)),
    )
}

/// Commits the whole composition exactly as the preedit renders it —
/// `Σ nailed.display_text + derived(pending)` under the continuous phase
/// (`transition.rs:443`). This is the literal-commit key.
pub fn commit_raw(settings: &EngineSettings, generation: u64) -> Option<ComposingTransition> {
    dispatch(
        composing_request::Method::CommitRaw(CommitRaw {}),
        "composingCommitRaw",
        generation,
        Some(continuous_app_config(settings)),
    )
}

/// Finalizes the composition and appends `text` after it, as one engine
/// step — the space bar and mid-composition punctuation.
pub fn commit_preedit_then_insert_external(
    text: &str,
    settings: &EngineSettings,
    generation: u64,
) -> Option<ComposingTransition> {
    dispatch(
        composing_request::Method::CommitPreeditThenInsertExternal(
            CommitPreeditThenInsertExternal {
                text: text.to_owned(),
            },
        ),
        "composingCommitPreeditThenInsertExternal",
        generation,
        Some(continuous_app_config(settings)),
    )
}

/// Abandons the composition without writing anything to the document.
pub fn reset(generation: u64) -> Option<ComposingTransition> {
    dispatch(
        composing_request::Method::Reset(Reset {}),
        "composingReset",
        generation,
        None,
    )
}

/// Promotes an active composition into the continuous phase. Safe to send
/// unconditionally: the engine no-ops on an empty buffer and on a composition
/// that is already continuous (`transition.rs:496-502`).
pub fn enter_continuous(settings: &EngineSettings, generation: u64) -> Option<ComposingTransition> {
    dispatch(
        composing_request::Method::EnterContinuous(EnterContinuous {}),
        "composingEnterContinuous",
        generation,
        Some(app_config(settings)),
    )
}

/// What one `FetchAtPos` carries besides the settings. Computed once per
/// keystroke by the caller and passed to BOTH fetch phases, so the neutral
/// and boosted answers describe one composition under one set of rules.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FetchArgs<'a> {
    /// What the user committed before; the engine turns it into a boost.
    /// Empty is not degraded: "rank without usage history".
    pub frequency_rows: &'a [FrequencyRow],
    /// The clock the engine's recency ranking reads.
    pub now_ms: i64,
    /// `0` is not "no sources": the engine reads it as "platform did not
    /// wire this" and searches all of them — see `lexicon::enabled_sources_bitmask`.
    pub enabled_sources_bitmask: u32,
    /// The user's own dictionary rows matching the raw buffer, columns as
    /// stored — the engine dedupes `(roman, hanji)` and folds to canonical TL.
    pub custom_entries: &'a [CustomEntry],
}

/// Reads the candidates for the current continuous composition.
///
/// Read-only, so it must be sent under the composition's EXISTING generation:
/// a bumped generation resets the engine before the query runs
/// (`engine/composing/src/handle.rs:61-66`).
pub fn fetch_at_pos(
    settings: &EngineSettings,
    generation: u64,
    args: &FetchArgs<'_>,
) -> Option<ContinuousFetchResult> {
    let fetch = FetchAtPos {
        position: 0,
        frequency_entries: args.frequency_rows.iter().map(frequency_entry).collect(),
        now_ms: args.now_ms,
        custom_entries: args.custom_entries.iter().map(custom_dict_entry).collect(),
        enabled_sources_bitmask: args.enabled_sources_bitmask,
        // §34/S22 — positive platform setting → inverted proto disable gate
        // (the field's own comment carries why), so 顯示當咧拍的字 ON leaves the
        // preedit literal leading the list and Enter commits what was typed.
        // CROSS-PLATFORM INVARIANT — mirrors
        // `macos/Sources/TaigiInputMethodCore/Engine/RustEngineBridge+Composing.swift`
        // `composingFetchAtPos`, which inverts the same setting onto the same field.
        literal_roman_candidate_disabled: !settings.is_literal_roman_candidate_enabled,
    };
    let response = composing_response(
        composing_request::Method::FetchAtPos(fetch),
        "composingFetchAtPos",
        generation,
        Some(continuous_app_config(settings)),
    )?;
    let candidates = response.continuous.as_ref().map(|continuous| {
        continuous
            .candidates
            .iter()
            .map(ContinuousCandidate::decode)
            .collect()
    });
    Some(ContinuousFetchResult {
        transition: ComposingTransition::decode(&response),
        candidates,
    })
}

/// What a candidate commit round-trips. Every field except `document_text`
/// comes verbatim from the `ContinuousCandidate` the user picked — in
/// particular `consumed_bytes` is the candidate's `consumed_span_end`, an
/// absolute offset into the pending raw buffer (`composing.proto:228`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitContinuousArgs<'a> {
    /// The platform's rendering of the candidate for the document.
    pub document_text: &'a str,
    /// The identity keys the engine learns from (Core Principle #7).
    pub canonical_text: &'a str,
    pub association_tl: &'a str,
    pub consumed_bytes: u32,
    pub syllable_count: u32,
}

/// Commits one candidate returned by `fetch_at_pos`. Consuming the whole
/// pending buffer makes this a final commit; anything less nails the segment
/// and stays continuous, writing nothing (`transition.rs:842-889`, Model B).
pub fn commit_continuous(
    args: &CommitContinuousArgs<'_>,
    settings: &EngineSettings,
    generation: u64,
) -> Option<ComposingTransition> {
    dispatch(
        composing_request::Method::CommitContinuous(CommitContinuous {
            display_text: args.document_text.to_owned(),
            consumed_bytes: args.consumed_bytes,
            syllable_count: args.syllable_count,
            canonical_text: args.canonical_text.to_owned(),
            association_tl: args.association_tl.to_owned(),
        }),
        "composingCommitContinuous",
        generation,
        Some(continuous_app_config(settings)),
    )
}

fn dispatch(
    method: composing_request::Method,
    op: &str,
    generation: u64,
    config: Option<protos::engine::AppConfig>,
) -> Option<ComposingTransition> {
    composing_response(method, op, generation, config)
        .map(|response| ComposingTransition::decode(&response))
}

fn composing_response(
    method: composing_request::Method,
    op: &str,
    generation: u64,
    config: Option<protos::engine::AppConfig>,
) -> Option<ComposingResponse> {
    let payload = request::Payload::Composing(ComposingRequest {
        method: Some(method),
    });
    match roundtrip(payload, op, generation, config)? {
        response::Payload::Composing(response) => Some(response),
        other => {
            record_failure(op, &format!("expected a composing payload, got {other:?}"));
            None
        }
    }
}

/// One learned row on the wire. `count` is clamped rather than trusted to
/// fit: the column is a 64-bit SQLite integer and the field is 32-bit, and a
/// saturating conversion is a wrong boost where a trapping one is a crash in
/// the middle of a keystroke.
fn frequency_entry(row: &FrequencyRow) -> FrequencyEntry {
    FrequencyEntry {
        display_text_key: row.word.clone(),
        count: u32::try_from(row.count.max(0)).unwrap_or(u32::MAX),
        last_used_ms: row.last_used_ms,
        canonical_tl: row.tl.clone(),
    }
}

/// One custom-dictionary row on the wire. An empty stored hanzi maps to an
/// ABSENT `hanji`: the engine reads absence as "romanization-only entry"
/// while an empty string would be a hanji that renders as nothing.
fn custom_dict_entry(row: &CustomEntry) -> CustomDictEntry {
    CustomDictEntry {
        roman: row.roman.clone(),
        hanji: (!row.hanzi.is_empty()).then(|| row.hanzi.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequency_count_saturates_and_custom_hanzi_absence_is_kept() {
        let entry = frequency_entry(&FrequencyRow {
            word: "台".into(),
            tl: "tâi".into(),
            count: i64::MAX,
            last_used_ms: 7,
        });
        assert_eq!(entry.count, u32::MAX);
        assert_eq!(entry.canonical_tl, "tâi");
        let negative = frequency_entry(&FrequencyRow {
            word: "x".into(),
            tl: String::new(),
            count: -5,
            last_used_ms: 0,
        });
        assert_eq!(negative.count, 0);

        let roman_only = custom_dict_entry(&CustomEntry {
            roman: "gau5-tsa2".into(),
            hanzi: String::new(),
        });
        assert_eq!(roman_only.hanji, None);
        let with_hanzi = custom_dict_entry(&CustomEntry {
            roman: "gau5-tsa2".into(),
            hanzi: "𠢕早".into(),
        });
        assert_eq!(with_hanzi.hanji.as_deref(), Some("𠢕早"));
    }
}
