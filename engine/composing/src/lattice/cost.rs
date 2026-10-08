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
//! 2026-05-17; `docs/releases/v3.5.8/plan.md` §整句 lattice + walker S5).
//!
//! ## The model (khiin `segmenter.rs:82-92`, E1 unified word frequency)
//!
//! ```text
//! base       = walker_cost / 1000                      // −ln p of the word, dictionary.bin v4
//! base       = min(base, user_entry_cost_cap(s))       // multi-syllable selected word only
//! cost       = base / toneless_len^LETTER_COUNT_BIAS   // longer spelling → cheaper
//!                   × syllable_count^SYLLABLE_COUNT_BIAS
//! applied_δ  = syll <= 1 ? user_weight_delta × SINGLE_SYLLABLE_SCALE
//!                        : user_weight_delta
//! cost      −= ln(1 + applied_δ)                       // user preference = log-space discount
//! ```
//!
//! `walker_cost` is one corpus probability per `(hanji, TL)` word, built
//! from the segmented corpus (`dictionary/build/walker_lm.py`, Lidstone
//! add-α over the dictionary vocabulary, stored as milli-nats), so a
//! single character and a phrase are priced on the same scale
//! (`docs/architecture/unified-word-frequency-roadmap.md` §3). Before E1
//! the edge was priced on `DictionaryRecord.frequency`, which mixes
//! character counts (including occurrences inside other words) with word
//! counts — `kau3siu7` composed 到受 over 教授.
//!
//! `p < 1`, so `−ln p > 0` for every **dict** edge (the branch is gated
//! on `dict_hit`, never on the cost value). The decisive structural
//! property the old max-Σ model lacked: **every dict edge pays its own
//! `−ln p` corpus-normalization toll**, so a finer segmentation
//! accumulates strictly more cost and a real phrase out-competes its
//! single-character decomposition. An OOV
//! (no-dict-hit) edge does NOT take this probability path at all —
//! RC0 prices it as khiin's per-char `BIG` (`OOV_PER_CHAR_PENALTY *
//! toneless_len`); see the OOV section below.
//!
//! ## OOV (no-dict-hit) edges — the v3.5.8 RC0 fix
//!
//! khiin has TWO distinct mechanisms; S5/S7 conflated them. (1) A
//! **known dictionary word** whose corpus probability is `<= 0` is
//! floored at `p = 1e-5 / 10^word_len` and priced on the `ln(1/p)`
//! curve (`segmenter.rs:75-92`) — a corpus-gap floor for an *indexed*
//! word. (2) A span with **no dictionary word** is not priced by any
//! probability at all: `segment_min_cost:198-206` advances one char
//! paying `BIG = 1e10`, so a contiguous uncovered region accumulates
//! `BIG` **per char**. Real words are `ln`-scale (~5-30), so in khiin
//! a single OOV char dominates any dict-coverable segmentation at any
//! length.
//!
//! S5 priced a genuine OOV span with mechanism (1)'s smooth
//! corpus-normalized probability (`ln`-scale). A whole-buffer OOV blob
//! is ONE edge, so it paid khiin's `÷ word_len^0.2` discount once over
//! the full buffer and undercut a dict-covering path that accumulates a
//! per-edge `ln(CORPUS/freq)` toll **linearly in edge count**. Past ~6
//! dict edges the blob won → `any_dict == false` → the carve-out
//! rendered bare roman (`ginalangtsiahpngbesai → "gin a lang tsiah png
//! be sai"` instead of 囡仔人食飯袂使; threshold exactly 6 syllables,
//! reproduced against the real dictionary 2026-05-18). S7's
//! `UNKNOWN_SYLLABLE_DECAY` only softened the slope; it did not
//! de-conflate the two mechanisms (`taiuanta`/`taiuantai` only happened
//! to fall below the 6-edge threshold).
//!
//! RC0 de-conflates them: an OOV edge is khiin's mechanism (2) —
//! `OOV_PER_CHAR_PENALTY * toneless_len` (per-char `BIG`), unbiased and
//! with no user discount. So "OOV loses to any dict-coverable path"
//! holds at **every length** — a *cost property* (khiin's own `BIG`
//! constant), NOT a lexicographic `dict_hit` short-circuit (Codex
//! pre-impl S7 Q1 / RC0 Q2). For a buffer with **no dictionary hit
//! anywhere** the min-cost path is still all-OOV; the user-facing
//! per-syllable romanization is produced by an explicit carve-out in
//! `continuous::fetch_walker_slot0_inner`, **outside** this cost function
//! (Codex pre-impl S5 Q2) — the OOV pricing here only governs *path
//! selection* (dict path vs OOV blob), never the rendered string.

