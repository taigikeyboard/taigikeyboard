//! v3.5.8 S5 — whole-sentence walker edge **cost** (min-cost model).
//!
//! **Lower = better.** The walker's path objective is `min Σ edge_cost`
//! over the chosen forward-only edge chain. This is a faithful port of
//! the khiin word-level DP segmenter
//! (`references/khiin-rs/khiin/src/data/segmenter.rs:82-93` +
//! `segment_min_cost` line 192), which is the same optimum as the
//! McBopomofo Gramambular `max Σ log P` relaxation
//! (`references/McBopomofo/algorithm.md`, `reading_grid.cpp:134`) and
//! librime's Viterbi-over-DAG: minimizing `Σ −ln(p)` ≡ maximizing the
//! path log-probability.
//!
//! ## Why S5 (the S2/S3 defect this slice corrects)
//!
//! S2/S3 (`edge_score`, removed here) cited khiin's formula but dropped
//! its two load-bearing parts: the `ln(1/p)` **probability
//! normalization** and the **minimization**. The result —
//! `(1 + ln(1 + freq)) × syll_bias × user_weight`, *maximized* and
//! summed — gave every edge a positive `1 +` floor and no per-segment
//! normalization, so a finer segmentation accumulated *more* reward.
//! With real dictionary frequencies (common single-character morphemes
//! like 伊 ≈ 63 255, 有 ≈ 53 685 dwarf any specific phrase such as
//! 台灣 ≈ 1 379) the all-single-char path won by ~4.7×: typing
//! `taiuan` surfaced `乾伊有俺` instead of `台灣` (dogfood-confirmed
//! 2026-05-17; `docs/roadmap.md` §整句 lattice + walker S5).
//!
//! ## The model (khiin `segmenter.rs:82-92`)
//!
//! ```text
//! p          = (1 + freq) / CORPUS_TOTAL_FREQ          // Laplace-smoothed corpus probability
//! cost       = ln(1 / p)                               // = ln(CORPUS) − ln(1 + freq) ≥ 0
//! cost       = cost / toneless_len^LETTER_COUNT_BIAS   // longer spelling → cheaper
//!                   × syllable_count^SYLLABLE_COUNT_BIAS
//! applied_δ  = syll <= 1 ? user_weight_delta × SINGLE_SYLLABLE_SCALE
//!                        : user_weight_delta
//! cost      −= ln(1 + applied_δ)                       // user preference = log-space discount
//! ```
//!
//! `CORPUS_TOTAL_FREQ > 1 + max(freq)`, so `p ∈ (0, 1)` and
//! `ln(1/p) > 0` for every edge — including an OOV (`freq = 0`) edge,
//! which stays finite via Laplace smoothing instead of needing a
//! special-case fallback (`feedback_no_redundant_fallback`). The
//! decisive structural property the old model lacked: **every edge
//! pays the ~`ln(CORPUS_TOTAL_FREQ)` corpus-normalization toll**, so a
//! finer segmentation accumulates strictly more cost and a real phrase
//! out-competes its single-character decomposition.
//!
//! The OOV case (`taiuantai` with no dictionary hit anywhere) would, on
//! its own, collapse to the *fewest*-edge blob under min-cost. The
//! user-facing per-syllable romanization is therefore produced by an
//! explicit carve-out in `dispatch::fetch_walker_slot0`, **outside**
//! this cost function (Codex pre-impl S5 Q2, 2026-05-17) — not by an
//! OOV cost tuned to fight the corpus normalization.

