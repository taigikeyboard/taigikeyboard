//! Continuous candidate ordering: the 8-dimensional [`SortKey`] and its NaN-safe
//! score wrapper.

use std::cmp::Reverse;

use ranking::source_tier_rank;

use super::RawCandidate;

/// The eight-dimension [`SortKey`] sort every continuous fetch ends with.
/// `stable_idx` is the pre-sort element position (see
/// [`merge_custom_dedupe_sort`](super::candidate::merge_custom_dedupe_sort) for why it is stamped via `enumerate()`
/// rather than inside the key extractor).
pub(super) fn sort_by_sort_key(out: Vec<RawCandidate>, raw_len: u32) -> Vec<RawCandidate> {
    let mut indexed: Vec<(SortKey, RawCandidate)> = out
        .into_iter()
        .enumerate()
        .map(|(i, c)| (SortKey::new(&c, raw_len, i as u32), c))
        .collect();
    indexed.sort_by_key(|(key, _)| *key);
    indexed.into_iter().map(|(_, c)| c).collect()
}

// v3.5.8 Phase 9.1 — SortKey
//
// Encodes the eight-dimension lexicographic sort policy pinned in
// `docs/releases/v3.5.8/plan.md` § Phase 9 (+ whole-sentence lattice + walker S8). Field
// order in this struct matches `#[derive(Ord)]`'s lexicographic
// comparison; `Reverse<T>` flips individual dimensions whose policy
// is descending. NaN-safe because floats are wrapped in `NonNanF64`
// which coerces NaN to `f64::MIN` at construction.
//
// Order: coverage_kind, tier, -user_weight, -score, -freq,
// -coverage, source_rank, stable_idx. S8 moved `-coverage` from
// dim 3 (above score) down to dim 6 (a weak tiebreak below
// score/freq): the slot-0 whole-sentence walker now owns phrase
// priority, so longest-coverage-first inside a tier only buried the
// short single-syllable first-segment candidate.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct SortKey {
    /// v3.5.8 Phase 9 Item 10 — leading dim. `0` for full-syllable
    /// (the pre-Item-10 `fetch_candidates_for_keys_with_barriers` path) and `1`
    /// for partial-prefix ([`fetch_partial_prefix_candidates`](super::fetch_partial_prefix_candidates)).
    /// Sits ahead of [`tier`](Self::tier) because partial-prefix
    /// candidates have `consumed_span_end == raw_len`
    /// (Q15.4 → `tier = 0`); without this dim a partial-prefix
    /// tier-0 candidate would outrank a future full-syllable
    /// tier-1 candidate, violating §15.5 "rank below regardless of
    /// frequency". See `docs/engine/continuous-candidate-display.md`
    /// §15.5.
    coverage_kind: u8,
    /// `0` = Tier 0 (full-buffer coverage), `1` = Tier 1 (partial).
    /// Roadmap and spec both use the "Tier 0 = full buffer" labelling
    /// (`docs/releases/v3.5.8/plan.md` § Phase 9 / `docs/engine/continuous-input-
    /// ranking.md` §1.1).
    tier: u8,
    /// Descending: [`RawCandidate::user_weight`] — a selected word
    /// (`> 0.0`) precedes every never-selected one (`0.0`) whatever
    /// their dictionary frequency. `f64` so two selections seconds
    /// apart do not collapse into a tie.
    neg_user_weight: Reverse<NonNanF64>,
    /// Descending: higher `freq × syll_bias × boost` wins.
    neg_score: Reverse<NonNanF64>,
    /// Descending: raw freq as a secondary tie-break independent of
    /// the syllable-biased score.
    neg_freq: Reverse<u32>,
    /// Descending: longer coverage wins — but only as a weak tiebreak
    /// AFTER `neg_score` / `neg_freq`. v3.5.8 whole-sentence lattice + walker S8
    /// relocated this from dim 3 to here. The slot-0 whole-sentence
    /// walker owns phrase priority, so a graded longest-coverage-first
    /// rule inside a tier only buried the short single-syllable
    /// first-segment candidate the user wants for segment-by-segment
    /// selection. Coverage now separates two candidates only when their
    /// score AND freq are equal — aligned with librime's per-segment
    /// menu, which keeps multi-length candidates but never lets a
    /// longer code-length bury a shorter strict match
    /// (`references/librime/src/rime/gear/script_translator.cc`
    /// `kNumExactMatchOnTop`).
    neg_coverage: Reverse<u32>,
    /// Ascending: `custom=0, kautian=1, taigitv=2, stti=3, kungge=4,
    /// default=5` per `ranking::source_tier_rank`.
    source_rank: u8,
    /// Insertion index — deterministic by caller-provided `keys`
    /// order × `prefix_index.lookup_exact` FST byte-sort.
    stable_idx: u32,
}

