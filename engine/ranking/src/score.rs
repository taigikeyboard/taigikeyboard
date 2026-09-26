//! Continuous-input ranking primitives — source rank, user-frequency
//! boost + decay, frequency map, and the dictionary-derived score.
//!
//! Consumed by `lexicon::continuous` and `composing` (the Continuous
//! `FetchAtPos` path, the only production ranking path).

// ---------------------------------------------------------------------------
// v3.5.8 Phase 9.1 — Continuous-input source rank table.
//
// Lower rank = higher priority in the lexicographic continuous sort_key
// (per docs/releases/v3.5.8/plan.md § Phase 9 sort_key formula).
//
// Custom-dictionary entries (PR-9.6) take rank 0 via the `is_custom`
// flag; bitmask-derived ranks start at 1 (kautian > taigitv > stti >
// kungge). Drift between this table and
// `dictionary/common/source_bits.py` bit positions is a
// cross-platform invariant violation.
// ---------------------------------------------------------------------------

const CONTINUOUS_SOURCE_BITS: &[(u16, u8)] = &[
    (1 << 0, 1), // kautian
    (1 << 1, 2), // taigitv
    (1 << 7, 3), // stti
    (1 << 6, 4), // kungge
];

/// Source rank returned when `bitmask` has no known source bit set.
/// Higher than any explicit-source rank so unknown-source entries
/// sort last on the source dimension.
pub const CONTINUOUS_DEFAULT_SOURCE_RANK: u8 = 5;

/// Per-selection boost increment for the Continuous-input
/// `user_freq_boost`. Mirrors the additive `0.1` previously hard-coded
/// in [`calculate_continuous_score`]; pinning it as a public constant
/// is the cross-platform invariant axis for PR-9.3a + PR-9.3b/c
/// (`docs/releases/v3.5.8/plan.md` § Phase 9 跨平台 invariant 常數). Platforms MUST NOT
/// redefine — single source of truth per
/// `.claude/rules/cross-platform-alignment.md` §3a.
pub const BOOST_ALPHA: f32 = 0.1;

/// Saturation ceiling for the Continuous-input `user_freq_boost`.
/// Counts above `(MAX_BOOST − 1) / BOOST_ALPHA = 40` produce the same
/// boost (`5.0`); guards against a single hot entry dominating the
/// candidate list after dozens of selections (stale-dominance defense
/// from `docs/engine/continuous-input-ranking.md` §3.2 Gap B). Same
/// cross-platform invariant policy as [`BOOST_ALPHA`].
pub const MAX_BOOST: f32 = 5.0;

/// First-match-wins source rank for the Continuous-input sort_key.
/// Returns `0` when `is_custom`, else looks up the first matching
/// bit in `CONTINUOUS_SOURCE_BITS`, else
/// [`CONTINUOUS_DEFAULT_SOURCE_RANK`].
///
/// Cross-platform invariant: this fn is the single source of truth
/// for Continuous-input source ordering. Platform-side ranking code
/// MUST NOT redefine the table; per
/// `.claude/rules/cross-platform-alignment.md` §3a.
pub fn source_tier_rank(bitmask: u16, is_custom: bool) -> u8 {
    if is_custom {
        return 0;
    }
    for (bit, rank) in CONTINUOUS_SOURCE_BITS {
        if bitmask & bit != 0 {
            return *rank;
        }
    }
    CONTINUOUS_DEFAULT_SOURCE_RANK
}

/// Per-candidate user-frequency snapshot. Caller-supplied so engine stays
/// stateless. `last_used_ms == 0` means "never used".
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FrequencyData {
    /// Cumulative selection count. [`user_freq_boost`] saturates it via
    /// [`MAX_BOOST`].
    pub count: i32,
    /// Last selection in epoch-ms. `0` means never used; the recency
    /// helpers treat this and any non-positive value as "never".
    pub last_used_ms: i64,
}

impl FrequencyData {
    /// This entry's decayed user-selection weight at `now_ms` —
    /// [`decayed_user_weight_delta`] over the saturated count. The one
    /// place the `i32` count is widened back to `u32` (negative → 0).
    pub fn user_weight(&self, now_ms: i64) -> f64 {
        let count = u32::try_from(self.count).unwrap_or(0);
        decayed_user_weight_delta(count, now_ms, self.last_used_ms)
    }
}