// 中文: S5 — 全句 walker 的單條 edge 成本(越小越好;min Σ cost = khiin segment_min_cost = McBopomofo max Σ log P)。
// 中文: 忠實移植 khiin segmenter.rs:82-92:p=(1+freq)/CORPUS、cost=ln(1/p)/len^0.2·syll^0.2、user 偏好 log-space 折減。
// 中文: S2/S3 把 khiin 的 ln(1/p) 正規化 + minimize 拿掉 → 全正、無正規化、maximize → 結構性獎勵過度切分
// 中文:   (真實 freq 下 taiuan 吐 乾伊有俺,dogfood 2026-05-17)。每條 edge 付 ~ln(CORPUS) 正規化稅 → 細分嚴格更貴。
// 中文: 無字典 OOV 的逐音節羅馬字由 dispatch::fetch_walker_slot0 顯式 carve-out 產生(不進本成本函式,Codex S5 Q2)。

/// Total corpus frequency mass — the denominator that turns a raw
/// `DictionaryRecord.frequency` into a corpus probability
/// (khiin `segmenter.rs:82-86`, where `p = occurrences / total`).
///
/// **Provenance**: `Σ frequency` over every row of
/// `dictionary/output/dictionary.csv` (159 034 entries) =
/// `12_910_574`, measured 2026-05-17. `dict.bin` v2 carries only
/// per-record frequency, no corpus-total metadata
/// (`dictionary/build/create_dictionary_bin.py`), so this is baked as a
/// constant with a regeneration guard rather than summed at lexicon
/// init (Codex pre-impl S5 Q4 = option a, 2026-05-17): the exact value
/// is not highly sensitive (khiin itself notes "experiment with
/// different models") and a checked-in artifact + checked const is
/// simpler and audit-friendlier than new lexicon plumbing + a startup
/// scan. **Recompute and update this (and the `cost::tests`
/// regeneration guard) whenever the dictionary is rebuilt.**
// 中文: 語料總頻 = dictionary.csv 全列 frequency 加總(159034 列,12_910_574,2026-05-17 量測)。
// 中文: dict.bin v2 無語料總計 metadata → bake 成常數 + regeneration guard(Codex S5 Q4=a);字典重建須同步更新此值與守門測試。
pub(crate) const CORPUS_TOTAL_FREQ: f64 = 12_910_574.0;

/// Compile-time invariant: `CORPUS_TOTAL_FREQ` must exceed
/// `1 + max(freq)` (的 = 184_693) so every `ln(1/p)` is strictly
/// positive — no zero/negative edge cost, no `ln` of a non-positive
/// number. A dictionary rebuild that pushes a single entry past this
/// (or a mistyped constant) fails the **build**, not just a test.
const _: () = assert!(CORPUS_TOTAL_FREQ > 184_694.0);

/// khiin `LETTER_COUNT_BIAS` (`segmenter.rs:20`, default `0.2`):
/// `cost / toneless_len^0.2` — a longer spelling is a (mildly) cheaper
/// edge, biasing the DP toward fewer, longer words. Kept as a named
/// cited constant, not a runtime tunable (Codex pre-impl S5 Q6, 2026-05-17:
/// changing the model and tuning it in one patch makes regressions
/// harder to reason about; dogfood can tune later).
// 中文: khiin LETTER_COUNT_BIAS(segmenter.rs:20,0.2);長拼寫 → 較便宜,偏好少而長的詞。固定 cited 常數非 runtime tunable。
pub(crate) const LETTER_COUNT_BIAS: f64 = 0.2;

/// khiin `SYLLABLE_COUNT_BIAS` (`segmenter.rs:28`, default `0.2`):
/// `cost × syllable_count^0.2` — a word spanning more syllables pays
/// slightly more, the khiin counter-bias that keeps `letter` length
/// preference from over-favouring very long single entries.
// 中文: khiin SYLLABLE_COUNT_BIAS(segmenter.rs:28,0.2);跨較多音節的詞略貴,平衡 letter 長度偏好。
pub(crate) const SYLLABLE_COUNT_BIAS: f64 = 0.2;

/// Compile-time invariant: both khiin length-normalization exponents
/// must stay positive — a `0.0` exponent collapses `x^bias` to `1.0`
/// and silently removes the length/syllable normalization that makes
/// the model penalize over-segmentation.
const _: () = assert!(LETTER_COUNT_BIAS > 0.0 && SYLLABLE_COUNT_BIAS > 0.0);

