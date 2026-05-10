//! Per-candidate ranking score.
//!
//! Pure-CPU formula; the v3.5.2 ranking slice collapsed
//! `CandidateProcessor.calculateScore` from both platforms into this
//! crate (Android mirror deleted PR #192). Constants verified
//! byte-identical at audit time (`docs/engine/ranking-slice-audit.md`
//! § 1.3) and are pinned here as the single source of truth.
//!
//! ```text
//! total = userFreqScore
//!       + recencyBonus
//!       + exactBonus
//!       + completionPenalty
//!       + closenessBonus
//!       + baseFreqScore
//! ```
//!
//! - `userFreqScore` dominates (cap 100 hits × weight 100 → max 10_000).
//! - `recencyBonus` (200) fires when the user's last selection was
//!   strictly less than 1 hour ago (`< RECENCY_WINDOW_MS`).
//! - `exactBonus` (100) and `completionPenalty` (-1000) separate exact
//!   matches from completions in the cold-start tail.
//! - `closenessBonus` (0..=500) rewards length match between candidate
//!   base and input base.
//! - `baseFreqScore` is the dictionary-frequency proxy times the
//!   first-match `SOURCE_TIERS` multiplier.

// 中文: 候選詞評分公式,六項加權加總(使用者頻率、最近使用、完全相符、補全懲罰、長度接近度、字典分數)。

use protos::engine::{ScoreBreakdown, TaigiWord};

use phonetics::taigi_unicode_base_form;

// ---------------------------------------------------------------------------
// Cross-platform invariant constants — single source of truth for both
// platforms (Android mirror deleted PR #192; iOS residual unrelated).
// Pinned by `docs/engine/ranking-slice-audit.md` § 1.3.
// ---------------------------------------------------------------------------

// 中文: 使用者頻率次數上限。
const USER_FREQ_CAP: i32 = 100;
// 中文: 使用者頻率每次的加權倍率。
const USER_FREQ_WEIGHT: i32 = 100;
// 中文: 最近使用判定視窗(1 小時內);超過則不給最近使用加分。
const RECENCY_WINDOW_MS: i64 = 60 * 60 * 1000;
// 中文: 最近使用加分。
const RECENCY_BONUS: i32 = 200;
// 中文: 完全相符加分。
const EXACT_BONUS: i32 = 100;
// 中文: 補全(非完全相符)懲罰分數。
const COMPLETION_PENALTY: i32 = -1000;
// 中文: 長度接近度的最大加分上限。
const CLOSENESS_WEIGHT: i32 = 500;
// 中文: 字典 length_score 的縮放除數。
const BASE_FREQ_DIVISOR: i32 = 10;
// 中文: 字典來源 tier 倍率分母。
const TIER_DENOMINATOR: i32 = 10;
// 中文: 沒有命中任何 tier 時使用的預設分子。
const DEFAULT_TIER_NUMERATOR: i32 = 10;

/// Maps a dictionary-source bit position to the `baseFreqScore` multiplier
/// numerator. First-match-wins on overlapping bits — the entry order here
/// is authoritative. Mirrors `dictionary/common/source_bits.py`.
// 中文: 字典來源優先權表,bit 位對應 base_freq_score 的倍率分子;多 bit 命中時依此處順序取第一個。
const SOURCE_TIERS: &[(u32, i32)] = &[
    (0, 15), // kautian
    (1, 13), // taigitv
    (7, 12), // stti
    (6, 11), // kungge
];

/// Per-candidate user-frequency snapshot. Caller-supplied so engine stays
/// stateless. `last_used_ms == 0` means "never used"; the recency bonus
/// gate guards against a stray bonus for never-seen entries.
// 中文: 單一候選詞的使用者頻率資料,last_used_ms == 0 代表沒用過。
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct FrequencyData {
    // 中文: 累計被選用次數。
    pub(crate) count: i32,
    // 中文: 上次使用的 epoch 毫秒,0 代表從未使用。
    pub(crate) last_used_ms: i64,
}