/// Per-candidate user-frequency map keyed by the
/// `(display_text, canonical_tl)` PAIR identity (Core Principle #7;
/// v3.6.1 R5). `display_text` = `hanji` if non-empty else `roman` (same
/// key the platform writes to `user_frequency.db` on commit and the
/// Continuous-input [`RawCandidate::display_text`] carries);
/// `canonical_tl` = the candidate's canonical-TL reading
/// ([`RawCandidate::canonical_tl`], snapshotted before the POJ-render
/// pass). One `display_text` (e.g. 重) holds one bucket PER reading so
/// Polyphonic characters (重/tîng vs 重/tāng) keep separate counts.
///
/// Internally a nested `display_text → (canonical_tl → FrequencyData)`
/// map: the outer level lets [`get`](Self::get) borrow `&str` and the
/// inner level expresses the tolerant fallback directly. The engine builds
/// this once per fetch from its `user_frequency.db` rows (`dispatch` crate,
/// `user_data::frequency_map`) and reuses it across the whole batch.
///
/// **Legacy `canonical_tl == ""` bucket**: pre-R5 rows / old-backup
/// imports that could not be re-keyed carry an empty `canonical_tl`.
/// [`get`](Self::get) consults that bucket as a tolerant fallback for ANY
/// reading of the `display_text` whose exact `(display, tl)` entry is
/// absent — both 重/tîng and 重/tāng inherit the old merged 重 count until
/// each is re-learned, at which point the exact bucket shadows the legacy
/// one. Self-healing; the legacy row is never deleted.
///
/// **Duplicate-key policy**: last-write-wins within one
/// `(display, tl)` bucket. The store answers at most one row per pair
/// (UNIQUE(word, tl)); duplicates coalesce all the same.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FrequencyMap {
    by_display: std::collections::HashMap<String, std::collections::HashMap<String, FrequencyData>>,
}

impl FrequencyMap {
    /// Empty map — cold-start neutral (every [`get`](Self::get) returns
    /// [`FrequencyData::default`]).
    pub fn new() -> Self {
        Self::default()
    }

    /// Pre-size the outer (`display_text`) level. Inner maps grow lazily.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            by_display: std::collections::HashMap::with_capacity(capacity),
        }
    }

    /// Insert one `(display_text, canonical_tl)` bucket. Last-write-wins
    /// on an exact pair collision.
    pub fn insert(&mut self, display_text: String, canonical_tl: String, data: FrequencyData) {
        self.by_display
            .entry(display_text)
            .or_default()
            .insert(canonical_tl, data);
    }

    /// Tolerant pair lookup: exact `(display, tl)` first, then the legacy
    /// `(display, "")` fallback bucket. NEVER sums — the exact bucket
    /// shadows the legacy one. Returns [`FrequencyData::default`] (neutral
    /// cold-start: `user_freq_boost(0) = 1.0`, `decayed_user_weight_delta(0, _, 0) = 0.0`)
    /// when neither is present, so absent entries reproduce the pre-R5
    /// never-used behaviour. A `canonical_tl == ""` query consults the
    /// legacy bucket once (no redundant second probe).
    pub fn get(&self, display_text: &str, canonical_tl: &str) -> FrequencyData {
        let Some(inner) = self.by_display.get(display_text) else {
            return FrequencyData::default();
        };
        let exact = inner.get(canonical_tl);
        // Legacy fallback only when the query carries a reading — an empty
        // `canonical_tl` already probed the legacy bucket above.
        let legacy = if canonical_tl.is_empty() {
            None
        } else {
            inner.get("")
        };
        exact.or(legacy).copied().unwrap_or_default()
    }
}

/// `(display_text, canonical_tl, data)` rows, [`insert`](FrequencyMap::insert)ed
/// in order — a later row for the same pair wins.
impl FromIterator<(String, String, FrequencyData)> for FrequencyMap {
    fn from_iter<I: IntoIterator<Item = (String, String, FrequencyData)>>(rows: I) -> Self {
        let rows = rows.into_iter();
        let mut map = Self::with_capacity(rows.size_hint().0);
        for (display_text, canonical_tl, data) in rows {
            map.insert(display_text, canonical_tl, data);
        }
        map
    }
}