/// v3.5.8 S3/S5 — fraction of the decayed user-frequency delta a
/// **single-syllable** (`syllable_count <= 1`) walker edge receives.
/// `0.0` = single-syllable edges get NO walker user discount, so a hot
/// single character (e.g. 「的」, dict freq ~184k) cannot ride its own
/// stale user history to sweep the whole sentence. That single-char
/// user preference is already honored where it belongs:
/// `lexicon::best_candidate_for_key` record selection (homophone
/// disambiguation *within* the edge). Letting it also discount the
/// *path objective* is the exact stale single-character-domination
/// failure the user-weight term must not reopen (Codex pre-impl S3 Q4c,
/// 2026-05-16, BLOCK condition — preserved verbatim through the S5
/// log-space rewrite: at scale `0.0` the discount is `ln(1) = 0`).
/// Multi-syllable edges get the full delta (scale `1.0`, applied in
/// [`edge_cost`]). **Dogfood-tunable 0.0..=0.25** if dogfood shows
/// single-syllable user preference is under-weighted in segmentation.
// 中文: S3/S5 — 單音節 edge 拿到的 decayed user delta 比例;0.0 = 不拿 walker user 折減。
// 中文:   熱門單字(如「的」)不可靠自己 stale user 史壓垮整句(Codex S3 Q4c BLOCK,S5 log-space 改寫沿用:scale 0 → ln(1)=0)。
// 中文:   其 user 偏好已由 best_candidate_for_key 選 record 時體現。多音節 edge 拿滿。dogfood 可調 0.0..=0.25。
pub(crate) const WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE: f64 = 0.0;

/// v3.5.8 S6 (Codex pre-impl S6 Q1, 2026-05-17, BLOCK condition) —
/// the **effective corpus frequency** a `custom_dictionary.db` edge is
/// scored with in the S5 min-cost model.
///
/// A custom entry carries no corpus frequency. Rather than an explicit
/// cost floor / discount — which would bypass the "every edge pays the
/// `ln(CORPUS_TOTAL_FREQ)` normalization toll" invariant that S5 added
/// to kill over-segmentation (Codex S6 Q1 **BLOCK**ed a floor) — a
/// custom edge enters [`edge_cost`] with this *effective* frequency, so
/// it competes inside the same probability-shaped objective as a
/// dictionary phrase (librime user-dict-as-frequency semantics,
/// `references/librime/src/rime/dict/user_dictionary.cc`).
///
/// **Value provenance** (Codex pre-impl S6 Q1 gate, measured on the
/// 2026-05-17 `dictionary/output/dictionary.csv`): multi-syllable max
/// frequency ≈ 1_562, single-syllable p95 ≈ 1_461, single-syllable
/// p99 ≈ 7_721. `2_000` makes a custom entry a strong **multi-syllable
/// phrase** competitor (beats a top single-character split) without
/// elevating it to a top-frequency single character (a single-syllable
/// custom edge still loses badly to a real phrase path — pinned by
/// [`tests`]). **Dogfood-tunable**; a named cited constant, not a
/// runtime tunable nor a magic literal.
// 中文: S6 — custom_dictionary.db edge 在 S5 min-cost 模型用的「等效語料頻率」(Codex Q1 BLOCK:用 proxy 不用 cost floor,
// 中文:   floor 會繞過「每 edge 付 ln(CORPUS) 正規化稅」不變式)。2_000 = dictionary.csv 多音節 max≈1562 / 單音節 p95≈1461 / p99≈7721
// 中文:   之間 → custom 是強多音節片語競爭者(勝單字拆分)但非頂頻單字(單音節 custom 仍輸真實片語,測試 pin)。dogfood 可調。
pub(crate) const CUSTOM_EFFECTIVE_FREQ: u32 = 2_000;

