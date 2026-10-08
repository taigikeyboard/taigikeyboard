//! Continuous candidate ordering: the ten-dimension [`CandidateSortKey`] the
//! candidate list and the walker's edge pick share, the [`CandidateRankFacts`]
//! it reads, and its NaN-safe score wrapper.

use std::cmp::Reverse;

use crate::{source_tier_rank, CONTEXT_RANK_NONE};

/// The candidate fields the continuous sort reads — everything
/// [`CandidateSortKey::new`] needs besides the buffer length and the
/// insertion index. Built by `lexicon::RawCandidate::rank_facts`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CandidateRankFacts {
    /// `lexicon::COVERAGE_KIND_*`: full syllable `0` < partial prefix `1`
    /// < abbreviation `2`.
    pub coverage_kind: u8,
    /// Raw-buffer byte span `(start, end)` the candidate consumes.
    pub consumed_span: (u32, u32),
    /// Time-decayed user-selection weight; `0.0` = never selected.
    pub user_weight: f64,
    /// `CONTEXT_RANK_*` — previous-word continuation layer.
    pub context_rank: u8,
    /// `dict.bin` corpus cost (`−ln p`, milli-nats); [`WALKER_COST_UNPRICED`]
    /// for a candidate the corpus does not price (custom, learned, synthesized).
    pub walker_cost: u16,
    /// [`crate::calculate_continuous_score`].
    pub score: f32,
    /// Raw dictionary frequency.
    pub frequency: u32,
    /// Dictionary source bitmask (`dict.bin` source bits).
    pub bitmask: u16,
    /// `true` for a custom-dictionary entry (source rank 0).
    pub is_custom: bool,
}

/// [`CandidateRankFacts::walker_cost`] of a candidate the corpus does not price
/// (custom, learned, synthesized): it sorts after every priced candidate of
/// its tier unless user weight or the context lifts it (E1 P5b).
pub const WALKER_COST_UNPRICED: u16 = u16::MAX;

/// The [`CandidateSortKey`] sort every continuous fetch
/// ends with. `stable_idx` is the pre-sort element position, stamped via
/// `enumerate()` BEFORE the sort rather than inside the key extractor so
/// it stays the caller's insertion order.
pub fn sort_by_candidate_key<T>(
    items: Vec<T>,
    raw_len: u32,
    facts: impl Fn(&T) -> CandidateRankFacts,
) -> Vec<T> {
    let mut indexed: Vec<(CandidateSortKey, T)> = items
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            (
                CandidateSortKey::new(&facts(&item), raw_len, i as u32),
                item,
            )
        })
        .collect();
    indexed.sort_by_key(|(key, _)| *key);
    indexed.into_iter().map(|(_, item)| item).collect()
}

// v3.5.8 Phase 9.1 — CandidateSortKey
//
// Encodes the lexicographic sort policy pinned in
// `docs/releases/v3.5.8/plan.md` § Phase 9 (+ whole-sentence lattice + walker S8). Field
// order in this struct matches `#[derive(Ord)]`'s lexicographic
// comparison; `Reverse<T>` flips individual dimensions whose policy
// is descending. NaN-safe because floats are wrapped in `NonNanF64`
// which coerces NaN to `f64::MIN` at construction.
//
// Order: coverage_kind, tier, -user_weight, context_rank, walker_cost, -score,
// -freq, -coverage, source_rank, stable_idx. Bigram P5 (§56) added
// `context_rank` between `-user_weight` and `-score`; E1 P5a added
// `walker_cost` below it for the walker's edge pick, and P5b made the
// candidate list read it too, so the list and slot 0 share one scale. S8 moved `-coverage` from
// dim 3 (above score) down to dim 6 (a weak tiebreak below
// score/freq): the slot-0 whole-sentence walker now owns phrase
// priority, so longest-coverage-first inside a tier only buried the
// short single-syllable first-segment candidate.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CandidateSortKey {
    /// v3.5.8 Phase 9 Item 10 — leading dim. `0` for full-syllable
    /// (the pre-Item-10 `fetch_candidates_for_keys_with_barriers` path) and `1`
    /// for partial-prefix (`lexicon::fetch_partial_prefix_candidates`).
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
    /// Descending: [`CandidateRankFacts::user_weight`] — a selected word
    /// (`> 0.0`) precedes every never-selected one (`0.0`) whatever
    /// their dictionary frequency. `f64` so two selections seconds
    /// apart do not collapse into a tie.
    neg_user_weight: Reverse<NonNanF64>,
    /// Ascending: [`CandidateRankFacts::context_rank`] — a continuation of the
    /// previous word (user-learned first, then bundled) precedes the rest
    /// (§56). Below `user_weight` so a selected word still beats every
    /// never-selected one; above `score` so the context is more than a
    /// tie-break. All-`CONTEXT_RANK_NONE` (no context) changes nothing.
    context_rank: u8,
    /// Ascending: [`CandidateRankFacts::walker_cost`] — the word the corpus
    /// writes most first, in the list and in the walker's edge pick alike
    /// (E1 P5a / P5b, `docs/architecture/unified-word-frequency-roadmap.md`).
    /// Equal costs (corpus-unseen words share one smoothed cost; nearby counts
    /// round to one milli-nat) fall to `score` and the dimensions below. An
    /// unpriced candidate (custom, learned) sorts after every priced one
    /// unless a dimension above decides.
    walker_cost: u16,
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
    /// default=5` per [`source_tier_rank`].
    source_rank: u8,
    /// Insertion index — deterministic by caller-provided `keys`
    /// order × `prefix_index.lookup_exact` FST byte-sort.
    stable_idx: u32,
}

