//! v3.5.8 連續輸入 (Continuous Input) Phase 5 — span-local candidate fetch.
//!
//! Given a **canonical-TL ASCII** input, a starting byte position, and a
//! list of valid syllable-end byte offsets (produced by
//! `composing::syllabifier::tl::valid_span_endings`), return all
//! dictionary candidates whose toneless TL key equals `input[pos..end]`
//! for some `end` in `endings`. Each candidate is scored via
//! [`ranking::calculate_continuous_score`] and tagged with the consumed
//! byte span and syllable count so the UI can decide what to commit.
//!
//! # Why span-local lookup
//!
//! Pure longest-match (khiin-rs `khiin/src/data/segmenter.rs:122`) would
//! commit `tsua` to span 4 and lose the `珠 (tsu, span=3)` candidate.
//! Global lattice (librime `src/rime/algo/syllabifier.cc`) is over-built
//! for our scope. Multi-cut span-local fetch is the canonical middle
//! ground per `docs/roadmap.md:203`.
//!
//! Multi-syllable candidates (e.g. `珠仔`) and single-syllable candidates
//! (e.g. `紙`) under the same toneless key (`tl:tsua`) are distinguished
//! by `DictionaryRecord::syllable_count`, which the v2 dict.bin layout
//! carries since Phase 1 (`engine/lexicon/src/dictionary_reader.rs:38-51`).
//!
//! # Contracts
//!
//! - **`input` MUST be canonical TL ASCII** (lowercase or mixed-case).
//!   POJ → TL canonicalization happens upstream via
//!   `phonetics::canonicalize_syllable`. **TPS Bopomofo input is NOT
//!   directly accepted here** — `tps::valid_span_endings` operates on
//!   the raw Bopomofo string and produces TPS byte offsets, which would
//!   not form valid `tl:` FST keys. Phase 6 will introduce the TPS →
//!   TL key mapping at the dispatch boundary; this module remains TL-
//!   only.
//! - `endings` SHOULD be ascending UTF-8 char boundaries within
//!   `input[pos..]`. Out-of-range or non-boundary endings are silently
//!   skipped (matches the syllabifier's safe contract).
//! - `enabled_sources_bitmask` follows the same wire format as
//!   `lexicon::search()` — bit 12 = variant gate, bit 9 = khiin gate,
//!   bits 0..=11 = per-source enables, `u32::MAX` = all sources on.
//! - `user_freq_boost` is multiplicative (`1.0` = no boost). Caller MUST
//!   pass a finite, non-negative `f32`. Non-finite (`NaN` / `±∞`) values
//!   are coerced to `0.0` so the descending-score ordering guarantee
//!   holds (see `fetch_candidates_for_endings` impl). Caller composes
//!   the boost from the platform-side `user_frequency.db` (see
//!   `feedback_user_data_sqlite_stays_native`).
//!
//! # Ordering
//!
//! Returned candidates are sorted by `score` descending; ties keep
//! insertion order (first by `endings` order, then by `prefix_index`
//! rowid order — `lookup_exact` is deterministic per build). The
//! comparator coerces `NaN` scores to `-∞` so even a contract-violating
//! caller cannot break the ordering invariant.
//!
//! # Phase 6 boundary
//!
//! Phase 6 added the proto request / response carriers
//! (`ContinuousResponse` / `CandidateMessage` in `composing.proto`) and
//! the dispatch wiring (`composing/src/dispatch.rs::handle_fetch_at_pos`),
//! plus the TPS → TL key mapping that runs at the dispatch boundary
//! before calling [`fetch_candidates_for_keys`]. The earlier
//! `fetch_candidates_for_endings` entry remains the TL/POJ path
//! (canonical TL ASCII inputs only).

// 中文: v3.5.8 連續輸入 Phase 5 — 多 span 候選查詢入口。
// 中文: 對 endings 中的每個 end 以 input[pos..end] 為 toneless TL key 查 FST,
// 中文:   把每個命中包成 RawCandidate 並用 ContinuousScore 公式打分後 desc 排序回傳。

use std::cmp::Reverse;

use crate::dictionary_reader::{DictionaryReader, DictionaryRecord, Filter};
use crate::prefix_index::PrefixIndex;
use ranking::{calculate_continuous_score, source_tier_rank};

/// `RawCandidate.form` discriminator. Phase 5 only emits notone candidates
/// because span-local lookup is always over `tl:<toneless>` keys
/// (`docs/roadmap.md:312`); Phase 6+ may extend with hanzi (0) / numeric
/// (2) / abbrev (3) when proto-side carriers exist (Codex pre-impl
/// review 2026-05-10 Fork 5 ACCEPT).
// 中文: Phase 5 唯一支援的 form 標籤 (notone);其他 form 留給 Phase 6+。
pub const FORM_NOTONE: u8 = 1;