/// v3.5.8 Phase 9.3a — Continuous-input `user_freq_boost(count)`:
///
/// `boost = min(1.0 + count × BOOST_ALPHA, MAX_BOOST)`
///
/// Caller-side helper paired with [`calculate_continuous_score`]. The
/// saturation guards against a single hot entry dominating the
/// candidate list after dozens of selections (stale-dominance defense
/// from `docs/engine/continuous-input-ranking.md` §3.2 Gap B). Pure
/// fn — no state, no clock; the saturation constants live as public
/// cross-platform invariants ([`BOOST_ALPHA`] / [`MAX_BOOST`]).
///
/// `count = 0` (entry absent or never selected) → boost = `1.0` (no
/// amplification). Saturates at `count >= (MAX_BOOST − 1) / BOOST_ALPHA = 40`.
pub fn user_freq_boost(count: u32) -> f32 {
    let raw = 1.0 + count as f32 * BOOST_ALPHA;
    raw.min(MAX_BOOST)
}

/// v3.5.8 S3 — exponential **time constant** (τ) for the
/// Continuous-input whole-sentence walker's user-frequency edge
/// weight. librime `formula_d`
/// (`references/librime/src/rime/algo/dynamics.h` —
/// `d + da·exp((ta − t) / 200)`) decays over an integer per-commit
/// *tick*; we have no tick model, only the platform wall clock, so the
/// walker decays over `now_ms − last_used_ms` epoch-ms instead.
///
/// This is the time constant of `exp(−age / τ)`, NOT the 50% point:
/// the weight decays to `1/e ≈ 0.37` after τ and to `0.5` after
/// `τ · ln 2 ≈ 20.8 days` for the τ = 30-day default. Named for the
/// mathematical role rather than "half-life" to keep the formula
/// honest (`~/.claude/rules/ai-friendly-code.md` naming). 30 days is the right
/// initial shape for an IME: strong over days, meaningful over weeks,
/// noticeably stale over months. **Dogfood-tunable in 14..=90 days**
/// (Codex pre-impl S3 Q4a, 2026-05-16) — kept a named constant, not a
/// magic literal, so retuning is a one-line change.
pub const USER_WEIGHT_DECAY_TAU_MS: i64 = 30 * 24 * 60 * 60 * 1000;

/// v3.5.8 S3 — time-decayed user-frequency boost **delta** for one
/// whole-sentence-walker lattice edge (librime `formula_d` adapted to
/// wall-clock; closes Continuous-input Gap B → goal G2,
/// `docs/engine/continuous-input-ranking.md` §3.2 / §7). Returns the
/// amount **above** the neutral `1.0` that the edge's chosen candidate
/// has earned from past user selections, exponentially decayed by
/// recency:
///
/// ```text
/// decay = exp(−age_ms / USER_WEIGHT_DECAY_TAU_MS)
/// delta = (user_freq_boost(count) − 1.0) × decay
/// ```
///
/// The `(user_freq_boost(count) − 1.0)` base is the **already
/// saturated** delta — the [`MAX_BOOST`] cap is applied **before** the
/// time decay. Decaying the raw `count` first and capping afterwards
/// would keep a `count = 1000` entry pinned at `MAX_BOOST` for months
/// (it would have to decay below an *effective* count of 40 before the
/// cap released), which is exactly the stale single-entry dominance
/// this slice must avoid (Codex pre-impl S3 Q4a/Q4c BLOCK condition,
/// 2026-05-16).
///
/// Caller-injected `now_ms` / `last_used_ms` keep this pure +
/// stateless (same cross-platform-invariant contract as
/// [`user_freq_boost`] — engine is the single source of truth,
/// platforms MUST NOT redefine). Returns `0.0` (→ neutral weight `1.0`
/// at the call site) for **three bad-clock classes**: `now_ms <= 0`
/// (no wall clock injected), `last_used_ms <= 0` (never selected), and
/// `now_ms < last_used_ms` (clock skew). The walker turns this
/// per-edge delta into a syllable-aware log-space cost discount
/// (single-syllable edges are damped so a hot single character cannot
/// ride the discount to sweep the whole sentence — see
/// `composing::lattice::cost::edge_cost`).
///
/// Also the Continuous candidate's `RawCandidate.user_weight` — the
/// leading user-preference dimension of the lexicon `SortKey` and of
/// the walker's per-edge homophone pick (2026-09-14): any selected
/// word (`> 0.0`) precedes every never-selected one.
pub fn decayed_user_weight_delta(count: u32, now_ms: i64, last_used_ms: i64) -> f64 {
    // Single source of truth for "is this user-frequency timestamp
    // usable".
    if now_ms <= 0 || last_used_ms <= 0 || now_ms < last_used_ms {
        return 0.0;
    }
    let age_ms = now_ms - last_used_ms; // >= 0 by the guard above
    let decay = (-(age_ms as f64) / USER_WEIGHT_DECAY_TAU_MS as f64).exp();
    // Cap BEFORE decay: `user_freq_boost` already saturates at
    // `MAX_BOOST` (count >= 40), so `base_delta` is in `0.0..=4.0`.
    let base_delta = f64::from(user_freq_boost(count)) - 1.0;
    base_delta * decay
}