impl SortKey {
    pub(super) fn new(candidate: &RawCandidate, raw_len: u32, stable_idx: u32) -> Self {
        let (start, end) = candidate.consumed_span;
        let coverage_bytes = end.saturating_sub(start);
        let tier: u8 = if end == raw_len { 0 } else { 1 };
        // v3.5.8 Phase 9 Item 12 — `is_custom` is now carried on the
        // candidate (`record_to_candidate` → false, dict.bin source
        // bits; `custom_entry_to_candidate` → true, forces rank 0).
        // Before Item 12 this was hardcoded `false` because no caller
        // could produce a custom candidate yet.
        let source_rank = source_tier_rank(candidate.bitmask, candidate.is_custom);
        Self {
            coverage_kind: candidate.coverage_kind,
            tier,
            neg_user_weight: Reverse(NonNanF64::new(candidate.user_weight)),
            neg_score: Reverse(NonNanF64::new(f64::from(candidate.score))),
            neg_freq: Reverse(candidate.frequency),
            neg_coverage: Reverse(coverage_bytes),
            source_rank,
            stable_idx,
        }
    }
}

/// `f64` newtype with a total order via [`f64::total_cmp`] after
/// coercing `NaN` to [`f64::MIN`]. Lets [`SortKey`] derive `Ord`
/// without a hand-written comparator, while still defending against
/// `NaN` leakage from a contract-violating `user_freq_boost`
/// (`calculate_continuous_score` docs).
#[derive(Debug, Clone, Copy)]
pub(super) struct NonNanF64(f64);

impl NonNanF64 {
    fn new(v: f64) -> Self {
        Self(if v.is_nan() { f64::MIN } else { v })
    }
}

impl PartialEq for NonNanF64 {
    fn eq(&self, other: &Self) -> bool {
        self.0.total_cmp(&other.0) == std::cmp::Ordering::Equal
    }
}

impl Eq for NonNanF64 {}

impl PartialOrd for NonNanF64 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NonNanF64 {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

#[cfg(test)]
mod sort_key_tests {
    //! Hermetic unit tests for the v3.5.8 Phase 9.1 `SortKey` policy.
    //! Hermetic-fixture regression tests (`taiuantaigi` / `e` / `taixyz`
    //! acceptance matrix) live alongside in
    //! `engine/lexicon/tests/span_local_fetch.rs`, using the same
    //! `build_fixture` synthetic dict.bin + FST builder.
    use super::*;
    use crate::continuous::{
        CandidateMode, COVERAGE_KIND_FULL, COVERAGE_KIND_PARTIAL_PREFIX, FORM_NOTONE,
    };

    /// Convenience builder so each test only specifies the dimensions
    /// it exercises. Fields not exercised default to neutral values:
    /// `frequency = 0`, `bitmask = 0` (→ source rank = default = 5),
    /// `score = 0.0`, `syllable_count = 1`, `user_weight = 0.0` (never
    /// selected = the cold-start default).
    fn cand(
        span_start: u32,
        span_end: u32,
        score: f32,
        frequency: u32,
        bitmask: u16,
    ) -> RawCandidate {
        cand_with_user_weight(span_start, span_end, score, frequency, bitmask, 0.0)
    }