// Walker model parameters — engine-only, no platform mirror.
//
// The `pub(crate) const` below (`LETTER_COUNT_BIAS`, `SYLLABLE_COUNT_BIAS`,
// `OOV_PER_CHAR_PENALTY`, `WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE`,
// `CUSTOM_EDGE_COST`, `USER_ENTRY_EFFECTIVE_COUNT`) parameterize `edge_cost` (dictionary branch + OOV pricing branch). All are
// named, cited constants — not runtime tunables nor magic literals.
//
// **Do not consolidate with the Cluster 1 ranking constants** in
// `engine/ranking/src/score.rs` (`BOOST_ALPHA`, `MAX_BOOST`,
// `RECENCY_WINDOW_MS`, `CONTINUOUS_DEFAULT_SOURCE_RANK`,
// `USER_WEIGHT_DECAY_TAU_MS`). Those are `pub` cross-platform invariants per
// `docs/contributing/cross-platform-alignment.md` §3a — platforms mirror them and the
// `pub` surface is part of the contract. The walker constants here are
// `pub(crate)`, engine-only (no platform sees them), and stay with the cost
// model they parameterize (v3.5.9 A3 reframe).

/// `DictionaryRecord::walker_cost` unit: milli-nats (`dictionary.bin` v4,
/// `docs/engine/binary-format.md`).
const MILLI_NATS_PER_NAT: f64 = 1000.0;

/// khiin `LETTER_COUNT_BIAS` (`segmenter.rs:20`, default `0.2`):
/// `cost / toneless_len^0.2` — a longer spelling is a (mildly) cheaper
/// edge, biasing the DP toward fewer, longer words. Kept as a named
/// cited constant, not a runtime tunable (Codex pre-impl S5 Q6, 2026-05-17:
/// changing the model and tuning it in one patch makes regressions
/// harder to reason about; dogfood can tune later).
pub(crate) const LETTER_COUNT_BIAS: f64 = 0.2;

/// khiin `SYLLABLE_COUNT_BIAS` (`segmenter.rs:28`, default `0.2`):
/// `cost × syllable_count^0.2` — a word spanning more syllables pays
/// slightly more, the khiin counter-bias that keeps `letter` length
/// preference from over-favouring very long single entries.
pub(crate) const SYLLABLE_COUNT_BIAS: f64 = 0.2;

/// Compile-time invariant: both khiin length-normalization exponents
/// must stay positive — a `0.0` exponent collapses `x^bias` to `1.0`
/// and silently removes the length/syllable normalization that makes
/// the model penalize over-segmentation.
const _: () = assert!(LETTER_COUNT_BIAS > 0.0 && SYLLABLE_COUNT_BIAS > 0.0);

/// v3.5.8 RC0 fix — the per-char penalty an **OOV (no-dict-hit)** span
/// pays. khiin's literal `BIG` (`segmenter.rs:29` `const BIG: f64 =
/// 1e10`).
///
/// khiin has TWO distinct mechanisms that S5/S7 conflated:
///
/// 1. `segmenter.rs:75-92` (`Segmenter::new`) — a **known dictionary
///    word** whose corpus probability is `<= 0.0` is floored at
///    `p = 1e-5 / 10^word_len` and priced on the `ln(1/p)` curve. This
///    is a corpus-gap floor for an *indexed* word, NOT the price of a
///    genuinely uncovered span.
/// 2. `segment_min_cost:198-206` — a span with **no dictionary word**
///    is not priced by any probability at all: the DP advances one
///    char paying `costs[i-1] + BIG` (`BIG = 1e10`), so a contiguous
///    uncovered region accumulates `BIG` **once per char**. Real words
///    are `ln`-scale (~5-30); a single OOV char (`1e10`) therefore
///    dominates any dict-coverable segmentation at any length — khiin
///    essentially never emits an OOV blob when a dict path exists.
///
/// S5 priced a genuine OOV span with mechanism (1)'s smooth
/// corpus-normalized probability (`ln`-scale), so a whole-buffer OOV
/// blob — one edge paying khiin's `÷ word_len^0.2` discount once over
/// the full buffer — undercut a dict-covering path that accumulates a
/// per-edge `ln(CORPUS/freq)` toll linearly in edge count. Past ~6
/// dict edges the blob won → `any_dict == false` → the
/// `continuous::fetch_walker_slot0_inner` carve-out rendered bare roman
/// (`ginalangtsiahpngbesai → "gin a lang tsiah png be sai"` instead of
/// 囡仔人食飯袂使; threshold exactly 6 syllables, reproduced against the
/// real dictionary 2026-05-18). S7's `UNKNOWN_SYLLABLE_DECAY` only
/// softened the slope; it did not de-conflate the two mechanisms.
///
/// RC0 de-conflates them: an OOV edge costs `OOV_PER_CHAR_PENALTY *
/// toneless_len` (khiin's per-char `BIG`; `toneless_len` = the edge's
/// char count = khiin `word_len`), unbiased and with no user discount
/// (khiin's `BIG` is raw; an OOV edge always carries
/// `user_weight_delta = 0`). The dictionary branch keeps the full
/// khiin cost-map formula. This makes "OOV loses to any dict-coverable
/// path" hold at every length — as a **cost property** (khiin's own
/// `BIG` constant), NOT a lexicographic `dict_hit` short-circuit
/// (Codex pre-impl S7 Q1 / RC0 Q2). A cited constant, not a runtime
/// tunable (same rationale as the khiin bias exponents).
pub(crate) const OOV_PER_CHAR_PENALTY: f64 = 1e10;