/// Compute the score breakdown for a single candidate.
// 中文: 計算單一候選詞的六項分數明細(ScoreBreakdown)。
pub(crate) fn calculate_score(
    word: &TaigiWord,
    normalized_input: &str,
    freq: FrequencyData,
    now_ms: i64,
) -> ScoreBreakdown {
    let candidate_base = roman_to_base(&word.roman);
    let input_base = input_to_base(normalized_input);

    let capped_user_freq = freq.count.min(USER_FREQ_CAP);
    let user_freq_score = capped_user_freq * USER_FREQ_WEIGHT;

    let recency_bonus = if freq.last_used_ms > 0 && (now_ms - freq.last_used_ms) < RECENCY_WINDOW_MS
    {
        RECENCY_BONUS
    } else {
        0
    };

    let exact_bonus = if candidate_base == input_base {
        EXACT_BONUS
    } else {
        0
    };
    let completion_penalty = if candidate_base == input_base {
        0
    } else {
        COMPLETION_PENALTY
    };

    let input_len = (input_base.chars().count() as i32).max(1);
    let candidate_len = (candidate_base.chars().count() as i32).max(1);
    let match_ratio =
        f64::from(input_len.min(candidate_len)) / f64::from(input_len.max(candidate_len));
    let closeness_bonus = (match_ratio * f64::from(CLOSENESS_WEIGHT)) as i32;

    let raw_base = word.length_score.unwrap_or(0) / BASE_FREQ_DIVISOR;
    let base_freq_score = raw_base * tier_numerator(word.source_bitmask) / TIER_DENOMINATOR;

    ScoreBreakdown {
        user_freq_score,
        recency_bonus,
        exact_bonus,
        completion_penalty,
        closeness_bonus,
        base_freq_score,
    }
}

/// Sum of all six fields — mirrors `ScoreBreakdown.total` accessor on
/// both platforms. Internal helper; engine returns the breakdown and lets
/// the platform / sort layer compute totals as needed.
// 中文: 六項分數加總取得最終總分,行為對齊 iOS/Android 的 ScoreBreakdown.total。
#[inline]
pub(crate) fn total(breakdown: &ScoreBreakdown) -> i32 {
    breakdown.user_freq_score
        + breakdown.recency_bonus
        + breakdown.exact_bonus
        + breakdown.completion_penalty
        + breakdown.closeness_bonus
        + breakdown.base_freq_score
}

/// v3.5.8 連續輸入 (Continuous Input) Phase 5 score formula:
///
/// `score = freq × (1.0 + 0.1 × max(0, syllable_count − 1)) × user_freq_boost`
///
/// Pure-multiplicative `f32`, no bigram, no recency / exact / closeness /
/// tier components — those live in the additive [`calculate_score`]
/// pipeline and apply only to the legacy single-segment IME path. The
/// Continuous slice ranks span-local candidates emitted by
/// `lexicon::continuous::fetch_candidates_for_endings`, where the
/// per-syllable bias rewards multi-syllable words like `珠仔(syll=2)`
/// over `紙/珠(syll=1)` when the user's input spans a multi-syllable
/// reach.
///
/// `user_freq_boost` is caller-supplied so this fn stays stateless;
/// callers compose it from their own user-frequency store
/// (`user_frequency.db` on the platform side, see
/// `feedback_user_data_sqlite_stays_native`). `1.0` = no boost. Caller
/// MUST pass a finite, non-negative `f32` — this fn does no clamping
/// (it is a pure pricing formula). The downstream sort comparator in
/// `lexicon::continuous::fetch_candidates_for_endings` defends against
/// `NaN` leakage by coercing it to `f32::MIN`, but negative or `+∞`
/// boosts will produce semantically nonsensical rankings.
///
/// Cited mainstream IME parallel: khiin-rs `khiin/src/data/segmenter.rs`
/// uses `cost = ln(1/p) / word_len_bias × syllable_bias`. We pick a
/// simpler multiplicative form per `docs/roadmap.md:460`.
// 中文: v3.5.8 連續輸入 Phase 5 排序公式:freq × (1 + 0.1×(syll−1)) × user_freq_boost。
// 中文: 純 f32 倍乘式,不接 bigram / recency / closeness;與既有 calculate_score 不重疊。
// 中文: user_freq_boost 由呼叫端注入 (傳 1.0 即無 boost),保持本函式無狀態。
pub fn calculate_continuous_score(freq: u32, syllable_count: u8, user_freq_boost: f32) -> f32 {
    let syll_bias = 1.0 + 0.1 * f32::from(syllable_count.saturating_sub(1));
    freq as f32 * syll_bias * user_freq_boost
}