/// One span-local candidate. Mirrors the 5-field shape pinned by
/// `docs/roadmap.md:329-336`.
// 中文: 單一 span-local 候選詞,5 欄位對應 roadmap §Phase 5 規格。
#[derive(Debug, Clone, PartialEq)]
pub struct RawCandidate {
    /// Byte span `(start, end)` in the original input that this candidate
    /// consumes on commit. `start` always equals the `pos` passed to
    /// [`fetch_candidates_for_endings`]; `end` is one of the offsets in
    /// `endings`.
    // 中文: 此候選 commit 時消耗的 byte 區間 (start = pos,end ∈ endings)。
    pub consumed_span: (u32, u32),
    /// Number of TL syllables in the matched dictionary entry, copied
    /// from `DictionaryRecord::syllable_count` (1..=4 by builder cap).
    // 中文: 對應字典條目的音節數 (builder 端上限 4)。
    pub syllable_count: u8,
    /// What the user sees / what gets committed: hanji if available,
    /// otherwise the stored TL romanization.
    // 中文: 上屏顯示文字 — 有漢字用漢字,否則回退到 TL 羅馬字。
    pub display_text: String,
    /// Result of [`ranking::calculate_continuous_score`].
    // 中文: 連續輸入排序分數 (見 ranking::calculate_continuous_score)。
    pub score: f32,
    /// Always [`FORM_NOTONE`] in Phase 5.
    // 中文: 候選來源 form 標籤 (Phase 5 永遠是 FORM_NOTONE)。
    pub form: u8,
    /// Raw dictionary frequency before any bias / boost. Carried
    /// alongside the multiplicative `score` so the v3.5.8 Phase 9.1
    /// `SortKey` can use raw freq as an explicit tie-break dimension
    /// distinct from `adjusted_score`. Always equals
    /// `DictionaryRecord::frequency` for candidates produced by
    /// `fetch_candidates_for_keys`.
    // 中文: 字典原始 freq;sort_key 內作為與 adjusted_score 區隔的二次 tie-break 維度。
    pub frequency: u32,
    /// Dictionary source bitmask copied verbatim from
    /// [`DictionaryRecord::bitmask`]. Used by the v3.5.8 Phase 9.1
    /// `SortKey` to derive `source_tier_rank` at sort time per
    /// `docs/roadmap.md` § Phase 9. Carrying it on the candidate (vs.
    /// re-reading the dictionary record) lets the sort be a pure
    /// function of the returned `RawCandidate` vector.
    // 中文: 字典 source bitmask;sort_key 依此呼 ranking::source_tier_rank 取得排序 rank。
    pub bitmask: u16,
}

