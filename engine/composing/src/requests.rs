//! Decode `ComposingRequest` → `Intent`, apply against `Engine`, encode the
//! `ComposingResponse`. The generation-mismatch reset path also lives here
//! (per plan §5b.2).
//!
//! `Intent::FetchAtPos` is the v3.5.8 Phase 6 read-only continuous-input
//! candidate query. Unlike the other 12+ intents (which mutate engine
//! state via `transition::apply`), `FetchAtPos` needs lexicon state
//! (`lexicon::EngineHandle::with_state` — prefix index + dictionary +
//! syllable inventory) and so it is resolved here in dispatch outside
//! the pure transition table. After v3.5.9 A2 the candidate-assembly
//! 6-step seam lives in [`crate::continuous::assemble_candidates`];
//! dispatch only handles the phase/hanji guards, the proto →
//! domain hoists (`mode`, the source filter from the toggles), and wire encoding.
//!
//! The mode-aware key construction lives in `composing::continuous`
//! per the Phase 5 module contract pinned in
//! `engine/lexicon/src/continuous/`: TL/English emit `tl:<lowered>`,
//! POJ emits `poj:<lowered>` (v3.5.9 B-2 PR #309 promoted POJ to a
//! first-class FST key family via `phonetics::KeyFamily::for_input_mode`),
//! and TPS emits `tps:<bopomofo_toneless>` against the C-0 emit of
//! `dictionary.fst` (v3.5.9 D / C-3b promoted TPS to first-class via
//! the same shadow → lattice path TL/POJ already walk; the legacy
//! `build_keys_tps` `tl:`-folded path is retired).

use crate::api::{CaretDirection, CommitScript, ComposingError, Engine, Intent, Phase, UserRows};
use crate::continuous::{assemble_candidates, retain_first_by_key, roman_reading_eq, ListShape};
use crate::derived::buffer_input_mode;
use lexicon::{
    classification::is_hanji, ConsumedSpan, LearnedEntry, RawCandidate, SyllableInventory,
    COVERAGE_KIND_FULL, FORM_NOTONE,
};
use protos::engine::{
    composing_request, AppConfig, CandidateMessage, CommitScript as WireCommitScript,
    ComposingRequest, ComposingResponse, ContinuousResponse, DictionarySourceToggles, FetchAtPos,
};
use ranking::WALKER_COST_UNPRICED;

/// Decode the proto request into a typed `Intent`. Returns `MissingMethod`
/// when `oneof method` is empty.
pub fn decode_intent(req: &ComposingRequest) -> Result<Intent, ComposingError> {
    use composing_request::Method;
    let Some(method) = req.method.clone() else {
        return Err(ComposingError::MissingMethod);
    };
    Ok(match method {
        Method::Start(m) => Intent::Start { text: m.text },
        Method::Append(m) => Intent::Append { ch: m.char },
        Method::AppendHyphen(_) => Intent::AppendHyphen,
        Method::ReplaceLast(m) => Intent::ReplaceLast {
            replacement: m.replacement,
        },
        Method::DeleteBackward(_) => Intent::DeleteBackward,
        Method::CommitRaw(_) => Intent::CommitRaw,
        Method::CommitPreeditThenInsertExternal(m) => {
            Intent::CommitPreeditThenInsertExternal { text: m.text }
        }
        Method::Reset(_) => Intent::Reset,
        // The user rows are the engine's own reads (`UserRows`); a platform
        // sends none.
        Method::FetchAtPos(m) => {
            fetch_at_pos_intent(&m, UserRows::default(), ranking::ContextRanks::default())
        }
        Method::CommitContinuous(m) => Intent::CommitContinuous {
            script: commit_script(m.script()),
            roman: m.roman,
            canonical_text: m.canonical_text,
            association_tl: m.association_tl,
            hanji: m.hanji,
            consumed_bytes: m.consumed_bytes as usize,
            syllable_count: clamp_syllable_count(m.syllable_count),
        },
        Method::TelexKey(m) => Intent::TelexKey { key: m.key },
        Method::TpsKey(m) => Intent::TpsKey { key: m.key },
        Method::CommitAsShown(_) => Intent::CommitAsShown,
        Method::CommitAsTyped(_) => Intent::CommitAsTyped,
        Method::MoveCaret(m) => Intent::MoveCaret {
            direction: match m.direction() {
                protos::engine::CaretDirection::Left => Some(CaretDirection::Left),
                protos::engine::CaretDirection::Right => Some(CaretDirection::Right),
                protos::engine::CaretDirection::Start => Some(CaretDirection::Start),
                protos::engine::CaretDirection::End => Some(CaretDirection::End),
                protos::engine::CaretDirection::Unspecified => None,
            },
        },
    })
}

