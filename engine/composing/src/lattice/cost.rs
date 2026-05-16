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
//! Single-character-domination defense (McBopomofo epsilon-boost) and
//! `user_frequency.db` time-decay (librime `formula_d`) are **S3**, not
//! here (plan `/Users/alexsu/.claude/plans/greedy-questing-cake.md` §S3);
//! S2 is unigram + length bias only.

// 中文: S2 — 全句 walker 的單條 edge 成本 (越大越好)。
// 中文: 與 ranking::calculate_continuous_score 同 cost 家族的 log 對應:ln(1+freq)×(1+0.1×(syll−1))。
// 中文: +1 平滑讓無字典 (freq=0) 的 edge 仍有限 → 純羅馬字路徑是 walker 自然最佳路徑,非特例 fallback。
// 中文: 防單字壓句 epsilon-boost / user-freq 時間衰減屬 S3,不在此片。

use ranking::BOOST_ALPHA;

/// Log-space unigram score for one lattice edge. `frequency` is the
/// chosen dictionary candidate's raw `DictionaryRecord.frequency`
/// (`0` when the edge has no dict hit — a pure-roman syllable span);
/// `syllable_count` is that candidate's syllable count (`>= 1`).
///
/// `(1 + ln(1 + freq)) × syll_bias` where
/// `syll_bias = 1 + BOOST_ALPHA × (syll − 1)` mirrors
/// `calculate_continuous_score`'s multiplicative syllable bias so a
/// 2-syllable phrase edge is rewarded over two 1-syllable atomic edges
/// at equal per-syllable frequency (the `臺灣台語` vs `臺/灣/台/語`
/// case). The leading `1 +` keeps every reachable edge a strictly
/// positive contribution so a longer all-zero-frequency path (the
/// no-dict roman case) accumulates a higher total than a shorter one
/// — the deterministic lever that yields per-syllable
/// segmentation `tai uan tai` (see `walker::walk_best` tie contract).
// 中文: 單 edge 的 log 空間 unigram 分數;syll_bias 對齊 calculate_continuous_score 的音節倍率。
// 中文: 前導 `1 +` 讓每條可達 edge 皆為正貢獻 → 全零頻 (無字典) 路徑「edge 越多總分越高」,
// 中文:   產生逐音節分詞 tai uan tai (見 walker::walk_best 的 tie 契約)。
pub(crate) fn edge_score(frequency: u32, syllable_count: u8) -> f64 {
    let unigram = 1.0 + (1.0 + f64::from(frequency)).ln();
    let syll = f64::from(syllable_count.max(1));
    let syll_bias = 1.0 + f64::from(BOOST_ALPHA) * (syll - 1.0);
    unigram * syll_bias
}

#[cfg(test)]
mod tests {
    use super::edge_score;

    #[test]
    fn zero_frequency_edge_is_strictly_positive() {
        // No-dict edge must still contribute a finite positive score so
        // the roman best-path exists (Bug 2 / §1 subsumed by the walker,
        // not a fallback).
        let s = edge_score(0, 1);
        assert!(s > 0.0 && s.is_finite(), "{s}");
        // ln(1) = 0, so it is exactly the leading `1 +` × bias(1) = 1.0.
        assert!((s - 1.0).abs() < 1e-9, "{s}");
    }

    #[test]
    fn higher_frequency_scores_higher() {
        assert!(edge_score(1000, 1) > edge_score(10, 1));
        assert!(edge_score(10, 1) > edge_score(0, 1));
    }

    #[test]
    fn syllable_bias_rewards_longer_words() {
        // At equal frequency a 2-syllable edge beats a 1-syllable edge
        // (the phrase-priority lever).
        assert!(edge_score(100, 2) > edge_score(100, 1));
        assert!(edge_score(100, 4) > edge_score(100, 2));
    }

    #[test]
    fn syllable_zero_clamps_to_one_no_underflow() {
        // syllable_count is u8 from a record; defend against a leaked 0.
        assert_eq!(edge_score(0, 0), edge_score(0, 1));
    }

    #[test]
    fn two_atomic_edges_outscore_one_phrase_edge_at_equal_zero_frequency() {
        // The no-dict tie lever: with all frequencies 0, a finer
        // (more-edge) segmentation accumulates a higher total than a
        // coarser one — `tai`+`uan` (2 × 1.0) beats a single
        // `taiuan` phrase edge (1 × bias). Even the 2-syllable phrase
        // bias (1.1) cannot beat two unit edges (2.0).
        let two_atomic = edge_score(0, 1) + edge_score(0, 1);
        let one_phrase = edge_score(0, 2);
        assert!(
            two_atomic > one_phrase,
            "two_atomic={two_atomic} one_phrase={one_phrase}"
        );
    }
}