    fn cand_with_user_weight(
        span_start: u32,
        span_end: u32,
        score: f32,
        frequency: u32,
        bitmask: u16,
        user_weight: f64,
    ) -> RawCandidate {
        RawCandidate {
            consumed_span: (span_start, span_end),
            syllable_count: 1,
            display_text: String::new(),
            roman: String::new(),
            hanji: None,
            canonical_tl: String::new(),
            score,
            form: FORM_NOTONE,
            frequency,
            bitmask,
            mode: CandidateMode::Hant,
            user_weight,
            coverage_kind: COVERAGE_KIND_FULL,
            is_custom: false,
        }
    }

    /// v3.5.8 Phase 9 Item 10 — partial-prefix variant for the new
    /// `coverage_kind` dim. Defaults to user_weight=0.0 (never
    /// selected) so tests can isolate the coverage_kind axis without
    /// mixing in user preference.
    fn cand_partial(
        span_start: u32,
        span_end: u32,
        score: f32,
        frequency: u32,
        bitmask: u16,
    ) -> RawCandidate {
        let mut c = cand_with_user_weight(span_start, span_end, score, frequency, bitmask, 0.0);
        c.coverage_kind = COVERAGE_KIND_PARTIAL_PREFIX;
        c
    }

    #[test]
    fn tier0_full_buffer_beats_tier1_partial_even_when_score_lower() {
        // Phase 9.1 headline behavior: Tier 0 (full buffer) wins over
        // Tier 1 (partial) regardless of raw score. Mirrors the
        // `taiuantaigi` motivation case (`docs/releases/v3.5.8/plan.md` § Phase 9
        // sort_key formula).
        let raw_len: u32 = 11;
        let phrase = cand(0, 11, 15.6, 12, 0); // Tier 0 by span_end == raw_len.
        let single = cand(0, 3, 31281.0, 31281, 0); // Tier 1, dominant score.

        assert!(
            SortKey::new(&phrase, raw_len, 0) < SortKey::new(&single, raw_len, 1),
            "Tier 0 phrase must precede Tier 1 single-char in lexicographic sort"
        );
    }

    #[test]
    fn within_tier_higher_score_beats_longer_coverage() {
        // v3.5.8 whole-sentence lattice + walker S8 (was
        // `within_tier_longer_coverage_beats_shorter`, which pinned the
        // pre-S8 policy that caused the `guaikingkahuekhoo` dogfood
        // bug). `-coverage_bytes` is now dim 6, BELOW `-adjusted_score`
        // / `-freq`. A 3-byte coverage with score 1000 must now win
        // over a 6-byte coverage with score 100 when both are Tier 1.
        let raw_len: u32 = 9; // neither span hits full buffer
        let longer = cand(0, 6, 100.0, 100, 0);
        let shorter = cand(0, 3, 1000.0, 1000, 0);

        assert!(SortKey::new(&shorter, raw_len, 0) < SortKey::new(&longer, raw_len, 1));
    }

    #[test]
    fn single_syllable_first_segment_not_buried_by_longer_prefix() {
        // Dogfood bug pin (`guaikingkahuekhoo` → 「我」/Guá buried).
        // When no span-local candidate covers the full buffer (the
        // whole-sentence path is the slot-0 walker, prepended
        // separately), a high-freq single-syllable first-segment
        // candidate must rank ABOVE a longer left-anchored prefix
        // candidate of lower freq — so segment-by-segment selection is
        // fast. Both Tier 1; only score/freq vs coverage differ.
        let raw_len: u32 = 17; // guaikingkahuekhoo; no span hits it
        let single = cand(0, 3, 31281.0, 31281, 0); // gua → 我
        let longer_prefix = cand(0, 6, 1379.0, 1379, 0); // a 2-syll prefix
        assert!(
            SortKey::new(&single, raw_len, 0) < SortKey::new(&longer_prefix, raw_len, 1),
            "high-freq single-syllable first segment must not be buried \
             below a lower-freq longer prefix"
        );
    }

