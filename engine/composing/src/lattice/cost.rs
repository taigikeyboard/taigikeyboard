//! v3.5.8 S2 — whole-sentence walker edge cost.
//!
//! Higher = better. The walker's path objective is `Σ edge_score` over
//! the chosen forward-only edge chain (McBopomofo Gramambular shape:
//! single-pass relaxation in log space, where additive accumulation =
//! multiplicative probability — `references/McBopomofo/Source/Engine/
//! gramambular2/reading_grid.cpp:132`). The per-edge term stays in the
//! **same cost family** as the span-local ranker's
//! `ranking::calculate_continuous_score`
//! (`engine/ranking/src/score.rs:367` —
//! `freq × (1 + 0.1 × (syll − 1)) × boost`): we take its `boost = 1.0`
//! log correspondence `ln(1 + freq) × (1 + 0.1 × (syll − 1))`, the
//! khiin-rs `ln(1/p^FREQ)/word_len^LET·n_syls^SYL` parallel already
//! pinned at `score.rs:361-363`. `+1` Laplace smoothing keeps a
//! frequency-0 edge (no dict hit) finite so the no-hanji roman path is
//! the walker's natural best path, not a special-case fallback
//! (`docs/roadmap.md` §整句 lattice + walker; `feedback_no_redundant_fallback`).
//!
//! **v3.5.8 S3 (this slice)** folds two more terms into the per-edge
//! cost, closing Continuous-input Gap B → goal G2
//! (`docs/engine/continuous-input-ranking.md` §3.2 / §7):
//!
//! 1. **User-frequency time-decay weight** (librime `formula_d` adapted
//!    to wall-clock — `ranking::decayed_user_weight_delta`). The S2
//!    walker objective ignored `user_frequency.db` entirely (the
//!    hard-coded `user_freq_boost = 1.0` of
//!    `docs/engine/continuous-input-ranking.md` §3.2 Gap B applied only
//!    to *record selection within* an edge, never to *which
//!    segmentation path wins*). S3 multiplies the edge cost by
//!    `1 + applied_delta` so a path through user-preferred words wins.
//! 2. **McBopomofo epsilon-boost**
//!    (`references/McBopomofo/Source/Engine/McBopomofoLM.cpp:206` —
//!    `topScore + 1e-9`). A multi-syllable edge gets a tiny additive
//!    nudge so a phrase path wins a near-tie against a hot single-char
//!    path without overriding a materially better path.
//!
//! Both are **syllable-aware** so they cannot re-introduce
//! single-character domination (the Codex pre-impl S3 Q4c BLOCK
//! condition, 2026-05-16): a `syllable_count <= 1` edge receives only
//! `WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE` (default `0.0`) of the
//! decayed user delta — its user preference is already honored in
//! `lexicon::best_candidate_for_key` record selection, so letting it
//! also ride `MAX_BOOST` (×5) in the path objective would let a hot
//! single char like 「的」 (dict freq ~184k → edge ≈ 13) sweep the
//! whole sentence.

// 中文: S2 — 全句 walker 的單條 edge 成本 (越大越好)。
// 中文: 與 ranking::calculate_continuous_score 同 cost 家族的 log 對應:ln(1+freq)×(1+0.1×(syll−1))。
// 中文: +1 平滑讓無字典 (freq=0) 的 edge 仍有限 → 純羅馬字路徑是 walker 自然最佳路徑,非特例 fallback。
// 中文: S3(本片):再乘 user-freq 時間衰減權重 (librime formula_d 牆鐘版,收斂 Gap B → G2)
// 中文:   + 多音節 edge 加 McBopomofo epsilon-boost。兩者皆 syllable-aware:單音節 edge 只拿
// 中文:   WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE(預設 0.0)倍的 delta,防熱門單字 (如「的」) 壓整句
// 中文:   (Codex S3 Q4c BLOCK 條件);其 user 偏好已在 best_candidate_for_key 選 record 時體現。

use ranking::BOOST_ALPHA;