/// v3.5.8 Continuous Input Phase 5 score formula:
///
/// `score = freq × (1.0 + 0.1 × max(0, syllable_count − 1))`
///
/// Pure-multiplicative `f32`, no bigram, no recency / exact / closeness /
/// tier components. The Continuous slice ranks span-local candidates emitted by
/// `lexicon::continuous::fetch_candidates_for_keys_with_barriers` (the
/// production span-local entry; the test-only `fetch_candidates_for_endings`
/// wrapper lives in `engine/lexicon/tests/common/mod.rs`), where the
/// per-syllable bias
/// rewards multi-syllable words like `珠仔(syll=2)` over `紙/珠(syll=1)`
/// when the user's input spans a multi-syllable reach.
///
/// Purely dictionary-derived. User preference is NOT folded in here
/// any more (2026-09-14): it is the separate, leading
/// `RawCandidate.user_weight` dimension ([`FrequencyData::user_weight`]),
/// so the same signal is not represented twice with different clock
/// guards. The lexicon sort comparator defends against `NaN` by
/// coercing it low.
///
/// Cited mainstream IME parallel: khiin-rs `khiin/src/data/segmenter.rs`
/// uses `cost = ln(1/p) / word_len_bias × syllable_bias`. We pick a
/// simpler multiplicative form per `docs/releases/v3.5.8/plan.md` § Sort_key 公式 (PR-9.1 source-of-truth).
pub fn calculate_continuous_score(freq: u32, syllable_count: u8) -> f32 {
    let syll_bias = 1.0 + BOOST_ALPHA * f32::from(syllable_count.saturating_sub(1));
    freq as f32 * syll_bias
}

#[cfg(test)]
mod tests {
    use super::*;

    const KAUTIAN_BIT: u16 = 1 << 0;
    const TAIGITV_BIT: u16 = 1 << 1;
    const ITAIGI_BIT: u16 = 1 << 2;
    const KUNGGE_BIT: u16 = 1 << 6;
    const STTI_BIT: u16 = 1 << 7;

    fn freq(count: i32, last_used_ms: i64) -> FrequencyData {
        FrequencyData {
            count,
            last_used_ms,
        }
    }

    // -----------------------------------------------------------------------
    // v3.5.8 Phase 5 — calculate_continuous_score
    // -----------------------------------------------------------------------

    #[test]
    fn continuous_score_single_syllable_baseline() {
        // syll=1 → bias = 1.0; user_freq_boost = 1.0 → score == freq.
        assert_eq!(calculate_continuous_score(100, 1), 100.0);
        assert_eq!(calculate_continuous_score(0, 1), 0.0);
    }

    #[test]
    fn continuous_score_syllable_bias_increments_by_ten_percent() {
        // freq = 100, boost = 1.0:
        //   syll=1 → 100.0
        //   syll=2 → 110.0
        //   syll=3 → 120.0
        //   syll=4 → 130.0
        assert_eq!(calculate_continuous_score(100, 1), 100.0);
        assert_eq!(calculate_continuous_score(100, 2), 110.0);
        assert!((calculate_continuous_score(100, 3) - 120.0).abs() < 1e-4);
        assert_eq!(calculate_continuous_score(100, 4), 130.0);
    }

