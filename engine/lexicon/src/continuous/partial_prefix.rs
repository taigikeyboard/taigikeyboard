//! Partial-prefix continuous fetch: candidates whose key extends a buffer the
//! syllabifier could not end.

use phonetics::KeyFamily;

use crate::dictionary_reader::{DictionaryReader, Filter};

use super::candidate::{merge_custom_dedupe_sort, record_to_candidate};
use super::reach::SyllableReach;
use super::toneless_match::matches_continuous_toneless_prefix_key;
use super::{
    ConsumedSpan, ContinuousFetchCtx, RawCandidate, COVERAGE_KIND_PARTIAL_PREFIX,
    PARTIAL_PREFIX_HYDRATE_CAP, PARTIAL_PREFIX_OUTPUT_CAP,
};

/// v3.5.8 Phase 9 Item 10 — partial-prefix candidate fetch for the
/// continuous-input path. Called when the syllabifier failed to find a
/// single valid syllable ending inside `raw` (so
/// [`fetch_candidates_for_keys_with_barriers`] would return empty) and we want the
/// candidate strip to surface engine prefix-match hits below any
/// future full-syllable matches. See
/// `docs/engine/continuous-candidate-display.md` §15.3.D + §15.5.
///
/// Pipeline:
///
/// 1. `prefix_index.lookup_prefix(&key.1)` — FST byte-sorted rowid scan.
/// 2. Hydrate budget cap at [`PARTIAL_PREFIX_HYDRATE_CAP`] rowids —
///    bounds `dict.record` work for single-char prefixes (`tl:k`,
///    `tl:t`) that match hundreds of FST entries. Set well above the
///    output cap so high-frequency short candidates landing past the
///    legacy byte-sort first 30 still reach the sort.
/// 3. Hydrate via `dict.record(rowid)`; drop rows that fail the
///    `enabled_sources_bitmask` filter (D-12 invariant parity with
///    [`fetch_candidates_for_keys_with_barriers`]).
/// 4. Build candidates with `coverage_kind = COVERAGE_KIND_PARTIAL_PREFIX`
///    and `consumed_span = key.0` (caller pins `(0, raw.len())` per
///    Q15.4 — partial-prefix candidates always final-commit).
/// 5. Sort via the same [`CandidateSortKey`] (`coverage_kind` is `1` for
///    the whole batch; never-selected custom rows lead the never-selected
///    rest through `preference_class`).
/// 6. Truncate to [`PARTIAL_PREFIX_OUTPUT_CAP`] after the sort, so the
///    returned slice is the globally best-scoring subset (not the FST
///    byte-sort prefix). Custom entries participate in the sort and
///    contribute to the output count.
///
/// `raw_len` is the byte length of the original pending buffer
/// (`Phase::Continuous { raw }.len()`); kept here for the Tier 0/1
/// (`consumed_span_end == raw_len`) downstream dim even though every
/// partial-prefix candidate has `consumed_span_end == raw_len` today,
/// so its `tier` is always 0 within `coverage_kind == 1`. This keeps
/// the `raw_len` / ctx contract shared with [`fetch_candidates_for_keys_with_barriers`].
///
/// `freq_map` + `now_ms` propagate user-frequency boost and recency
/// rank to partial-prefix hits identically to the full-syllable
/// path. An empty map + `now_ms = 0` is the cold-start neutral.
///
/// **Caller obligation** — `key.1` MUST carry an FST namespace plus
/// a non-empty body (e.g. `"tl:gu"`). Passing the bare namespace
/// (`"tl:"`) is permitted by the empty-string guard but is treated
/// as a legitimate "match every entry under the namespace" query
/// — it hydrates up to [`PARTIAL_PREFIX_HYDRATE_CAP`] FST entries
/// under that prefix, which is a footgun when triggered by misuse
/// rather than design. Production callers go through
/// `composing::shadow::build_partial_prefix_key` (v3.5.9 B-2 renamed
/// from `build_partial_prefix_key_tl` since the emitter is now
/// mode-aware), which returns `None` when the toneless body would be
/// empty and therefore never emits a bare `"tl:"` / `"poj:"` alone.
///
/// [`CandidateSortKey`]: ranking::CandidateSortKey
/// [`fetch_candidates_for_keys_with_barriers`]: super::fetch_candidates_for_keys_with_barriers
pub fn fetch_partial_prefix_candidates(
    key: &(ConsumedSpan, String),
    raw_len: u32,
    ctx: &ContinuousFetchCtx<'_>,
) -> Vec<RawCandidate> {
    let mut out = fetch_partial_prefix_candidates_unbounded(key, raw_len, ctx);
    out.truncate(PARTIAL_PREFIX_OUTPUT_CAP);
    out
}