/// Compile-time invariant: the OOV per-char penalty must dominate any
/// realistic dict-coverable path sum. A single dict edge's `−ln p`
/// is at most `u16::MAX` milli-nats ≈ 65.5 (13.2 for an unseen word
/// today), scaled by the khiin biases to `O(100)`; even a thousand-edge
/// sentence stays `O(1e5)`.
/// `1e10` per OOV char is `>= 1e5` orders clear of that, so a
/// dict-coverable buffer can never collapse to an OOV blob (RC0). The
/// guard also keeps the penalty strictly positive (finite f64 at the
/// `u8::MAX` toneless-length extreme: `1e10 * 255 = 2.55e12`).
const _: () = assert!(OOV_PER_CHAR_PENALTY >= 1e5);

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
pub(crate) const WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE: f64 = 0.0;

/// The corpus count a user entry stands for: a `custom_dictionary.db`
/// row, a learned phrase, or a multi-syllable word the user selected is
/// priced like a word seen this often (librime records committed phrases
/// into the same user dictionary as the corpus words,
/// `references/librime/src/rime/dict/user_dictionary.cc`). It fixes
/// [`CUSTOM_EDGE_COST`] and the shape of the selection fade in
/// [`user_entry_cost_cap`].
///
/// **Value provenance** (Codex pre-impl S6 Q1, 2026-05-17, kept through
/// E1 P3): `2_000` made a custom entry a strong **multi-syllable phrase**
/// competitor — it beats a split into the two cheapest single characters
/// — without elevating it to a top single character. The rule tests in
/// [`tests`] pin both sides on today's costs. **Dogfood-tunable**; E1 P4
/// kept it — the gold set has no user entries to calibrate it on.
pub(crate) const USER_ENTRY_EFFECTIVE_COUNT: f64 = 2_000.0;

/// `walker_cost` (milli-nats) of a user entry: the walker model's price
/// of a word seen [`USER_ENTRY_EFFECTIVE_COUNT`] times —
/// `round(−ln((2_000 + α) / (N + α·V)) × 1000)` with the
/// `dictionary/output/walker_lm_stats.txt` of 2026-10-08 (α 10,
/// N 3,627,713, V 167,446 → 7,877.7). A custom edge enters [`edge_cost`]
/// at this cost and a learned edge is capped at it, so both compete
/// inside the same probability-shaped objective as a dictionary word
/// (Codex S6 Q1 **BLOCK**ed a cost special-case outside it). Re-derive it
/// when α or the corpus changes (E1 P4).
pub(crate) const CUSTOM_EDGE_COST: u16 = 7_878;

/// The cost cap (nats, before the length terms) a multi-syllable edge
/// whose word the user selected is priced at:
/// `CUSTOM_EDGE_COST + ln((1 + F) / (1 + F·s))`, `F` =
/// [`USER_ENTRY_EFFECTIVE_COUNT`], `s` = `user_weight_delta /
/// BOOST_ALPHA` clamped to `0..=1` — the exact cost-domain translation of
/// the pre-E1 frequency floor `F·s` (E1 roadmap D2). One fresh selection
/// (`s = 1`) is the full custom tier; the cap rises as the selection
/// decays, so a single stale pick stops overriding the corpus, while
/// repeated picks (δ saturates at 4) keep it at the tier. At `s = 0` the
/// cap is `CUSTOM_EDGE_COST + ln(1 + F)` ≈ 15.5 nats, above an unseen
/// word's cost — no lift, finite.
fn user_entry_cost_cap(user_weight_delta: f64) -> f64 {
    let selection = (user_weight_delta / f64::from(ranking::BOOST_ALPHA)).clamp(0.0, 1.0);
    f64::from(CUSTOM_EDGE_COST) / MILLI_NATS_PER_NAT
        + ((1.0 + USER_ENTRY_EFFECTIVE_COUNT) / (1.0 + USER_ENTRY_EFFECTIVE_COUNT * selection)).ln()
}