    #[test]
    fn coverage_breaks_tie_only_when_score_and_freq_equal() {
        // S8: `-coverage_bytes` survives as a weak deterministic
        // tiebreak — when score AND freq are identical, the longer
        // coverage still precedes the shorter one (same Tier 1).
        let raw_len: u32 = 9;
        let longer = cand(0, 6, 100.0, 100, 0);
        let shorter = cand(0, 3, 100.0, 100, 0);

        assert!(SortKey::new(&longer, raw_len, 0) < SortKey::new(&shorter, raw_len, 1));
    }

    #[test]
    fn within_same_tier_and_coverage_higher_score_wins() {
        let raw_len: u32 = 4;
        let high = cand(0, 4, 100.0, 100, 0);
        let low = cand(0, 4, 88.0, 80, 0);

        assert!(SortKey::new(&high, raw_len, 0) < SortKey::new(&low, raw_len, 1));
    }

    #[test]
    fn source_rank_breaks_ties_when_score_and_freq_match() {
        // Same span, score, freq — only `bitmask` differs.
        // kautian (bit 0, rank 1) should precede an unknown source
        // (rank 5).
        let raw_len: u32 = 3;
        const KAUTIAN_BIT: u16 = 1 << 0;
        let kautian = cand(0, 3, 100.0, 100, KAUTIAN_BIT);
        let unknown = cand(0, 3, 100.0, 100, 0);

        assert!(SortKey::new(&kautian, raw_len, 0) < SortKey::new(&unknown, raw_len, 1));
    }

    #[test]
    fn stable_idx_breaks_ties_when_all_else_equal() {
        // Identical RawCandidate, only the synthetic insertion index
        // varies — earlier index must sort first.
        let raw_len: u32 = 3;
        let a = cand(0, 3, 100.0, 100, 0);
        let b = cand(0, 3, 100.0, 100, 0);

        assert!(SortKey::new(&a, raw_len, 0) < SortKey::new(&b, raw_len, 1));
    }

    #[test]
    fn nan_score_is_coerced_to_minimum_not_panic() {
        // Phase 5 NaN-defense invariant preserved: a NaN score loses
        // every comparison instead of poisoning the sort.
        let raw_len: u32 = 3;
        let nan = cand(0, 3, f32::NAN, 100, 0);
        let normal = cand(0, 3, 0.001, 100, 0);

        // NaN coerced to f32::MIN → with Reverse<>, NaN ends up LAST
        // (largest sort_key in ascending order).
        assert!(SortKey::new(&normal, raw_len, 0) < SortKey::new(&nan, raw_len, 1));
    }

    #[test]
    fn sort_key_reads_user_weight_from_candidate() {
        // `SortKey::new` reads `candidate.user_weight` verbatim — no
        // sentinel, no recomputation.
        let raw_len: u32 = 3;
        let never = SortKey::new(&cand(0, 3, 1.0, 1, 0), raw_len, 0);
        assert_eq!(never.neg_user_weight, Reverse(NonNanF64::new(0.0)));
        let selected = SortKey::new(&cand_with_user_weight(0, 3, 1.0, 1, 0, 0.1), raw_len, 1);
        assert_eq!(selected.neg_user_weight, Reverse(NonNanF64::new(0.1)));
    }

    #[test]
    fn selected_word_beats_never_selected_regardless_of_score() {
        // Headline behaviour (2026-09-14 更新/敬神 bug): within the same
        // `(coverage_kind, tier)` bucket a word the user selected ONCE
        // (weight 0.1) precedes a never-selected homophone whose
        // dictionary score is 25× higher — user preference is a
        // lexicographic dim above `-score`, not a capped multiplier.
        let raw_len: u32 = 3;
        let selected_rare = cand_with_user_weight(0, 3, 1.1, 1, 0, 0.1);
        let never_common = cand(0, 3, 27.5, 25, 0);
        assert!(
            SortKey::new(&selected_rare, raw_len, 1) < SortKey::new(&never_common, raw_len, 0),
            "user_weight > 0 must precede user_weight == 0 whatever the score"
        );
    }