impl CandidateSortKey {
    pub fn new(facts: &CandidateRankFacts, raw_len: u32, stable_idx: u32) -> Self {
        let (start, end) = facts.consumed_span;
        let coverage_bytes = end.saturating_sub(start);
        let tier: u8 = if end == raw_len { 0 } else { 1 };
        // v3.5.8 Phase 9 Item 12 — `is_custom` is carried on the
        // candidate (`record_to_candidate` → false, dict.bin source
        // bits; `custom_entry_to_candidate` → true, forces rank 0).
        let source_rank = source_tier_rank(facts.bitmask, facts.is_custom);
        Self {
            coverage_kind: facts.coverage_kind,
            tier,
            neg_user_weight: Reverse(NonNanF64::new(facts.user_weight)),
            context_rank: facts.context_rank,
            walker_cost: facts.walker_cost,
            neg_score: Reverse(NonNanF64::new(f64::from(facts.score))),
            neg_freq: Reverse(facts.frequency),
            neg_coverage: Reverse(coverage_bytes),
            source_rank,
            stable_idx,
        }
    }

    /// [`CandidateSortKey::new`] with the context dimension neutral — the
    /// context-free order the walker's edge pick anchors on (§56).
    pub fn without_context(facts: &CandidateRankFacts, raw_len: u32, stable_idx: u32) -> Self {
        Self {
            context_rank: CONTEXT_RANK_NONE,
            ..Self::new(facts, raw_len, stable_idx)
        }
    }
}

/// `f64` newtype with a total order via [`f64::total_cmp`] after
/// coercing `NaN` to [`f64::MIN`]. Lets [`CandidateSortKey`] derive `Ord`
/// without a hand-written comparator, while still defending against
/// `NaN` leakage from a contract-violating `user_freq_boost`
/// ([`crate::calculate_continuous_score`] docs).
#[derive(Debug, Clone, Copy)]
struct NonNanF64(f64);

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
mod tests {
    //! Hermetic unit tests for the v3.5.8 Phase 9.1 `CandidateSortKey`
    //! policy. Hermetic-fixture regression tests (`taiuantaigi` / `e` /
    //! `taixyz` acceptance matrix) live in
    //! `engine/lexicon/tests/span_local_fetch.rs`.
    use super::*;
    use crate::{CONTEXT_RANK_BUNDLED, CONTEXT_RANK_USER};

    /// `lexicon::COVERAGE_KIND_FULL` / `COVERAGE_KIND_PARTIAL_PREFIX`.
    const COVERAGE_KIND_FULL: u8 = 0;
    const COVERAGE_KIND_PARTIAL_PREFIX: u8 = 1;

    /// Kautian source bit (1 << 0) — source rank 1.
    const KAUTIAN_BIT: u16 = 1 << 0;

    /// The cost every corpus-unseen word shares at α 10 — one value for
    /// every fixture, so tests of the other dimensions see a cost tie.
    const UNSEEN_WALKER_COST: u16 = 13_181;

