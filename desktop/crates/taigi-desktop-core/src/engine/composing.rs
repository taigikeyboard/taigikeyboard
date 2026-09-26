//! Composing slice of the engine bridge: the intents the desktop sends and
//! the decoding of what comes back. Port of `RustEngineBridge+Composing.swift`.
//!
//! This is a subset of the engine's sixteen intents, for the reasons that
//! file documents at length: `AppendHyphen` is an alias for `Append("-")`,
//! `ReplaceLast` is TPS-only, `Start` is unnecessary (`Append` enters
//! `Phase::Composing` from Idle), `SelectSuggestion` double-counts the nailed
//! prefix under `Phase::Continuous` (`transition.rs:724`) so the literal
//! commit is `CommitRaw`. Candidate navigation is a permanent platform-side
//! concern (`cross-platform-alignment.md` §4.1).
//!
//! Every op answers `None` when the round-trip itself failed, which is a
//! different thing from the engine answering that it is idle.

use protos::engine::{
    composing_request, request, response, Append, CaretDirection as WireCaretDirection,
    CommitContinuous, CommitPreeditThenInsertExternal, CommitRaw, ComposingRequest,
    ComposingResponse, DeleteBackward, EnterContinuous, FetchAtPos, MoveCaret, Reset, TelexKey,
};

use crate::keys::CaretDirection;

use super::bridge::{continuous_app_config, record_failure, roundtrip};
use super::transition::{ComposingTransition, ContinuousCandidate, ContinuousFetchResult};
use crate::settings::EngineSettings;

/// Appends one typed character to the raw buffer.
///
/// Carries the continuous config, as every op that re-renders the
/// composition does: under `Phase::Continuous` the answer is the whole
/// marked region, nailed prefix included, and the prefix's word-boundary
/// spacing reads the two flags only that config sets. With the base config
/// a nail rendered `台gi` and the next keystroke `台 gi` (found by the
/// composing-caret round, 2026-09-09).
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
        Some(continuous_app_config(settings)),
    )
}

/// Applies one Telex key to the pending syllable's tone — or, for `z`,
/// types the affricate initial the input mode spells (`composing.proto`
/// `TelexKey`, `engine/composing/src/telex.rs`). Carries the same config
/// as `append` (`z` resolves by `input_mode`, the prefix by the spacing
/// flags). Port of `composingTelexKey` (`RustEngineBridge+Composing.swift`).
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
        Some(continuous_app_config(settings)),
    )
}

/// Drops the character before the caret. Same config as `append`.
pub fn delete_backward(settings: &EngineSettings, generation: u64) -> Option<ComposingTransition> {
    dispatch(
        composing_request::Method::DeleteBackward(DeleteBackward {}),
        "composingDeleteBackward",
        generation,
        Some(continuous_app_config(settings)),
    )
}

/// Steps the caret one character inside the pending tail (`composing.proto`
/// `MoveCaret`). The buffer is untouched, so the engine answers with an
/// `UpdatePreedit` carrying the new caret and nothing else — no fetch is
/// requested. Same config as `append`: the answer re-renders the
/// composition the way the last keystroke did, so a move never changes the
/// text on screen (`composingMoveCaret`, `RustEngineBridge+Composing.swift`).
pub fn move_caret(
    direction: CaretDirection,
    settings: &EngineSettings,
    generation: u64,
) -> Option<ComposingTransition> {
    let wire = match direction {
        CaretDirection::Left => WireCaretDirection::Left,
        CaretDirection::Right => WireCaretDirection::Right,
    };
    dispatch(
        composing_request::Method::MoveCaret(MoveCaret {
            direction: wire as i32,
        }),
        "composingMoveCaret",
        generation,
        Some(continuous_app_config(settings)),
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
        // Same config as `append`: already under Continuous the answer is a
        // snapshot whose `display_text` the manager mirrors, and a snapshot
        // rendered with the base config would put the space back.
        Some(continuous_app_config(settings)),
    )
}

/// What one `FetchAtPos` carries besides the settings. The user's own data
/// is not among it: the engine reads its stores itself and ranks in one
/// call (user-data-engine-roadmap P3b / P5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FetchArgs {
    /// The clock the engine's recency ranking reads.
    pub now_ms: i64,
    /// `0` is not "no sources": the engine reads it as "platform did not
    /// wire this" and searches all of them — see `lexicon::enabled_sources_bitmask`.
    pub enabled_sources_bitmask: u32,
}

/// Reads the candidates for the current continuous composition.
///
/// Read-only, so it must be sent under the composition's EXISTING generation:
/// a bumped generation resets the engine before the query runs
/// (`engine/composing/src/handle.rs:61-66`).
pub fn fetch_at_pos(
    settings: &EngineSettings,
    generation: u64,
    args: &FetchArgs,
) -> Option<ContinuousFetchResult> {
    let fetch = FetchAtPos {
        now_ms: args.now_ms,
        enabled_sources_bitmask: args.enabled_sources_bitmask,
        // §34/S22 — positive platform setting → inverted proto disable gate
        // (the field's own comment carries why), so Show Typed Text First ON leaves the
        // preedit literal leading the list and Enter commits what was typed.
        // CROSS-PLATFORM INVARIANT — mirrors
        // `macos/Sources/TaigiInputMethodCore/Engine/RustEngineBridge+Composing.swift`
        // `composingFetchAtPos`, which inverts the same setting onto the same field.
        literal_roman_candidate_disabled: !settings.is_literal_roman_candidate_enabled,
        // The engine reads the user's dictionary only with this setting on.
        custom_dictionary_disabled: !settings.is_custom_dict_enabled,
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
    /// §50 — the picked candidate's hanji, `None` for a hanji-less pick; the
    /// engine learns a composition only when every segment carried one.
    pub hanji: Option<&'a str>,
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
            hanji: args.hanji.filter(|h| !h.is_empty()).map(str::to_owned),
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
