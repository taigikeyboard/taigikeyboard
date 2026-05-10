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

use crate::dictionary_reader::{DictionaryReader, DictionaryRecord, Filter};
use crate::prefix_index::PrefixIndex;
use ranking::calculate_continuous_score;

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

    fetch_candidates_for_keys(
        &keys,
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
/// Same scoring + ordering contract as
/// [`fetch_candidates_for_endings`] (NaN-safe descending by score,
/// stable on ties).
// 中文: Phase 6 新增 — 模式無關的 span-local 候選查詢;接受 (consumed_span, "tl:<key>") pair list,讓 dispatch 端集中處理 TL vs TPS key 構造。
pub fn fetch_candidates_for_keys(
    keys: &[(ConsumedSpan, String)],
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

    // Stable sort by score desc; ties fall back to insertion order
    // (which is deterministic: caller-provided keys order ×
    // `lookup_exact` FST byte-sort). NaN scores (only reachable if
    // the caller violates the `user_freq_boost` finite contract) are
    // coerced to `f32::MIN` so the descending-order invariant holds.
    out.sort_by(|a, b| {
        let bs = if b.score.is_nan() { f32::MIN } else { b.score };
        let as_ = if a.score.is_nan() { f32::MIN } else { a.score };
        bs.partial_cmp(&as_).unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

fn record_to_candidate(
    record: DictionaryRecord,
    consumed_span: ConsumedSpan,
    user_freq_boost: f32,
) -> RawCandidate {
    let DictionaryRecord {
        frequency,
        syllable_count,
        hanzi,
        tl,
        ..
    } = record;
    let display_text = hanzi.unwrap_or(tl);
    let score = calculate_continuous_score(frequency, syllable_count, user_freq_boost);
    RawCandidate {
        consumed_span,
        syllable_count,
        display_text,
        score,
        form: FORM_NOTONE,
    }
}