    /// Convenience builder so each test only specifies the dimensions
    /// it exercises. Fields not exercised default to neutral values:
    /// `bitmask = 0` (→ source rank = default = 5), `user_weight = 0.0`
    /// (never selected = the cold-start default), no context, the shared
    /// unseen corpus cost.
    fn cand(
        span_start: u32,
        span_end: u32,
        score: f32,
        frequency: u32,
        bitmask: u16,
    ) -> CandidateRankFacts {
        cand_with_user_weight(span_start, span_end, score, frequency, bitmask, 0.0)
    }

    fn cand_with_user_weight(
        span_start: u32,
        span_end: u32,
        score: f32,
        frequency: u32,
        bitmask: u16,
        user_weight: f64,
    ) -> CandidateRankFacts {
        CandidateRankFacts {
            coverage_kind: COVERAGE_KIND_FULL,
            consumed_span: (span_start, span_end),
            user_weight,
            context_rank: CONTEXT_RANK_NONE,
            walker_cost: UNSEEN_WALKER_COST,
            score,
            frequency,
            bitmask,
            is_custom: false,
        }
    }

    /// v3.5.8 Phase 9 Item 10 — partial-prefix variant for the
    /// `coverage_kind` dim. Never selected, so tests isolate the
    /// coverage_kind axis without mixing in user preference.
    fn cand_partial(
        span_start: u32,
        span_end: u32,
        score: f32,
        frequency: u32,
        bitmask: u16,
    ) -> CandidateRankFacts {
        CandidateRankFacts {
            coverage_kind: COVERAGE_KIND_PARTIAL_PREFIX,
            ..cand(span_start, span_end, score, frequency, bitmask)
        }
    }

    fn cand_with_context(score: f32, context_rank: u8) -> CandidateRankFacts {
        CandidateRankFacts {
            context_rank,
            ..cand(0, 3, score, score as u32, 0)
        }
    }

    fn key(facts: &CandidateRankFacts, raw_len: u32, stable_idx: u32) -> CandidateSortKey {
        CandidateSortKey::new(facts, raw_len, stable_idx)
    }

    #[test]
    fn sort_orders_by_key_and_keeps_insertion_order_on_ties() {
        let raw_len: u32 = 3;
        let items = vec![
            ("tie-first", cand(0, 3, 1.0, 1, 0)),
            ("high", cand(0, 3, 100.0, 100, 0)),
            ("tie-second", cand(0, 3, 1.0, 1, 0)),
        ];
        let sorted = sort_by_candidate_key(items, raw_len, |(_, facts)| *facts);
        let order: Vec<&str> = sorted.iter().map(|(name, _)| *name).collect();
        assert_eq!(order, ["high", "tie-first", "tie-second"]);
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
            key(&phrase, raw_len, 0) < key(&single, raw_len, 1),
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

        assert!(key(&shorter, raw_len, 0) < key(&longer, raw_len, 1));
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
            key(&single, raw_len, 0) < key(&longer_prefix, raw_len, 1),
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

        assert!(key(&longer, raw_len, 0) < key(&shorter, raw_len, 1));
    }

    #[test]
    fn within_same_tier_and_coverage_higher_score_wins() {
        let raw_len: u32 = 4;
        let high = cand(0, 4, 100.0, 100, 0);
        let low = cand(0, 4, 88.0, 80, 0);

        assert!(key(&high, raw_len, 0) < key(&low, raw_len, 1));
    }

    #[test]
    fn source_rank_breaks_ties_when_score_and_freq_match() {
        // Same span, score, freq — only `bitmask` differs.
        // kautian (bit 0, rank 1) should precede an unknown source
        // (rank 5).
        let raw_len: u32 = 3;
        let kautian = cand(0, 3, 100.0, 100, KAUTIAN_BIT);
        let unknown = cand(0, 3, 100.0, 100, 0);

        assert!(key(&kautian, raw_len, 0) < key(&unknown, raw_len, 1));
    }