/// Same hydrate-custom-dedupe-sort pipeline as
/// [`fetch_partial_prefix_candidates`] but **without** the
/// `PARTIAL_PREFIX_OUTPUT_CAP` truncate at the tail.
///
/// Exists to address the Codex PR #351 r3321758666 finding: for inputs
/// whose exact key has many homophones (Codex example: production
/// `tl_notone = hong` has 30+ exact rows; `hong` syllabifies so Step 4b
/// fires), the post-sort truncate inside the bounded wrapper consumes
/// the entire `PARTIAL_PREFIX_OUTPUT_CAP` budget on rows that the
/// caller is about to drop via a cross-batch FULL/PARTIAL dedupe. The
/// caller then sees zero strict-prefix extensions even though they
/// exist in the dictionary's prefix range.
///
/// The fix surfaces the un-truncated sorted pool so the caller can
/// apply its FULL-block exclude BEFORE truncating. The empty-keys
/// branch (which has no FULL block to exclude against) keeps using
/// the bounded wrapper unchanged.
///
/// Caller obligation: must apply its own truncate (typically
/// `PARTIAL_PREFIX_OUTPUT_CAP`) after the cross-batch filter; the
/// returned vec is bounded only by `PARTIAL_PREFIX_HYDRATE_CAP` +
/// custom count.
pub fn fetch_partial_prefix_candidates_unbounded(
    key: &(ConsumedSpan, String),
    raw_len: u32,
    ctx: &ContinuousFetchCtx<'_>,
) -> Vec<RawCandidate> {
    let (span, fst_key) = key;
    if fst_key.is_empty() && ctx.custom.is_empty() {
        return Vec::new();
    }
    let filter = Filter::from_enabled_bitmask(ctx.enabled_sources_bitmask);
    // Pre-allocate to the worst-case pool size so the hydration loop
    // does not grow `out` through the 16/32/64/128/256/512 doubling
    // sequence. Saves 2-3 reallocs on single-char prefixes that
    // saturate `HYDRATE_CAP`.
    let mut out: Vec<RawCandidate> =
        Vec::with_capacity(PARTIAL_PREFIX_HYDRATE_CAP + ctx.custom.len());
    // `take(PARTIAL_PREFIX_HYDRATE_CAP)` bounds `dict.record` work for
    // single-char prefixes (`tl:k`, `tl:t`) that match hundreds of FST
    // entries. The cap is intentionally set far above the visible
    // output size: per-record hydration is a few mmap slice reads
    // plus 2-4 small String allocations, so the extra budget is cheap,
    // but it lets high-frequency short candidates landing past the
    // legacy byte-sort first 30 (e.g. `tl:ki` after a wall of
    // `tl:ka-*` phrases) reach the CandidateSortKey sort.
    //
    // This fn does NOT truncate — the bounded wrapper
    // `fetch_partial_prefix_candidates` applies `PARTIAL_PREFIX_OUTPUT_CAP`
    // for callers that don't need cross-batch exclude; Step 4b in
    // `composing::continuous` takes the un-truncated pool, applies its
    // FULL-block exclude, then truncates itself. Filter rejects still
    // consume hydration budget — goal is bounding worst-case work, not
    // maximizing hits.
    // Item 12: guard the unbounded `lookup_prefix("")` scan — with the
    // early-return now gated on `fst_key.is_empty() && custom.is_empty()`,
    // an empty `fst_key` + non-empty `custom` reaches here and must NOT
    // trigger a whole-FST scan.
    if !fst_key.is_empty() {
        // Spend the hydrate budget on the SHORTEST matched keys first (all
        // modes). The FST wire separator `0xFF` is greater than any UTF-8
        // byte, so a short exact key (`tps:ㄍㄚ` / `tl:ka`) byte-sorts AFTER
        // every longer extension of it; a plain `lookup_prefix(..).take(cap)`
        // therefore front-loads the deepest, longest (rarest) words and
        // buries the short high-frequency single-syllable readings past the
        // cap — user-reported: typing `ㄍ` surfaced only multi-syllable
        // phrases, the common single chars never reached the ranker. Length
        // bucketing is a hydration-budget policy only; the visible order is
        // still the downstream `CandidateSortKey` (recency / score / frequency).
        //
        // Acronym `*_abbrev` keys live in their own `*-abbrev:` family
        // (`create_fst.py`; `fetch_abbrev_candidates` is their only reader),
        // so this range holds phonetic readings only and the shortest bucket
        // is the single-syllable keys — no per-key exclusion (the pre-tag
        // `skip_abbrev` heuristic) is needed to keep 是/sī from being starved
        // by a wall of `tl:sb`-style acronym keys.
        // §35 — the TPS partial hydration resolves through the same
        // ambiguity pattern as the exact paths (single lookup authority):
        // bare `ㄇ` lists ㆬ… words alongside ㄇ… words. The matched key
        // travels with each rowid so the record guard below validates what
        // the pattern actually hit, not the literal prefix (Codex
        // post-impl 2026-08-19 BLOCK 1). TL/POJ keep the plain lookup.
        let hits: Vec<(String, u32)> = if fst_key.starts_with(KeyFamily::Tps.prefix()) {
            ctx.prefix_index
                .lookup_prefix_shortest_first_tps_readings(fst_key, PARTIAL_PREFIX_HYDRATE_CAP)
        } else {
            ctx.prefix_index
                .lookup_prefix_shortest_first(fst_key, PARTIAL_PREFIX_HYDRATE_CAP)
                .into_iter()
                .map(|rowid| (fst_key.to_string(), rowid))
                .collect()
        };
        // Loop-invariant: the family and the typed body's length are the
        // same for every hydrated row.
        let reach = SyllableReach::new(fst_key);
        for (matched_key, rowid) in hits {
            let Some(record) = ctx.dict.record(rowid) else {
                continue;
            };
            if !DictionaryReader::passes_filter(record.bitmask, record.kautian_subtag, &filter) {
                continue;
            }
            // Codex PR #351 r3319500948 — drop `tl_abbrev` / `poj_abbrev` /
            // `tps_abbrev` collisions whose FST key happens to share the
            // input prefix. Mirrors the span-local + walker guard at
            // `fetch_candidates_for_keys_with_barriers` /
            // `best_candidate_for_key_with_barriers` (`matches_continuous_toneless_key`)
            // but uses the prefix-aware `*_prefix_key` variant — the
            // partial-prefix path's key body is a STRICT PREFIX of the
            // toneless, so equality would reject every legitimate
            // extension hit.
            // §35 — prefix guard runs on the MATCHED key: a substituted
            // hit (`tps:ㆬㄒㄧ` under typed `tps:ㄇ`) reconstructs to the
            // matched form; the literal prefix would reject it.
            if !matches_continuous_toneless_prefix_key(&matched_key, &record.tl) {
                continue;
            }
            // Tone pin: a strict-prefix extension is only eligible when
            // the syllables typed so far carry the typed tones (§17) / the
            // space-closed unmarked tone (§41). TL / POJ read the typed body
            // the pin carries; TPS aligns its closes on the MATCHED key (a
            // §35 hit `tps:ㆬㄒㄧ` under typed `ㄇ˫ㄒ` reconstructs to `ㆬ`),
            // each close cut at its own offset, so the extension's later
            // syllables past the last close never take part.
            if !ctx.tone_pin.admits(Some(&matched_key), &record.tl) {
                continue;
            }
            // Syllable reach: a strict-prefix extension may not carry a
            // syllable the user never typed into (`tsuisi` must not surface
            // 水社寮 `tsuí-siā-liâu`). The custom merge below answers the
            // same rule through [`custom_reach`].
            if !reach
                .as_ref()
                .is_none_or(|reach| reach.admits(&matched_key, &record.tl))
            {
                continue;
            }
            let effective = DictionaryReader::effective_source_bitmask(
                record.bitmask,
                record.kautian_subtag,
                &filter,
            );
            out.push(record_to_candidate(
                record,
                effective,
                *span,
                ctx.freq_map,
                ctx.now_ms,
                ctx.context,
                COVERAGE_KIND_PARTIAL_PREFIX,
            ));
        }
    }
    let typed_key = (!fst_key.is_empty()).then_some(fst_key.as_str());
    merge_custom_dedupe_sort(out, ctx, raw_len, typed_key, COVERAGE_KIND_PARTIAL_PREFIX)
}