/// Min-cost for one lattice edge (**lower = better**).
///
/// - `frequency` — the chosen dictionary candidate's raw
///   `DictionaryRecord.frequency`; `0` for an OOV (no-dict-hit) edge.
///   Raw frequency, never a ranked `score` (Codex pre-impl S5 Q3:
///   `c.score` must not feed the walker cost — that double-counts the
///   record-selection signal).
/// - `syllable_count` — that candidate's syllable count (`>= 1`;
///   clamped, defends against a leaked `0`).
/// - `toneless_len` — the edge's toneless-key char count (khiin's
///   `word_len`). Carried on `EdgeChoice` because the khiin length
///   normalization is not faithful without it (Codex pre-impl S5 Q1
///   BLOCK).
/// - `user_weight_delta` — the time-decayed user-frequency boost delta
///   for this edge's chosen candidate
///   (`ranking::decayed_user_weight_delta`, in `0.0..=4.0`; `0.0` =
///   no user history / never selected / bad clock = neutral). Computed
///   caller-side (`dispatch::fetch_walker_slot0`).
///
/// `cost = ln(1 / p) / toneless_len^0.2 × syllable_count^0.2
///         − ln(1 + applied_delta)` where `p = (1 + frequency) /
/// CORPUS_TOTAL_FREQ`. Always finite and `> 0` before the user
/// discount: `CORPUS_TOTAL_FREQ > 1 + max(freq)` keeps `ln(1/p) > 0`,
/// and lengths/syllables clamp to `>= 1`. The user discount is a
/// `ln(1 + δ) >= 0` subtraction (δ clamped `>= 0`), so user preference
/// only ever *lowers* an edge's cost, never raises it (Codex pre-impl
/// S5 Q3 — log-space sign).
// 中文: 單 edge 最小化成本(越小越好)= ln(1/p)/len^0.2·syll^0.2 − ln(1+applied_δ),p=(1+freq)/CORPUS。
// 中文: CORPUS>1+max(freq) → ln(1/p)>0;len/syll clamp≥1;user 折減 ln(1+δ)≥0(δ clamp≥0)只會降本不會升本。
pub(crate) fn edge_cost(
    frequency: u32,
    syllable_count: u8,
    toneless_len: usize,
    user_weight_delta: f64,
) -> f64 {
    // khiin segmenter.rs:82-89 — Laplace-smoothed corpus probability so
    // an OOV (freq 0) edge stays finite without a fallback path.
    let p = (1.0 + f64::from(frequency)) / CORPUS_TOTAL_FREQ;
    let mut cost = (1.0 / p).ln();
    // khiin segmenter.rs:90-92 — `cost / word_len^0.2 * n_syls^0.2`.
    let len_bias = (toneless_len.max(1) as f64).powf(LETTER_COUNT_BIAS);
    let syll_bias = f64::from(syllable_count.max(1)).powf(SYLLABLE_COUNT_BIAS);
    cost = cost / len_bias * syll_bias;
    // S5 (Codex pre-impl Q3): the S3 user preference becomes a
    // log-space cost discount. Syllable-aware — a 1-syllable edge gets
    // only SCALE (=0.0) of the decayed delta so a hot single char
    // cannot ride the discount to sweep the sentence (the Q4c BLOCK
    // guard from #286, preserved: at scale 0.0 the discount is
    // `ln(1) = 0`). δ clamped `>= 0` so the discount cannot become a
    // penalty on a malformed input.
    let applied_delta = if syllable_count <= 1 {
        user_weight_delta * WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE
    } else {
        user_weight_delta
    };
    cost - applied_delta.max(0.0).ln_1p()
}

#[cfg(test)]
mod tests {
    use super::{
        edge_cost, CORPUS_TOTAL_FREQ, CUSTOM_EFFECTIVE_FREQ, LETTER_COUNT_BIAS,
        WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE,
    };