    // INVARIANT_CONTINUOUS_CONTEXT_RERANK (behavioral-invariants §56): a
    // continuation of the previous word precedes a higher-scored stranger.
    #[test]
    fn context_hit_beats_higher_score() {
        let raw_len: u32 = 3;
        let stranger = cand_with_context(1000.0, CONTEXT_RANK_NONE);
        let bundled = cand_with_context(10.0, CONTEXT_RANK_BUNDLED);
        let user = cand_with_context(1.0, CONTEXT_RANK_USER);
        assert!(key(&bundled, raw_len, 1) < key(&stranger, raw_len, 0));
        assert!(key(&user, raw_len, 2) < key(&bundled, raw_len, 1));
    }

    // INVARIANT_CONTINUOUS_CONTEXT_RERANK: the context never crosses the
    // dimensions above it — coverage kind, tier, user weight.
    #[test]
    fn context_hit_never_crosses_tier_coverage_kind_or_user_weight() {
        let raw_len: u32 = 6;
        let full_stranger = cand(0, 6, 1.0, 1, 0);
        let mut partial_hit = cand_partial(0, 6, 1000.0, 1000, 0);
        partial_hit.context_rank = CONTEXT_RANK_USER;
        assert!(key(&full_stranger, raw_len, 0) < key(&partial_hit, raw_len, 1));

        let tier0_stranger = cand(0, 6, 1.0, 1, 0);
        let mut tier1_hit = cand(0, 3, 1000.0, 1000, 0);
        tier1_hit.context_rank = CONTEXT_RANK_USER;
        assert!(key(&tier0_stranger, raw_len, 0) < key(&tier1_hit, raw_len, 1));

        let selected_stranger = cand_with_user_weight(0, 6, 1.0, 1, 0, 0.1);
        let mut never_selected_hit = cand(0, 6, 1000.0, 1000, 0);
        never_selected_hit.context_rank = CONTEXT_RANK_USER;
        assert!(key(&selected_stranger, raw_len, 0) < key(&never_selected_hit, raw_len, 1));
    }

    // INVARIANT_CONTINUOUS_CONTEXT_RERANK: no context (every rank NONE) is
    // the context-free order, and `without_context` neutralises a hit.
    #[test]
    fn no_context_is_the_context_free_order() {
        let raw_len: u32 = 3;
        let high = cand_with_context(100.0, CONTEXT_RANK_NONE);
        let low = cand_with_context(10.0, CONTEXT_RANK_NONE);
        assert!(key(&high, raw_len, 0) < key(&low, raw_len, 1));
        let low_hit = cand_with_context(10.0, CONTEXT_RANK_USER);
        assert_eq!(
            CandidateSortKey::without_context(&low_hit, raw_len, 1),
            key(&low, raw_len, 1)
        );
    }

    fn cand_with_cost(score: f32, walker_cost: u16) -> CandidateRankFacts {
        CandidateRankFacts {
            walker_cost,
            ..cand(0, 3, score, score as u32, 0)
        }
    }

    // E1 P5a / P5b: the corpus dimension beats the dictionary score and sits
    // below user weight and context.
    #[test]
    fn walker_cost_beats_score_below_user_weight_and_context() {
        let raw_len: u32 = 3;
        let frequent = cand_with_cost(1000.0, 9_000);
        let corpus_common = cand_with_cost(10.0, 5_000);
        assert!(key(&corpus_common, raw_len, 1) < key(&frequent, raw_len, 0));
        let selected = CandidateRankFacts {
            user_weight: 0.1,
            ..cand_with_cost(1.0, 13_000)
        };
        assert!(key(&selected, raw_len, 2) < key(&corpus_common, raw_len, 1));
        let context_hit = CandidateRankFacts {
            context_rank: CONTEXT_RANK_BUNDLED,
            ..cand_with_cost(1.0, 13_000)
        };
        assert!(key(&context_hit, raw_len, 3) < key(&corpus_common, raw_len, 1));
        // Equal cost → the score decides.
        let corpus_equal = cand_with_cost(10.0, 9_000);
        assert!(key(&frequent, raw_len, 0) < key(&corpus_equal, raw_len, 1));
    }