    #[test]
    fn continuous_score_syll_zero_does_not_underflow() {
        // syllable_count = 0 must clamp to bias = 1.0 (saturating_sub(1)).
        // Defensive: builder caps at 1..=4 (`MAX_SYLLABLES`), but the FFI
        // contract is u8 so a zero could leak in; should not panic / wrap.
        assert_eq!(calculate_continuous_score(50, 0), 50.0);
    }

    #[test]
    fn continuous_score_multi_syllable_outranks_single_when_freq_equal() {
        // The whole point of the syll bias: 珠仔(syll=2) outranks 紙(syll=1)
        // at equal dictionary frequency, so multi-syll candidates surface.
        let single = calculate_continuous_score(100, 1);
        let pair = calculate_continuous_score(100, 2);
        let quad = calculate_continuous_score(100, 4);
        assert!(pair > single);
        assert!(quad > pair);
    }

    // -----------------------------------------------------------------------
    // v3.5.8 Phase 9.1 — source_tier_rank
    // -----------------------------------------------------------------------

    #[test]
    fn source_tier_rank_is_custom_short_circuits() {
        // `is_custom=true` overrides any bitmask content with rank 0.
        assert_eq!(source_tier_rank(0, true), 0);
        assert_eq!(source_tier_rank(KAUTIAN_BIT, true), 0);
        assert_eq!(source_tier_rank(u16::MAX, true), 0);
    }

    #[test]
    fn source_tier_rank_first_match_wins_in_table_order() {
        // Table order: kautian(1) → taigitv(2) → stti(3) → kungge(4).
        // Overlapping bits should resolve to the lowest rank present.
        assert_eq!(source_tier_rank(KAUTIAN_BIT, false), 1);
        assert_eq!(source_tier_rank(TAIGITV_BIT, false), 2);
        assert_eq!(source_tier_rank(STTI_BIT, false), 3);
        assert_eq!(source_tier_rank(KUNGGE_BIT, false), 4);
        // Kautian + kungge → kautian (entry-order first match).
        assert_eq!(source_tier_rank(KAUTIAN_BIT | KUNGGE_BIT, false), 1);
    }

    // -----------------------------------------------------------------------
    // v3.5.8 Phase 9.3a — user_freq_boost
    // -----------------------------------------------------------------------