/// Fetch every dictionary candidate whose toneless TL key matches
/// `input[pos..end]` for some `end` in `endings`. TL/POJ entry for
/// span-local lookup; see module docs for the full contract.
///
/// Internally a thin wrapper around [`fetch_candidates_for_keys`]: it
/// maps each `end` to a `(consumed_span, "tl:<lowered>")` pair. TPS
/// callers must NOT use this entry — they go through the Phase-6
/// dispatcher path that builds keys via `phonetics::tps_to_tl` and
/// calls [`fetch_candidates_for_keys`] directly.
// 中文: TL/POJ 連續輸入入口 — 把 endings 轉成 (consumed_span, "tl:<lowered>") pairs 後委派給 fetch_candidates_for_keys。
// 中文: TPS 路徑請走 Phase 6 dispatcher,先用 phonetics::tps_to_tl 轉出 toneless TL key 再呼叫 fetch_candidates_for_keys。
pub fn fetch_candidates_for_endings(
    input: &str,
    pos: usize,
    endings: &[usize],
    enabled_sources_bitmask: u32,
    user_freq_boost: f32,
    prefix_index: &PrefixIndex,
    dict: &DictionaryReader,
) -> Vec<RawCandidate> {
    if endings.is_empty() || pos >= input.len() || !input.is_char_boundary(pos) {
        return Vec::new();
    }

    let lower = input.to_ascii_lowercase();
    let mut keys: Vec<(ConsumedSpan, String)> = Vec::with_capacity(endings.len());
    for &end in endings {
        if end <= pos || end > lower.len() || !lower.is_char_boundary(end) {
            continue;
        }
        // The toneless TL key is `tl:` + the lowered span with every
        // ASCII digit dropped — the digit half of the upstream
        // `notone.py::remove_tone` regex `[\d\-]`
        // (`dictionary/common/notone.py`). Phase 1b guarantees
        // fused-toneless storage (e.g. `珠仔 → tl:tsua`,
        // `台北 → tl:taipak`), and the syllabifier hands us endings
        // for both numeric (`tai1bak4`) and toneless (`taibak`) input
        // forms; stripping here lets numeric-tone input still hit the
        // fused toneless FST key. The hyphen half of the regex is NOT
        // applied at this layer because the syllabifier itself walks
        // contiguous syllable bytes via `inv.contains(...)` and the
        // inventory has no hyphenated entries — hyphen-input handling
        // is deferred to Phase 9 (per Phase 6 dispatch limitations
        // note in `engine/composing/src/dispatch.rs::build_keys_tl`).
        // Python `\d` is Unicode-decimal but TL canonical input only
        // uses ASCII `0..=9`, so `is_ascii_digit()` is sound under the
        // module input contract above.
        let segment = &lower[pos..end];
        let toneless: String = segment.chars().filter(|c| !c.is_ascii_digit()).collect();
        if toneless.is_empty() {
            continue;
        }
        keys.push(((pos as u32, end as u32), format!("tl:{toneless}")));
    }

    // Phase 9.1: pass full `input.len()` for the Tier 1 predicate.
    // Even when `pos != 0`, candidates whose `consumed_span_end`
    // reaches the full input length still qualify for Tier 0 — Tier 1
    // is full-buffer coverage, not `input.len() - pos`.
    fetch_candidates_for_keys(
        &keys,
        input.len() as u32,
        enabled_sources_bitmask,
        user_freq_boost,
        prefix_index,
        dict,
    )
}

/// Span aliases for [`fetch_candidates_for_keys`]: `(start_byte, end_byte)`
/// in the user-facing input buffer (TL ASCII or TPS Bopomofo bytes,
/// depending on caller). The engine only stores these verbatim in the
/// returned `RawCandidate.consumed_span`; FST lookup uses the paired key.
// 中文: ConsumedSpan = 使用者輸入緩衝中的 byte 區間 (TL/POJ 為 ASCII;TPS 為 Bopomofo bytes)。
pub type ConsumedSpan = (u32, u32);

/// Mode-agnostic span-local fetch entry. Each input pair is
/// `(consumed_span, fst_key)`: `consumed_span` is the user-facing
/// byte range that committing this candidate will eat, and `fst_key`
/// is the already-prefixed FST lookup key (e.g. `"tl:tsua"`). The
/// caller (Phase 6 dispatcher) is responsible for building keys from
/// the user input — TL/POJ path goes through
/// [`fetch_candidates_for_endings`]; TPS path uses
/// `phonetics::tps_to_tl` per syllable, strips the trailing tone
/// digit, and prepends `"tl:"`.
///
/// # v3.5.8 Phase 9.1 — lexicographic SortKey
///
/// `raw_len` is the byte length of the original pending buffer
/// (`Phase::Continuous { raw }.len()`); it is the predicate input
/// for Tier 1 (`consumed_span_end == raw_len`). Sorting follows
/// `docs/roadmap.md` § Phase 9 sort_key formula:
///
/// ```text
/// (tier, -coverage_bytes, recency_rank, -adjusted_score,
///  -freq, source_tier_rank, stable_idx)
/// ```
///
/// PR-9.1 carries sentinel `recency_rank = 1` (no FrequencyEntry
/// plumbing yet — that lands in PR-9.3a per `docs/roadmap.md` Phase 9
/// row 9.3a). NaN scores (only reachable if the caller violates the
/// `user_freq_boost` finite contract) are coerced to `f32::MIN` at
/// `SortKey` construction so the descending-order invariant holds.
// 中文: Phase 6 新增 — 模式無關的 span-local 候選查詢;接受 (consumed_span, "tl:<key>") pair list,讓 dispatch 端集中處理 TL vs TPS key 構造。
// 中文: Phase 9.1 改:接 raw_len (= pending buffer 長度) 用於 Tier 1 判定;排序用 SortKey 七維 lexicographic。
pub fn fetch_candidates_for_keys(
    keys: &[(ConsumedSpan, String)],
    raw_len: u32,
    enabled_sources_bitmask: u32,
    user_freq_boost: f32,
    prefix_index: &PrefixIndex,
    dict: &DictionaryReader,
) -> Vec<RawCandidate> {
    if keys.is_empty() {
        return Vec::new();
    }

    let filter = Filter::from_enabled_bitmask(enabled_sources_bitmask);
    let mut out: Vec<RawCandidate> = Vec::new();

    for (span, key) in keys {
        for rowid in prefix_index.lookup_exact(key) {
            let Some(record) = dict.record(rowid) else {
                continue;
            };
            if !DictionaryReader::passes_filter(record.bitmask, &filter) {
                continue;
            }
            out.push(record_to_candidate(record, *span, user_freq_boost));
        }
    }

    // Phase 9.1 lexicographic sort. `stable_idx` is stamped from
    // pre-sort element position via `enumerate()` BEFORE any sorting
    // machinery runs, so the index reflects insertion order (caller-
    // provided `keys` order × `prefix_index.lookup_exact` FST byte-
    // sort) and is independent of how the sort algorithm chooses to
    // invoke the key extractor. `slice::sort_by_cached_key` does NOT
    // contractually pin the order in which it calls the key fn;
    // earlier revisions that incremented a counter inside the closure
    // were silently relying on stdlib internals (Codex PR #262
    // r3216153007).
    let mut indexed: Vec<(SortKey, RawCandidate)> = out
        .into_iter()
        .enumerate()
        .map(|(i, c)| (SortKey::new(&c, raw_len, i as u32), c))
        .collect();
    indexed.sort_by_key(|(key, _)| *key);
    indexed.into_iter().map(|(_, c)| c).collect()
}