    // E1 P5b: an unpriced candidate (custom, learned) follows every priced
    // one in its tier — custom's source rank 0 sits below the cost — unless
    // the user selected it or the context names it.
    #[test]
    fn unpriced_candidate_follows_priced_unless_selected_or_in_context() {
        let raw_len: u32 = 3;
        let unseen = cand_with_cost(1.0, UNSEEN_WALKER_COST);
        let custom = CandidateRankFacts {
            is_custom: true,
            ..cand_with_cost(0.0, WALKER_COST_UNPRICED)
        };
        assert!(key(&unseen, raw_len, 1) < key(&custom, raw_len, 0));
        let selected_custom = CandidateRankFacts {
            user_weight: 0.1,
            ..custom
        };
        assert!(key(&selected_custom, raw_len, 0) < key(&unseen, raw_len, 1));
        let context_custom = CandidateRankFacts {
            context_rank: CONTEXT_RANK_USER,
            ..custom
        };
        assert!(key(&context_custom, raw_len, 0) < key(&unseen, raw_len, 1));
    }

    #[test]
    fn stable_idx_breaks_ties_when_all_else_equal() {
        // Identical facts, only the synthetic insertion index varies —
        // earlier index must sort first.
        let raw_len: u32 = 3;
        let a = cand(0, 3, 100.0, 100, 0);
        let b = cand(0, 3, 100.0, 100, 0);

        assert!(key(&a, raw_len, 0) < key(&b, raw_len, 1));
    }

    #[test]
    fn nan_score_is_coerced_to_minimum_not_panic() {
        // Phase 5 NaN-defense invariant preserved: a NaN score loses
        // every comparison instead of poisoning the sort.
        let raw_len: u32 = 3;
        let nan = cand(0, 3, f32::NAN, 100, 0);
        let normal = cand(0, 3, 0.001, 100, 0);

        // NaN coerced to f64::MIN → with Reverse<>, NaN ends up LAST
        // (largest sort key in ascending order).
        assert!(key(&normal, raw_len, 0) < key(&nan, raw_len, 1));
    }

    #[test]
    fn sort_key_reads_user_weight_from_facts() {
        // `CandidateSortKey::new` reads `facts.user_weight` verbatim — no
        // sentinel, no recomputation.
        let raw_len: u32 = 3;
        let never = key(&cand(0, 3, 1.0, 1, 0), raw_len, 0);
        assert_eq!(never.neg_user_weight, Reverse(NonNanF64::new(0.0)));
        let selected = key(&cand_with_user_weight(0, 3, 1.0, 1, 0, 0.1), raw_len, 1);
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
            key(&selected_rare, raw_len, 1) < key(&never_common, raw_len, 0),
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
        assert!(key(&heavy_rare, raw_len, 1) < key(&light_common, raw_len, 0));
        let tie_high = cand_with_user_weight(0, 3, 100.0, 100, 0, 0.5);
        let tie_low = cand_with_user_weight(0, 3, 1.0, 1, 0, 0.5);
        assert!(key(&tie_high, raw_len, 1) < key(&tie_low, raw_len, 0));
    }

    #[test]
    fn empty_span_yields_zero_coverage_without_panic() {
        // Defensive: a degenerate `(2, 2)` span (consumed_span_end ==
        // start) must produce a key with `neg_coverage = Reverse(0)`,
        // not panic in `saturating_sub`.
        let raw_len: u32 = 4;
        let key = key(&cand(2, 2, 0.0, 0, 0), raw_len, 0);
        assert_eq!(key.neg_coverage, Reverse(0));
        // Tier 1 because end (2) != raw_len (4).
        assert_eq!(key.tier, 1);
    }

    // ----- v3.5.8 Phase 9 Item 10 — `coverage_kind` dim -----

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
        let mut partial_strong = cand_partial(0, 3, f32::MAX, u32::MAX, KAUTIAN_BIT);
        partial_strong.user_weight = 4.0;
        assert!(
            key(&full_weak, raw_len, 0) < key(&partial_strong, raw_len, 1),
            "Item 10: full-syllable must precede partial-prefix regardless of other dims"
        );
    }

    #[test]
    fn within_partial_prefix_inner_dims_still_apply() {
        // Inside the `coverage_kind = 1` bucket the remaining dims still
        // drive ordering — verify with two partials where only `-score`
        // differs.
        let raw_len: u32 = 4;
        let high = cand_partial(0, 4, 100.0, 100, 0);
        let low = cand_partial(0, 4, 1.0, 1, 0);
        assert!(
            key(&high, raw_len, 0) < key(&low, raw_len, 1),
            "inside coverage_kind=1, higher score still wins"
        );
    }
}