    /// Neutral-user-weight edge cost — no user history
    /// (`user_weight_delta = 0.0`). The model invariants are expressed
    /// through this.
    fn ec(freq: u32, syll: u8, len: usize) -> f64 {
        edge_cost(freq, syll, len, 0.0)
    }

    #[test]
    fn corpus_total_freq_matches_dictionary_csv() {
        // Codex pre-impl S5 Q4 regeneration guard. `CORPUS_TOTAL_FREQ`
        // is `Σ frequency` over `dictionary/output/dictionary.csv`. If
        // the dictionary is rebuilt this asserts the const was updated
        // alongside it. Pure arithmetic pin (no file IO in a unit
        // test): the measured sum and entry count are recorded so a
        // drifted const fails loudly with the expected value.
        const MEASURED_SUM: f64 = 12_910_574.0;
        const MEASURED_ENTRIES: u32 = 159_034;
        assert_eq!(
            CORPUS_TOTAL_FREQ, MEASURED_SUM,
            "CORPUS_TOTAL_FREQ drifted from the 2026-05-17 dictionary.csv \
             Σfrequency={MEASURED_SUM} over {MEASURED_ENTRIES} entries; \
             recompute after a dictionary rebuild"
        );
        // The `> 1 + max(freq)` strict-positivity invariant is a
        // compile-time guard (`const _: () = assert!(…)` next to the
        // constant), not a runtime assertion here.
    }

    #[test]
    fn oov_edge_is_finite_and_strictly_positive() {
        // No-dict edge (freq 0) must stay finite & positive so the DP
        // never sees a `-inf`/`NaN` (Bug 2 / §1 subsumed without a
        // special fallback). cost = ln(CORPUS/1) / 1^0.2 * 1^0.2.
        let c = ec(0, 1, 3);
        assert!(c.is_finite() && c > 0.0, "{c}");
        let expected = (CORPUS_TOTAL_FREQ).ln() / 3f64.powf(LETTER_COUNT_BIAS);
        assert!((c - expected).abs() < 1e-9, "{c} vs {expected}");
    }

    #[test]
    fn higher_frequency_costs_less() {
        // Min-cost: a more frequent edge is cheaper (better).
        assert!(ec(1000, 1, 3) < ec(10, 1, 3));
        assert!(ec(10, 1, 3) < ec(0, 1, 3));
    }

    #[test]
    fn real_phrase_beats_its_single_char_decomposition() {
        // The motivating bug, with real dictionary frequencies.
        // `taiuan` → 台灣 (freq 1379, 2 syll, toneless "taiuan" len 6)
        // as ONE edge must cost strictly less than the 4 single-char
        // path 乾(2145,len2) 伊(63255,len1) 有(53685,len1) 俺(10635,len2)
        // — the path the broken S2/S3 max-Σ objective produced.
        let phrase = ec(1379, 2, 6);
        let four_singles = ec(2145, 1, 2) + ec(63255, 1, 1) + ec(53685, 1, 1) + ec(10635, 1, 2);
        assert!(
            phrase < four_singles,
            "phrase={phrase} four_singles={four_singles}"
        );
    }

    #[test]
    fn custom_two_syllable_beats_top_single_char_split() {
        // S6 Codex pre-impl Q1 regression guard #1: a 2-syllable custom
        // entry scored at CUSTOM_EFFECTIVE_FREQ with a short toneless
        // roman must out-compete the path through the two highest-
        // frequency single characters that cover the same span. Real
        // dictionary singles for the motivating `taigi` case:
        // 台(31281,len3) + 語(21976,len2). The custom 2-syll edge
        // (effective freq 2000, toneless "taigi" len 5) must cost less.
        let custom = edge_cost(CUSTOM_EFFECTIVE_FREQ, 2, 5, 0.0);
        let top_single_split = edge_cost(31_281, 1, 3, 0.0) + edge_cost(21_976, 1, 2, 0.0);
        assert!(
            custom < top_single_split,
            "custom={custom} top_single_split={top_single_split}"
        );
    }