/// The fetch intent for a wire `FetchAtPos`, ranked with `user_rows` and
/// `context`. Its source filter is the request's toggles resolved by the
/// lexicon encoder (every dictionary off → `0`, no dictionary candidates,
/// §57), or every source when the request carries no toggles (fixtures, an
/// un-wired sender — see the `FetchAtPos` proto comment).
pub fn fetch_at_pos_intent(
    fetch: &FetchAtPos,
    user_rows: UserRows,
    context: ranking::ContextRanks,
) -> Intent {
    Intent::FetchAtPos {
        now_ms: fetch.now_ms,
        enabled_sources_bitmask: source_filter_bitmask(fetch.toggles.as_ref()),
        literal_roman_candidate_disabled: fetch.literal_roman_candidate_disabled,
        user_rows,
        context,
    }
}

/// The dictionary source filter `toggles` resolve to: every source when a
/// request names none, `0` when it turns every dictionary off (§57).
pub(crate) fn source_filter_bitmask(toggles: Option<&DictionarySourceToggles>) -> u32 {
    toggles.map_or(u32::MAX, lexicon::api::dictionary_filter_bitmask)
}

/// Pure dispatch entry: decode the proto request into an `Intent` and
/// apply it against `engine`. Generation-mismatch handling lives one
/// layer up in `EngineHandle::handle` (`handle.rs`); this fn is the
/// in-process Rust API also used directly by the workspace tests.
///
/// The read-only intent (`FetchAtPos`) is short-circuited via
/// [`query`] (not `engine.apply`) so the pure transition table stays free
/// of lexicon access. See module docs.
pub fn handle(
    req: &ComposingRequest,
    engine: &mut Engine,
    config: &AppConfig,
) -> Result<ComposingResponse, ComposingError> {
    let intent = decode_intent(req)?;
    Ok(apply(intent, engine, config))
}

/// Apply an already-decoded intent. Read-only intents are answered by
/// [`query`] so the pure transition table never sees them.
pub fn apply(intent: Intent, engine: &mut Engine, config: &AppConfig) -> ComposingResponse {
    if intent.is_read_only() {
        query(&intent, engine, config)
    } else {
        engine.apply(intent, config)
    }
}

/// Answer a read-only intent (`Intent::is_read_only`) from `engine` without
/// mutating it — the `&Engine` receiver is the type-level guarantee that
/// `EngineHandle` relies on when it runs a fetch against a released-lock
/// clone. Callers gate on `is_read_only`; a mutating intent here is a
/// programming error.
pub fn query(intent: &Intent, engine: &Engine, config: &AppConfig) -> ComposingResponse {
    match intent {
        Intent::FetchAtPos {
            now_ms,
            enabled_sources_bitmask,
            literal_roman_candidate_disabled,
            user_rows,
            context,
        } => handle_fetch_at_pos(
            engine,
            *now_ms,
            user_rows,
            context,
            *enabled_sources_bitmask,
            *literal_roman_candidate_disabled,
            config,
        ),
        mutating => unreachable!("query() called with mutating intent {mutating:?}"),
    }
}

