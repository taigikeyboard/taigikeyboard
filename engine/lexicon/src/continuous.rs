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
//! This module is the pure-Rust testable surface; the proto request /
//! response carriers (`FetchAtPosResp` etc.), the `EngineHandle` /
//! `dispatch` wiring, AND the TPS → TL key mapping all land in Phase 6
//! (`docs/roadmap.md:347-360`). Phase 5 deliberately does NOT add an
//! `Intent::FetchAtPos` shim (Codex pre-impl review 2026-05-10 Fork 1
//! ACCEPT).

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
/// `input[pos..end]` for some `end` in `endings`. See module docs for
/// the full contract.
// 中文: 連續輸入 Phase 5 主入口 — 對 endings 每個 end 查 toneless TL FST,合併打分排序後回傳。
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

    let filter = Filter::from_enabled_bitmask(enabled_sources_bitmask);
    let lower = input.to_ascii_lowercase();
    let mut out: Vec<RawCandidate> = Vec::new();

    for &end in endings {
        if end <= pos || end > lower.len() || !lower.is_char_boundary(end) {
            continue;
        }
        // The toneless TL key is `tl:` + the lowered span. Phase 1b
        // guarantees fused-toneless storage (e.g. `珠仔` → `tl:tsua`,
        // not `tl:tsu-a`), so an exact match here suffices — prefix
        // expansion would over-collect (e.g. `tl:tsuah`).
        let key = format!("tl:{}", &lower[pos..end]);
        let span = (pos as u32, end as u32);
        for rowid in prefix_index.lookup_exact(&key) {
            let Some(record) = dict.record(rowid) else {
                continue;
            };
            if !DictionaryReader::passes_filter(record.bitmask, &filter) {
                continue;
            }
            out.push(record_to_candidate(record, span, user_freq_boost));
        }
    }

    // Stable sort by score desc; ties fall back to insertion order
    // (which is deterministic: `endings` ascending × `lookup_exact`
    // FST byte-sort). NaN scores (only reachable if the caller violates
    // the `user_freq_boost` finite contract) are coerced to `f32::MIN`
    // so the descending-order invariant holds even under contract abuse.
    out.sort_by(|a, b| {
        let bs = if b.score.is_nan() { f32::MIN } else { b.score };
        let as_ = if a.score.is_nan() { f32::MIN } else { a.score };
        bs.partial_cmp(&as_).unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

fn record_to_candidate(
    record: DictionaryRecord,
    consumed_span: (u32, u32),
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
