//! Whole-buffer abbreviation fetch (`*-abbrev:` key family).

use crate::dictionary_reader::{DictionaryReader, Filter};
use ranking::sort_by_candidate_key;

use super::candidate::{
    dedupe_by_roman_hanji_span, merge_custom_abbreviations, record_to_candidate,
};
use super::{ContinuousFetchCtx, RawCandidate, COVERAGE_KIND_ABBREV};

/// Whole-buffer abbreviation lookup — every dictionary row indexed under
/// `fst_key` in the abbreviation family (`tl-abbrev:ss` → 鎖匙 `só-sî`,
/// `tps-abbrev:ㄙㄒ` → 鎖匙; `create_fst.py` emits one `*-abbrev:` key per
/// ≥ 2-syllable row, `dictionary/common/abbrev.py`). `behavioral-invariants.md`
/// §46.
///
/// The family tag is the authority: a `*-abbrev:` key can only be an
/// abbreviation face, so no per-record face check is needed (build-time
/// parity `tests/roman_num_face_parity.rs` / `tests/tps_abbrev_parity.rs`
/// pins the column the key came from). The phonetic families never hold
/// an abbreviation key, which is what lets the span-local / walker /
/// partial-prefix fetches stay acronym-free without any heuristic. Caller
/// gate: `composing::shadow::abbrev_query_key` (letters / glyphs only —
/// no digit, tone mark or separator, so no [`TonePin`] or barrier can
/// apply).
///
/// Per row: source `filter`, then [`record_to_candidate`] with
/// [`COVERAGE_KIND_ABBREV`] at `consumed_span = (0, raw_len)`;
/// `syllable_count` stays the record's (鎖匙 = 2). Sorted by the same
/// [`CandidateSortKey`]; **not truncated** — the caller excludes rows already on
/// screen first, then applies [`PARTIAL_PREFIX_OUTPUT_CAP`] (the Step 4b
/// lesson, Codex PR #351). A custom entry whose abbreviation face IS the
/// key body joins the block the same way ([`merge_custom_abbreviations`]);
/// the phonetic fetches never offer it on an acronym. Hydration is
/// uncapped: `tl-abbrev:tt` resolves ~8.5k rowids and a rank-blind cap in
/// rowid order would drop the frequent words the sort exists to surface.
///
/// [`CandidateSortKey`]: ranking::CandidateSortKey
/// [`PARTIAL_PREFIX_OUTPUT_CAP`]: super::PARTIAL_PREFIX_OUTPUT_CAP
/// [`TonePin`]: super::TonePin
pub fn fetch_abbrev_candidates(
    fst_key: &str,
    raw_len: u32,
    ctx: &ContinuousFetchCtx<'_>,
) -> Vec<RawCandidate> {
    let filter = Filter::from_enabled_bitmask(ctx.enabled_sources_bitmask);
    let rowids = ctx.prefix_index.lookup_exact(fst_key);
    let mut out: Vec<RawCandidate> = Vec::with_capacity(rowids.len());
    for rowid in rowids {
        let Some(record) = ctx.dict.record(rowid) else {
            continue;
        };
        if !DictionaryReader::passes_filter(record.bitmask, record.kautian_subtag, &filter) {
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
            (0, raw_len),
            ctx.freq_map,
            ctx.now_ms,
            ctx.context,
            COVERAGE_KIND_ABBREV,
        ));
    }
    merge_custom_abbreviations(&mut out, fst_key, raw_len, ctx);
    dedupe_by_roman_hanji_span(&mut out);
    sort_by_candidate_key(out, raw_len, RawCandidate::rank_facts)
}