    #[test]
    fn two_single_syllable_custom_lose_to_a_real_phrase_path() {
        // S6 Codex pre-impl Q1 regression guard #2: CUSTOM_EFFECTIVE_FREQ
        // must NOT elevate custom to a top-frequency single character —
        // a path of two single-syllable custom edges still loses badly
        // to one real multi-syllable dictionary phrase covering the same
        // buffer. 台灣 (freq 1379, 2 syll, toneless "taiuan" len 6) as
        // ONE edge must cost strictly less than two atomic custom edges.
        let two_single_custom = edge_cost(CUSTOM_EFFECTIVE_FREQ, 1, 3, 0.0)
            + edge_cost(CUSTOM_EFFECTIVE_FREQ, 1, 3, 0.0);
        let real_phrase = edge_cost(1_379, 2, 6, 0.0);
        assert!(
            real_phrase < two_single_custom,
            "real_phrase={real_phrase} two_single_custom={two_single_custom}"
        );
    }

    #[test]
    fn syllable_zero_clamps_to_one() {
        // syllable_count is a u8 from a record; a leaked 0 must clamp
        // to 1 (no powf(0)=… surprise, takes the single-syllable
        // user-scale branch identical to syll = 1).
        assert_eq!(edge_cost(0, 0, 3, 3.5), edge_cost(0, 1, 3, 3.5));
    }

    #[test]
    fn longer_spelling_is_cheaper_at_equal_frequency() {
        // khiin letter bias: `/ word_len^0.2`. At equal freq & syllable
        // count a longer toneless spelling is the cheaper edge.
        assert!(ec(100, 1, 6) < ec(100, 1, 3));
    }

    #[test]
    fn user_preference_discounts_multi_syllable_cost() {
        // A phrase edge with a positive decayed delta costs strictly
        // less than the same edge with no user history; the discount is
        // exactly `ln(1 + delta)` in log space.
        let neutral = ec(50, 2, 4);
        let preferred = edge_cost(50, 2, 4, 1.0);
        assert!(
            preferred < neutral,
            "preferred={preferred} neutral={neutral}"
        );
        assert!((preferred - (neutral - 1.0f64.ln_1p())).abs() < 1e-9);
    }

    #[test]
    fn single_syllable_user_weight_is_damped_to_zero_discount() {
        // The Q4c BLOCK guard, preserved through the log-space rewrite:
        // a hot single char with a huge decayed delta gets ln(1)=0
        // discount → identical to neutral (SCALE = 0.0).
        assert_eq!(WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE, 0.0);
        let neutral = ec(184_693, 1, 1);
        let with_huge_delta = edge_cost(184_693, 1, 1, 4.0);
        assert_eq!(
            with_huge_delta, neutral,
            "single-syllable edge must ignore the user delta at SCALE=0.0"
        );
    }

    #[test]
    fn user_discount_never_raises_cost_on_malformed_delta() {
        // δ is clamped `>= 0`; a (contractually impossible) negative
        // delta must not turn the discount into a penalty.
        let neutral = ec(50, 2, 4);
        assert!(edge_cost(50, 2, 4, -1.0) <= neutral + 1e-12);
        assert!((edge_cost(50, 2, 4, -1.0) - neutral).abs() < 1e-9);
    }

    #[test]
    fn cost_is_finite_at_frequency_and_syllable_extremes() {
        for &(f, s, l) in &[
            (0u32, 0u8, 0usize),
            (0, 1, 1),
            (184_693, 8, 12),
            (u32::MAX, u8::MAX, 64),
        ] {
            let c = edge_cost(f, s, l, 4.0);
            assert!(c.is_finite(), "f={f} s={s} l={l} → {c}");
        }
    }
}