/// Phase 6 read-only continuous-input candidate query.
///
/// Returns a `ComposingResponse` snapshot of the current engine state
/// with `continuous: Some(ContinuousResponse { candidates })` populated
/// when `Phase::Continuous` and lexicon state is available; in any
/// other case returns the snapshot with `continuous = None` (an empty
/// candidate list is encoded as the carrier present with empty
/// `candidates`, distinct from "FetchAtPos was a no-op because state
/// was wrong").
///
/// v3.5.9 A2: the candidate-assembly 6-step seam lives in
/// [`crate::continuous::assemble_candidates`]; this fn does the
/// phase/hanji guards and the wire encoding around it.
///
/// Under a Hanji conversion the request asks for, the list is the word
/// before the caret's (H4): the tail from [`Engine::word_list_start`] as
/// [`ListShape::Word`], spans shifted back into the tail's coordinates.
fn handle_fetch_at_pos(
    engine: &Engine,
    now_ms: i64,
    user_rows: &UserRows,
    context: &ranking::ContextRanks,
    enabled_sources_bitmask: u32,
    literal_roman_candidate_disabled: bool,
    config: &AppConfig,
) -> ComposingResponse {
    let snapshot = engine.snapshot(config);
    let state = engine.snapshot_state();
    let Phase::Continuous { raw: tail, .. } = &state.phase else {
        return snapshot;
    };
    let word_list_start = engine.word_list_start(config);
    let shape = match word_list_start {
        Some(_) => ListShape::Word,
        None => ListShape::Sentence,
    };
    let list_start = word_list_start.unwrap_or(0);
    let raw = &tail[list_start..];
    // v3.5.8 Phase 9 Item 11 — hanji guard (§15.3.E). The only input
    // modes are TL/POJ/TPS romanization; CJK never legitimately enters
    // the composing buffer. When it leaks in (paste, stale selection
    // residue) short-circuit to an empty candidate carrier instead of
    // letting the syllabifier / lexicon scan garbage. Ports the platform
    // D-8 guard (`LexiconService` Hanji classification) into the engine
    // so the behavior survives the Item 13 platform-fallback retire.
    if is_hanji(raw) {
        return with_continuous(snapshot, ContinuousResponse::default());
    }
    // v3.5.9 D / C-3b — mode upgrade: the buffer's content, not the
    // config, decides TPS. Any Bopomofo char (`phonetics::contains_tps`,
    // the same detection `derived::derived_display` uses) makes the fetch
    // `InputMode::Tps`, whatever `input_mode` says; a Bopomofo-free buffer
    // composes under `composing_mode`, which reads the TPS layout
    // (`input_mode = "tps"`, the only TPS wire — the engine ships inside
    // each app) as TL. So a TPS buffer that opens with `-` or a lone tone
    // mark takes the TL tables until its first Bopomofo glyph.
    //
    // Single-source mode flow into `assemble_candidates`: the seam
    // derives every TPS-gated branch from `mode == InputMode::Tps`
    // internally — pre-C-3b had a parallel `is_tps: bool` arg that
    // duplicated this axis (`is_tps = contains_tps(raw)`, dual source
    // of truth). Dropping the bool eliminates split-brain risk. The whole
    // tail decides, also for a list starting inside it.
    let mode = buffer_input_mode(tail, config);
    // Learned phrases (§50): one row per reading, the most-learned
    // separator form first.
    let learned = first_learned_per_reading(&user_rows.learned);
    // v3.5.9 A2 seam — the 6-step assemble_candidates contract
    // (key build → span-local/partial fetch → recase → walker slot-0
    // prepend → POJ presentation pass → return). Wire encoding (step 6)
    // happens below via `raw_to_proto_candidate` + `with_continuous`.
    // `enabled_sources_bitmask` (resolved at decode) flows into
    // `ContinuousFetchCtx` so the span-local + partial-prefix fetchers
    // apply the same `Filter` the Tab3 browse path uses.
    let mut candidates = assemble_candidates(
        raw,
        &user_rows.frequency,
        now_ms,
        &user_rows.custom,
        &learned,
        mode,
        enabled_sources_bitmask,
        context,
        config.rendered_syllable_joiner(),
        config.force_lowercase_nasal_marker,
        shape,
    );
    // INVARIANT_CONTINUOUS_LITERAL_ROMAN_CANDIDATE (§34): whenever composing
    // in TL/POJ — tone or no tone — surface the current composing result
    // (= the preedit WYSIWYG) as a roman-only candidate at index 0, so Hanji-romanization
    // mixing commits the romanization in one tap without toggling 文/A and
    // the list does not jump when a tone is added. Display-layer prepend —
    // the segmentation / cost primitive (`assemble_candidates`) is never
    // touched (incidents S5/§18/S9). NOT re-run through Step 5's POJ recase:
    // `derived_display` is already the mode-correct POJ/TL literal.
    //
    // §34 / S22 toggle (Show Typed Text First): when the user turns the setting OFF
    // the platform sends `literal_roman_candidate_disabled = true` and the
    // forced prepend is skipped — the dedupe `retain` lives inside this
    // block so it is skipped too. This suppresses ONLY the §34 WYSIWYG
    // prepend; any roman-only / OOV-synth candidate `assemble_candidates`
    // produced on its own stays. Inverted sentinel: proto3 default `false`
    // = show (legacy always-on), so un-wired builds are unaffected.
    if !literal_roman_candidate_disabled {
        if let Some(mut literal) = literal_roman_candidate(raw, config, mode) {
            // Drop a pre-existing IDENTICAL bare-roman (hanji-absent)
            // so the literal is not duplicated; dict rows with hanji stay (a
            // `tâi`/台 dict candidate and a bare `tâi` commit differ — Codex
            // pre-impl F5).
            candidates.retain(|c| !(c.hanji.is_none() && c.roman == literal.roman));
            if config.is_single_script_display() {
                adopt_collapsed_dict_identity(&mut literal, &candidates);
            }
            candidates.insert(0, literal);
        }
    }
    // §44 Romanization Only display dedupe — must run AFTER the literal prepend (a pass
    // inside `assemble_candidates` never sees the literal → two `tâi` cells).
    if config.is_roman_only_display()
        && matches!(mode, phonetics::InputMode::Tl | phonetics::InputMode::Poj)
    {
        dedupe_display_roman(&mut candidates);
    }
    if list_start > 0 {
        let list_start = list_start as u32;
        for candidate in &mut candidates {
            candidate.consumed_span.0 += list_start;
            candidate.consumed_span.1 += list_start;
        }
    }
    with_continuous(
        snapshot,
        ContinuousResponse {
            candidates: candidates.into_iter().map(raw_to_proto_candidate).collect(),
        },
    )
}