    #[test]
    fn user_freq_boost_count_zero_returns_one() {
        // No selections → no amplification.
        assert!((user_freq_boost(0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn user_freq_boost_increments_by_alpha_per_count() {
        // Linear region: boost = 1.0 + count × 0.1.
        assert!((user_freq_boost(1) - 1.1).abs() < 1e-6);
        assert!((user_freq_boost(10) - 2.0).abs() < 1e-6);
        assert!((user_freq_boost(20) - 3.0).abs() < 1e-6);
        // Boundary just below saturation.
        assert!((user_freq_boost(39) - 4.9).abs() < 1e-5);
    }

    #[test]
    fn user_freq_boost_saturates_at_max_boost() {
        // Saturation: 40 selections → 5.0, anything above stays at 5.0.
        assert!((user_freq_boost(40) - MAX_BOOST).abs() < 1e-6);
        assert!((user_freq_boost(50) - MAX_BOOST).abs() < 1e-6);
        assert!((user_freq_boost(100) - MAX_BOOST).abs() < 1e-6);
        assert!((user_freq_boost(1_000_000) - MAX_BOOST).abs() < 1e-6);
        // Stale-dominance defense: even u32::MAX cannot exceed MAX_BOOST.
        assert!((user_freq_boost(u32::MAX) - MAX_BOOST).abs() < 1e-6);
    }

    #[test]
    fn frequency_map_insert_is_last_write_wins() {
        // Duplicate `(display_text, canonical_tl)` rows coalesce silently:
        // the later one wins.
        let mut map = FrequencyMap::new();
        map.insert("台".to_owned(), "tâi".to_owned(), freq(1, 100));
        map.insert("台".to_owned(), "tâi".to_owned(), freq(7, 700));
        let data = map.get("台", "tâi");
        assert_eq!(data.count, 7);
        assert_eq!(data.last_used_ms, 700);
    }

    #[test]
    fn frequency_map_pair_key_separates_homograph_readings() {
        // R5 / Core Principle #7: polyphonic character — same hanji 重, two readings
        // tîng (重複) vs tāng (重量) — keep SEPARATE buckets. The
        // pre-R5 hanji-only key merged them.
        let mut map = FrequencyMap::new();
        map.insert("重".to_owned(), "tîng".to_owned(), freq(3, 100));
        map.insert("重".to_owned(), "tāng".to_owned(), freq(9, 900));
        assert_eq!(map.get("重", "tîng").count, 3);
        assert_eq!(map.get("重", "tāng").count, 9);
    }

    #[test]
    fn frequency_map_legacy_empty_tl_is_tolerant_fallback() {
        // A pre-R5 / old-backup row carries canonical_tl == "". Any
        // reading whose exact (display, tl) bucket is absent falls back
        // to it; once a reading is re-learned its exact bucket shadows
        // the legacy one (NEVER summed).
        let mut map = FrequencyMap::new();
        map.insert("重".to_owned(), String::new(), freq(5, 500)); // legacy merged 重
        map.insert("重".to_owned(), "tîng".to_owned(), freq(2, 200)); // re-learned tîng
                                                                      // Exact reading shadows legacy (no sum: 2, not 7).
        assert_eq!(map.get("重", "tîng").count, 2);
        // Not-yet-re-learned reading inherits the legacy bucket.
        assert_eq!(map.get("重", "tāng").count, 5);
        // A direct empty-tl query also hits the legacy bucket.
        assert_eq!(map.get("重", "").count, 5);
    }

    #[test]
    fn frequency_map_missing_pair_is_neutral_cold_start() {
        // Absent display OR absent reading with no legacy fallback →
        // default (count 0, last_used 0) so user_freq_boost(0)=1.0.
        let mut map = FrequencyMap::new();
        map.insert("我".to_owned(), "guá".to_owned(), freq(4, 400));
        assert_eq!(map.get("無", "bô"), FrequencyData::default());
        assert_eq!(map.get("我", "góa"), FrequencyData::default()); // no legacy bucket
        assert_eq!(map.get("我", "guá").count, 4); // exact reading still present
    }

    #[test]
    fn source_tier_rank_falls_back_to_default_when_no_known_bit() {
        // Bits the table does not enumerate (e.g., itaigi=2, dev=10,
        // khiin=9, variant=12) all fall through to the default rank.
        assert_eq!(
            source_tier_rank(ITAIGI_BIT, false),
            CONTINUOUS_DEFAULT_SOURCE_RANK
        );
        assert_eq!(source_tier_rank(0, false), CONTINUOUS_DEFAULT_SOURCE_RANK);
        assert_eq!(
            source_tier_rank(1 << 10, false),
            CONTINUOUS_DEFAULT_SOURCE_RANK
        );
    }

    // -----------------------------------------------------------------------
    // v3.5.8 S3 — decayed_user_weight_delta (Gap B → G2)
    // -----------------------------------------------------------------------

    #[test]
    fn decayed_delta_zero_when_never_selected_or_count_zero() {
        let now = 1_000_000_000_000_i64;
        // last_used_ms <= 0 → never selected → neutral (0.0 delta).
        assert_eq!(decayed_user_weight_delta(50, now, 0), 0.0);
        assert_eq!(decayed_user_weight_delta(50, now, -1), 0.0);
        // count 0 → user_freq_boost(0) = 1.0 → base_delta 0 → 0.0 even
        // when freshly used.
        assert_eq!(decayed_user_weight_delta(0, now, now), 0.0);
    }

    #[test]
    fn decayed_delta_zero_for_bad_clock_classes() {
        let last = 1_000_000_000_000_i64;
        // now_ms <= 0 (no wall clock injected).
        assert_eq!(decayed_user_weight_delta(40, 0, last), 0.0);
        assert_eq!(decayed_user_weight_delta(40, -5, last), 0.0);
        // Clock skew: now strictly before the recorded selection.
        assert_eq!(decayed_user_weight_delta(40, last - 1, last), 0.0);
    }

    #[test]
    fn decayed_delta_is_full_saturated_delta_at_zero_age() {
        // age 0 → decay = exp(0) = 1.0. count >= 40 saturates
        // user_freq_boost at MAX_BOOST (5.0) → base_delta = 4.0.
        let t = 1_000_000_000_000_i64;
        let d = decayed_user_weight_delta(40, t, t);
        assert!((d - 4.0).abs() < 1e-9, "{d}");
        // count 1000 (well past saturation) is the SAME 4.0 — the cap
        // is applied before decay, so a huge historic count is NOT
        // pinned high (the Q4a/Q4c stale-dominance BLOCK condition).
        let d_huge = decayed_user_weight_delta(1000, t, t);
        assert!((d_huge - 4.0).abs() < 1e-9, "{d_huge}");
        assert_eq!(d, d_huge);
    }

    #[test]
    fn decayed_delta_caps_before_decay_not_after() {
        // Codex post-impl P2: the zero-age test above cannot tell the
        // correct formula apart from the BLOCKED "decay raw count THEN
        // cap" alternative — both saturate to 4.0 at age 0. Pin the
        // distinction at NONZERO age with a saturated historic count.
        //
        // count = 1000 (far past the count=40 saturation point).
        //   CORRECT (cap before decay): base_delta = 4.0, then × e^-1
        //     at one τ → ≈ 4/e ≈ 1.4715.
        //   BLOCKED  (decay raw count, cap after): effective_count =
        //     1000 × e^-1 ≈ 368 → still saturates user_freq_boost → 5.0
        //     → delta ≈ 4.0 (stale single-entry dominance for months).
        // So the value at one τ MUST be ~4/e, never ~4.0.
        let last = 1_000_000_000_000_i64;
        let at_one_tau = decayed_user_weight_delta(1000, last + USER_WEIGHT_DECAY_TAU_MS, last);
        assert!(
            (at_one_tau - 4.0 / std::f64::consts::E).abs() < 1e-9,
            "saturated count must decay (cap-before-decay): got {at_one_tau}, \
             want ~{} (the blocked formula would give ~4.0)",
            4.0 / std::f64::consts::E
        );
        // And it is identical to count=40 at the same age — the cap
        // collapses both to the same pre-decay base_delta of 4.0.
        let count40_one_tau = decayed_user_weight_delta(40, last + USER_WEIGHT_DECAY_TAU_MS, last);
        assert_eq!(at_one_tau, count40_one_tau);
    }

    #[test]
    fn decayed_delta_decays_toward_zero_with_age() {
        let last = 1_000_000_000_000_i64;
        let fresh = decayed_user_weight_delta(40, last, last);
        let one_tau = decayed_user_weight_delta(40, last + USER_WEIGHT_DECAY_TAU_MS, last);
        let ten_tau = decayed_user_weight_delta(40, last + 10 * USER_WEIGHT_DECAY_TAU_MS, last);
        // Monotonically non-increasing with age, strictly decreasing here.
        assert!(fresh > one_tau, "fresh={fresh} one_tau={one_tau}");
        assert!(one_tau > ten_tau, "one_tau={one_tau} ten_tau={ten_tau}");
        // At one τ the delta is base_delta / e (≈ 4.0 × 0.3679).
        assert!(
            (one_tau - 4.0 / std::f64::consts::E).abs() < 1e-9,
            "{one_tau}"
        );
        // Far future → effectively neutral (never negative, never NaN).
        assert!(
            ten_tau >= 0.0 && ten_tau.is_finite() && ten_tau < 1e-3,
            "{ten_tau}"
        );
    }

    #[test]
    fn decayed_delta_half_point_is_tau_times_ln2() {
        // The constant is the τ of exp(−age/τ); the 50% point is
        // τ·ln2 (documented contract — guards the naming rationale).
        let last = 1_000_000_000_000_i64;
        let half_age = (USER_WEIGHT_DECAY_TAU_MS as f64 * std::f64::consts::LN_2) as i64;
        let d = decayed_user_weight_delta(40, last + half_age, last);
        assert!(
            (d - 2.0).abs() < 1e-3,
            "delta at τ·ln2 should be ~half of 4.0, got {d}"
        );
    }
}