fn record_to_candidate(
    record: DictionaryRecord,
    consumed_span: ConsumedSpan,
    user_freq_boost: f32,
) -> RawCandidate {
    let DictionaryRecord {
        bitmask,
        frequency,
        syllable_count,
        hanzi,
        tl,
    } = record;
    let display_text = hanzi.unwrap_or(tl);
    let score = calculate_continuous_score(frequency, syllable_count, user_freq_boost);
    RawCandidate {
        consumed_span,
        syllable_count,
        display_text,
        score,
        form: FORM_NOTONE,
        frequency,
        bitmask,
    }
}

// ---------------------------------------------------------------------------
// v3.5.8 Phase 9.1 — SortKey
//
// Encodes the seven-dimension lexicographic sort policy pinned in
// `docs/roadmap.md` § Phase 9. Field order in this struct matches
// `#[derive(Ord)]`'s lexicographic comparison; `Reverse<T>` flips
// individual dimensions whose policy is descending. NaN-safe
// because scores are wrapped in `NonNanF32` which coerces NaN to
// `f32::MIN` at construction.
// ---------------------------------------------------------------------------

// 中文: Phase 9.1 排序鍵 — 7 維 lexicographic,asc/desc 由 Reverse<T> 控;NaN 在 NonNanF32 內 coerce 成 f32::MIN。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SortKey {
    /// `0` = Tier 0 (full-buffer coverage), `1` = Tier 1 (partial).
    /// Roadmap and spec both use the "Tier 0 = full buffer" labelling
    /// (`docs/roadmap.md` § Phase 9 / `docs/engine/continuous-input-
    /// ranking.md` §1.1).
    // 中文: tier — 0 為 Tier 0 (consumed_span_end == raw_len),1 為 Tier 1 (部分覆蓋)。
    tier: u8,
    /// Descending: longer coverage wins within tier.
    // 中文: coverage 長度 desc,同 tier 內長覆蓋優先。
    neg_coverage: Reverse<u32>,
    /// `0` = recent (last_used_ms within `RECENCY_WINDOW_MS`), `1` =
    /// stale/never-used. PR-9.1 always sets `1`; PR-9.3a will compute
    /// from `FrequencyEntry.last_used_ms`.
    // 中文: recency_rank — 0 = recent (PR-9.3a 才填入), PR-9.1 一律 1。
    recency_rank: u8,
    /// Descending: higher `freq × syll_bias × boost` wins.
    // 中文: adjusted_score desc;NaN coerce 成 f32::MIN 於 NonNanF32 內。
    neg_score: Reverse<NonNanF32>,
    /// Descending: raw freq as a secondary tie-break independent of
    /// adjusted_score (only differs when boost ≠ 1.0 once 9.3a lands).
    // 中文: freq desc 作為 adjusted_score 之外的二次 tie-break (9.3a boost 不為 1.0 後才會分歧)。
    neg_freq: Reverse<u32>,
    /// Ascending: `custom=0, kautian=1, taigitv=2, stti=3, kungge=4,
    /// default=5` per `ranking::source_tier_rank`.
    // 中文: source 來源 rank asc;ranking::source_tier_rank 為單一 source-of-truth。
    source_rank: u8,
    /// Insertion index — deterministic by caller-provided `keys`
    /// order × `prefix_index.lookup_exact` FST byte-sort.
    // 中文: stable_idx — 插入順序,保證 deterministic tie-break。
    stable_idx: u32,
}