/// Romanization Only display dedupe (§44) — key = the **rendered roman alone**,
/// first-seen wins (top-ranked sorted row, or the §34 literal when it is in
/// the group). Mirror of `continuous::dedupe_display_hanji_for_tps` for the
/// other script; keys on the roman the user actually sees — the POJ
/// presentation pass already ran.
///
/// The consumed span was part of the key until 2026-09-03, on the reasoning
/// that a partial-prefix row and a full-buffer row are different actions.
/// They are — but under a single-script display they render as the same
/// string, so the user has no way to tell which cell commits which slice and
/// the second cell reads as a defect (USER: "the same Hanji or romanization must not appear twice").
/// A cell that reads exactly like an earlier one is never listed.
fn dedupe_display_roman(candidates: &mut Vec<RawCandidate>) {
    retain_first_by_key(candidates, |c| Some(c.roman.clone()));
}

/// Under a single-script display the §34 literal absorbs the dictionary row
/// that reads the same (`dedupe_display_roman` under Romanization Only; the platform's
/// Hanji with Romanization roman-cell dedupe under it), so tapping the literal becomes the only
/// way to commit that word. The literal therefore takes the absorbed row's
/// identity — `display_text` (the `canonical_text` NextWord and the frequency records key on)
/// and `canonical_tl` — while `roman` / `hanji: None` stay, so the cell still
/// reads, orders and writes the preedit literal. First-seen (top-ranked) wins
/// among homophones, the rule that picks the visible cell. Not applied under
/// Pairing, where the dictionary row keeps its own cell.
///
/// Consequence, deliberate: a single-syllable literal that inherits a
/// dictionary word joins a compound run with the nailed prefix
/// (`api::nailed_prefix`, `台` + literal `gí` → `tâi-gí`), exactly as the
/// absorbed dictionary cell would have.
fn adopt_collapsed_dict_identity(literal: &mut RawCandidate, candidates: &[RawCandidate]) {
    let Some(absorbed) = candidates
        .iter()
        .find(|c| c.hanji.is_some() && c.roman == literal.roman)
    else {
        return;
    };
    literal.display_text = absorbed.display_text.clone();
    literal.canonical_tl = absorbed.canonical_tl.clone();
}

/// Build the literal-roman candidate for Hanji-romanization fast input
/// (`INVARIANT_CONTINUOUS_LITERAL_ROMAN_CANDIDATE` §34 / dogfood S22).
///
/// Surfaces the **current composing result** — the preedit literal
/// (`derived_display`) — as a roman-only candidate **whenever** composing
/// in TL/POJ, tone or no tone. The candidate always mirrors the underline,
/// so the list does not jump when a tone is added: `tai`→`tai`,
/// `tai5`→`tâi`, `nng7`→`nn̄g`, `taigi`→`taigi`, `tai5-gi2`→`tâi-gí`. This
/// makes mixed Hanji-romanization input commit the romanization in one tap
/// without toggling 文/A, even in Hanji mode (PhahTaigi parity — the lomaji
/// candidate is always present, USER 2026-06-06: "the logic should be consistent").
///
/// Returns `Some` when:
/// * `mode` is TL or POJ — TPS is hanji-first (diacritic-glyph tones,
///   promoted to `InputMode::Tps` upstream); English excluded.
/// * the preedit literal is non-empty.
///
/// The candidate is roman-only (`hanji = None`),
/// with `roman ==` the preedit literal. `display_text` preserves the prior
/// literal learning key (except the dictionary identity inherited under a
/// single-script display — `adopt_collapsed_dict_identity`)
/// — WYSIWYG with the underline (§30 literal-no-fold: tone marks only, no spelling fold). It
/// mirrors the preedit EXACTLY. Each tone digit closes a segment: a valid
/// syllable takes its usual mark, anything else marks its last vowel
/// cluster (`tai5gi2` → `tâigí`, `taigi2` → `taigí`).
/// Tones 1 and 4 consume their digits without a visible mark. It
/// carries `canonical_tl` via `canonical_tl_form` of that stable learning
/// text, preserving tone boundaries and existing `(text, TL)` records.
fn literal_roman_candidate(
    raw: &str,
    config: &AppConfig,
    mode: phonetics::InputMode,
) -> Option<RawCandidate> {
    if !matches!(mode, phonetics::InputMode::Tl | phonetics::InputMode::Poj) {
        return None;
    }
    let literal = crate::derived::derived_display(raw, config);
    if literal.is_empty() {
        return None;
    }
    let learning_text = crate::derived::learning_text(raw, config);
    let canonical_tl = phonetics::api::canonical_tl_form(&learning_text, mode);
    Some(RawCandidate {
        consumed_span: (0, raw.len() as u32),
        syllable_count: literal.split('-').count().min(u8::MAX as usize) as u8,
        display_text: learning_text,
        roman: literal,
        hanji: None,
        canonical_tl,
        score: 0.0,
        form: FORM_NOTONE,
        frequency: 0,
        walker_cost: WALKER_COST_UNPRICED,
        bitmask: 0,
        user_weight: 0.0,
        context_rank: ranking::CONTEXT_RANK_NONE,
        coverage_kind: COVERAGE_KIND_FULL,
        is_custom: false,
    })
}

