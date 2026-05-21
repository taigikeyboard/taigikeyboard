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
/// Recency window for both the legacy additive [`calculate_score`] and
/// the Phase 9.1 lexicographic [`recency_rank`]. An entry is "recent"
/// when `0 <= (now_ms − last_used_ms) < RECENCY_WINDOW_MS`. One hour
/// in epoch-ms.
///
/// **Path divergence (intentional)**: [`recency_rank`] (Phase 9.3a)
/// additionally rejects `now_ms <= 0`, `last_used_ms <= 0`, and
/// clock-skew (`now_ms < last_used_ms`) as stale; [`calculate_score`]
/// (legacy path) keeps its pre-9.3a behaviour and only filters on
/// `last_used_ms > 0`. Tightening the legacy guards would change
/// scoring for the non-Continuous path and is out of scope for this
/// slice (`feedback_round_hygiene.md`).
// 中文: 最近使用判定視窗(1 小時內);Phase 9.1 SortKey 與 legacy 加總公式共用同一閾值。
// 中文: legacy calculate_score 不採用 9.3a 的 clock-invalid guard,以維持 pre-9.3a 行為。
pub const RECENCY_WINDOW_MS: i64 = 60 * 60 * 1000;
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

// ---------------------------------------------------------------------------
// v3.5.8 Phase 9.1 — Continuous-input source rank table.
//
// Lower rank = higher priority in the lexicographic continuous sort_key
// (per docs/roadmap.md § Phase 9 sort_key formula). Distinct from
// legacy `SOURCE_TIERS` above, which encodes additive multiplier
// numerators for the non-continuous `calculate_score` path.
//
// Custom-dictionary entries (PR-9.6) take rank 0 via the `is_custom`
// flag; bitmask-derived ranks start at 1 and mirror the entry order
// of legacy `SOURCE_TIERS`. Drift between this table and
// `dictionary/common/source_bits.py` bit positions is a
// cross-platform invariant violation.
// ---------------------------------------------------------------------------

// 中文: Phase 9.1 連續輸入排序使用的來源 rank 表;rank 越小越優先,custom=0 由 is_custom 旗標進入。
const CONTINUOUS_SOURCE_BITS: &[(u16, u8)] = &[
    (1 << 0, 1), // kautian
    (1 << 1, 2), // taigitv
    (1 << 7, 3), // stti
    (1 << 6, 4), // kungge
];

/// Source rank returned when `bitmask` has no known source bit set.
/// Higher than any explicit-source rank so unknown-source entries
/// sort last on the source dimension.
// 中文: 未命中任何已知來源 bit 時使用的 fallback rank。
pub const CONTINUOUS_DEFAULT_SOURCE_RANK: u8 = 5;

/// Per-selection boost increment for the Continuous-input
/// `user_freq_boost`. Mirrors the additive `0.1` previously hard-coded
/// in [`calculate_continuous_score`]; pinning it as a public constant
/// is the cross-platform invariant axis for PR-9.3a + PR-9.3b/c
/// (`docs/roadmap.md` § Phase 9 跨平台常數表). Platforms MUST NOT
/// redefine — single source of truth per
/// `rules/cross-platform-alignment.md` §3a.
// 中文: Phase 9.3a — 每次使用者選用,boost 增量 0.1;跨平台不可重定義。
pub const BOOST_ALPHA: f32 = 0.1;

/// Saturation ceiling for the Continuous-input `user_freq_boost`.
/// Counts above `(MAX_BOOST − 1) / BOOST_ALPHA = 40` produce the same
/// boost (`5.0`); guards against a single hot entry dominating the
/// candidate list after dozens of selections (stale-dominance defense
/// from `docs/engine/continuous-input-ranking.md` §3.2 Gap B). Same
/// cross-platform invariant policy as [`BOOST_ALPHA`].
// 中文: Phase 9.3a — boost 飽和上限 5×,40 次以上選擇後不再放大,防 stale dominance。
pub const MAX_BOOST: f32 = 5.0;

/// First-match-wins source rank for the Continuous-input sort_key.
/// Returns `0` when `is_custom`, else looks up the first matching
/// bit in `CONTINUOUS_SOURCE_BITS`, else
/// [`CONTINUOUS_DEFAULT_SOURCE_RANK`].
///
/// Cross-platform invariant: this fn is the single source of truth
/// for Continuous-input source ordering. Platform-side ranking code
/// MUST NOT redefine the table; per
/// `rules/cross-platform-alignment.md` §3a.
// 中文: 連續輸入排序的來源 rank;custom=0,字典 bit 依表內順序 1..=4,未知=5。
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
/// stateless. `last_used_ms == 0` means "never used"; the recency bonus
/// gate guards against a stray bonus for never-seen entries.
// 中文: 單一候選詞的使用者頻率資料,last_used_ms == 0 代表沒用過。
#[derive(Debug, Clone, Copy, Default)]
pub struct FrequencyData {
    /// Cumulative selection count. Capped at [`USER_FREQ_CAP`] by the
    /// legacy additive `calculate_score`; Continuous boost uses
    /// [`user_freq_boost`] which has its own saturation via
    /// [`MAX_BOOST`].
    // 中文: 累計被選用次數。
    pub count: i32,
    /// Last selection in epoch-ms. `0` means never used; the recency
    /// helpers treat this and any non-positive value as "never".
    // 中文: 上次使用的 epoch 毫秒,0 代表從未使用。
    pub last_used_ms: i64,
}