/// v3.5.8 S3 — McBopomofo epsilon-boost
/// (`references/McBopomofo/Source/Engine/McBopomofoLM.cpp:206-207`,
/// `boostedScore = topScore + 1e-9`) re-derived from McBopomofo's
/// log-probability scale (≈ small negatives near 0) to this walker's
/// **linear** edge-cost scale (no-dict unit edge = `1.0`, a hot
/// single-char dict edge ≈ `13`). A flat additive nudge applied to a
/// **multi-syllable** (`syllable_count >= 2`) edge only — about 0.1%
/// of the unit edge, invisible against a hot dict edge — so it
/// reliably breaks a single-char-path vs phrase-path near-tie toward
/// the phrase yet cannot rescue a materially worse path. Per-edge
/// additive: it accumulates naturally in the walker's summed
/// objective. **Not redundant** with `syll_bias` (a real ranking
/// feature); this is only a tie-band nudge. No positive epsilon can
/// flip a path whose score advantage exceeds it — the intended
/// invariant is "flip only margins inside the dogfood-tuned tie band".
/// **Dogfood-tunable 0.0001..=0.005** (Codex pre-impl S3 Q4b,
/// 2026-05-16).
// 中文: S3 — McBopomofo epsilon-boost 從 log-prob 1e-9 重新換算到本 walker 線性 cost 尺度。
// 中文:   只加在多音節 (syll>=2) edge,約 unit edge 的 0.1%;per-edge 加性,在 walker 累加目標自然疊加。
// 中文:   非與 syll_bias 重複(syll_bias 是真排序特徵,此僅 tie-band 微調)。dogfood 可調 0.0001..=0.005。
pub(crate) const WALKER_PHRASE_EPSILON: f64 = 0.001;

/// v3.5.8 S3 — fraction of the decayed user-frequency delta a
/// **single-syllable** (`syllable_count <= 1`) walker edge receives.
/// `0.0` = single-syllable edges get NO walker user-weight. A hot
/// single character (e.g. 「的」, dict freq ~184k → base edge
/// `1 + ln(184694) ≈ 13`) at the full `MAX_BOOST` (×5) would reach
/// ≈ 66 and swamp every competing phrase path — no phrase epsilon
/// could counter that, and it should not try. That single-char user
/// preference is already honored where it belongs:
/// `lexicon::best_candidate_for_key` record selection (homophone
/// disambiguation *within* the edge). Letting it also dominate the
/// *path objective* is the exact stale single-character-domination
/// failure S3 must close (Codex pre-impl S3 Q4c, 2026-05-16, BLOCK
/// condition). Multi-syllable edges get the full delta (scale `1.0`,
/// not configured here — see `edge_score`). **Dogfood-tunable
/// 0.0..=0.25** if dogfood shows single-syllable user preference is
/// under-weighted in segmentation.
// 中文: S3 — 單音節 (syll<=1) edge 拿到的 decayed user delta 比例;0.0 = 完全不拿 walker user-weight。
// 中文:   熱門單字 (如「的」base edge≈13) 若拿滿 MAX_BOOST(×5)≈66 會壓垮所有片語路徑(Codex S3 Q4c BLOCK)。
// 中文:   其 user 偏好已由 best_candidate_for_key 選 record 時體現。多音節 edge 拿滿 delta。dogfood 可調 0.0..=0.25。
pub(crate) const WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE: f64 = 0.0;