/// Inventory-injected hermetic test seam for continuous-input key
/// building: the same [`crate::shadow::build_continuous_keys`] that
/// [`crate::continuous::assemble_candidates`] runs (barriers and typed
/// hyphen runs included), so the `engine/composing/tests/build_keys_*.rs`
/// integration tests pin production keys against a hermetic inventory
/// without installing the global `LexiconHandle` singleton. Public
/// (`#[doc(hidden)]`) for the test crate boundary; production callers
/// must NOT reach for it. The pipeline is documented on
/// [`crate::shadow::build_shadow_lattice_with_barriers`] and
/// [`crate::shadow::left_anchored_keys_and_restrictions`].
#[doc(hidden)]
pub fn build_continuous_keys_with_inventory(
    raw: &str,
    inv: &SyllableInventory,
    mode: phonetics::InputMode,
) -> Vec<(ConsumedSpan, String)> {
    crate::shadow::build_continuous_keys(raw, inv, mode).keys
}

/// Learned phrases (§50) — one row per reading. Both strings are canonical
/// already (hanji as committed, canonical TL as the engine learned it) and
/// stay as stored; the per-mode lattice key is derived at the walker
/// (`learned_edge_key`).
///
/// The same pair learned under two typed separators (`guá-sī` / `guá--sī`)
/// is two stored rows under `UNIQUE(hanji, roman)`; only the first in store
/// order (`learn_count DESC, updated_at DESC` — the form the user composed
/// most, then most recently) is kept, so a corrected separator wins over the
/// slip and one slip never displaces a settled phrase.
fn first_learned_per_reading(entries: &[LearnedEntry]) -> Vec<LearnedEntry> {
    let mut out: Vec<LearnedEntry> = Vec::with_capacity(entries.len());
    for e in entries
        .iter()
        .filter(|e| !e.hanji.is_empty() && !e.canonical_tl.is_empty())
    {
        let same_reading = out.iter().any(|kept| {
            kept.hanji == e.hanji && roman_reading_eq(&kept.canonical_tl, &e.canonical_tl)
        });
        if !same_reading {
            out.push(e.clone());
        }
    }
    out
}

fn raw_to_proto_candidate(c: RawCandidate) -> CandidateMessage {
    CandidateMessage {
        consumed_span_end: c.consumed_span.1,
        syllable_count: c.syllable_count as u32,
        display_text: c.display_text,
        // v3.5.8 Phase 9 Item 5 — `roman` is always non-empty for a
        // dictionary-sourced candidate; it is the display romanization
        // for the active input mode (TL, or POJ-display after the
        // continuous seam's POJ presentation pass). `hanji` is a proto3
        // `optional string` so prost serializes `None` as wire-absent
        // (distinguishes a roman-only candidate from defective empty-string emission).
        // See `docs/engine/continuous-candidate-display.md` §4.2.
        roman: c.roman,
        hanji: c.hanji,
        // v3.6.1 R2 — identity sidechannel: canonical TL (NOT the
        // POJ-rendered display `roman`). The platform round-trips it back
        // into `CommitContinuous.association_tl` on tap. Empty only for
        // TPS-OOV hanji-absent candidates with no recoverable dict TL.
        canonical_tl: c.canonical_tl,
    }
}

/// `snapshot` already has the right `preedit` / `effect` / `is_composing`
/// for `Phase::Continuous`; only the continuous
/// carrier needs population.
fn with_continuous(
    mut snapshot: ComposingResponse,
    continuous: ContinuousResponse,
) -> ComposingResponse {
    snapshot.continuous = Some(continuous);
    snapshot
}

/// The script a `CommitContinuous` names: `None` for `UNSPECIFIED`, which
/// prost also reads an unknown (newer) script as — the commit is ignored.
fn commit_script(script: WireCommitScript) -> Option<CommitScript> {
    match script {
        WireCommitScript::Unspecified => None,
        WireCommitScript::Lead => Some(CommitScript::Lead),
        WireCommitScript::Other => Some(CommitScript::Other),
        WireCommitScript::Hanji => Some(CommitScript::Hanji),
        WireCommitScript::Roman => Some(CommitScript::Roman),
    }
}

/// Clamp the syllable count into `u8`. FST romanization keys are only
/// emitted for readings of at most 4 syllables
/// (`dictionary/build/create_fst.py::MAX_SYLLABLES_TL_NUM`); the proto
/// field is u32 so callers could in principle send larger values.
/// Saturate to `u8::MAX` so Continuous transition.rs sees a typed
/// value matching `NailedSegment.syllable_count: u8`.
fn clamp_syllable_count(value: u32) -> u8 {
    value.min(u8::MAX as u32) as u8
}