impl SortKey {
    fn new(candidate: &RawCandidate, raw_len: u32, stable_idx: u32) -> Self {
        let (start, end) = candidate.consumed_span;
        let coverage_bytes = end.saturating_sub(start);
        let tier: u8 = if end == raw_len { 0 } else { 1 };
        // TODO(Phase 9.3a): replace sentinel with derived value from
        // `FrequencyEntry { last_used_ms }` once user-freq plumbing
        // lands. Pinned by `recency_rank_field_is_sentinel_one_in_pr_9_1`
        // test — flip both call sites together.
        let recency_rank: u8 = 1;
        // PR-9.1 always sees dict.bin records (is_custom=false). PR-9.6
        // will introduce platform-side custom dict merge; that merge
        // point is responsible for tagging the custom rank, not this fn.
        let source_rank = source_tier_rank(candidate.bitmask, false);
        Self {
            tier,
            neg_coverage: Reverse(coverage_bytes),
            recency_rank,
            neg_score: Reverse(NonNanF32::new(candidate.score)),
            neg_freq: Reverse(candidate.frequency),
            source_rank,
            stable_idx,
        }
    }
}

/// `f32` newtype with a total order via [`f32::total_cmp`] after
/// coercing `NaN` to [`f32::MIN`]. Lets [`SortKey`] derive `Ord`
/// without a hand-written comparator, while still defending against
/// `NaN` leakage from a contract-violating `user_freq_boost`
/// (`calculate_continuous_score` docs).
// 中文: f32 newtype + NaN coerce f32::MIN,讓 SortKey derive(Ord) 而不必手寫 cmp。
#[derive(Debug, Clone, Copy)]
struct NonNanF32(f32);

impl NonNanF32 {
    fn new(v: f32) -> Self {
        Self(if v.is_nan() { f32::MIN } else { v })
    }
}

impl PartialEq for NonNanF32 {
    fn eq(&self, other: &Self) -> bool {
        self.0.total_cmp(&other.0) == std::cmp::Ordering::Equal
    }
}

impl Eq for NonNanF32 {}

impl PartialOrd for NonNanF32 {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NonNanF32 {
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
    //
    // 中文: SortKey 排序政策的 hermetic 單元測試;hermetic 整合 regression 在 tests/span_local_fetch.rs。
    use super::*;

    /// Convenience builder so each test only specifies the dimensions
    /// it exercises. Fields not exercised default to neutral values:
    /// `frequency = 0`, `bitmask = 0` (→ source rank = default = 5),
    /// `score = 0.0`, `syllable_count = 1`.
    fn cand(
        span_start: u32,
        span_end: u32,
        score: f32,
        frequency: u32,
        bitmask: u16,
    ) -> RawCandidate {
        RawCandidate {
            consumed_span: (span_start, span_end),
            syllable_count: 1,
            display_text: String::new(),
            score,
            form: FORM_NOTONE,
            frequency,
            bitmask,
        }
    }

    #[test]
    fn tier0_full_buffer_beats_tier1_partial_even_when_score_lower() {
        // Phase 9.1 headline behavior: Tier 0 (full buffer) wins over
        // Tier 1 (partial) regardless of raw score. Mirrors the
        // `taiuantaigi` motivation case (`docs/roadmap.md` § Phase 9
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
    fn within_tier_longer_coverage_beats_shorter() {
        // Per Codex R2 Q2.c — `-coverage_bytes` precedes `-adjusted_score`
        // inside the tier. A 6-byte coverage with score 100 still wins
        // over a 3-byte coverage with score 1000 when both are Tier 1.
        let raw_len: u32 = 9; // neither span hits full buffer
        let longer = cand(0, 6, 100.0, 100, 0);
        let shorter = cand(0, 3, 1000.0, 1000, 0);

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
    fn recency_rank_field_is_sentinel_one_in_pr_9_1() {
        // PR-9.1 contract: no FrequencyEntry plumbing yet, so every
        // SortKey carries `recency_rank = 1`. PR-9.3a will change this
        // and the assertion should be tightened then.
        let raw_len: u32 = 3;
        let key = SortKey::new(&cand(0, 3, 1.0, 1, 0), raw_len, 0);
        assert_eq!(key.recency_rank, 1);
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
}