/// Per-candidate user-frequency map keyed by `TaigiWord.displayText`
/// (= `hanji` if non-empty else `roman`) — same key as the
/// Continuous-input [`RawCandidate::display_text`] in
/// `lexicon::continuous`. Engine builds this once per request from the
/// proto's `FrequencyEntry` list and reuses it across the whole batch.
///
/// **Duplicate-key policy**: last-write-wins via
/// [`HashMap::insert`]. Platform-side `user_frequency.db` queries
/// SHOULD pre-dedupe by `display_text_key` before sending the
/// `FrequencyEntry[]` snapshot; for legacy callers, duplicates are
/// silently coalesced.
// 中文: 使用者頻率查詢表,key = 候選顯示文字(漢字優先,否則用羅馬字)。
// 中文: 同 key 的多筆 entry 以最後一筆為準(insert 覆寫);平台側建議先 dedupe。
pub type FrequencyMap = std::collections::HashMap<String, FrequencyData>;

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
// 中文: Phase 9.3a — Continuous boost 飽和公式;count=0 → 1.0,>=40 → 5.0。
pub fn user_freq_boost(count: u32) -> f32 {
    let raw = 1.0 + count as f32 * BOOST_ALPHA;
    raw.min(MAX_BOOST)
}

/// v3.5.8 Phase 9.3a — Continuous-input `SortKey.recency_rank` helper.
/// Returns `0` ("recent") when the entry was selected strictly inside
/// the [`RECENCY_WINDOW_MS`] window, else `1` ("stale or never").
///
/// The recency gate is **defensive against three classes of bad clock
/// input**:
/// - `now_ms <= 0` — platform shim did not inject a wall clock (e.g.
///   on engine startup before the first `FetchAtPos` round). All
///   entries fall through to `1` so cold-start does not falsely
///   promote stale entries.
/// - `last_used_ms <= 0` — entry has never been selected; the legacy
///   `calculate_score` uses the same `last_used_ms > 0` guard.
/// - `now_ms < last_used_ms` — clock skew (platform clock moved
///   backwards). Treat as stale rather than recent to avoid
///   non-monotonic ranking.
///
/// Otherwise: `0` when `now_ms − last_used_ms < RECENCY_WINDOW_MS`,
/// else `1`. The strict-less-than boundary matches the inclusive
/// `< RECENCY_WINDOW_MS` policy in `calculate_score` (the boundary
/// is shared so the two scoring paths see the "recent / stale" axis
/// identically).
// 中文: Phase 9.3a — SortKey recency 計算;不合理 now_ms/last_used_ms 一律視為 stale (rank=1)。
pub fn recency_rank(now_ms: i64, last_used_ms: i64) -> u8 {
    if now_ms <= 0 || last_used_ms <= 0 || now_ms < last_used_ms {
        return 1;
    }
    if (now_ms - last_used_ms) < RECENCY_WINDOW_MS {
        0
    } else {
        1
    }
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
// 中文: S3 — walker user-freq 邊權重的指數時間常數 τ;librime formula_d 用 commit tick,
// 中文:   我們無 tick model,改用 now_ms − last_used_ms 牆鐘衰減。weight 在 τ 後降到 1/e,
// 中文:   在 τ·ln2 ≈ 20.8 天降到 0.5(τ=30 天)。dogfood 可調 14..=90 天。
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
/// [`user_freq_boost`] / [`recency_rank`] — engine is the single
/// source of truth, platforms MUST NOT redefine). Returns `0.0`
/// (→ neutral weight `1.0` at the call site) for the **same three
/// bad-clock classes [`recency_rank`] rejects**: `now_ms <= 0` (no
/// wall clock injected), `last_used_ms <= 0` (never selected), and
/// `now_ms < last_used_ms` (clock skew). The walker turns this
/// per-edge delta into a syllable-aware log-space cost discount
/// (single-syllable edges are damped so a hot single character cannot
/// ride the discount to sweep the whole sentence — see
/// `composing::lattice::cost::edge_cost`).
// 中文: S3 — 單條 walker lattice edge 的時間衰減 user-freq boost「delta」(librime formula_d 牆鐘版,收斂 Gap B → G2)。
// 中文: 回傳超過中性 1.0 的量:decay = exp(−age/τ);delta = (user_freq_boost(count) − 1.0) × decay。
// 中文: 關鍵:cap 在衰減「之前」套用(用已飽和的 boost delta 再衰減);先衰減 raw count 再 cap 會讓
// 中文:   count=1000 的 entry 卡在 MAX_BOOST 數月 → 正是本片要消除的 stale 單一 entry dominance(Codex S3 Q4a/Q4c BLOCK)。
// 中文: now_ms/last_used_ms 由 caller 注入(pure/stateless,與 user_freq_boost/recency_rank 同跨平台不可重定義契約);
// 中文:   三類壞時鐘(now<=0 / last<=0 / skew)回 0.0 = 中性,與 recency_rank guard 一致。
pub fn decayed_user_weight_delta(count: u32, now_ms: i64, last_used_ms: i64) -> f64 {
    // Same bad-clock guard policy as `recency_rank` — single source of
    // truth for "is this user-frequency timestamp usable".
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

/// Build a [`FrequencyMap`] from the proto-wire `FrequencyEntry[]`.
/// Single source of truth for both the legacy
/// [`process::process_candidates`] path and the Continuous-input
/// dispatcher in `composing/src/dispatch.rs::handle_fetch_at_pos`.
///
/// `entry.count: u32` is the wire type; the in-memory [`FrequencyData`]
/// keeps `count: i32` for compatibility with the legacy
/// [`calculate_score`] additive formula (which caps at [`USER_FREQ_CAP`]).
/// `u32::MAX > i32::MAX` so we saturate on conversion to prevent wrap.
///
/// **Continuous path note**: the boost helper [`user_freq_boost`] takes
/// `u32`, so callers re-widen `FrequencyData.count` back via
/// `u32::try_from(..).unwrap_or(0)` before applying the boost — the
/// `u32 → i32` saturation is a no-op for any realistic platform count
/// (selections are bounded by user actions), and the boost itself
/// saturates at [`MAX_BOOST`] regardless of the converted count's
/// magnitude. See `engine/lexicon/src/continuous.rs::record_to_candidate`.
// 中文: 從 proto FrequencyEntry[] 建查詢表;count u32 → i32 用 saturate 防 wrap。
// 中文: Continuous boost 路徑會再 i32 → u32 (saturate to 0) 回轉,實務 count 永遠 < i32::MAX,飽和不會發生。
pub fn build_frequency_map(entries: &[protos::engine::FrequencyEntry]) -> FrequencyMap {
    let mut map = FrequencyMap::with_capacity(entries.len());
    for entry in entries {
        map.insert(
            entry.display_text_key.clone(),
            FrequencyData {
                count: i32::try_from(entry.count).unwrap_or(i32::MAX),
                last_used_ms: entry.last_used_ms,
            },
        );
    }
    map
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
/// `lexicon::continuous::fetch_candidates_for_keys` (the production
/// span-local entry; `fetch_candidates_for_endings` is the test-only
/// wrapper after v3.5.9 D7+D8 #306), where the per-syllable bias
/// rewards multi-syllable words like `珠仔(syll=2)` over `紙/珠(syll=1)`
/// when the user's input spans a multi-syllable reach.
///
/// `user_freq_boost` is caller-supplied so this fn stays stateless;
/// callers compose it from their own user-frequency store
/// (`user_frequency.db` on the platform side, see
/// `feedback_user_data_sqlite_stays_native`). `1.0` = no boost. Caller
/// MUST pass a finite, non-negative `f32` — this fn does no clamping
/// (it is a pure pricing formula). The downstream sort comparator in
/// `lexicon::continuous::fetch_candidates_for_keys` defends against
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
    let syll_bias = 1.0 + BOOST_ALPHA * f32::from(syllable_count.saturating_sub(1));
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

    // -----------------------------------------------------------------------
    // v3.5.8 Phase 9.1 — source_tier_rank
    // -----------------------------------------------------------------------

    #[test]
    fn source_tier_rank_is_custom_short_circuits() {
        // `is_custom=true` overrides any bitmask content with rank 0.
        assert_eq!(source_tier_rank(0, true), 0);
        assert_eq!(source_tier_rank(KAUTIAN_BIT as u16, true), 0);
        assert_eq!(source_tier_rank(u16::MAX, true), 0);
    }

    #[test]
    fn source_tier_rank_first_match_wins_in_table_order() {
        // Table order: kautian(1) → taigitv(2) → stti(3) → kungge(4).
        // Overlapping bits should resolve to the lowest rank present.
        assert_eq!(source_tier_rank(KAUTIAN_BIT as u16, false), 1);
        assert_eq!(source_tier_rank(TAIGITV_BIT as u16, false), 2);
        assert_eq!(source_tier_rank(STTI_BIT as u16, false), 3);
        assert_eq!(source_tier_rank(KUNGGE_BIT as u16, false), 4);
        // Kautian + kungge → kautian (entry-order first match).
        assert_eq!(
            source_tier_rank((KAUTIAN_BIT | KUNGGE_BIT) as u16, false),
            1
        );
    }

    // -----------------------------------------------------------------------
    // v3.5.8 Phase 9.3a — user_freq_boost + recency_rank
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
    fn recency_rank_zero_when_inside_window() {
        // Strict less-than boundary; `now_ms - last_used_ms == window - 1`
        // is still recent, `== window` flips to stale.
        let last = 1_000_000_000_i64;
        assert_eq!(recency_rank(last + RECENCY_WINDOW_MS - 1, last), 0);
        assert_eq!(recency_rank(last + 1, last), 0);
    }

    #[test]
    fn recency_rank_one_at_or_outside_window() {
        let last = 1_000_000_000_i64;
        // Inclusive boundary at the window edge counts as stale.
        assert_eq!(recency_rank(last + RECENCY_WINDOW_MS, last), 1);
        // Anything older is also stale.
        assert_eq!(recency_rank(last + 2 * RECENCY_WINDOW_MS, last), 1);
    }

    #[test]
    fn recency_rank_one_when_now_ms_is_non_positive() {
        // Phase 9.3a Codex risk #1: `now_ms = 0` from a platform that
        // has not injected the wall clock must NOT falsely promote
        // stale entries to recent. All entries fall through to rank 1.
        let last = 1_000_000_000_i64;
        assert_eq!(recency_rank(0, last), 1);
        assert_eq!(recency_rank(-1, last), 1);
        assert_eq!(recency_rank(i64::MIN, last), 1);
    }

    #[test]
    fn recency_rank_one_when_last_used_ms_is_non_positive() {
        // `last_used_ms = 0` = never selected. Even with a valid
        // wall-clock `now_ms`, an unused entry stays stale.
        let now = 1_000_000_000_i64;
        assert_eq!(recency_rank(now, 0), 1);
        assert_eq!(recency_rank(now, -1), 1);
    }

    #[test]
    fn recency_rank_one_under_clock_skew() {
        // Clock skew defense: a platform clock that moved backwards
        // (so `now_ms < last_used_ms`) must still produce rank 1, not
        // wrap a negative delta into the comparison.
        let last = 1_000_000_000_i64;
        let now = last - 5_000; // 5s earlier than the recorded selection.
        assert_eq!(recency_rank(now, last), 1);
        // Far backwards skew also handled.
        assert_eq!(recency_rank(0_i64.wrapping_sub(1), last), 1);
    }

    #[test]
    fn build_frequency_map_dedupes_duplicate_keys_last_write_wins() {
        // Codex pre-impl risk: platform-side `user_frequency.db`
        // sometimes ships duplicate `display_text_key` rows. The map
        // must coalesce silently (later entry wins).
        let entries = vec![
            protos::engine::FrequencyEntry {
                display_text_key: "台".to_owned(),
                count: 1,
                last_used_ms: 100,
            },
            protos::engine::FrequencyEntry {
                display_text_key: "台".to_owned(),
                count: 7,
                last_used_ms: 700,
            },
        ];
        let map = build_frequency_map(&entries);
        assert_eq!(map.len(), 1);
        let data = map.get("台").expect("台 present");
        assert_eq!(data.count, 7);
        assert_eq!(data.last_used_ms, 700);
    }

    #[test]
    fn build_frequency_map_saturates_count_beyond_i32_max() {
        // Wire `count: u32` is wider than the in-memory `count: i32`.
        // Saturate at conversion to avoid sign-flip wraparound on the
        // legacy additive formula path. The Continuous boost path is
        // unaffected (it consumes u32 directly).
        let entries = vec![protos::engine::FrequencyEntry {
            display_text_key: "x".to_owned(),
            count: u32::MAX,
            last_used_ms: 1,
        }];
        let map = build_frequency_map(&entries);
        assert_eq!(map.get("x").unwrap().count, i32::MAX);
    }

    #[test]
    fn source_tier_rank_falls_back_to_default_when_no_known_bit() {
        // Bits the table does not enumerate (e.g., itaigi=2, dev=10,
        // khiin=9, variant=12) all fall through to the default rank.
        assert_eq!(
            source_tier_rank(ITAIGI_BIT as u16, false),
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
    fn decayed_delta_bad_clock_classes_match_recency_rank_policy() {
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