#[cfg(test)]
mod tests {
    //! Unit tests for the pure helpers that stayed in requests.rs after
    //! v3.5.9 A2 (`raw_to_proto_candidate`, `clamp_syllable_count`).
    //! Helpers that moved into [`crate::continuous`] carry their tests
    //! into that module verbatim. Dispatch-level integration (decode
    //! round-trip, degraded paths) lives in
    //! `engine/composing/tests/dispatch_continuous.rs` because it needs
    //! the `Engine` + lexicon singletons.

    use super::*;
    use lexicon::FORM_NOTONE;

    #[test]
    fn clamp_syllable_count_saturates() {
        assert_eq!(clamp_syllable_count(0), 0);
        assert_eq!(clamp_syllable_count(1), 1);
        assert_eq!(clamp_syllable_count(255), 255);
        assert_eq!(clamp_syllable_count(256), 255);
        assert_eq!(clamp_syllable_count(u32::MAX), 255);
    }

    /// v3.5.8 Phase 9 Item 5 — `raw_to_proto_candidate` must propagate
    /// `roman` and `hanji` onto the wire. Records with hanji carry both;
    /// roman-only records emit `roman` only and leave proto `hanji` as
    /// `None` (proto3 `optional string` wire-absent, NOT `Some("")`).
    #[test]
    fn raw_to_proto_candidate_propagates_roman_and_some_hanji() {
        let raw = RawCandidate {
            consumed_span: (0, 7),
            syllable_count: 2,
            display_text: "臺灣".to_owned(),
            roman: "tâi-uân".to_owned(),
            hanji: Some("臺灣".to_owned()),
            canonical_tl: "tâi-uân".to_owned(),
            score: 1.5,
            form: FORM_NOTONE,
            frequency: 12,
            walker_cost: WALKER_COST_UNPRICED,
            bitmask: 0,
            user_weight: 0.0,
            context_rank: ranking::CONTEXT_RANK_NONE,
            coverage_kind: lexicon::COVERAGE_KIND_FULL,
            is_custom: false,
        };
        let proto = raw_to_proto_candidate(raw);
        assert_eq!(proto.roman, "tâi-uân");
        assert_eq!(proto.hanji.as_deref(), Some("臺灣"));
        assert_eq!(proto.display_text, "臺灣");
        // R2 — canonical TL sidechannel propagates onto the wire.
        assert_eq!(proto.canonical_tl, "tâi-uân");
    }

    #[test]
    fn raw_to_proto_candidate_emits_none_hanji_for_tailo() {
        let raw = RawCandidate {
            consumed_span: (0, 3),
            syllable_count: 1,
            display_text: "tāi".to_owned(),
            roman: "tāi".to_owned(),
            hanji: None,
            canonical_tl: "tāi".to_owned(),
            score: 0.5,
            form: FORM_NOTONE,
            frequency: 3,
            walker_cost: WALKER_COST_UNPRICED,
            bitmask: 0,
            user_weight: 0.0,
            context_rank: ranking::CONTEXT_RANK_NONE,
            coverage_kind: lexicon::COVERAGE_KIND_FULL,
            is_custom: false,
        };
        let proto = raw_to_proto_candidate(raw);
        assert_eq!(proto.roman, "tāi");
        assert!(proto.hanji.is_none());
        assert_eq!(proto.display_text, "tāi");
    }

    /// §44 Romanization Only display dedupe keys on the rendered roman ALONE: two rows
    /// reading `tâi` collapse even when they consume different slices of the
    /// buffer, because a single-script cell shows the user nothing that tells
    /// the two apart (USER 2026-09-03). First-seen — the top-ranked row, or
    /// the §34 literal — wins; a different roman is never touched.
    #[test]
    fn dedupe_display_roman_collapses_same_roman_across_spans() {
        fn row(roman: &str, hanji: Option<&str>, span: (u32, u32)) -> RawCandidate {
            RawCandidate {
                consumed_span: span,
                syllable_count: 1,
                display_text: hanji.unwrap_or(roman).to_owned(),
                roman: roman.to_owned(),
                hanji: hanji.map(str::to_owned),
                canonical_tl: roman.to_owned(),
                score: 1.0,
                form: FORM_NOTONE,
                frequency: 1,
                walker_cost: WALKER_COST_UNPRICED,
                bitmask: 0,
                user_weight: 0.0,
                context_rank: ranking::CONTEXT_RANK_NONE,
                coverage_kind: lexicon::COVERAGE_KIND_FULL,
                is_custom: false,
            }
        }
        let mut candidates = vec![
            row("tâi", None, (0, 4)),
            row("tâi", Some("台"), (0, 4)),
            row("tâi", Some("臺"), (0, 3)),
            row("tâi-gí", Some("台語"), (0, 6)),
        ];
        dedupe_display_roman(&mut candidates);
        assert_eq!(
            candidates
                .iter()
                .map(|c| (c.roman.as_str(), c.hanji.as_deref()))
                .collect::<Vec<_>>(),
            vec![("tâi", None), ("tâi-gí", Some("台語"))],
        );
    }