    #[test]
    fn higher_user_weight_beats_lower_when_both_selected() {
        // Two selected homophones: the heavier (more / more recent
        // selections) wins; score only breaks a weight tie.
        let raw_len: u32 = 3;
        let heavy_rare = cand_with_user_weight(0, 3, 1.0, 1, 0, 2.0);
        let light_common = cand_with_user_weight(0, 3, 100.0, 100, 0, 0.5);
        assert!(SortKey::new(&heavy_rare, raw_len, 1) < SortKey::new(&light_common, raw_len, 0));
        let tie_high = cand_with_user_weight(0, 3, 100.0, 100, 0, 0.5);
        let tie_low = cand_with_user_weight(0, 3, 1.0, 1, 0, 0.5);
        assert!(SortKey::new(&tie_high, raw_len, 1) < SortKey::new(&tie_low, raw_len, 0));
    }

    #[test]
    fn empty_span_yields_zero_coverage_without_panic() {
        // Defensive: a degenerate `(2, 2)` span (consumed_span_end ==
        // start) must produce SortKey with `neg_coverage = Reverse(0)`,
        // not panic in `saturating_sub`.
        let raw_len: u32 = 4;
        let key = SortKey::new(&cand(2, 2, 0.0, 0, 0), raw_len, 0);
        assert_eq!(key.neg_coverage, Reverse(0));
        // Tier 1 because end (2) != raw_len (4).
        assert_eq!(key.tier, 1);
    }

    // ----- v3.5.8 Phase 9 Item 10 — `coverage_kind` SortKey dim -----

    #[test]
    fn coverage_kind_full_beats_partial_regardless_of_other_dims() {
        // Item 10 headline invariant: a partial-prefix candidate with
        // MAX freq + MAX score + a heavy user weight + best source rank
        // must STILL lose to a full-syllable candidate with min freq,
        // min score, never selected, and worst source rank — the
        // leading `coverage_kind` dim is load-bearing.
        // Both candidates have `end == raw_len` so their `tier`
        // dimension is identical (0) — the regression this guards
        // against is a tier-0 partial beating a tier-1 full when
        // `coverage_kind` is mis-ordered.
        let raw_len: u32 = 3;
        let full_weak = cand_with_user_weight(0, 3, 0.001, 1, 0, 0.0);
        let mut partial_strong = cand_partial(0, 3, f32::MAX, u32::MAX, KAUTIAN_BIT_U16);
        partial_strong.user_weight = 4.0;
        // Sanity: the partial helper sets `coverage_kind = 1` while
        // the full helper leaves it at the default `0`.
        assert_eq!(full_weak.coverage_kind, COVERAGE_KIND_FULL);
        assert_eq!(partial_strong.coverage_kind, COVERAGE_KIND_PARTIAL_PREFIX);
        assert!(
            SortKey::new(&full_weak, raw_len, 0) < SortKey::new(&partial_strong, raw_len, 1),
            "Item 10: full-syllable must precede partial-prefix regardless of other dims"
        );
    }

    #[test]
    fn within_partial_prefix_inner_dims_still_apply() {
        // Inside the `coverage_kind = 1` bucket the
        // `(tier, recency, -score, -freq, -coverage, source, stable_idx)`
        // policy (post-S8 order) still drives ordering — verify with
        // two partials where only `-score` differs.
        let raw_len: u32 = 4;
        let high = cand_partial(0, 4, 100.0, 100, 0);
        let low = cand_partial(0, 4, 1.0, 1, 0);
        assert!(
            SortKey::new(&high, raw_len, 0) < SortKey::new(&low, raw_len, 1),
            "inside coverage_kind=1, higher score still wins"
        );
    }

    /// Bitmask helper for the kautian source bit (1 << 0); declared
    /// here as a local `u16` to keep the test fixture self-contained
    /// without re-importing from `ranking::score::tests`.
    const KAUTIAN_BIT_U16: u16 = 1 << 0;
}