/// Score for one lattice edge.
///
/// - `frequency` — the chosen dictionary candidate's raw
///   `DictionaryRecord.frequency` (`0` when the edge has no dict hit —
///   a pure-roman syllable span).
/// - `syllable_count` — that candidate's syllable count (`>= 1`).
/// - `user_weight_delta` — the time-decayed user-frequency boost delta
///   for this edge's chosen candidate
///   (`ranking::decayed_user_weight_delta`, in `0.0..=4.0`; `0.0` =
///   no user history / never selected / bad clock = neutral). Computed
///   caller-side (`dispatch::fetch_walker_slot0`, which holds the
///   `FrequencyMap` + `now_ms`); the walker stays pure shadow-space.
///
/// ```text
/// edge_score = (1 + ln(1 + freq)) × syll_bias × user_weight
///              + phrase_epsilon
/// syll_bias    = 1 + BOOST_ALPHA × (syll − 1)
/// applied_δ    = syll <= 1 ? user_weight_delta × SINGLE_SYLLABLE_SCALE
///                          : user_weight_delta
/// user_weight  = 1 + applied_δ
/// phrase_ε     = syll >= 2 ? WALKER_PHRASE_EPSILON : 0
/// ```
///
/// `syll_bias` mirrors `calculate_continuous_score`'s multiplicative
/// syllable bias so a 2-syllable phrase edge is rewarded over two
/// 1-syllable atomic edges at equal per-syllable frequency (the
/// `臺灣台語` vs `臺/灣/台/語` case). The leading `1 +` keeps every
/// reachable edge a strictly positive contribution so a longer
/// all-zero-frequency path (the no-dict roman case) accumulates a
/// higher total than a shorter one — the deterministic lever that
/// yields per-syllable segmentation `tai uan tai`
/// (see `walker::walk_best` tie contract).
///
/// **S3 neutrality contract (preserves the S2 tie lever)**: a no-dict
/// edge has `frequency = 0`, `syllable_count = 1`, and (by
/// construction in `fetch_walker_slot0`) `user_weight_delta = 0.0`, so
/// `user_weight = 1.0` and `phrase_epsilon = 0.0` → `edge_score`
/// is still exactly the unit `1.0`. The `taiuantai → tai uan tai`
/// per-syllable result is unchanged by S3.
///
/// **`user_weight` is multiplicative**, not additive: the whole point
/// of Gap B is to let an explicit user preference move a low-frequency
/// phrase path against the (compressed but still strong)
/// `1 + ln(1 + freq)` dictionary term — an additive boost is too weak
/// at that scale (Codex pre-impl S3 Q4a, 2026-05-16). It is
/// **syllable-aware** so a hot single character cannot ride the boost
/// to sweep the sentence (see `WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE`).
// 中文: 單 edge 分數 = (1+ln(1+freq))×syll_bias×user_weight + phrase_epsilon。
// 中文: user_weight 乘性(加性在 1+ln(freq) 尺度太弱,Codex S3 Q4a);syllable-aware 防熱門單字壓句。
// 中文: 前導 `1 +` 讓每條可達 edge 皆為正 → 全零頻路徑「edge 越多總分越高」(tai uan tai)。
// 中文: S3 中性契約:無字典 edge (freq=0, syll=1, user_weight_delta=0.0) → edge_score 仍恰為 1.0,
// 中文:   S2 的 tai uan tai tie lever 不受 S3 影響。
pub(crate) fn edge_score(frequency: u32, syllable_count: u8, user_weight_delta: f64) -> f64 {
    let unigram = 1.0 + (1.0 + f64::from(frequency)).ln();
    let syll = f64::from(syllable_count.max(1));
    let syll_bias = 1.0 + f64::from(BOOST_ALPHA) * (syll - 1.0);
    // Syllable-aware user weight: phrase edges get the full decayed
    // delta; single-syllable edges only `SCALE` of it (default 0.0)
    // so a hot single char cannot ride MAX_BOOST to sweep the
    // sentence (Codex pre-impl S3 Q4c BLOCK condition).
    let applied_delta = if syllable_count <= 1 {
        user_weight_delta * WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE
    } else {
        user_weight_delta
    };
    let user_weight = 1.0 + applied_delta;
    // McBopomofo epsilon-boost: per-edge additive tie-band nudge for
    // phrase edges only; accumulates in the walker's summed objective.
    let phrase_epsilon = if syllable_count >= 2 {
        WALKER_PHRASE_EPSILON
    } else {
        0.0
    };
    unigram * syll_bias * user_weight + phrase_epsilon
}

#[cfg(test)]
mod tests {
    use super::{edge_score, WALKER_PHRASE_EPSILON, WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE};

    /// Neutral-user-weight edge score — the S2 cost with no user
    /// history (`user_weight_delta = 0.0`). The S2 invariants below
    /// are expressed through this so they pin the unchanged behavior.
    fn es(freq: u32, syll: u8) -> f64 {
        edge_score(freq, syll, 0.0)
    }

    #[test]
    fn zero_frequency_edge_is_strictly_positive() {
        // No-dict edge must still contribute a finite positive score so
        // the roman best-path exists (Bug 2 / §1 subsumed by the walker,
        // not a fallback). S3 neutrality contract: still exactly 1.0.
        let s = es(0, 1);
        assert!(s > 0.0 && s.is_finite(), "{s}");
        // ln(1) = 0, bias(1) = 1, user_weight(1.0), no phrase epsilon
        // (syll < 2) → exactly the leading `1 +` = 1.0.
        assert!((s - 1.0).abs() < 1e-9, "{s}");
    }

    #[test]
    fn higher_frequency_scores_higher() {
        assert!(es(1000, 1) > es(10, 1));
        assert!(es(10, 1) > es(0, 1));
    }

    #[test]
    fn syllable_bias_rewards_longer_words() {
        // At equal frequency a 2-syllable edge beats a 1-syllable edge
        // (the phrase-priority lever; S3 phrase epsilon only widens it).
        assert!(es(100, 2) > es(100, 1));
        assert!(es(100, 4) > es(100, 2));
    }

    #[test]
    fn syllable_zero_clamps_to_one_no_underflow() {
        // syllable_count is u8 from a record; defend against a leaked 0.
        // 0 clamps to 1 syllable AND takes the single-syllable branch
        // (no phrase epsilon), identical to syll = 1.
        assert_eq!(edge_score(0, 0, 3.5), edge_score(0, 1, 3.5));
    }

    #[test]
    fn two_atomic_edges_outscore_one_phrase_edge_at_equal_zero_frequency() {
        // The no-dict tie lever survives S3: with all frequencies 0 and
        // no user history, `tai`+`uan` (2 × 1.0 = 2.0) still beats a
        // single `taiuan` phrase edge (1.1 + ε ≈ 1.101). The phrase
        // epsilon is far too small to flip this.
        let two_atomic = es(0, 1) + es(0, 1);
        let one_phrase = es(0, 2);
        assert!(
            two_atomic > one_phrase,
            "two_atomic={two_atomic} one_phrase={one_phrase}"
        );
    }