/// Strip roman to a comparison base form: drop hyphens, drop ASCII space,
/// preprocess Taigi-specific Unicode (POJ nasal markers + `o͘`), drop
/// combining marks (Unicode `Mn`), drop decimal digits (Unicode `Nd`),
/// lowercase. Used by both candidate side and (transitively via
/// [`input_to_base`]) the input side of the exact / completion
/// comparison.
///
/// CROSS-PLATFORM INVARIANT (pre-Path-G platform parity contract): the
/// pre-PR-#192 Android `CandidateProcessor.romanToBase` used Kotlin
/// `Character.NON_SPACING_MARK` + `Char.isDigit`, and the original iOS
/// mirror (now deleted save residual) used Swift
/// `Unicode.Scalar.Properties.generalCategory == .nonspacingMark` +
/// `Character.isNumber`. Rust uses strict
/// `GeneralCategory::NonspacingMark` (`Mn`) + `DecimalNumber` (`Nd`) —
/// Nd ⊂ Swift's `isNumber` (Nd ∪ Nl ∪ No), Nd == Kotlin's `isDigit`.
/// For the actual Taigi input space (Latin + POJ/TL diacritics + ASCII
/// tone digits 1-9) all three predicates produced byte-identical output;
/// the strict Nd choice retains alignment with the narrower historical
/// Kotlin contract while remaining a subset of Swift's predicate.
// 中文: 把候選羅馬字轉成比對用的 base 形式:去連字號/空白、處理 POJ 鼻音與 o͘、去 Mn/Nd、轉小寫。
fn roman_to_base(roman: &str) -> String {
    let no_hyphens: String = roman.chars().filter(|c| *c != '-' && *c != ' ').collect();
    let with_oo = taigi_unicode_base_form(&no_hyphens);
    with_oo
        .chars()
        .filter(|c| !is_nonspacing_mark(*c))
        .filter(|c| !is_decimal_digit(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

/// Strip tone digits from a normalized input and lowercase. Replaces the
/// historical platform `CandidateProcessor.inputToBase`. Note: input is
/// assumed already in numeric tone form (no diacritics) — the digit
/// filter catches the trailing tone digit (`tai5tsi3` → `taitsi`). Uses
/// the same strict `Nd` predicate as [`roman_to_base`] for parity with
/// the pre-Path-G platform contract.
// 中文: 把使用者輸入轉成比對用 base:輸入已是數字調(無變音符號),只去尾調數字並轉小寫。
fn input_to_base(normalized_input: &str) -> String {
    normalized_input
        .chars()
        .filter(|c| !is_decimal_digit(*c))
        .flat_map(char::to_lowercase)
        .collect()
}

/// First-match-wins multiplier numerator. Returns the default when
/// `bitmask` is `None` or no `SOURCE_TIERS` entry's bit is set.
// 中文: 依 source_bitmask 取得字典 tier 倍率分子;沒命中或 None 時回傳預設值。
fn tier_numerator(bitmask: Option<u32>) -> i32 {
    let Some(bits) = bitmask else {
        return DEFAULT_TIER_NUMERATOR;
    };
    for (bit, numerator) in SOURCE_TIERS {
        if bits & (1 << bit) != 0 {
            return *numerator;
        }
    }
    DEFAULT_TIER_NUMERATOR
}

/// True if `c`'s general category is `Mn` (non-spacing mark) — exact
/// match for Swift's
/// `Unicode.Scalar.Properties.generalCategory == .nonspacingMark` and
/// Kotlin's `Character.getType(c) == Character.NON_SPACING_MARK`. The
/// combining tone marks (CCC 230) and `\u{0358}` (CCC 232) post-NFD all
/// fall under this category. Strict `Mn` (not `CCC > 0`) is required
/// because `Mc` / `Me` marks and the rare `Mn` characters with
/// `CCC == 0` would otherwise diverge from the platform predicate.
// 中文: 嚴格用 General Category Mn 判定非間隔符號,行為與 iOS/Android 對齊。
fn is_nonspacing_mark(c: char) -> bool {
    use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};
    c.general_category() == GeneralCategory::NonspacingMark
}

/// True if `c`'s general category is `Nd` (decimal digit). Mirrors
/// Kotlin's `Character.isDigit(c)` exactly and is a subset of Swift's
/// `Character.isNumber` (which also includes `Nl` and `No`); the input
/// space here is restricted to ASCII tone digits, so the predicates
/// agree on every reachable input.
// 中文: 嚴格用 General Category Nd 判定十進位數字,對齊 Kotlin Char.isDigit。
fn is_decimal_digit(c: char) -> bool {
    use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};
    c.general_category() == GeneralCategory::DecimalNumber
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(roman: &str, length_score: Option<i32>, source_bitmask: Option<u32>) -> TaigiWord {
        TaigiWord {
            id: 0,
            roman: roman.to_owned(),
            hanji: None,
            length_score,
            source_bitmask,
        }
    }

    fn freq(count: i32, last_used_ms: i64) -> FrequencyData {
        FrequencyData {
            count,
            last_used_ms,
        }
    }

    // -----------------------------------------------------------------------
    // §6 INVARIANT — score determinism + ordering
    // -----------------------------------------------------------------------

    #[test]
    fn invariant_score_is_deterministic() {
        let w = word("gua", Some(50), None);
        let f = freq(2, 1_000_000);
        let now = 5_000_000;
        let a = calculate_score(&w, "gua", f, now);
        let b = calculate_score(&w, "gua", f, now);
        assert_eq!(a, b);
    }

    #[test]
    fn invariant_user_freq_dominates_ranking() {
        let user_word = word("gua", Some(10), None);
        let dict_word = word("gua", Some(100), None);
        let user_freq = freq(1, 0);
        let cold_freq = FrequencyData::default();
        let now = 10_000_000_000;

        let user_score = total(&calculate_score(&user_word, "gua", user_freq, now));
        let dict_score = total(&calculate_score(&dict_word, "gua", cold_freq, now));
        assert!(
            user_score > dict_score,
            "user entry must outrank dict entry (user={user_score}, dict={dict_score})"
        );
    }

    #[test]
    fn invariant_completion_penalty_separates_tiers() {
        let exact_w = word("gua", Some(50), None);
        let completion_w = word("guan", Some(50), None);
        let f = FrequencyData::default();
        let now = 10_000_000_000;

        let exact = calculate_score(&exact_w, "gua", f, now);
        let completion = calculate_score(&completion_w, "gua", f, now);

        assert_eq!(exact.exact_bonus, EXACT_BONUS);
        assert_eq!(completion.exact_bonus, 0);
        assert_eq!(completion.completion_penalty, COMPLETION_PENALTY);
        assert_eq!(exact.completion_penalty, 0);
        assert!(total(&exact) > total(&completion));
    }

    #[test]
    fn invariant_recency_window_is_exactly_1_hour() {
        let w = word("gua", Some(50), None);
        let inside = freq(0, 1);
        let just_inside_now = 1 + (RECENCY_WINDOW_MS - 1);
        let on_boundary_now = 1 + RECENCY_WINDOW_MS;
        let past_now = 1 + RECENCY_WINDOW_MS + 1;

        assert_eq!(
            calculate_score(&w, "gua", inside, just_inside_now).recency_bonus,
            RECENCY_BONUS
        );
        assert_eq!(
            calculate_score(&w, "gua", inside, on_boundary_now).recency_bonus,
            0
        );
        assert_eq!(
            calculate_score(&w, "gua", inside, past_now).recency_bonus,
            0
        );

        let never = FrequencyData::default();
        assert_eq!(
            calculate_score(&w, "gua", never, just_inside_now).recency_bonus,
            0
        );
    }

    #[test]
    fn invariant_roman_to_base_strips_tones_hyphens_digits() {
        let cases = &[
            ("tâi-gí", "taigi"),
            ("tâi gí", "taigi"),
            ("gua2", "gua"),
            ("GUA2", "gua"),
            ("h\u{00F3}\u{0358}", "hoo"),
            ("sa\u{207F}", "sann"),
            ("", ""),
        ];
        for (input, expected) in cases {
            assert_eq!(roman_to_base(input), *expected, "input='{input}'");
        }
    }

    #[test]
    fn input_to_base_strips_digits_and_lowercases() {
        assert_eq!(input_to_base("tai5tsi3"), "taitsi");
        assert_eq!(input_to_base("taitsi"), "taitsi");
        assert_eq!(input_to_base("GUA2"), "gua");
        assert_eq!(input_to_base(""), "");
    }

    /// Strict `Mn` (general category) parity check vs the simpler
    /// `CCC > 0` heuristic. Pins the predicate to the platform contract:
    /// non-Mn combining marks (`Mc`, `Me`) must NOT be stripped, and Mn
    /// marks must always be stripped regardless of CCC. Catches future
    /// drift if the implementation slips back to a CCC-only check.
    #[test]
    fn roman_to_base_uses_strict_mn_general_category() {
        // U+1885 MONGOLIAN LETTER ALI GALI BALUDA — General Category = Mn
        // with Canonical_Combining_Class = 0. A `CCC > 0` check would
        // KEEP this character; the strict Mn predicate must STRIP it.
        let with_mn_ccc0 = "ab\u{1885}c";
        assert_eq!(
            roman_to_base(with_mn_ccc0),
            "abc",
            "Mn with CCC==0 must still be stripped (strict Mn check)"
        );

        // U+0903 DEVANAGARI SIGN VISARGA — General Category = Mc
        // (spacing mark). Platform Mn-only filters keep it; the engine
        // must too. A naive `CCC > 0` filter would strip it.
        let with_mc = "ab\u{0903}c";
        assert_eq!(
            roman_to_base(with_mc),
            "ab\u{0903}c",
            "Mc spacing marks must NOT be stripped (strict Mn check)"
        );
    }

    /// Strict `Nd` parity check vs the original `is_ascii_digit`
    /// heuristic. Pins the digit predicate to Kotlin `Char.isDigit`
    /// (Unicode Decimal_Number) — non-ASCII Nd inputs must be stripped
    /// just like ASCII tone digits. Drift here would silently keep
    /// candidate-side characters that the platform helpers strip.
    #[test]
    fn roman_to_base_strips_non_ascii_decimal_digits() {
        // U+0967 DEVANAGARI DIGIT ONE — General Category = Nd. ASCII-only
        // digit filter would KEEP this; the strict Nd predicate must
        // STRIP it.
        assert_eq!(
            roman_to_base("ab\u{0967}c"),
            "abc",
            "non-ASCII Nd must be stripped to match Kotlin Char.isDigit"
        );
        assert_eq!(
            input_to_base("ab\u{0967}c"),
            "abc",
            "input_to_base must use the same Nd predicate as roman_to_base"
        );
    }

    // -----------------------------------------------------------------------
    // §6 INVARIANT — tier bonus
    // -----------------------------------------------------------------------

    const KAUTIAN_BIT: u32 = 1 << 0;
    const TAIGITV_BIT: u32 = 1 << 1;
    const ITAIGI_BIT: u32 = 1 << 2;
    const KUNGGE_BIT: u32 = 1 << 6;
    const STTI_BIT: u32 = 1 << 7;

    #[test]
    fn invariant_tier_bonus_preserves_frequency_ordering() {
        let kautian_word = word("x", Some(100), Some(KAUTIAN_BIT));
        let default_word = word("x", Some(160), Some(ITAIGI_BIT));
        let now = 0;
        let kautian = calculate_score(&kautian_word, "x", FrequencyData::default(), now);
        let default = calculate_score(&default_word, "x", FrequencyData::default(), now);
        assert_eq!(kautian.base_freq_score, 15);
        assert_eq!(default.base_freq_score, 16);
        assert!(default.base_freq_score > kautian.base_freq_score);
    }

    #[test]
    fn invariant_tier_bonus_first_match_wins() {
        let both = word("x", Some(100), Some(KAUTIAN_BIT | KUNGGE_BIT));
        let kautian_only = word("x", Some(100), Some(KAUTIAN_BIT));
        let kungge_only = word("x", Some(100), Some(KUNGGE_BIT));
        let now = 0;
        let f = FrequencyData::default();
        let both_s = calculate_score(&both, "x", f, now);
        let kautian_s = calculate_score(&kautian_only, "x", f, now);
        let kungge_s = calculate_score(&kungge_only, "x", f, now);
        assert_eq!(both_s.base_freq_score, kautian_s.base_freq_score);
        assert!(both_s.base_freq_score > kungge_s.base_freq_score);
    }

    #[test]
    fn tier_numerator_table_first_match_wins() {
        assert_eq!(tier_numerator(Some(KAUTIAN_BIT)), 15);
        assert_eq!(tier_numerator(Some(TAIGITV_BIT)), 13);
        assert_eq!(tier_numerator(Some(STTI_BIT)), 12);
        assert_eq!(tier_numerator(Some(KUNGGE_BIT)), 11);
        // kautian + kungge → kautian (entry-order first-match)
        assert_eq!(tier_numerator(Some(KAUTIAN_BIT | KUNGGE_BIT)), 15);
        // No tier bit set
        assert_eq!(tier_numerator(Some(ITAIGI_BIT)), 10);
        // None bitmask
        assert_eq!(tier_numerator(None), 10);
    }

    #[test]
    fn closeness_bonus_caps_at_500_on_exact_length_match() {
        let w = word("aaa", Some(0), None);
        let s = calculate_score(&w, "bbb", FrequencyData::default(), 0);
        assert_eq!(s.closeness_bonus, 500);
    }

    #[test]
    fn closeness_bonus_decays_with_length_mismatch() {
        let w = word("aaaa", Some(0), None);
        let s = calculate_score(&w, "aa", FrequencyData::default(), 0);
        // ratio = 2/4 → 0.5 → 250
        assert_eq!(s.closeness_bonus, 250);
    }

    // -----------------------------------------------------------------------
    // v3.5.8 Phase 5 — calculate_continuous_score
    // -----------------------------------------------------------------------

    #[test]
    fn continuous_score_single_syllable_baseline() {
        // syll=1 → bias = 1.0; user_freq_boost = 1.0 → score == freq.
        assert_eq!(calculate_continuous_score(100, 1, 1.0), 100.0);
        assert_eq!(calculate_continuous_score(0, 1, 1.0), 0.0);
    }

    #[test]
    fn continuous_score_syllable_bias_increments_by_ten_percent() {
        // freq = 100, boost = 1.0:
        //   syll=1 → 100.0
        //   syll=2 → 110.0
        //   syll=3 → 120.0
        //   syll=4 → 130.0
        assert_eq!(calculate_continuous_score(100, 1, 1.0), 100.0);
        assert_eq!(calculate_continuous_score(100, 2, 1.0), 110.0);
        assert!((calculate_continuous_score(100, 3, 1.0) - 120.0).abs() < 1e-4);
        assert_eq!(calculate_continuous_score(100, 4, 1.0), 130.0);
    }

    #[test]
    fn continuous_score_user_freq_boost_is_multiplicative() {
        // boost = 2.0 doubles the result regardless of syllable count.
        assert_eq!(calculate_continuous_score(100, 1, 2.0), 200.0);
        assert_eq!(calculate_continuous_score(100, 2, 2.0), 220.0);
        // boost = 0.0 zeroes everything (cold-start sentinel for tests).
        assert_eq!(calculate_continuous_score(100, 2, 0.0), 0.0);
    }

    #[test]
    fn continuous_score_syll_zero_does_not_underflow() {
        // syllable_count = 0 must clamp to bias = 1.0 (saturating_sub(1)).
        // Defensive: builder caps at 1..=4 (`MAX_SYLLABLES`), but the FFI
        // contract is u8 so a zero could leak in; should not panic / wrap.
        assert_eq!(calculate_continuous_score(50, 0, 1.0), 50.0);
    }

    #[test]
    fn continuous_score_multi_syllable_outranks_single_when_freq_equal() {
        // The whole point of the syll bias: 珠仔(syll=2) outranks 紙(syll=1)
        // at equal dictionary frequency, so multi-syll candidates surface.
        let single = calculate_continuous_score(100, 1, 1.0);
        let pair = calculate_continuous_score(100, 2, 1.0);
        let quad = calculate_continuous_score(100, 4, 1.0);
        assert!(pair > single);
        assert!(quad > pair);
    }
}
