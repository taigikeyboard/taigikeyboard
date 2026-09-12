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
//! dispatch only handles the phase/hanzi/position guards, the proto →
//! domain hoists (`mode`, `freq_map`, `custom`), and wire encoding.
//!
//! The mode-aware key construction lives in `composing::continuous`
//! per the Phase 5 module contract pinned in
//! `engine/lexicon/src/continuous.rs`: TL/English emit `tl:<lowered>`,
//! POJ emits `poj:<lowered>` (v3.5.9 B-2 PR #309 promoted POJ to a
//! first-class FST key family via `composing::shadow::mode_key_prefix`),
//! and TPS emits `tps:<bopomofo_toneless>` against the C-0 emit of
//! `dictionary.fst` (v3.5.9 D / C-3b promoted TPS to first-class via
//! the same shadow → lattice path TL/POJ already walk; the legacy
//! `build_keys_tps` `tl:`-folded path is retired).

use crate::api::{CaretDirection, ComposingError, Engine, Intent, Phase};
use crate::continuous::{assemble_candidates, retain_first_by_key};
use crate::shadow::{build_shadow_lattice_with_barriers, left_anchored_keys_and_restrictions};
use lexicon::{
    classification::is_hanzi, derive_mode, ConsumedSpan, CustomEntry, RawCandidate,
    SyllableInventory, COVERAGE_KIND_FULL, FORM_NOTONE,
};
use phonetics::contains_tps;
use protos::engine::{
    composing_request, AppConfig, CandidateMessage, ComposingRequest, ComposingResponse,
    ContinuousResponse, CustomDictEntry, FrequencyEntry,
};
use ranking::build_frequency_map;

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
        Method::CommitDerived(_) => Intent::CommitDerived,
        Method::CommitRaw(_) => Intent::CommitRaw,
        Method::SelectSuggestion(m) => Intent::SelectSuggestion { text: m.text },
        Method::CommitPreeditThenInsertExternal(m) => {
            Intent::CommitPreeditThenInsertExternal { text: m.text }
        }
        Method::Reset(_) => Intent::Reset,
        Method::SetSelectedCandidateIndex(m) => {
            Intent::SetSelectedCandidateIndex { index: m.index }
        }
        Method::QueryState(_) => Intent::QueryState,
        Method::EnterContinuous(_) => Intent::EnterContinuous,
        Method::FetchAtPos(m) => Intent::FetchAtPos {
            position: m.position,
            frequency_entries: m.frequency_entries,
            now_ms: m.now_ms,
            custom_entries: m.custom_entries,
            enabled_sources_bitmask: m.enabled_sources_bitmask,
            literal_roman_candidate_disabled: m.literal_roman_candidate_disabled,
        },
        Method::CommitContinuous(m) => Intent::CommitContinuous {
            display_text: m.display_text,
            canonical_text: m.canonical_text,
            association_tl: m.association_tl,
            consumed_bytes: m.consumed_bytes as usize,
            syllable_count: clamp_syllable_count(m.syllable_count),
        },
        Method::ResetContinuous(_) => Intent::ResetContinuous,
        Method::TelexKey(m) => Intent::TelexKey { key: m.key },
        Method::MoveCaret(m) => Intent::MoveCaret {
            direction: match m.direction() {
                protos::engine::CaretDirection::Left => Some(CaretDirection::Left),
                protos::engine::CaretDirection::Right => Some(CaretDirection::Right),
                protos::engine::CaretDirection::Unspecified => None,
            },
        },
    })
}

