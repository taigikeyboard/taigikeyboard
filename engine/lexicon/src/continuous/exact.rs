//! Exact-key continuous fetch: every dictionary candidate whose toneless key
//! equals a span-local key.

use phonetics::KeyFamily;

use crate::dictionary_reader::{DictionaryReader, Filter};
use crate::prefix_index::PrefixIndex;

use super::candidate::{merge_custom_dedupe_sort, record_to_candidate};
use super::tone_pin::TonePin;
use super::toneless_match::matches_continuous_toneless_key;
use super::{ConsumedSpan, ContinuousFetchCtx, RawCandidate, COVERAGE_KIND_FULL};

/// Resolve a continuous lookup key to its stored readings.
///
/// TPS keys go through the ambiguity-aware automaton
/// (`PrefixIndex::lookup_exact_tps_readings`, §35): one index walk
/// returns every reading of the pressed keys, substitution-count
/// ascending so the user's literal text always resolves first. TL /
/// POJ / hanji keys keep the plain exact lookup — byte-identical
/// behavior, zero automaton cost (`matched_key` = the query key,
/// `subst` = 0).
///
/// Every consumer MUST validate records against the returned
/// `matched_key`, never the query key: a substituted reading's
/// `tps_notone` reconstruction equals the MATCHED key, and the
/// literal-key guard would reject every recovered word (Codex
/// pre-impl 2026-08-19 BLOCK 3).
fn for_each_exact_reading(
    prefix_index: &PrefixIndex,
    key: &str,
    final_only_offsets: &[usize],
    mut visit: impl FnMut(&str, u32),
) {
    if key.starts_with(KeyFamily::Tps.prefix()) {
        for (matched_key, rowid, _subst) in
            prefix_index.lookup_exact_tps_readings(key, final_only_offsets)
        {
            visit(&matched_key, rowid);
        }
    } else {
        // TL / POJ / hanji: byte-identical to the pre-§35 exact lookup —
        // the matched key IS the query key, no per-rowid allocation.
        for rowid in prefix_index.lookup_exact(key) {
            visit(key, rowid);
        }
    }
}

/// Every dictionary candidate for one exact continuous key, in FST rowid
/// order, handed to `sink` one at a time (each carries its record's
/// `walker_cost`). The single visitor the span-local fetch
/// ([`fetch_candidates_for_keys_with_barriers`], collects all) and the
/// walker edge provider ([`best_candidate_for_key_with_barriers`], keeps
/// the min cost) share, so the two layers cannot drift on which rows qualify —
/// §35: an edge the expanded segmenter admitted must find its dictionary
/// payload, or the layers split authority (Codex pre-impl BLOCK 3).
///
/// Per row, validated against the MATCHED key (a substituted TPS reading
/// reconstructs to the matched key, never the query key — see
/// [`for_each_exact_reading`]): the source `filter`, the `tl_abbrev` /
/// `poj_abbrev` / `tps_abbrev` acronym-collision guard
/// ([`matches_continuous_toneless_key`] — continuous input is
/// phonetic-syllable, not acronym; normal-mode `lexicon::search` keeps
/// acronym matching), and the key's [`TonePin`] — the TPS typed closes
/// (§17 case 4 / §41: `ㄉㄞ˫ㄨㄢ` keeps tāi-uân, drops tâi-uân; `ㄒㄧ`␣
/// keeps si1, drops si2/5/7; `ㄐㄧㆵ`␣ keeps tsit4, drops tsit8) or the §17
/// typed-tone pin (`teng5sek` keeps têng-sek, drops téng-sek).
/// Without the pin on the walker side the span-local list and the
/// whole-sentence slot 0 would disagree and a wrong-tone word would
/// reappear at index 0.
///
/// Exact hits are always `COVERAGE_KIND_FULL`: the key came from a valid
/// syllable ending. Partial-prefix hits go through
/// [`fetch_partial_prefix_candidates_unbounded`] instead. `filter` is
/// built by the caller.
///
/// [`best_candidate_for_key_with_barriers`]: super::best_candidate_for_key_with_barriers
/// [`fetch_partial_prefix_candidates_unbounded`]: super::fetch_partial_prefix_candidates_unbounded
pub(super) fn exact_candidates_for_key(
    key: &str,
    tps_final_only: &[usize],
    tone_pin: &TonePin,
    consumed_span: ConsumedSpan,
    filter: &Filter,
    ctx: &ContinuousFetchCtx<'_>,
    mut sink: impl FnMut(RawCandidate),
) {
    for_each_exact_reading(
        ctx.prefix_index,
        key,
        tps_final_only,
        |matched_key, rowid| {
            let Some(record) = ctx.dict.record(rowid) else {
                return;
            };
            if !DictionaryReader::passes_filter(record.bitmask, record.kautian_subtag, filter) {
                return;
            }
            if !matches_continuous_toneless_key(matched_key, &record.tl) {
                return;
            }
            if !tone_pin.admits(Some(matched_key), &record.tl) {
                return;
            }
            let effective = DictionaryReader::effective_source_bitmask(
                record.bitmask,
                record.kautian_subtag,
                filter,
            );
            sink(record_to_candidate(
                record,
                effective,
                consumed_span,
                ctx.freq_map,
                ctx.now_ms,
                ctx.context,
                COVERAGE_KIND_FULL,
            ));
        },
    );
}