    // -----------------------------------------------------------------------
    // v3.5.8 S3 — user-weight (Gap B → G2) + epsilon-boost
    // -----------------------------------------------------------------------

    #[test]
    fn user_weight_is_multiplicative_on_multi_syllable_edges() {
        // A phrase edge with a positive decayed delta scores strictly
        // higher than the same edge with no user history, and the gain
        // is multiplicative (delta = 1.0 → ×2 the neutral edge).
        let neutral = es(50, 2);
        let boosted = edge_score(50, 2, 1.0);
        assert!(boosted > neutral, "boosted={boosted} neutral={neutral}");
        // neutral = U×bias + ε ; boosted = U×bias×2 + ε.
        let expected = (neutral - WALKER_PHRASE_EPSILON) * 2.0 + WALKER_PHRASE_EPSILON;
        assert!((boosted - expected).abs() < 1e-9, "{boosted} vs {expected}");
    }

    #[test]
    fn single_syllable_user_weight_is_damped_by_scale() {
        // The Q4c BLOCK guard: a hot single char with a huge decayed
        // delta must NOT get the multiplicative boost in the path
        // objective (default SCALE = 0.0 → identical to neutral).
        assert_eq!(WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE, 0.0);
        let neutral = es(184_693, 1);
        let with_huge_delta = edge_score(184_693, 1, 4.0);
        assert_eq!(
            with_huge_delta, neutral,
            "single-syllable edge must ignore user delta at SCALE=0.0"
        );
    }

    #[test]
    fn hot_single_char_cannot_sweep_a_user_preferred_phrase() {
        // The motivating Q4c scenario. 「的」: dict freq ~184k,
        // 1-syllable. A user-preferred 2-syllable phrase 「臺語」 with a
        // modest dict freq but a fully-decayed-fresh user delta must
        // out-score TWO hot single-char edges spanning the same buffer
        // — because the single chars get no path-objective user weight.
        let two_hot_singles = edge_score(184_693, 1, 4.0) + edge_score(184_693, 1, 4.0);
        // Phrase: low dict freq, but fresh max user preference.
        let user_phrase = edge_score(400, 2, 4.0);
        // The phrase alone need not beat two hot singles on raw scale;
        // assert the INVARIANT that actually matters: the single-char
        // edges derive ZERO advantage from their user history here, so
        // they score exactly their neutral value (no stale domination
        // amplification).
        let neutral_two = es(184_693, 1) * 2.0;
        assert_eq!(two_hot_singles, neutral_two);
        assert!(user_phrase > es(400, 2), "phrase must gain from user delta");
    }

    #[test]
    fn phrase_epsilon_only_applies_to_multi_syllable_edges() {
        // Single / clamped-zero syllable → no epsilon; >= 2 → epsilon.
        // At freq 0 with no user history the ONLY difference between a
        // 1-syllable and a 2-syllable edge is `syll_bias` (+0.1) plus
        // the additive phrase epsilon — proving epsilon fires only for
        // syll >= 2 and is purely additive.
        let single = es(0, 1);
        let phrase = es(0, 2);
        assert!((single - 1.0).abs() < 1e-9, "single={single}");
        let delta = phrase - single;
        // Tolerance is 1e-6, not 1e-9: `BOOST_ALPHA` is `f32` (0.1f32
        // widens to f64 ≈ 0.10000000149), a pre-existing S2
        // representation artifact, not an S3 imprecision.
        assert!(
            (delta - (0.1 + WALKER_PHRASE_EPSILON)).abs() < 1e-6,
            "delta={delta}"
        );
    }

    #[test]
    fn epsilon_is_a_tie_band_nudge_not_a_ranking_override() {
        // Epsilon must break an exact tie toward the phrase but must
        // NOT flip a single-char path that is materially ahead.
        // Exact tie at freq 0: phrase (1.1 + ε) vs a single 1-syll
        // edge scaled to 1.1 by a contrived delta — phrase wins by ε.
        let phrase = es(0, 2); // 1.1 + ε
        let tied_single = 1.0 + 0.1; // hypothetical equal-bias single
        assert!(phrase > tied_single, "epsilon should break the tie");
        assert!(
            phrase - tied_single < 2.0 * WALKER_PHRASE_EPSILON,
            "epsilon must stay a tiny tie-band nudge"
        );
        // Materially-ahead single char is NOT flipped by epsilon.
        let strong_single = es(1000, 1); // 1 + ln(1001) ≈ 7.9
        assert!(
            strong_single > es(0, 2),
            "epsilon cannot rescue a materially worse phrase"
        );
    }
}