/// Pure dispatch entry: decode the proto request into an `Intent` and
/// apply it against `engine`. Generation-mismatch handling lives one
/// layer up in `EngineHandle::handle` (`handle.rs`); this fn is the
/// in-process Rust API also used directly by the workspace tests.
///
/// Read-only intents (`FetchAtPos`, `QueryState`) are short-circuited via
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
        Intent::QueryState => engine.snapshot(config),
        Intent::FetchAtPos {
            position,
            frequency_entries,
            now_ms,
            custom_entries,
            enabled_sources_bitmask,
            literal_roman_candidate_disabled,
        } => handle_fetch_at_pos(
            engine,
            *position,
            frequency_entries,
            *now_ms,
            custom_entries,
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
/// `position != 0` is reserved for future partial-fetch use; the
/// engine treats it as an empty result today (matches the
/// `FetchAtPos.position` proto comment).
///
/// v3.5.9 A2: the candidate-assembly 6-step seam lives in
/// [`crate::continuous::assemble_candidates`]; this fn does the
/// phase/hanzi/position guards, the proto→domain hoists, and the wire
/// encoding around it.
fn handle_fetch_at_pos(
    engine: &Engine,
    position: u32,
    frequency_entries: &[FrequencyEntry],
    now_ms: i64,
    custom_entries: &[CustomDictEntry],
    enabled_sources_bitmask: u32,
    literal_roman_candidate_disabled: bool,
    config: &AppConfig,
) -> ComposingResponse {
    let snapshot = engine.snapshot(config);
    let state = engine.snapshot_state();
    let Phase::Continuous { raw, .. } = &state.phase else {
        return snapshot;
    };
    // v3.5.8 Phase 9 Item 11 — hanzi guard (§15.3.E). The only input
    // modes are TL/POJ/TPS romanization; CJK never legitimately enters
    // the composing buffer. When it leaks in (paste, stale selection
    // residue) short-circuit to an empty candidate carrier instead of
    // letting the syllabifier / lexicon scan garbage. Ports the platform
    // D-8 guard (`LexiconService` Hanzi classification) into the engine
    // so the behavior survives the Item 13 platform-fallback retire.
    // Runs ahead of the reserved-position check because contaminated
    // `raw` is dead regardless of `position`.
    if is_hanzi(raw) {
        return with_continuous(snapshot, ContinuousResponse::default());
    }
    if position != 0 {
        // Position field is reserved (always 0 in v3.5.8); non-zero
        // returns an empty candidate carrier so the caller can still
        // tell "FetchAtPos was reached" vs "wrong phase".
        return with_continuous(snapshot, ContinuousResponse::default());
    }
    // v3.5.9 D / C-3b — mode upgrade: the raw buffer trumps
    // `config.input_mode` when TPS Bopomofo is detected. Existing
    // platform `AppConfig` builders map TPS to `"tl"` / `"poj"` for
    // legacy reasons (`ios/.../RustEngineBridge+Composing.swift:584-599`,
    // `android/.../engine/RustEngineBridge.kt:1270-1288` — both fold
    // TPS into `is_translate_swapped` and keep `input_mode` as the
    // underlying romanization choice), so the config string alone
    // would mis-classify a TPS buffer. Bopomofo chars are unambiguous
    // (`phonetics::contains_tps` mirrors the same detection used in
    // `engine/composing/src/derived.rs:27`), so we promote the parsed
    // mode to `InputMode::Tps` whenever any Bopomofo char is present.
    //
    // Single-source mode flow into `assemble_candidates`: the seam
    // derives every TPS-gated branch from `mode == InputMode::Tps`
    // internally — pre-C-3b had a parallel `is_tps: bool` arg that
    // duplicated this axis (`is_tps = contains_tps(raw)`, dual source
    // of truth). Dropping the bool eliminates split-brain risk.
    //
    // The POJ-vs-TL/English branch in the seam is unaffected: platform
    // builders DO map POJ → `"poj"` so config is reliable for that
    // axis; only TPS needs the `contains_tps` override.
    let mode = if contains_tps(raw) {
        phonetics::InputMode::Tps
    } else {
        phonetics::api::parse_input_mode(&config.input_mode)
    };
    // Phase 9.3a: hoist proto-shaped `FrequencyEntry[]` into the
    // domain-typed `FrequencyMap` once per fetch; `lexicon` consumes
    // `&FrequencyMap` and stays proto-agnostic. Empty list → empty
    // map → `user_freq_boost(0) = 1.0` for every candidate (backward
    // -compatible with PR-9.2 platform builds that have not wired
    // user-frequency plumbing yet).
    let freq_map = build_frequency_map(frequency_entries);
    // v3.5.8 Phase 9 Item 12: hoist proto-shaped `CustomDictEntry[]`
    // into the domain-typed `CustomEntry` list once per fetch (mirror
    // of `build_frequency_map` above); `lexicon` consumes
    // `&[CustomEntry]` and stays proto-agnostic. Empty list = no
    // custom matches / feature disabled → zero synthesized candidates
    // and the `(roman, hanji)` dedupe is a no-op (backward-compatible
    // with builds that never set `FetchAtPos.custom_entries`).
    let custom = build_custom_entries(custom_entries);
    // PR-9.6 — normalise the source-toggle bitmask at the proto→domain
    // boundary: proto3 default `0` means "platform did not wire this"
    // (older / un-wired build) and maps to `u32::MAX` (legacy all-on),
    // reproducing pre-PR-9.6 behaviour where continuous candidates
    // ignored toggles. A real bitmask is never `0` because
    // `compute_filters` always sets the `dev` bit, so `0` is an
    // unambiguous absence marker. Normalising here (mirroring the
    // `build_frequency_map` / `build_custom_entries` hoists) keeps
    // `assemble_candidates` taking an already-resolved enabled bitmask —
    // no domain code has to know about the wire sentinel.
    let enabled_sources_bitmask = if enabled_sources_bitmask == 0 {
        u32::MAX
    } else {
        enabled_sources_bitmask
    };
    // v3.5.9 A2 seam — the 6-step assemble_candidates contract
    // (key build → span-local/partial fetch → recase → walker slot-0
    // prepend → POJ presentation pass → return). Wire encoding (step 6)
    // happens below via `raw_to_proto_candidate` + `with_continuous`.
    // PR-9.6 — `enabled_sources_bitmask` (already sentinel-normalised
    // above) flows into `ContinuousFetchCtx` so the span-local + partial
    // -prefix fetchers apply the same `Filter` the Tab3 browse path uses.
    let mut candidates = assemble_candidates(
        raw,
        &freq_map,
        now_ms,
        &custom,
        mode,
        enabled_sources_bitmask,
    );
    // INVARIANT_CONTINUOUS_LITERAL_ROMAN_CANDIDATE (§34): whenever composing
    // in TL/POJ — tone or no tone — surface the current composing result
    // (= the preedit WYSIWYG) as a roman-only candidate at index 0, so 漢羅
    // mixing commits the romanization in one tap without toggling 文/A and
    // the list does not jump when a tone is added. Display-layer prepend —
    // the segmentation / cost primitive (`assemble_candidates`) is never
    // touched (incidents S5/§18/S9). NOT re-run through Step 5's POJ recase:
    // `derived_display` is already the mode-correct POJ/TL literal.
    //
    // §34 / S22 toggle (顯示當咧拍的字): when the user turns the setting OFF
    // the platform sends `literal_roman_candidate_disabled = true` and the
    // forced prepend is skipped — the dedupe `retain` lives inside this
    // block so it is skipped too. This suppresses ONLY the §34 WYSIWYG
    // prepend; any roman-only / OOV-synth candidate `assemble_candidates`
    // produced on its own stays. Inverted sentinel: proto3 default `false`
    // = show (legacy always-on), so un-wired builds are unaffected.
    if !literal_roman_candidate_disabled {
        if let Some(mut literal) = literal_roman_candidate(raw, config, mode) {
            // Drop a pre-existing IDENTICAL bare-roman (hanji-absent Tailo)
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
    // §44 羅馬字 display dedupe — must run AFTER the literal prepend (a pass
    // inside `assemble_candidates` never sees the literal → two `tâi` cells).
    if config.is_roman_only_display()
        && matches!(mode, phonetics::InputMode::Tl | phonetics::InputMode::Poj)
    {
        dedupe_display_roman(&mut candidates);
    }
    with_continuous(
        snapshot,
        ContinuousResponse {
            candidates: candidates.into_iter().map(raw_to_proto_candidate).collect(),
        },
    )
}

/// 羅馬字-mode display dedupe (§44) — key = the **rendered roman alone**,
/// first-seen wins (top-ranked sorted row, or the §34 literal when it is in
/// the group). Mirror of `continuous::dedupe_display_hanji_for_tps` for the
/// other script; keys on the roman the user actually sees — the POJ
/// presentation pass already ran.
///
/// The consumed span was part of the key until 2026-09-03, on the reasoning
/// that a partial-prefix row and a full-buffer row are different actions.
/// They are — but under a single-script display they render as the same
/// string, so the user has no way to tell which cell commits which slice and
/// the second cell reads as a defect (USER: 「相同的漢字 or 羅馬字不能重複出現」).
/// A cell that reads exactly like an earlier one is never listed.
fn dedupe_display_roman(candidates: &mut Vec<RawCandidate>) {
    retain_first_by_key(candidates, |c| Some(c.roman.clone()));
}

/// Under a single-script display the §34 literal absorbs the dictionary row
/// that reads the same (`dedupe_display_roman` under 羅馬字; the platform's
/// 漢羅濫 roman-cell dedupe under 濫), so tapping the literal becomes the only
/// way to commit that word. The literal therefore takes the absorbed row's
/// identity — `display_text` (the `canonical_text` NextWord and 詞頻 key on)
/// and `canonical_tl` — while `roman` / `hanji: None` stay, so the cell still
/// reads, orders and writes the preedit literal. First-seen (top-ranked) wins
/// among 同音異字, the rule that picks the visible cell. Not applied under
/// 並排, where the dictionary row keeps its own cell.
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

/// Build the literal-roman candidate for 漢羅 fast input
/// (`INVARIANT_CONTINUOUS_LITERAL_ROMAN_CANDIDATE` §34 / dogfood S22).
///
/// Surfaces the **current composing result** — the preedit literal
/// (`derived_display`) — as a roman-only candidate **whenever** composing
/// in TL/POJ, tone or no tone. The candidate always mirrors the underline,
/// so the list does not jump when a tone is added: `tai`→`tai`,
/// `tai5`→`tâi`, `nng7`→`nn̄g`, `taigi`→`taigi`, `tai5-gi2`→`tâi-gí`. This
/// makes 漢羅 (mixed Han + roman) input commit the romanization in one tap
/// without toggling 文/A, even in 漢字 mode (PhahTaigi parity — the lomaji
/// candidate is always present, USER 2026-06-06 "邏輯 should consist").
///
/// Returns `Some` when:
/// * `mode` is TL or POJ — TPS is hanji-first (diacritic-glyph tones,
///   promoted to `InputMode::Tps` upstream); English excluded.
/// * the preedit literal is non-empty.
///
/// The candidate is roman-only (`hanji = None` → `CandidateMode::Tailo`),
/// with `roman == display_text ==` the preedit literal (except the identity
/// it inherits under a single-script display — `adopt_collapsed_dict_identity`)
/// — WYSIWYG with the underline (§30 literal-no-fold: tone marks only, no spelling fold). It
/// mirrors the preedit EXACTLY, so a tone-1/4 syllable or an unhyphenated
/// multi-syllable blob keeps its raw digits as the underline shows them
/// (`tai1`, `goa2ai3li2` — the engine does not auto-syllabify, §10.2). It
/// carries `canonical_tl` via `canonical_tl_form` so 詞頻 / 詞關聯 learn the
/// canonical `(∅, TL)` identity on commit (Core Principle #7; §24/§28).
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
    let canonical_tl = phonetics::api::canonical_tl_form(&literal, mode);
    Some(RawCandidate {
        consumed_span: (0, raw.len() as u32),
        syllable_count: literal.split('-').count().min(u8::MAX as usize) as u8,
        display_text: literal.clone(),
        roman: literal,
        hanji: None,
        canonical_tl,
        score: 0.0,
        form: FORM_NOTONE,
        frequency: 0,
        bitmask: 0,
        mode: derive_mode(None),
        recency_rank: 0,
        coverage_kind: COVERAGE_KIND_FULL,
        is_custom: false,
    })
}

/// Inventory-injected hermetic test seam for the shadow + key projection
/// pipeline. v3.5.9 A2 moved the production path through
/// [`crate::continuous::assemble_candidates`], which builds the shadow
/// lattice once and derives the left-anchored keys inline (D1 fold).
/// This wrapper survives only so the
/// `engine/composing/tests/build_keys_tl_{lattice,hyphen,poj_diacritic}.rs`
/// integration tests can drive a hermetic inventory without installing
/// the global `LexiconHandle` singleton. Public (`#[doc(hidden)]`) for
/// the test crate boundary; production callers must NOT reach for it
/// (use the seam).
///
/// Pipeline (A1 [`crate::shadow::build_shadow_lattice_with_barriers`] +
/// [`crate::shadow::left_anchored_keys_and_restrictions`], barriers
/// dropped):
///
/// 1. Lowercase `raw` (ASCII only).
/// 2. `shadow::canonicalize_poj_shadow` (Phase 9 Item 9; v3.5.9 B-2
///    reshape) emits POJ ASCII in POJ mode (`chiah` stays `chiah`) and
///    TL ASCII identity in TL mode (`tó-uī` stays `tó-uī`).
/// 3. `shadow::build_hyphen_shadow` (Phase 9 Item 8) strips ASCII `-`.
/// 4. The two byte-offset maps compose into a single
///    `shadow_to_raw_end` so downstream `consumed_span_end` lines up
///    with platform UI commit slicing.
/// 5. The lattice builder (`crate::lattice::build_lattice`) walks the
///    shadow against the inventory family selected by `mode` (B-2:
///    `tl:` vs `poj:`) — both transforms ran upstream.
/// 6. The left-anchored projection (`start == 0`) emits a fused
///    toneless `{mode_prefix}:<key>` per ending (`tl:` in TL/English
///    mode, `poj:` in POJ mode).
/// Injectable test seam for the FULL continuous-input key set — the base
/// reading's keys PLUS every alternate reading's
/// (`INVARIANT_TPS_DEFOLD_ENUMERATE` §35, TPS-only). Same shared
/// [`crate::shadow::build_continuous_keys`] production runs, so an
/// integration test can pin an alternate reading against a hermetic
/// inventory instead of only against production artifacts.
///
/// [`build_keys_tl_with_inventory`] stays the BASE-only seam: the pre-§35
/// tests that pin exact base key sets must keep seeing exactly those.
#[doc(hidden)]
pub fn build_continuous_keys_with_inventory(
    raw: &str,
    inv: &SyllableInventory,
    mode: phonetics::InputMode,
) -> Vec<(ConsumedSpan, String)> {
    crate::shadow::build_continuous_keys(raw, inv, mode).keys
}

#[doc(hidden)]
pub fn build_keys_tl_with_inventory(
    raw: &str,
    inv: &SyllableInventory,
    mode: phonetics::InputMode,
) -> Vec<(ConsumedSpan, String)> {
    // Barriers are discarded so the pre-§35 base key sets stay
    // byte-identical (the §35 alternates are the other seam's job).
    let (shadow, shadow_to_raw_end, lattice, _barriers) =
        build_shadow_lattice_with_barriers(raw, inv, mode);
    // v3.5.9 B-2 — `mode` makes the emitted key prefix match the inventory
    // family the shadow lattice was built against; `inv` drives longest-match
    // prefix suppression (`INVARIANT_CONTINUOUS_LONGEST_MATCH_PREFIX`).
    left_anchored_keys_and_restrictions(&shadow, &shadow_to_raw_end, &lattice, inv, mode, &[]).keys
}

/// v3.5.8 Phase 9 Item 12 — hoist proto-shaped `CustomDictEntry[]`
/// into the domain-typed [`CustomEntry`] list. Mirror of
/// `ranking::build_frequency_map`'s proto→domain boundary, kept here
/// so `lexicon` stays proto-agnostic. `roman` / `hanji` are the raw
/// stored `custom_dictionary.db` columns the platform marshalled
/// verbatim (NOT the legacy display-capitalized form) so the
/// `(roman, hanji)` dedupe key collides correctly against
/// `dict.bin`'s `DictionaryRecord.tl` / `.hanzi`. proto3 `optional
/// hanji` absent → `None` (romanization-only entry); present (even
/// empty) → `Some`.
///
/// v3.5.9 B-4 — `roman` is kept in its raw stored form here (TL or
/// POJ display, whichever the user typed). Canonicalization to TL
/// happens downstream at the `display_text` synthesis site only
/// (`lexicon::custom_entry_to_candidate` →
/// `phonetics::api::canonical_tl_form(roman, mode)`), NOT here. The
/// lattice / FST-key matching (`composing::shadow::custom_toneless_key`)
/// needs the user's native form to align with the mode-tagged
/// inventory introduced by B-1 / B-2; rewriting `roman` at this seam
/// would break that alignment for POJ-mode custom entries
/// (Codex pre-impl BLOCK #1, 2026-05-21).
fn build_custom_entries(entries: &[CustomDictEntry]) -> Vec<CustomEntry> {
    entries
        .iter()
        .map(|e| CustomEntry {
            roman: e.roman.clone(),
            hanji: e.hanji.clone(),
        })
        .collect()
}

fn raw_to_proto_candidate(c: RawCandidate) -> CandidateMessage {
    CandidateMessage {
        consumed_span_start: c.consumed_span.0,
        consumed_span_end: c.consumed_span.1,
        syllable_count: c.syllable_count as u32,
        display_text: c.display_text,
        score: c.score,
        form: c.form as u32,
        mode: c.mode.to_proto_i32(),
        // v3.5.8 Phase 9 Item 5 — `roman` is always non-empty for a
        // dictionary-sourced candidate; it is the display romanization
        // for the active input mode (TL, or POJ-display after the
        // continuous seam's POJ presentation pass). `hanji` is a proto3
        // `optional string` so prost serializes `None` as wire-absent
        // (distinguishes TAILO from defective empty-string emission).
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

/// `snapshot` already has the right `preedit` / `effect` / `is_composing` /
/// `selected_candidate_index` for `Phase::Continuous`; only the continuous
/// carrier needs population.
fn with_continuous(
    mut snapshot: ComposingResponse,
    continuous: ContinuousResponse,
) -> ComposingResponse {
    snapshot.continuous = Some(continuous);
    snapshot
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
    //! Unit tests for the pure helpers that stayed in dispatch.rs after
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
    /// `roman` and `hanji` onto the wire. HANT records carry both;
    /// TAILO records emit `roman` only and leave proto `hanji` as
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
            bitmask: 0,
            mode: lexicon::CandidateMode::Hant,
            recency_rank: 1,
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
            bitmask: 0,
            mode: lexicon::CandidateMode::Tailo,
            recency_rank: 1,
            coverage_kind: lexicon::COVERAGE_KIND_FULL,
            is_custom: false,
        };
        let proto = raw_to_proto_candidate(raw);
        assert_eq!(proto.roman, "tāi");
        assert!(proto.hanji.is_none());
        assert_eq!(proto.display_text, "tāi");
    }

    /// §44 羅馬字 display dedupe keys on the rendered roman ALONE: two rows
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
                bitmask: 0,
                mode: lexicon::CandidateMode::Hant,
                recency_rank: 1,
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
    // 漢羅 fast input: TL/POJ + written tone → roman-only literal candidate
    // (= preedit WYSIWYG) injected at index 0. Tests assert the GATE +
    // self-consistency; the literal string is asserted against
    // `derived_display` (the source of truth) rather than a hard-coded
    // diacritic so the two never drift.

    fn config_tl() -> AppConfig {
        AppConfig {
            tone_mode: String::new(),
            input_mode: "tl".to_string(),
            oo_doubletap_enabled: false,
            nn_doubletap_enabled: false,
            is_translate_swapped: false,
            is_association_recording_enabled: false,
            platform_id: 0,
            output_both_scripts: false,
            candidate_display_mode: 0,
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
        // roman-only (hanji None → Tailo), matching the preedit byte-for-byte.
        let cfg = config_tl();
        let cand = literal_roman_candidate("nng7", &cfg, phonetics::InputMode::Tl)
            .expect("nng7 must yield a literal roman candidate");
        assert_eq!(cand.roman, crate::derived::derived_display("nng7", &cfg));
        assert_eq!(cand.display_text, cand.roman); // WYSIWYG: display == roman
        assert_ne!(cand.roman, "nng7"); // a tone-mark conversion happened
        assert!(cand.hanji.is_none());
        assert_eq!(cand.mode, lexicon::CandidateMode::Tailo);
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
        // USER 2026-06-06 「邏輯 should consist」: toneless input ALSO surfaces
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
    fn literal_roman_candidate_mirrors_preedit_verbatim_with_digits() {
        // The candidate is EXACTLY the preedit (§30 / §10.2): a tone-1/4
        // syllable and an unhyphenated multi-syllable blob keep their raw
        // digits as the underline shows them — the engine does not
        // auto-syllabify, and the candidate must not diverge from the
        // underline (consistency).
        let cfg = config_tl();
        for raw in ["tai1", "goa2ai3li2", "tai5gi2"] {
            let cand = literal_roman_candidate(raw, &cfg, phonetics::InputMode::Tl).unwrap();
            assert_eq!(cand.roman, crate::derived::derived_display(raw, &cfg));
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
        // POJ `oo`/`nn` doubletap rewrites the spelling (`oo1`→`o͘1`); the
        // candidate mirrors the preedit verbatim (including any residual
        // tone-1/4 digit) — consistency with the underline, no special gate.
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
}