/// Mode-agnostic span-local fetch entry. Each input pair is
/// `(consumed_span, fst_key)`: `consumed_span` is the user-facing
/// byte range that committing this candidate will eat, and `fst_key`
/// is the already-prefixed FST lookup key (e.g. `"tl:tsua"` for TL/English,
/// `"poj:chiah"` for POJ, `"tps:ㄉㄞ"` for TPS — v3.5.9 B-2 PR #309
/// promoted POJ and v3.5.9 D / C-3b promoted TPS to first-class FST
/// families). The production caller
/// (`composing::continuous::assemble_candidates`) selects the
/// prefix via `phonetics::KeyFamily::for_input_mode(mode)` and feeds
/// pairs in directly for all modes.
///
/// # v3.5.8 Phase 9.1 — lexicographic CandidateSortKey
///
/// `raw_len` is the byte length of the original pending buffer
/// (`Phase::Continuous { raw }.len()`); it is the predicate input
/// for Tier 1 (`consumed_span_end == raw_len`). Sorting follows
/// [`CandidateSortKey`] (field order and why: `docs/engine/continuous-input-ranking.md` §3.2).
///
/// E1 P5b: `walker_cost` (the record's corpus cost) orders the list on the
/// walker's scale; `-adjusted_score` / `-freq` break an equal cost.
///
/// v3.5.8 whole-sentence lattice + walker S8: `-coverage_bytes` was relocated
/// from dim 3 to dim 6 (below `-adjusted_score` / `-freq`). With the
/// slot-0 whole-sentence walker owning phrase priority, a graded
/// longest-coverage-first rule inside a tier only buried the short
/// single-syllable first-segment candidate the user wants for
/// segment-by-segment selection. Coverage is now a weak tiebreak that
/// fires only when score AND freq are equal — matching librime's
/// per-segment menu, which keeps multi-length candidates but never lets
/// a longer code-length bury a shorter strict match
/// (`script_translator.cc` `kNumExactMatchOnTop`).
///
/// # v3.5.8 Phase 9.3a — user-frequency plumb
///
/// `freq_map` is the user-selection snapshot keyed by the
/// `(display_text, canonical_tl)` pair (Core Principle #7), built once
/// per fetch from the engine's `user_frequency.db` rows
/// (`composing::UserRows.frequency`). `now_ms` is the platform's
/// epoch-ms wall clock at fetch time. `record_to_candidate` looks
/// up each candidate by that pair, computes
/// [`ranking::user_freq_boost`] (saturated at
/// [`ranking::MAX_BOOST`]), and derives
/// `CandidateSortKey.neg_user_weight` via [`ranking::decayed_user_weight_delta`]
/// (which guards against `now_ms <= 0`, `last_used_ms <= 0`, and
/// clock skew). NaN scores (only reachable if the boost helper
/// produces a non-finite value — which it cannot under the
/// public contract) are coerced to `f64::MIN` at `CandidateSortKey`
/// construction so the descending-order invariant holds.
///
/// # Barriers
///
/// `tps_final_only[i]` = byte offsets into `keys[i].1` of glyphs
/// immediately before a stripped separator / hyphen barrier — those pattern
/// slots keep only Final-role readings (§31 boundary respect; §35).
/// Parallel-indexed rather than widening the key tuple so the many
/// existing `(span, key)` call sites and fixtures stay untouched; an
/// empty slice (or a short one) means "no barriers", which is also the
/// TL / POJ / English shape.
///
/// [`CandidateSortKey`]: ranking::CandidateSortKey
pub fn fetch_candidates_for_keys_with_barriers(
    keys: &[(ConsumedSpan, String)],
    tps_final_only: &[Vec<usize>],
    tone_pins: &[TonePin],
    raw_len: u32,
    ctx: &ContinuousFetchCtx<'_>,
) -> Vec<RawCandidate> {
    // Item 12: custom entries can still surface even when the
    // syllabifier produced no `keys` for the FST path (e.g. the
    // dispatcher only reaches `fetch_partial_prefix_candidates` when
    // `keys` is empty, so this fn's `keys.is_empty()` branch is dead
    // for production callers — but a future caller passing empty
    // `keys` + non-empty `custom` must still get the custom merge).
    if keys.is_empty() && ctx.custom.is_empty() {
        return Vec::new();
    }

    let filter = Filter::from_enabled_bitmask(ctx.enabled_sources_bitmask);
    let mut out: Vec<RawCandidate> = Vec::new();

    for (key_index, (span, key)) in keys.iter().enumerate() {
        let final_only: &[usize] = tps_final_only
            .get(key_index)
            .map(|offsets| offsets.as_slice())
            .unwrap_or(&[]);
        // `tone_pins[i]` is the tone constraint for `keys[i]` (§17 typed
        // digits / §41 space-closed TPS tail). A short or empty slice
        // means "no pin", the legacy all-tones shape.
        let tone_pin = tone_pins.get(key_index).unwrap_or(&TonePin::None);
        exact_candidates_for_key(key, final_only, tone_pin, *span, &filter, ctx, |cand| {
            out.push(cand)
        });
    }
    merge_custom_dedupe_sort(
        out,
        ctx,
        raw_len,
        ctx.whole_buffer_key.as_deref(),
        COVERAGE_KIND_FULL,
    )
}