    // ----- INVARIANT_CONTINUOUS_LITERAL_ROMAN_CANDIDATE (§34) -----
    // Hanji-romanization fast input: TL/POJ + written tone → roman-only literal candidate
    // (= preedit WYSIWYG) injected at index 0. Tests assert the GATE +
    // self-consistency; the literal string is asserted against
    // `derived_display` (the source of truth) rather than a hard-coded
    // diacritic so the two never drift.

    fn config_tl() -> AppConfig {
        AppConfig {
            input_mode: "tl".to_string(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_hanji_first: false,
            platform_id: 0,
            output_both_scripts: false,
            candidate_display_mode: 0,
            syllable_separator: 0,
            force_lowercase_nasal_marker: false,
            tps_or_maps_to_er: false,
            hanji_conversion: None,
        }
    }

    fn config_poj() -> AppConfig {
        AppConfig {
            input_mode: "poj".to_string(),
            ..config_tl()
        }
    }

    #[test]
    fn literal_roman_candidate_tl_single_syllable_is_tailo_wysiwyg() {
        // Headline case: `nng7` → the literal tone-marked roman (`nn̄g`),
        // roman-only (hanji None), matching the preedit byte-for-byte.
        let cfg = config_tl();
        let cand = literal_roman_candidate("nng7", &cfg, phonetics::InputMode::Tl)
            .expect("nng7 must yield a literal roman candidate");
        assert_eq!(cand.roman, crate::derived::derived_display("nng7", &cfg));
        assert_eq!(cand.display_text, cand.roman); // WYSIWYG: display == roman
        assert_ne!(cand.roman, "nng7"); // a tone-mark conversion happened
        assert!(cand.hanji.is_none());
        assert_eq!(cand.consumed_span, (0, 4));
        assert!(!cand.canonical_tl.is_empty()); // #7 identity sidechannel set
    }

    #[test]
    fn literal_roman_candidate_tl_goa2_keeps_literal_spelling() {
        // §30 literal: TL `goa2` displays `goá` (mark on `a`, NOT folded to
        // `guá`); the identity `canonical_tl` DOES fold to canonical TL `guá`.
        let cfg = config_tl();
        let cand = literal_roman_candidate("goa2", &cfg, phonetics::InputMode::Tl).unwrap();
        assert_eq!(cand.roman, crate::derived::derived_display("goa2", &cfg));
        assert_eq!(cand.canonical_tl, "gu\u{e1}"); // guá — cross-mode #7 identity
    }

    #[test]
    fn literal_roman_candidate_poj_uses_poj_diacritics() {
        // POJ mode is literal too: `goa2` → `góa` (mark on `o`).
        let cfg = config_poj();
        let cand = literal_roman_candidate("goa2", &cfg, phonetics::InputMode::Poj).unwrap();
        assert_eq!(cand.roman, crate::derived::derived_display("goa2", &cfg));
        assert!(cand.hanji.is_none());
    }

    #[test]
    fn literal_roman_candidate_hyphenated_multisyllable_included() {
        // Hyphenated fully-toned multi-syllable converts → included.
        let cfg = config_tl();
        let cand = literal_roman_candidate("tai5-gi2", &cfg, phonetics::InputMode::Tl).unwrap();
        assert_eq!(
            cand.roman,
            crate::derived::derived_display("tai5-gi2", &cfg)
        );
        assert!(cand.roman.contains('-'));
        assert_eq!(cand.syllable_count, 2);
    }

    #[test]
    fn literal_roman_candidate_toneless_now_shown() {
        // USER 2026-06-06: "the logic should be consistent": toneless input ALSO surfaces
        // the composing literal (= preedit), not only when toned. `taigi`
        // → `taigi`. (Previously gated on a written tone — now always shown.)
        let cfg = config_tl();
        let cand = literal_roman_candidate("taigi", &cfg, phonetics::InputMode::Tl).unwrap();
        assert_eq!(cand.roman, crate::derived::derived_display("taigi", &cfg));
        assert_eq!(cand.roman, "taigi");
        assert!(cand.hanji.is_none());
    }

    #[test]
    fn literal_roman_candidate_partial_single_syllable_shown() {
        // The candidate mirrors the preedit at every keystroke so the list
        // does not jump as the user types toward / past a tone. `ta` → `ta`.
        let cfg = config_tl();
        let cand = literal_roman_candidate("ta", &cfg, phonetics::InputMode::Tl).unwrap();
        assert_eq!(cand.roman, "ta");
    }

    #[test]
    fn literal_roman_candidate_preserves_digits_without_a_tone_target() {
        let cfg = config_tl();
        for raw in ["t2", "123", "tai-2"] {
            let cand = literal_roman_candidate(raw, &cfg, phonetics::InputMode::Tl).unwrap();
            assert_eq!(cand.roman, raw);
            assert_eq!(cand.roman, crate::derived::derived_display(raw, &cfg));
        }
    }

    #[test]
    fn literal_roman_candidate_permissive_tones_mirror_preview() {
        let cfg = config_tl();
        for (raw, expected) in [
            ("tai5gi2", "tâigí"),
            ("goa2ai3li2", "goáàilí"),
            ("tai1", "tai"),
            ("bak4", "bak"),
            ("tai1bak4", "taibak"),
            ("taigi2", "taigí"),
        ] {
            let cand = literal_roman_candidate(raw, &cfg, phonetics::InputMode::Tl).unwrap();
            assert_eq!(cand.roman, expected, "{raw}");
            assert_eq!(cand.roman, crate::derived::derived_display(raw, &cfg));
            assert_eq!(cand.consumed_span, (0, raw.len() as u32));
        }
    }

    #[test]
    fn literal_roman_candidate_preserves_existing_learning_keys() {
        for mode in [phonetics::InputMode::Tl, phonetics::InputMode::Poj] {
            let cfg = AppConfig {
                input_mode: if mode == phonetics::InputMode::Tl {
                    "tl"
                } else {
                    "poj"
                }
                .into(),
                ..config_tl()
            };
            for (raw, text, canonical_tl) in [
                ("a1i3", "a1i3", "a1i3"),
                ("ai3", "ài", "ài"),
                ("a1i1", "a1i1", "a1i1"),
                ("ai1", "ai1", "ai"),
                ("a4i3", "a4i3", "a4i3"),
                ("tai5gi2", "tai5gi2", "tai5gi2"),
            ] {
                let cand = literal_roman_candidate(raw, &cfg, mode).unwrap();
                assert_eq!(cand.display_text, text, "{mode:?}: {raw}");
                assert_eq!(cand.canonical_tl, canonical_tl, "{mode:?}: {raw}");
                assert_eq!(cand.roman, crate::derived::derived_display(raw, &cfg));
            }
        }
    }

    #[test]
    fn literal_roman_candidate_trailing_hyphen_mirrors_preedit() {
        // A trailing hyphen mirrors the preedit too (`tai5-` → `tâi-`); the
        // candidate stays consistent with the underline.
        let cfg = config_tl();
        let cand = literal_roman_candidate("tai5-", &cfg, phonetics::InputMode::Tl).unwrap();
        assert_eq!(cand.roman, crate::derived::derived_display("tai5-", &cfg));
    }

    #[test]
    fn literal_roman_candidate_empty_is_none() {
        // No composing content → no candidate.
        assert!(literal_roman_candidate("", &config_tl(), phonetics::InputMode::Tl).is_none());
    }

    #[test]
    fn literal_roman_candidate_tps_and_english_excluded() {
        // TPS is hanji-first (diacritic-glyph tones, not ASCII digits);
        // English is not Taigi romanization. Both excluded by the mode gate.
        let cfg = config_tl();
        assert!(literal_roman_candidate("nng7", &cfg, phonetics::InputMode::Tps).is_none());
        assert!(literal_roman_candidate("nng7", &cfg, phonetics::InputMode::English).is_none());
    }

    #[test]
    fn literal_roman_candidate_poj_doubletap_mirrors_preedit() {
        // POJ `oo`/`nn` doubletap preprocessing is preserved (`oo1`→`o͘`);
        // the candidate mirrors the preedit, including invisible tones 1/4.
        let cfg = AppConfig {
            input_mode: "poj".to_string(),
            oo_doubletap_enabled: true,
            nn_doubletap_enabled: true,
            ..config_tl()
        };
        for raw in ["oo1", "oo4", "oo2"] {
            let cand = literal_roman_candidate(raw, &cfg, phonetics::InputMode::Poj).unwrap();
            assert_eq!(cand.roman, crate::derived::derived_display(raw, &cfg));
        }
    }

    #[test]
    fn literal_learning_keys_preserve_poj_doubletap_and_nasal_case_settings() {
        for force_lowercase in [false, true] {
            let cfg = AppConfig {
                oo_doubletap_enabled: true,
                nn_doubletap_enabled: true,
                force_lowercase_nasal_marker: force_lowercase,
                ..config_poj()
            };
            for (raw, expected) in [
                ("oo1", "o͘1"),
                ("oo4", "o͘4"),
                ("oo2", "ó͘"),
                (
                    "SIANN1",
                    if force_lowercase {
                        "SIAⁿ1"
                    } else {
                        "SIAᴺ1"
                    },
                ),
            ] {
                let cand = literal_roman_candidate(raw, &cfg, phonetics::InputMode::Poj).unwrap();
                assert_eq!(cand.display_text, expected);
                assert_eq!(cand.roman, crate::derived::derived_display(raw, &cfg));
            }
        }
    }
}