/// Min-cost for one lattice edge (**lower = better**).
///
/// - `walker_cost` — the edge key's lowest `DictionaryRecord::walker_cost`
///   (milli-nats) across its homophones
///   (`lexicon::EdgeBest::span_walker_cost`, E1 D3), or
///   [`CUSTOM_EDGE_COST`] for a custom edge and at most that for an edge
///   with a learned phrase; **unused when `dict_hit == false`**. A corpus
///   price, never a ranked `score` (Codex pre-impl S5 Q3: `c.score` must
///   not feed the walker cost — that double-counts the record-selection
///   signal).
/// - `syllable_count` — the dict candidate's syllable count (`>= 1`;
///   clamped, defends against a leaked `0`); drives the khiin
///   `n_syls^0.2` bias. For an OOV edge it is **metadata only** (the
///   synthesized candidate's syllable sum) and does NOT enter the cost
///   (RC0 — the OOV penalty is char-length-keyed, not syllable-keyed;
///   Codex pre-impl RC0 Q3).
/// - `toneless_len` — the edge's toneless-key char count (khiin's
///   `word_len`). Both the dict `÷ word_len^0.2` bias and the OOV
///   per-char penalty multiplier (Codex pre-impl S5 Q1 BLOCK; RC0).
/// - `user_weight_delta` — the time-decayed user-frequency boost delta
///   for this edge's chosen candidate
///   (`ranking::decayed_user_weight_delta`, in `0.0..=4.0`; `0.0` =
///   no user history / never selected / bad clock = neutral). Computed
///   caller-side (`continuous::fetch_walker_slot0_inner`). OOV edges always
///   carry `0.0` and take no discount. On a multi-syllable edge it
///   also caps the base cost at [`user_entry_cost_cap`] — one fresh
///   selection is the custom tier, so a selected phrase beats a
///   single-syllable split of its span; the cap fades with the
///   selection's decay.
/// - `dict_hit` — `true` iff this edge is a lexicon-backed hit
///   (`dict.bin` record OR a `custom_dictionary.db` entry). The OOV
///   branch is selected solely from this flag, **never** inferred from
///   the cost value: a custom edge carries no corpus probability and is
///   priced at [`CUSTOM_EDGE_COST`] — it must use the dictionary
///   formula, not the OOV penalty (Codex pre-impl Q2; "never infer
///   no-dict from freq" invariant, `cost.rs` head + `requests.rs`
///   no-dict branch).
///
/// Two khiin mechanisms, de-conflated by RC0 (see
/// [`OOV_PER_CHAR_PENALTY`]):
/// - **dict** (`dict_hit`): khiin `segmenter.rs:82-92` —
///   `cost = base / toneless_len^0.2 × syllable_count^0.2
///   − ln(1 + applied_delta)`, `base = walker_cost / 1000` (capped for a
///   selected multi-syllable word). `>= 0` before the discount; the
///   `ln(1 + δ) >= 0` subtraction (δ clamped `>= 0`) only ever *lowers*
///   cost (Codex pre-impl S5 Q3 — log-space sign).
/// - **OOV** (`!dict_hit`): khiin `segment_min_cost:198-206` —
///   `cost = OOV_PER_CHAR_PENALTY × toneless_len` (khiin's `BIG` per
///   advanced char), unbiased, no user discount. `>= 1e10` per char
///   dominates every `ln`-scale dict-coverable path, so a
///   dict-coverable buffer never collapses to an OOV blob at any
///   length (RC0) — a **cost property**, not a lexicographic
///   `dict_hit` rule (Codex pre-impl S7 Q1 / RC0 Q2).
///
/// Always finite.
pub(crate) fn edge_cost(
    walker_cost: u16,
    syllable_count: u8,
    toneless_len: usize,
    user_weight_delta: f64,
    dict_hit: bool,
) -> f64 {
    if !dict_hit {
        // khiin `segment_min_cost:198-206`: a span with no dictionary
        // word is not priced by any probability — the DP pays `BIG`
        // per advanced char. `toneless_len` = the span's char count =
        // khiin `word_len`. Unbiased, no user discount (khiin's `BIG`
        // is raw; an OOV edge always carries `user_weight_delta = 0`).
        // `walker_cost` / `syllable_count` are intentionally unused here:
        // they are OOV metadata for the synthesized candidate, not cost
        // inputs (Codex pre-impl RC0 Q3).
        return OOV_PER_CHAR_PENALTY * toneless_len.max(1) as f64;
    }
    let mut base = f64::from(walker_cost) / MILLI_NATS_PER_NAT;
    // A user-selected multi-syllable word is user-dict evidence: price it
    // at most at the user-entry tier so its span is not split into
    // cheaper singles. Single-syllable edges keep the S3 damping policy
    // below (no user lift in segmentation).
    if syllable_count > 1 {
        base = base.min(user_entry_cost_cap(user_weight_delta));
    }
    // khiin segmenter.rs:90-92 — `cost / word_len^0.2 * n_syls^0.2`.
    let len_bias = (toneless_len.max(1) as f64).powf(LETTER_COUNT_BIAS);
    let syll_bias = f64::from(syllable_count.max(1)).powf(SYLLABLE_COUNT_BIAS);
    let cost = base / len_bias * syll_bias;
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
        edge_cost, user_entry_cost_cap, CUSTOM_EDGE_COST, OOV_PER_CHAR_PENALTY,
        USER_ENTRY_EFFECTIVE_COUNT, WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE,
    };

    // Production `walker_cost` values (milli-nats) from
    // `assets/dictionaries/dictionary.bin` (E1 P2b, 2026-10-08), with the
    // toneless key length the walker sees.
    const E_THE: u16 = 4_173; // 的 ê, len 1
    const I_HE: u16 = 4_400; // 伊 i, len 1
    const U_HAVE: u16 = 4_496; // 有 ū, len 1
    const TA_DRY: u16 = 10_647; // 乾 ta, len 2
    const AN_I: u16 = 11_166; // 俺 án, len 2
    const TA_SCORCH: u16 = 10_453; // 焦 ta, len 2
    const TAI_UAN: u16 = 7_166; // 台灣 tâi-uân, len 6
    const KAU_ARRIVE: u16 = 5_636; // 到 kàu, len 3
    const SIU_RECEIVE: u16 = 6_856; // 受 siū, len 3
    const KAU_SIU: u16 = 9_938; // 教授 kàu-siū, len 6
    const TAI: u16 = 8_561; // 台 tâi, len 3
    const GI_LANGUAGE: u16 = 10_571; // 語 gí, len 2
    /// An unseen dictionary word: `round(ln((N + α·V) / α) × 1000)`
    /// (`walker_lm_stats.txt` `unseen_cost`).
    const UNSEEN: u16 = 13_181;

    /// Neutral-user-weight **dict-hit** edge cost — no user history
    /// (`user_weight_delta = 0.0`, `dict_hit = true`).
    fn ec(walker_cost: u16, syll: u8, len: usize) -> f64 {
        edge_cost(walker_cost, syll, len, 0.0, true)
    }

    /// Neutral-user-weight **OOV** (no-dict-hit) edge cost. RC0:
    /// `walker_cost` AND `syll` are both irrelevant in this branch — an
    /// OOV edge is priced solely `OOV_PER_CHAR_PENALTY * len` (khiin's
    /// per-char `BIG`).
    fn oov(syll: u8, len: usize) -> f64 {
        edge_cost(0, syll, len, 0.0, false)
    }

    #[test]
    fn dict_edge_is_its_walker_cost_with_khiin_length_terms() {
        // trace: 台灣 7.166 / 6^0.2 × 2^0.2 = 7.166 / 1.43097 × 1.14870 = 5.7524
        let cost = ec(TAI_UAN, 2, 6);
        assert!((cost - 5.7524).abs() < 1e-3, "{cost}");
    }

    #[test]
    fn oov_edge_is_finite_and_strictly_positive() {
        // RC0: an OOV edge must stay finite & positive so the DP never
        // sees a `-inf`/`NaN`. Priced as khiin's per-char `BIG`:
        // `OOV_PER_CHAR_PENALTY * toneless_len`, unbiased.
        let c = oov(1, 3);
        assert!(c.is_finite() && c > 0.0, "{c}");
        let expected = OOV_PER_CHAR_PENALTY * 3.0;
        assert!((c - expected).abs() < 1e-9, "{c} vs {expected}");
    }

    #[test]
    fn unseen_dict_record_uses_dict_pricing_not_oov() {
        // Codex pre-impl Q2 BLOCK guard: a dictionary word the corpus
        // never saw is priced on the dictionary branch at the model's
        // smoothed cost, NEVER the OOV per-char penalty — the OOV branch
        // is gated on `dict_hit`.
        // trace: 13.181 / 6^0.2 × 2^0.2 = 10.5809
        let unseen = ec(UNSEEN, 2, 6);
        assert!((unseen - 10.5809).abs() < 1e-3, "{unseen}");
        assert!(unseen < oov(2, 6), "unseen dict {unseen} must be < OOV");
    }

    #[test]
    fn cheaper_walker_cost_costs_less() {
        assert!(ec(E_THE, 1, 3) < ec(TAI, 1, 3));
        assert!(ec(TAI, 1, 3) < ec(UNSEEN, 1, 3));
    }

    #[test]
    fn real_phrase_beats_its_single_char_decomposition() {
        // `taiuan` → 台灣 (2 syll, len 6) as ONE edge must cost less than
        // the 4 single-char path 乾(len2) 伊(len1) 有(len1) 俺(len2) — the
        // path the broken S2/S3 max-Σ objective produced.
        // trace: 5.752 vs 9.269 + 4.400 + 4.496 + 9.721 = 27.886
        let phrase = ec(TAI_UAN, 2, 6);
        let four_singles = ec(TA_DRY, 1, 2) + ec(I_HE, 1, 1) + ec(U_HAVE, 1, 1) + ec(AN_I, 1, 2);
        assert!(
            phrase < four_singles,
            "phrase={phrase} four_singles={four_singles}"
        );
    }

    #[test]
    fn kau3siu7_word_beats_the_character_composition() {
        // E1 motivating bug: before E1, 到 + 受 (character counts) cost
        // 11.1636 < 教授 11.2279. On one corpus scale the word wins.
        // trace: 教授 9.938 / 6^0.2 × 2^0.2 = 7.978;
        //        到 5.636 / 3^0.2 + 受 6.856 / 3^0.2 = 4.524 + 5.504 = 10.028
        let word = ec(KAU_SIU, 2, 6);
        let composition = ec(KAU_ARRIVE, 1, 3) + ec(SIU_RECEIVE, 1, 3);
        assert!(word < composition, "word={word} composition={composition}");
    }

    /// `key=value` lines of the tracked `dictionary/output/walker_lm_stats.txt`.
    fn walker_lm_stat(key: &str) -> f64 {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../dictionary/output/walker_lm_stats.txt");
        let stats = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        stats
            .lines()
            .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
            .unwrap_or_else(|| panic!("{key}= missing in {}", path.display()))
            .parse()
            .unwrap_or_else(|e| panic!("{key} in {}: {e}", path.display()))
    }

    #[test]
    fn custom_edge_cost_is_the_model_price_of_the_effective_count() {
        // Fails when the walker model changes (α, corpus) until
        // `CUSTOM_EDGE_COST` is re-derived — the drift `CORPUS_TOTAL_FREQ` had.
        // trace (2026-10-08): ln((3,627,713 + 10 × 167,446) / (2,000 + 10)) × 1000
        //      = 7,877.74 → 7,878
        let alpha = walker_lm_stat("alpha");
        let denominator = walker_lm_stat("tokens_N") + alpha * walker_lm_stat("vocabulary_V");
        let model_cost = (denominator / (USER_ENTRY_EFFECTIVE_COUNT + alpha)).ln() * 1000.0;
        assert_eq!(CUSTOM_EDGE_COST, model_cost.round() as u16);
    }

    #[test]
    fn custom_two_syllable_beats_the_cheapest_single_char_split() {
        // S6 Codex pre-impl Q1 regression guard #1: a 2-syllable custom
        // entry beats the split into the two cheapest single characters
        // even at the shortest spelling (len 2, no letter discount).
        // trace: custom 7.878 / 2^0.2 × 2^0.2 = 7.878 < 的 4.173 + 伊 4.400 = 8.573
        let custom = ec(CUSTOM_EDGE_COST, 2, 2);
        let cheapest_split = ec(E_THE, 1, 1) + ec(I_HE, 1, 1);
        assert!(
            custom < cheapest_split,
            "custom={custom} cheapest_split={cheapest_split}"
        );
        // `taigi`: the custom 台語 beats the real 台 + 語 split.
        // trace: 7.878 / 5^0.2 × 2^0.2 = 6.559 < 8.561 / 3^0.2 + 10.571 / 2^0.2 = 6.872 + 9.203
        let taigi = ec(CUSTOM_EDGE_COST, 2, 5);
        assert!(taigi < ec(TAI, 1, 3) + ec(GI_LANGUAGE, 1, 2));
    }

    #[test]
    fn single_syllable_custom_never_undercuts_the_top_single_char() {
        // S6 Codex pre-impl Q1 regression guard #2: a custom entry is not
        // elevated to a top single character. trace: 7.878 / 1 > 的 4.173
        assert!(ec(CUSTOM_EDGE_COST, 1, 1) > ec(E_THE, 1, 1));
    }

    #[test]
    fn two_single_syllable_custom_lose_to_a_real_phrase_path() {
        // 台灣 (2 syll, len 6) as ONE edge must cost less than two atomic
        // custom edges. trace: 5.752 < 2 × 7.878 / 3^0.2 = 12.648
        let two_single_custom = ec(CUSTOM_EDGE_COST, 1, 3) + ec(CUSTOM_EDGE_COST, 1, 3);
        let real_phrase = ec(TAI_UAN, 2, 6);
        assert!(
            real_phrase < two_single_custom,
            "real_phrase={real_phrase} two_single_custom={two_single_custom}"
        );
    }

    #[test]
    fn syllable_zero_clamps_to_one() {
        // syllable_count is a u8 from a record; a leaked 0 must clamp
        // to 1 on the dict branch (no powf(0)=… surprise, takes the
        // single-syllable user-scale branch identical to syll = 1).
        // The OOV branch ignores syllable_count entirely (RC0:
        // char-keyed penalty), so 0 vs 1 is trivially equal there.
        assert_eq!(
            edge_cost(UNSEEN, 0, 3, 3.5, true),
            edge_cost(UNSEEN, 1, 3, 3.5, true)
        );
        assert_eq!(
            edge_cost(0, 0, 3, 0.0, false),
            edge_cost(0, 1, 3, 0.0, false)
        );
    }

    #[test]
    fn oov_blob_loses_to_intended_best_dict_path() {
        // Earlier dogfood bug (`taiuanta`): the single OOV blob edge (0,8)
        // must cost strictly MORE than the dict-covering path 台灣(0,6) +
        // 焦(6,8). RC0: the blob is khiin's `BIG`-per-char
        // (`8 * OOV_PER_CHAR_PENALTY ≈ 8e10`) — a **cost property**, not a
        // lexicographic `dict_hit` rule (Codex pre-impl S7 Q1 / RC0 Q2).
        let oov_blob = oov(3, 8);
        let dict_path = ec(TAI_UAN, 2, 6) + ec(TA_SCORCH, 1, 2);
        assert!(
            oov_blob > dict_path,
            "oov_blob={oov_blob} must exceed dict_path={dict_path}"
        );
    }

    #[test]
    fn oov_costs_more_than_unseen_dict_for_multi_syllable() {
        // A multi-char OOV span must cost strictly more than an unseen
        // *dict* edge of the same shape — `dict_hit` selects the branch
        // (Codex pre-impl Q2).
        assert!(oov(3, 8) > ec(UNSEEN, 3, 8));
        assert!(oov(3, 8) > ec(u16::MAX, 3, 8));
    }

    #[test]
    fn oov_cost_is_per_char_and_length_independent_of_syllables() {
        // RC0: khiin `segment_min_cost:198-206` prices an uncovered
        // span as `BIG` per advanced char. So OOV cost is exactly
        // `OOV_PER_CHAR_PENALTY * toneless_len`, strictly increasing in
        // char length and INDEPENDENT of syllable count. This is what
        // structurally stops a long OOV blob from ever undercutting a
        // dict-coverable path (RC0).
        assert_eq!(oov(1, 6), OOV_PER_CHAR_PENALTY * 6.0);
        assert_eq!(oov(1, 8), oov(2, 8));
        assert_eq!(oov(2, 8), oov(3, 8));
        assert!(oov(1, 6) < oov(1, 7), "OOV cost must rise per char");
    }

    #[test]
    fn rc0_long_dict_path_beats_whole_buffer_oov_blob_any_length() {
        // RC0 core invariant at the cost level: a dict-covering path
        // of MANY edges (here 7 single-char dict words, the
        // `ginalangtsiahpngbesai`→囡仔人食飯袂使 shape) must stay
        // cheaper than a single whole-buffer OOV blob spanning the same
        // chars — length-independently.
        let blob_chars = 21usize; // ≈ "ginalangtsiahpngbesai"
        let oov_blob = oov(7, blob_chars);
        let dict_path: f64 = (0..7).map(|_| ec(UNSEEN, 1, 3)).sum();
        assert!(
            dict_path < oov_blob,
            "dict_path={dict_path} must stay < oov_blob={oov_blob}"
        );
        // …and a 20-edge sentence of the costliest words still loses to
        // one OOV char.
        let huge_dict: f64 = (0..20).map(|_| ec(u16::MAX, 1, 3)).sum();
        assert!(
            huge_dict < oov(1, 1),
            "even a 20-edge dict path ({huge_dict}) < one OOV char ({})",
            oov(1, 1)
        );
    }

    #[test]
    fn longer_spelling_is_cheaper_at_equal_walker_cost() {
        // khiin letter bias: `/ word_len^0.2`.
        assert!(ec(TAI, 1, 6) < ec(TAI, 1, 3));
    }

    #[test]
    fn user_preference_discounts_multi_syllable_cost() {
        // Below the user-entry cap the discount is exactly `ln(1 + delta)`.
        let neutral = ec(TAI_UAN, 2, 6);
        let preferred = edge_cost(TAI_UAN, 2, 6, 1.0, true);
        assert!((preferred - (neutral - 1.0f64.ln_1p())).abs() < 1e-9);
    }

    // A split the cold rare phrase loses: two common single characters.
    // trace: (6.000 + 6.500) / 3^0.2 = 10.0343
    fn common_split() -> f64 {
        ec(6_000, 1, 3) + ec(6_500, 1, 3)
    }

    #[test]
    fn selected_rare_phrase_is_capped_to_the_user_entry_tier() {
        // trace: cold unseen 2-syll phrase 13.181 / 4^0.2 × 2^0.2 = 11.475 > split 10.034
        let split = common_split();
        let cold = ec(UNSEEN, 2, 4);
        assert!(cold > split, "cold rare phrase loses the split");
        // One fresh pick: δ = BOOST_ALPHA → full tier.
        // trace: 7.878 / 4^0.2 × 2^0.2 − ln(1.1) = 6.858 − 0.095 = 6.763 < 10.034
        let one_fresh = f64::from(ranking::BOOST_ALPHA);
        let selected_once = edge_cost(UNSEEN, 2, 4, one_fresh, true);
        assert!(selected_once < split, "selected={selected_once}");
        let at_tier = ec(CUSTOM_EDGE_COST, 2, 4) - one_fresh.ln_1p();
        assert!((selected_once - at_tier).abs() < 1e-9);
        // A cap, not a jump past a genuinely common phrase: the same delta
        // on a phrase cheaper than the tier is only discounted.
        let below = edge_cost(TAI_UAN, 2, 4, one_fresh, true);
        assert!((below - (ec(TAI_UAN, 2, 4) - one_fresh.ln_1p())).abs() < 1e-9);
        // Single-syllable edges are NOT capped (S3 damping policy).
        assert_eq!(edge_cost(UNSEEN, 1, 3, 4.0, true), ec(UNSEEN, 1, 3));
    }

    #[test]
    fn user_entry_cap_fades_with_the_selection_weight() {
        // Codex post-impl 2026-09-15 P2: a boolean gate never expired. The
        // cap rises as δ decays: a single pick decayed to 5% of BOOST_ALPHA
        // (~90 days at τ = 30 d) still beats the split, at 0.25% (~180
        // days) the cap is above the unseen cost and the corpus order is
        // back; saturated repeat picks (δ = 4) stay at the full tier.
        // trace: cap(0.05) = 7.878 + ln(2001 / 101) = 10.864 → 9.458 − ln(1.005) < 10.034
        //        cap(0.0025) = 7.878 + ln(2001 / 6) = 13.688 > 13.181 → 11.475 − ln(1.00025) > 10.034
        let split = common_split();
        let alpha = f64::from(ranking::BOOST_ALPHA);
        assert!(edge_cost(UNSEEN, 2, 4, alpha * 0.05, true) < split);
        assert!(edge_cost(UNSEEN, 2, 4, alpha * 0.0025, true) > split);
        let saturated_expected = ec(CUSTOM_EDGE_COST, 2, 4) - 4.0f64.ln_1p();
        assert!((edge_cost(UNSEEN, 2, 4, 4.0, true) - saturated_expected).abs() < 1e-9);
    }

    #[test]
    fn user_entry_cap_without_selection_lifts_nothing() {
        // s = 0: cap = 7.878 + ln(2001) = 15.479 nats, above an unseen word.
        assert!((user_entry_cost_cap(0.0) - 15.4794).abs() < 1e-3);
        assert!(user_entry_cost_cap(0.0) > f64::from(UNSEEN) / 1000.0);
    }

    #[test]
    fn single_syllable_user_weight_is_damped_to_zero_discount() {
        // The Q4c BLOCK guard, preserved through the log-space rewrite:
        // a hot single char with a huge decayed delta gets ln(1)=0
        // discount → identical to neutral (SCALE = 0.0).
        assert_eq!(WALKER_SINGLE_SYLLABLE_USER_DELTA_SCALE, 0.0);
        let neutral = ec(E_THE, 1, 1);
        let with_huge_delta = edge_cost(E_THE, 1, 1, 4.0, true);
        assert_eq!(
            with_huge_delta, neutral,
            "single-syllable edge must ignore the user delta at SCALE=0.0"
        );
    }

    #[test]
    fn user_discount_never_raises_cost_on_malformed_delta() {
        // δ is clamped `>= 0`; a (contractually impossible) negative
        // delta must not turn the discount into a penalty.
        let neutral = ec(TAI_UAN, 2, 4);
        assert!((edge_cost(TAI_UAN, 2, 4, -1.0, true) - neutral).abs() < 1e-9);
    }

    #[test]
    fn cost_is_finite_at_walker_cost_and_syllable_extremes() {
        for &(w, s, l) in &[
            (0u16, 0u8, 0usize),
            (0, 1, 1),
            (E_THE, 8, 12),
            (u16::MAX, u8::MAX, 64),
        ] {
            // Both pricing branches must stay finite at the extremes.
            for dict_hit in [true, false] {
                let c = edge_cost(w, s, l, 4.0, dict_hit);
                assert!(c.is_finite(), "w={w} s={s} l={l} dict_hit={dict_hit} → {c}");
            }
        }
    }
}
