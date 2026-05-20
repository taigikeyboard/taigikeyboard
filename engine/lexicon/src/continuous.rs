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

use unicode_normalization::UnicodeNormalization;

use crate::dictionary_reader::{DictionaryReader, DictionaryRecord, Filter};
use crate::prefix_index::PrefixIndex;
use ranking::{
    calculate_continuous_score, recency_rank, source_tier_rank, user_freq_boost, FrequencyMap,
};

/// `RawCandidate.form` discriminator. Phase 5 only emits notone candidates
/// because span-local lookup is always over `tl:<toneless>` keys
/// (`docs/roadmap.md:312`); Phase 6+ may extend with hanzi (0) / numeric
/// (2) / abbrev (3) when proto-side carriers exist (Codex pre-impl
/// review 2026-05-10 Fork 5 ACCEPT).
// 中文: Phase 5 唯一支援的 form 標籤 (notone);其他 form 留給 Phase 6+。
pub const FORM_NOTONE: u8 = 1;

/// v3.5.8 Phase 9 Item 10 — `RawCandidate.coverage_kind` ordinal for
/// full-syllable hits (the pre-Item-10 path: `valid_span_endings`
/// returned at least one ending and `fetch_candidates_for_keys`
/// produced the candidate via `prefix_index.lookup_exact`).
// 中文: Item 10 — 完整音節覆蓋候選 (syllabifier 切出邊界,fetch_candidates_for_keys 走 lookup_exact)。
pub const COVERAGE_KIND_FULL: u8 = 0;

/// v3.5.8 Phase 9 Item 10 — `RawCandidate.coverage_kind` ordinal for
/// partial-prefix hits (syllabifier returned no ending, engine fell
/// through to [`fetch_partial_prefix_candidates`] via
/// `prefix_index.lookup_prefix`). Ranks strictly below
/// [`COVERAGE_KIND_FULL`] in [`SortKey`] regardless of any other
/// dimension; see `docs/engine/continuous-candidate-display.md` §15.5.
// 中文: Item 10 — 部分前綴覆蓋候選 (syllabifier 切不出邊界,改走 lookup_prefix);
// 中文:   SortKey 上強制排在 COVERAGE_KIND_FULL 之後,不被任何其他維度反超。
pub const COVERAGE_KIND_PARTIAL_PREFIX: u8 = 1;

/// v3.5.8 Phase 9 Item 10 — max partial-prefix rowids hydrated per
/// `FetchAtPos`. Single-char prefixes (`tl:t`) can match hundreds of
/// FST entries; hydrating every record before sort is wasted work
/// when the candidate strip only renders ~10 items. Pre-cap mirrors
/// the legacy `LexiconService.search` per-request cap, taking the
/// FST byte-sorted prefix in document order (which roughly aligns
/// with insertion order in `dictionary/build/create_fst.py`); see
/// `docs/engine/continuous-candidate-display.md` §15.8 risk row 1.
///
/// **Known limitation** (Codex pre-impl R6, 2026-05-15): pre-cap is
/// not ranking-aware — a globally high-frequency partial-prefix hit
/// landing after rowid 30 in FST byte order would be dropped. We
/// accept this as legacy-parity behaviour (the pre-§15 platform
/// lexicon path has the same property); see Item 13 retire notes
/// in `docs/engine/continuous-candidate-display.md` §15.4.
// 中文: Item 10 — partial-prefix lookup_prefix 在 dict.record hydration 前的 rowid 上限,對齊 legacy LexiconService 行為。
// 中文: 已知侷限 (R6) — 大頻率候選若落在 byte-sort 後 30 名外會被丟,屬 legacy parity 行為。
pub const PARTIAL_PREFIX_CAP: usize = 30;

/// MOE-aligned candidate-type discriminator (`VocType` analog). Carried
/// on every [`RawCandidate`] and wire-encoded onto
/// `protos::taigi::engine::CandidateMessage.mode` (Phase 9.2). Derived
/// from `DictionaryRecord.hanzi` presence + NFKD-normalized Latin-letter
/// detection by [`derive_mode`]; never emitted as
/// [`CandidateMode::Unspecified`] from Rust.
///
/// **Metadata-only in v3.5.8 Phase 9.2** — does NOT enter the seven-
/// dimension [`SortKey`] tie-break (per `docs/roadmap.md` § Phase 9 R2
/// Q3.a "reserve rank use until real collisions are measured"). The
/// existing `form` axis remains orthogonal (toneless / numeric / hanji
/// / abbrev) and unaffected.
// 中文: Phase 9.2 候選類型軸 — HANT 純漢字 / TAILO 純羅馬字 / MIXED 漢羅混排;
// 中文:   由 derive_mode 用 NFKD 規範化後判斷 Latin 字母命中;不入 SortKey。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CandidateMode {
    /// Proto3 default — Rust never emits this; platforms reading the
    /// wire treat it as "unknown carrier, ignore" rather than HANT.
    // 中文: proto3 預設值;Rust 永不主動發送,讀端視作 unknown carrier。
    Unspecified = 0,
    /// Hanji-only display (no Latin letters after NFKD normalization).
    // 中文: 純漢字顯示(NFKD 規範後無 Latin 字母)。
    Hant = 1,
    /// Roman/romanization-only display — `DictionaryRecord.hanzi` was
    /// `None`, so `display_text` fell back to the TL field.
    // 中文: 純羅馬字(`DictionaryRecord.hanzi == None`,display 退到 TL)。
    Tailo = 2,
    /// Hanji display containing at least one Latin letter after NFKD
    /// (e.g. `iáu未`, `ê早`, `屎î`, hypothetical fullwidth `Ａ字`).
    // 中文: 漢字內含 Latin 字母(NFKD 規範後判斷;含 NFC composed `ê`、全角等)。
    Mixed = 3,
}

impl CandidateMode {
    /// Wire-format integer matching `protos::CandidateMode`'s prost
    /// representation. Kept as a method so a future reshuffle of the
    /// proto enum values would fail this cast at compile time via the
    /// `as u32` discriminant.
    // 中文: 對齊 proto CandidateMode 的 wire 整數;reshuffle 會編譯期 break。
    pub const fn to_proto_i32(self) -> i32 {
        self as i32
    }
}

/// Derive the [`CandidateMode`] for a dictionary record. `hanzi.is_none()`
/// is the only TAILO path; otherwise the hanzi string is NFKD-normalized
/// (folding `ê` → `e` + combining circumflex and `Ａ` → `A`) and any
/// resulting ASCII alphabetic codepoint flips the candidate to MIXED.
/// Digits / punctuation / kana / private-use glyphs alone do NOT flip
/// MIXED — the intent is "Roman letters inside the hanji display",
/// matching MOE `VT_MIXED` for entries like `台BAR`.
///
/// **Single source of truth for `CandidateMode`.** `record_to_candidate`,
/// `custom_entry_to_candidate`, and the v3.5.8 S2 whole-sentence
/// walker's slot-0 synthesis (`composing::continuous::fetch_walker_slot0_inner`)
/// all derive `mode` through this fn so `CandidateMessage.mode` is
/// classified identically for span-local, custom, and synthesized
/// full-buffer candidates (Codex PR #285 P2, 2026-05-16 — a hand-rolled
/// `hanji.is_some()` binary in the synth path mis-emitted HANT for
/// mixed-script paths like `…hip相`). `pub` so `composing` reuses the
/// wire-visible classification instead of duplicating the NFKD rule.
// 中文: Phase 9.2 mode 推導 — hanzi=None → TAILO;NFKD 規範後若含 ASCII 字母 → MIXED;否則 HANT。
// 中文:   數字 / 標點 / 假名 / PUA 不算 MIXED — MIXED 限定「漢字顯示內含羅馬字母」。
// 中文: CandidateMode 唯一真相來源;record/custom/S2 walker synth 全走此(Codex PR #285 P2)。
pub fn derive_mode(hanzi: Option<&str>) -> CandidateMode {
    match hanzi {
        None => CandidateMode::Tailo,
        Some(text) if text.nfkd().any(|c| c.is_ascii_alphabetic()) => CandidateMode::Mixed,
        Some(_) => CandidateMode::Hant,
    }
}

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
    /// otherwise the stored TL romanization. Engine-authoritative
    /// commit key + `user_frequency.db` write key on both platforms.
    // 中文: 上屏顯示文字 — 有漢字用漢字,否則回退到 TL 羅馬字。
    pub display_text: String,
    /// v3.5.8 Phase 9 Item 5 — display romanization carried alongside
    /// `display_text` so platform UI can render dual-line cells
    /// (roman line + hanji line) the same way the legacy lexicon
    /// path does. At this lexicon layer it equals the underlying
    /// `DictionaryRecord.tl`; `dispatch::handle_fetch_at_pos` then
    /// applies the presentation transforms — per-segment recasing and,
    /// in POJ input mode, a TL→POJ-display rewrite (`oo`→`o͘`,
    /// `nn`→`ⁿ`, …) — before emission. NEVER consulted for the engine
    /// commit (which goes through `display_text`); the platform formats
    /// its document string from this presentation roman.
    // 中文: Phase 9 Item 5 — 顯示羅馬字 sidechannel;dual-line 候選列 render 來源。
    // 中文: 在 lexicon 層等於 DictionaryRecord.tl;dispatch::handle_fetch_at_pos 再做呈現轉換
    // 中文: (逐段 recase + POJ 模式下 TL→POJ-display)後才送出。引擎 commit 仍走 display_text,
    // 中文: 不查 roman;平台則由此呈現 roman 產生文件字串。
    pub roman: String,
    /// v3.5.8 Phase 9 Item 5 — hanji display carried alongside
    /// `display_text`. `None` iff `DictionaryRecord.hanzi.is_none()`
    /// (TAILO candidate); `Some` otherwise. On the wire this maps to
    /// `optional string hanji` so consumers can distinguish "TAILO
    /// — no hanji exists" from "wire-frame defect / absent field"
    /// (per `docs/engine/continuous-candidate-display.md` §4.2). UI
    /// uses this as the dual-line cell subtitle; engine commit still
    /// goes through `display_text`.
    // 中文: Phase 9 Item 5 — 漢字顯示用 sidechannel;TAILO 候選為 None。
    // 中文: 對應 proto optional;commit 不查此欄,只用於 dual-line 候選列 subtitle。
    pub hanji: Option<String>,
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
    /// MOE-aligned candidate-type discriminator (HANT / TAILO / MIXED).
    /// Derived by [`derive_mode`] from `DictionaryRecord.hanzi`.
    /// Metadata-only in Phase 9.2 — not consulted by [`SortKey`].
    // 中文: 候選類型軸 (Phase 9.2);由 derive_mode 從 hanzi 推導;不入 SortKey。
    pub mode: CandidateMode,
    /// v3.5.8 Phase 9.3a — `0` when this candidate's matching
    /// `FrequencyEntry` was selected strictly inside the
    /// `RECENCY_WINDOW_MS` window; `1` otherwise (stale, never used,
    /// or clock-skew). Computed once by [`record_to_candidate`] from
    /// the caller-built `FrequencyMap` + `now_ms`, and read verbatim
    /// by [`SortKey::new`]. Internal: NOT emitted on
    /// `CandidateMessage` today — platform UI does not yet render a
    /// "recently used" affordance, so adding a wire field is
    /// premature (PR-9.3c may revisit).
    // 中文: Phase 9.3a — 候選的最近使用 rank;由 record_to_candidate 從 FrequencyMap + now_ms 算好,SortKey 直接讀。
    // 中文: 目前不上 wire (CandidateMessage 沒帶);UI 沒「最近使用」標記需求,PR-9.3c 視情況補。
    pub recency_rank: u8,
    /// v3.5.8 Phase 9 Item 10 — coverage kind for the new partial-prefix
    /// path. [`COVERAGE_KIND_FULL`] for the existing
    /// `fetch_candidates_for_keys` lookup-exact path;
    /// [`COVERAGE_KIND_PARTIAL_PREFIX`] for
    /// [`fetch_partial_prefix_candidates`] hits.
    ///
    /// Internal axis only — does NOT enter `CandidateMessage` (same
    /// pattern as [`recency_rank`]; see
    /// `docs/engine/continuous-candidate-display.md` §15.5). Consumed
    /// solely by [`SortKey`] to push every partial-prefix candidate
    /// strictly below every full-syllable candidate in lexicographic
    /// order, irrespective of `tier`, score, recency, dict freq, or
    /// source rank.
    // 中文: Item 10 — 覆蓋類型旗標;COVERAGE_KIND_FULL=0 走 lookup_exact,COVERAGE_KIND_PARTIAL_PREFIX=1 走 lookup_prefix。
    // 中文: 不上 wire,僅供 SortKey 排序使用 — partial-prefix 永遠被排在 full-syllable 之後,不論其他維度。
    pub coverage_kind: u8,
    /// v3.5.8 Phase 9 Item 12 — `true` for candidates synthesized from
    /// a `custom_dictionary.db` entry ([`custom_entry_to_candidate`]),
    /// `false` for `dict.bin` FST hits ([`record_to_candidate`]). Read
    /// by [`SortKey::new`] (→ `source_tier_rank(bitmask, is_custom)`
    /// returns rank `0` when `true`, ahead of every `dict.bin` source
    /// tier) and by the `(roman, hanji)` dedupe winner policy
    /// (`docs/engine/continuous-input-ranking.md` §10.10): on a
    /// duplicate `(roman, hanji)` pair the lowest `source_tier_rank`
    /// survivor wins, so a custom entry always beats a `dict.bin`
    /// duplicate. Internal axis only — NOT emitted on
    /// `CandidateMessage` (same pattern as [`coverage_kind`] /
    /// `recency_rank`).
    // 中文: Item 12 — true=custom_dictionary.db 合成候選 (source rank 0,(roman,hanji) 去重必勝),false=dict.bin FST 命中。
    // 中文: 不上 wire,僅供 SortKey 與去重勝負政策使用。
    pub is_custom: bool,
}

/// v3.5.8 Phase 9 Item 12 — one `custom_dictionary.db` row hoisted
/// from `protos::engine::CustomDictEntry` (proto→domain boundary in
/// `composing/src/dispatch.rs::build_custom_entries`). `roman` /
/// `hanji` are the raw stored columns the platform marshalled
/// verbatim (no display capitalization) so the engine's
/// `(roman, hanji)` dedupe key collides correctly against
/// `dict.bin`'s `DictionaryRecord.tl` / `.hanzi`. `hanji = None`
/// is a romanization-only custom entry (mirrors
/// `DictionaryRecord.hanzi` / `RawCandidate.hanji` `Option` semantics
/// — drives [`derive_mode`] → `CandidateMode::Tailo`).
// 中文: Item 12 — 一筆 custom_dictionary.db 命中的 domain 形;roman/hanji 為原始欄位 (未顯示大寫化),
// 中文:   讓 (roman,hanji) 去重鍵能與 dict.bin 正確碰撞;hanji=None 為純羅馬字 custom 條目。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CustomEntry {
    pub roman: String,
    pub hanji: Option<String>,
}

/// v3.5.9 D7 — shared context for the continuous-input fetch entry
/// points ([`fetch_candidates_for_keys`],
/// [`fetch_partial_prefix_candidates`]). Each one previously took
/// eight positional arguments and tripped
/// `clippy::too_many_arguments`; bundling the six shared concerns
/// (filter / freq-map / clock / custom / readers) into one borrowed
/// struct collapses every call site to three or four args without
/// changing any behavior.
///
/// Field order matches the legacy `fetch_candidates_for_keys` arg
/// order so a reader scanning a git blame can map old positional args
/// onto the new fields without renaming work.
///
/// All fields are `pub` — construction is always a stack-local struct
/// literal at the call site (production builds it inside the composing
/// seam; integration tests build it once per test). No constructor is
/// needed.
// 中文: D7 — 連續輸入 fetch 入口共用的 context;把原本 8 個位置參數中重複的 6 個 (filter/freq_map/clock/custom/readers)
// 中文:   收進一個借用 struct,call site 縮到 3-4 args、移除 clippy::too_many_arguments allow。欄位順序對齊舊
// 中文:   fetch_candidates_for_keys 參數順序,讓 git blame 可逐欄對應;欄位全 pub,call site 用 struct literal 直接建。
pub struct ContinuousFetchCtx<'a> {
    /// `Filter::from_enabled_bitmask` input. Production passes
    /// `u32::MAX` (all sources enabled); tests narrow it to verify
    /// filter behaviour.
    pub enabled_sources_bitmask: u32,
    /// Per-`display_text` user-selection snapshot. Empty map +
    /// `now_ms = 0` is the cold-start neutral.
    pub freq_map: &'a FrequencyMap,
    /// Platform epoch-ms wall clock at fetch time.
    pub now_ms: i64,
    /// `custom_dictionary.db` hits to merge into the candidate list.
    /// Empty slice = no custom merge (the production wiring's
    /// cold-start default; also the hard-coded value inside the
    /// hidden test-only legacy endings wrapper).
    pub custom: &'a [CustomEntry],
    /// FST prefix index reader.
    pub prefix_index: &'a PrefixIndex,
    /// Dictionary record reader (mmap-backed).
    pub dict: &'a DictionaryReader,
}

/// v3.5.9 D8 — test-only entry: fetch every dictionary candidate
/// whose toneless TL key matches `input[pos..end]` for some `end` in
/// `endings`. Production code path goes through
/// `composing::continuous::fetch_via_lexicon_inner` →
/// [`fetch_candidates_for_keys`] directly with the platform's
/// `custom_entries`; the only callers reaching this entry are the
/// `engine/lexicon/tests/` integration tests, which explicitly never
/// carry custom-dict matches (the ctx wrapper below forces
/// `custom = &[]` before delegating). Marked `#[doc(hidden)]` so the
/// rustdoc public surface no longer advertises it.
///
/// Internally a thin wrapper around [`fetch_candidates_for_keys`]: it
/// maps each `end` to a `(consumed_span, "tl:<lowered>")` pair. TPS
/// callers must NOT use this entry — they go through the Phase-6
/// dispatcher path that builds keys via `phonetics::tps_to_tl` and
/// calls [`fetch_candidates_for_keys`] directly.
///
/// **v3.5.8 Phase 9.3a**: `ctx.freq_map` carries the per-display-text
/// user selection snapshot keyed by `RawCandidate::display_text`
/// (= `hanji ?? tl`); `ctx.now_ms` is the platform's epoch-ms wall
/// clock. Pass `&FrequencyMap::new()` + `now_ms = 0` for cold-start
/// neutral behaviour (boost = 1.0, recency_rank = 1 everywhere) —
/// `recency_rank()`'s guards (`now_ms <= 0`, `last_used_ms <= 0`,
/// clock skew) make this a safe default.
// 中文: D8 — test-only TL/POJ 連續輸入入口;production 走 composing::continuous::fetch_via_lexicon_inner 直呼
// 中文:   fetch_candidates_for_keys 帶平台 custom,只剩 lexicon 整合測試會落到這裡。doc(hidden) 隱藏 rustdoc 公開面。
// 中文: TPS 路徑請走 Phase 6 dispatcher,先用 phonetics::tps_to_tl 轉出 toneless TL key 再呼叫 fetch_candidates_for_keys。
// 中文: Phase 9.3a — ctx 中 freq_map+now_ms;空 map + now_ms=0 = cold-start neutral。
#[doc(hidden)]
pub fn fetch_candidates_for_endings(
    input: &str,
    pos: usize,
    endings: &[usize],
    ctx: &ContinuousFetchCtx<'_>,
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
        // applied at this layer because hyphenated TL input is folded
        // upstream by `composing::shadow::build_hyphen_shadow` (Phase
        // 9 Item 8): callers feed already-hyphenless segments here.
        // Preserving the no-strip invariant at this layer protects the
        // separation of concerns — if hyphens ever appear in a segment
        // reaching this fn it indicates an upstream contract violation
        // and the FST lookup correctly returns no match.
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
    // Item 12 + D7: this legacy test-only entry never carries custom-
    // dict matches. Rebuild an inner ctx with every field listed
    // explicitly (not `..*ctx`) so the forced-empty-custom contract
    // stays loud and survives any future ctx field that is not
    // `Copy` (Codex pre-impl SHOULD #2). Production dispatch goes
    // through `composing::continuous::fetch_via_lexicon_inner` →
    // `fetch_candidates_for_keys` directly with the platform's
    // `custom_entries`.
    // 中文: Item 12 + D7 — 此 test-only 入口不帶 custom;明確逐欄重建 inner ctx (不用 ..*ctx),
    // 中文:   讓「強制空 custom」契約清楚、且未來若加非-Copy 欄位也不會破。
    let inner = ContinuousFetchCtx {
        enabled_sources_bitmask: ctx.enabled_sources_bitmask,
        freq_map: ctx.freq_map,
        now_ms: ctx.now_ms,
        custom: &[],
        prefix_index: ctx.prefix_index,
        dict: ctx.dict,
    };
    fetch_candidates_for_keys(&keys, input.len() as u32, &inner)
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
/// (coverage_kind, tier, recency_rank, -adjusted_score,
///  -freq, -coverage_bytes, source_tier_rank, stable_idx)
/// ```
///
/// v3.5.8 整句 lattice + walker S8: `-coverage_bytes` was relocated
/// from dim 3 to dim 6 (below `-adjusted_score` / `-freq`). With the
/// slot-0 whole-sentence walker owning phrase priority, a graded
/// longest-coverage-first rule inside a tier only buried the short
/// single-syllable first-segment candidate the user wants for
/// segment-by-segment selection. Coverage is now a weak tiebreak that
/// fires only when score AND freq are equal — matching librime's
/// per-segment menu, which keeps multi-length candidates but never lets
/// a longer code-length bury a shorter strict match
/// (`script_translator.cc` `kNumExactMatchOnTop`).
///
/// # v3.5.8 Phase 9.3a — user-frequency plumb
///
/// `freq_map` is the per-`display_text` selection snapshot built once
/// per fetch by `composing/src/dispatch.rs::handle_fetch_at_pos` from
/// `FetchAtPos.frequency_entries`. `now_ms` is the platform's
/// epoch-ms wall clock at fetch time. [`record_to_candidate`] looks
/// up each candidate by `display_text`, computes
/// [`ranking::user_freq_boost`] (saturated at
/// [`ranking::MAX_BOOST`]), and derives
/// [`SortKey.recency_rank`](SortKey) via [`ranking::recency_rank`]
/// (which guards against `now_ms <= 0`, `last_used_ms <= 0`, and
/// clock skew). NaN scores (only reachable if the boost helper
/// produces a non-finite value — which it cannot under the
/// public contract) are coerced to `f32::MIN` at `SortKey`
/// construction so the descending-order invariant holds.
// 中文: Phase 6 新增 — 模式無關的 span-local 候選查詢;接受 (consumed_span, "tl:<key>") pair list,讓 dispatch 端集中處理 TL vs TPS key 構造。
// 中文: Phase 9.1 改:接 raw_len (= pending buffer 長度) 用於 Tier 1 判定;排序用 SortKey 8 維 lexicographic(S8:coverage 已降為 score/freq 之後弱 tiebreak)。
// 中文: Phase 9.3a 改:把 user_freq_boost f32 換成 (FrequencyMap + now_ms),record_to_candidate 內查表算 boost 與 recency。
// 中文: Phase 9 Item 12 改:接 custom 命中,合成 full-buffer 候選併入 out 後做 (roman,hanji) 去重 (排序前)。
// 中文: D7 改:其餘 6 個共用 arg (filter/freq_map/clock/custom/readers) 收進 ContinuousFetchCtx。
pub fn fetch_candidates_for_keys(
    keys: &[(ConsumedSpan, String)],
    raw_len: u32,
    ctx: &ContinuousFetchCtx<'_>,
) -> Vec<RawCandidate> {
    // Item 12: custom entries can still surface even when the
    // syllabifier produced no `keys` for the FST path (e.g. the
    // dispatcher only reaches `fetch_partial_prefix_candidates` when
    // `keys` is empty, so this fn's `keys.is_empty()` branch is dead
    // for production callers — but a future caller passing empty
    // `keys` + non-empty `custom` must still get the custom merge).
    // 中文: Item 12 — keys 空但 custom 非空時仍需合成 custom 候選 (不再 early-return)。
    if keys.is_empty() && ctx.custom.is_empty() {
        return Vec::new();
    }

    let filter = Filter::from_enabled_bitmask(ctx.enabled_sources_bitmask);
    let mut out: Vec<RawCandidate> = Vec::new();

    for (span, key) in keys {
        for rowid in ctx.prefix_index.lookup_exact(key) {
            let Some(record) = ctx.dict.record(rowid) else {
                continue;
            };
            if !DictionaryReader::passes_filter(record.bitmask, &filter) {
                continue;
            }
            // v3.5.8 — drop `tl_abbrev` acronym collisions: continuous
            // input is phonetic-syllable, not acronym (normal-mode
            // `lexicon::search` keeps acronym matching).
            if !matches_continuous_tl_toneless_key(key, &record.tl) {
                continue;
            }
            // Full-syllable path always emits `COVERAGE_KIND_FULL` — by
            // definition `valid_span_endings` produced an ending that
            // led to this `lookup_exact`. Partial-prefix hits flow
            // through `fetch_partial_prefix_candidates` instead and
            // carry `COVERAGE_KIND_PARTIAL_PREFIX`.
            // 中文: 完整音節路徑固定 COVERAGE_KIND_FULL;partial-prefix 改走另一條入口。
            out.push(record_to_candidate(
                record,
                *span,
                ctx.freq_map,
                ctx.now_ms,
                COVERAGE_KIND_FULL,
            ));
        }
    }

    // v3.5.8 Phase 9 Item 12 — merge `custom_dictionary.db` hits.
    // Each custom entry is synthesized as a full-buffer candidate
    // (`consumed_span = (0, raw_len)`, `coverage_kind =
    // COVERAGE_KIND_FULL`, `is_custom = true` → `source_tier_rank`
    // rank 0) and appended AFTER the FST hits so a `(roman, hanji)`
    // duplicate keeps the earlier-inserted `dict.bin` candidate only
    // when source ranks tie (they never do — custom rank 0 < every
    // `dict.bin` rank ≥ 1, so the custom entry always wins its
    // collision). Mirrors the legacy lexicon path's whole-input-block
    // treatment of custom dict. See
    // `docs/engine/continuous-input-ranking.md` §10.10.
    // 中文: Item 12 — custom 命中合成 full-buffer 候選 (is_custom→rank 0),append 在 dict.bin 之後;
    // 中文:   (roman,hanji) 碰撞時 custom rank 0 必勝 (見下方 dedupe)。
    for entry in ctx.custom {
        out.push(custom_entry_to_candidate(
            entry,
            raw_len,
            ctx.freq_map,
            ctx.now_ms,
            COVERAGE_KIND_FULL,
        ));
    }

    // v3.5.8 Phase 9 Item 12 — `(roman, hanji)` dedupe. `dict.bin` is
    // already collapsed by `dictionary/build/merge_csv.py:107`'s
    // `groupby(["hanzi", "_tl_key"])`, so the only realistic duplicate
    // is custom-vs-`dict.bin` sharing a `(roman, hanji)` pair. MUST run
    // BEFORE the `SortKey` sort: the winner is the lowest
    // `source_tier_rank` survivor (custom rank 0 beats any `dict.bin`
    // tier), which is NOT what the full 8-dim sort would pick (it
    // weighs `score`/`freq` ahead of `source_rank`, so a high-freq
    // `dict.bin` duplicate could otherwise mask the user's custom
    // entry). `(roman, hanji)` is the dual key (Codex pre-impl D1) so
    // romanization variants of the same hanji are preserved; S2
    // extends it with `consumed_span` (Codex pre-impl S2 Q1d) — both
    // the custom synth and its `dict.bin` duplicate are emitted at the
    // same `(0, raw_len)` span so the collapse still fires.
    // 中文: Item 12 + S2 — (roman,hanji,consumed_span) 去重;排序前;勝者 = source_tier_rank 最小 (custom rank 0 勝);
    // 中文:   custom 與 dict.bin 重複者皆在 (0,raw_len) 同 span,仍碰撞。
    dedupe_by_roman_hanji_span(&mut out);

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

/// v3.5.8 Phase 9 Item 10 — partial-prefix candidate fetch for the
/// continuous-input path. Called when the syllabifier failed to find a
/// single valid syllable ending inside `raw` (so
/// [`fetch_candidates_for_keys`] would return empty) and we want the
/// candidate strip to surface engine prefix-match hits below any
/// future full-syllable matches. See
/// `docs/engine/continuous-candidate-display.md` §15.3.D + §15.5.
///
/// Pipeline:
///
/// 1. `prefix_index.lookup_prefix(&key.1)` — FST byte-sorted rowid scan.
/// 2. Pre-cap at [`PARTIAL_PREFIX_CAP`] rowids (mirror legacy
///    `LexiconService` per-request cap; flood guard for single-char
///    prefixes like `tl:t`).
/// 3. Hydrate via `dict.record(rowid)`; drop rows that fail the
///    `enabled_sources_bitmask` filter (D-12 invariant parity with
///    [`fetch_candidates_for_keys`]).
/// 4. Build candidates with `coverage_kind = COVERAGE_KIND_PARTIAL_PREFIX`
///    and `consumed_span = key.0` (caller pins `(0, raw.len())` per
///    Q15.4 — partial-prefix candidates always final-commit).
/// 5. Sort via the same eight-dimension [`SortKey`]; the leading
///    `coverage_kind` dim is `1` here so the whole batch ranks below
///    any concurrent full-syllable hits if a caller ever merges them
///    (this fn produces partial-prefix candidates only).
///
/// `raw_len` is the byte length of the original pending buffer
/// (`Phase::Continuous { raw }.len()`); kept here for the Tier 0/1
/// (`consumed_span_end == raw_len`) downstream dim even though every
/// partial-prefix candidate has `consumed_span_end == raw_len` today,
/// so its `tier` is always 0 within `coverage_kind == 1`. This keeps
/// the entry signature symmetric with [`fetch_candidates_for_keys`].
///
/// `freq_map` + `now_ms` propagate user-frequency boost and recency
/// rank to partial-prefix hits identically to the full-syllable
/// path. An empty map + `now_ms = 0` is the cold-start neutral.
///
/// **Caller obligation** — `key.1` MUST carry an FST namespace plus
/// a non-empty body (e.g. `"tl:gu"`). Passing the bare namespace
/// (`"tl:"`) is permitted by the empty-string guard but is treated
/// as a legitimate "match every entry under the namespace" query
/// — it returns the first [`PARTIAL_PREFIX_CAP`] FST entries under
/// that prefix, which is a footgun when triggered by misuse rather
/// than design. Production callers go through
/// `composing/src/dispatch.rs::build_partial_prefix_key_tl`, which
/// returns `None` when the toneless body would be empty and
/// therefore never emits `"tl:"` alone.
// 中文: Item 10 — 部分前綴候選查詢。當 syllabifier 切不出音節邊界時 fall through 到 prefix_index.lookup_prefix。
// 中文: rowid pre-cap = PARTIAL_PREFIX_CAP (對齊 legacy LexiconService);hydrate 後 record_to_candidate 標 COVERAGE_KIND_PARTIAL_PREFIX。
// 中文: 全部候選共用 8 維 SortKey,coverage_kind 在最前面;沒有混合 full + partial 的 caller (dispatch 走互斥 branch)。
// 中文: 呼叫端責任 — key.1 須帶 namespace + 非空 body (例如 "tl:gu");bare namespace "tl:" 不會被擋,但會回前 PARTIAL_PREFIX_CAP 筆,生產路徑由 build_partial_prefix_key_tl 保證不會傳 bare namespace。
// 中文: Item 12 — custom 命中也併進 partial-prefix 路徑,標 COVERAGE_KIND_PARTIAL_PREFIX
// 中文:   (NOT FULL — 否則繞過 §15.5「partial 永遠排在 full 之下」),legacy custom dict prefix-visible 行為對齊。
// 中文: D7 改:其餘 6 個共用 arg 收進 ContinuousFetchCtx。
pub fn fetch_partial_prefix_candidates(
    key: &(ConsumedSpan, String),
    raw_len: u32,
    ctx: &ContinuousFetchCtx<'_>,
) -> Vec<RawCandidate> {
    let (span, fst_key) = key;
    if fst_key.is_empty() && ctx.custom.is_empty() {
        return Vec::new();
    }
    let filter = Filter::from_enabled_bitmask(ctx.enabled_sources_bitmask);
    let mut out: Vec<RawCandidate> = Vec::new();
    // Cap by `take(PARTIAL_PREFIX_CAP)` on the rowid stream so the
    // dict-hydration step is bounded even for single-char prefixes
    // (e.g. `tl:t`) that match thousands of FST entries. Mirrors the
    // legacy `LexiconService` per-request cap (§15.8 flood guard).
    // Filter rejects still consume budget — the goal is bounding
    // worst-case work, not maximizing hits past the cap.
    // 中文: 用 take(PARTIAL_PREFIX_CAP) 把 dict.record hydration 上限套在 rowid 流上,
    // 中文:   即使 tl:t 這種短前綴對到上千筆 FST entry 也只查前 30 筆;對齊 legacy LexiconService 限制。
    // Item 12: guard the unbounded `lookup_prefix("")` scan — with the
    // early-return now gated on `fst_key.is_empty() && custom.is_empty()`,
    // an empty `fst_key` + non-empty `custom` reaches here and must NOT
    // trigger a whole-FST scan.
    // 中文: Item 12 — fst_key 空 + custom 非空時會走到這裡,須擋掉 lookup_prefix("") 全表掃描。
    if !fst_key.is_empty() {
        for rowid in ctx
            .prefix_index
            .lookup_prefix(fst_key)
            .into_iter()
            .take(PARTIAL_PREFIX_CAP)
        {
            let Some(record) = ctx.dict.record(rowid) else {
                continue;
            };
            if !DictionaryReader::passes_filter(record.bitmask, &filter) {
                continue;
            }
            out.push(record_to_candidate(
                record,
                *span,
                ctx.freq_map,
                ctx.now_ms,
                COVERAGE_KIND_PARTIAL_PREFIX,
            ));
        }
    }

    // v3.5.8 Phase 9 Item 12 — merge `custom_dictionary.db` hits into
    // the partial-prefix path too (legacy custom dict is prefix-visible,
    // so Continuous must not hide the user's custom word while they are
    // still typing toward the first syllable boundary). Tagged
    // `COVERAGE_KIND_PARTIAL_PREFIX` — NOT `COVERAGE_KIND_FULL` — so
    // §15.5's "partial-prefix ranks strictly below full-syllable" rule
    // is preserved (Codex pre-impl D6). The span-aware dedupe then runs
    // before the sort, identical to `fetch_candidates_for_keys`.
    // 中文: Item 12 — custom 命中併入 partial-prefix,標 PARTIAL_PREFIX 不標 FULL,保 §15.5 排序不變式。
    for entry in ctx.custom {
        out.push(custom_entry_to_candidate(
            entry,
            raw_len,
            ctx.freq_map,
            ctx.now_ms,
            COVERAGE_KIND_PARTIAL_PREFIX,
        ));
    }
    dedupe_by_roman_hanji_span(&mut out);

    // Same `enumerate()`-pre-sort-stamping pattern as
    // `fetch_candidates_for_keys` to keep `stable_idx` deterministic
    // and independent of `slice::sort_by_cached_key` internals (PR-9.1
    // PR-bot R1 fix `be86f5f7`).
    let mut indexed: Vec<(SortKey, RawCandidate)> = out
        .into_iter()
        .enumerate()
        .map(|(i, c)| (SortKey::new(&c, raw_len, i as u32), c))
        .collect();
    indexed.sort_by_key(|(key, _)| *key);
    indexed.into_iter().map(|(_, c)| c).collect()
}

/// v3.5.8 S2 — single best dictionary candidate for one exact FST
/// key. Returns the highest-`score` [`record_to_candidate`] over
/// `prefix_index.lookup_exact(key)` (NaN coerced low via
/// [`NonNanF32`]; ties keep the first FST rowid for determinism), or
/// `None` when the key has no dict hit. Filter parity with the
/// production span-local path (`enabled_sources_bitmask = u32::MAX`,
/// `composing::continuous::fetch_via_lexicon_inner`).
///
/// The whole-sentence walker (`composing::lattice::walker`) calls
/// this once per lattice edge through a dispatch-injected edge
/// provider so the walker stays pure + shadow-space native and the
/// lexicon candidate construction is **reused, not duplicated**
/// (Codex pre-impl S2 Q1b, 2026-05-16). `consumed_span` is stamped
/// onto the returned candidate verbatim; the walker only reads
/// `roman` / `hanji` / `frequency` / `syllable_count` /
/// `display_text` off it.
// 中文: S2 — 單一 exact FST key 的最佳字典候選 (score 最大,NaN coerce 低,平手取首 rowid)。
// 中文: walker 每條 lattice edge 經 dispatch 注入的 provider 呼叫此函式 → walker 純 + shadow-space,
// 中文:   重用 (非複製) lexicon 候選構造 (Codex S2 Q1b)。consumed_span 原樣戳上,walker 只讀內容欄。
pub fn best_candidate_for_key(
    key: &str,
    consumed_span: ConsumedSpan,
    freq_map: &FrequencyMap,
    now_ms: i64,
    prefix_index: &PrefixIndex,
    dict: &DictionaryReader,
) -> Option<RawCandidate> {
    let filter = Filter::from_enabled_bitmask(u32::MAX);
    let mut best: Option<RawCandidate> = None;
    for rowid in prefix_index.lookup_exact(key) {
        let Some(record) = dict.record(rowid) else {
            continue;
        };
        if !DictionaryReader::passes_filter(record.bitmask, &filter) {
            continue;
        }
        // v3.5.8 — same `tl_abbrev` collision guard as the span-local
        // path so the whole-sentence walker never picks an acronym
        // record as an edge representative.
        if !matches_continuous_tl_toneless_key(key, &record.tl) {
            continue;
        }
        let cand = record_to_candidate(record, consumed_span, freq_map, now_ms, COVERAGE_KIND_FULL);
        let better = match &best {
            None => true,
            Some(b) => NonNanF32::new(cand.score) > NonNanF32::new(b.score),
        };
        if better {
            best = Some(cand);
        }
    }
    best
}

/// v3.5.8 §10.2 Option A — does the exact hanzi `hanji` resolve to a
/// dictionary entry that is **exactly two TL syllables**? Used by the
/// composing render/commit join ([`crate`] consumer
/// `composing::api::nailed_prefix`) to decide whether two adjacent
/// manually-nailed single-syllable segments reconstruct a known
/// 2-syllable compound (`查某` / `tsa-bóo`) and therefore render their
/// word boundary as an internal hyphen instead of a space.
///
/// Scans **all** `prefix_index.lookup_exact("hanzi:<hanji>")` rowids —
/// NOT [`best_candidate_for_key`], which returns a single ranking
/// winner and would make a presentation separator depend on score
/// (Codex pre-impl Q2 2026-05-18) — and returns `true` iff some record
/// has `syllable_count == 2` **and** `hanzi == Some(hanji)`. The
/// `syllable_count == 2` gate is load-bearing: many two-CJK-codepoint
/// dictionary entries are NOT two TL syllables (e.g. `先生 / sin-senn`,
/// `新婦 / sim-pū`); existence alone would over-hyphenate them (Codex
/// pre-impl N2 2026-05-18). The explicit `hanzi` re-check guards
/// against wrong rowids / future index drift; it cannot bridge
/// byte-different but visually-equivalent variant forms (an accepted
/// data-level limitation, identical to the toneless-key path).
// 中文: §10.2 Option A — 漢字 exact-key 查詢:hanji 是否為「恰好 2 音節」的詞庫詞。
// 中文: 掃全部 lookup_exact rowids(非 best_candidate_for_key — 分隔符不可依賴排序),
// 中文:   syllable_count==2 為必要閘:很多雙漢字條目非 2 音節(先生 / 新婦),
// 中文:   僅判存在會誤連;hanzi 再驗防 rowid 漂移(無法橋接位元不同的異體字)。
pub fn compound_hanji_exists(
    hanji: &str,
    prefix_index: &PrefixIndex,
    dict: &DictionaryReader,
) -> bool {
    let key = format!("hanzi:{hanji}");
    for rowid in prefix_index.lookup_exact(&key) {
        if let Some(record) = dict.record(rowid) {
            if record.syllable_count == 2 && record.hanzi.as_deref() == Some(hanji) {
                return true;
            }
        }
    }
    false
}

/// v3.5.8 — continuous-input abbreviation-collision guard. Returns
/// `true` iff `record_tl` genuinely matched the queried `key` via its
/// toneless spelling (`tl_notone`), not via its acronym (`tl_abbrev`).
///
/// `dictionary/build/create_fst.py` indexes `tl:<tl_notone>`,
/// `tl:<tl_num>` AND `tl:<tl_abbrev>` under one shared `tl:` FST
/// prefix. For NORMAL IME autocomplete (`lexicon::search::search`)
/// acronym matching is intentional — typing `gi` should surface 外夷
/// (`guā-î`, `tl_abbrev == "gi"`). For whole-sentence CONTINUOUS input
/// the user types phonetic syllables, so the first-syllable key `tl:gi`
/// also returning 外夷 is letter-mismatched noise: the candidate shares
/// no spelling with what was typed.
///
/// Keep a record only when its real TL display reduces to the queried
/// toneless body. `normalize_input` yields a per-syllable numeric-tone
/// form (e.g. `gín-á-lâng` → `gin2a2lang5`), so every ASCII tone digit
/// is dropped to reach the stored fused `tl_notone` surface
/// (`remove_tone(to_numeric_tone(tl)) == tl_notone` holds for every
/// `dictionary.csv` row — verified pre-impl).
///
/// Scope: only the `tl:` family is guarded — continuous only ever
/// builds `tl:` keys; `poj:` / `hanzi:` pass through untouched. A `tl:`
/// key whose body still carries an ASCII digit is a numeric-tone
/// (`tl:<tl_num>`) key, NOT a continuous toneless key, so the guard is
/// skipped rather than silently filtering a non-continuous caller.
// 中文: 連續輸入 abbrev 撞 key 守門 — 只有當 record 的去調拼寫真的等於查詢的 toneless key body
// 中文:   (即經 tl_notone 命中,而非 tl_abbrev 縮寫命中)才保留。
// 中文: 正常 IME 搜尋的 acronym 比對是刻意的(打 gi → 外夷),連續輸入是逐音節注音故為雜訊。
// 中文: normalize_input 產生數字調形,去掉尾端 ASCII 數字即還原成 FST 儲存的 tl_notone 面。
// 中文: 只守 tl: 族群;body 仍帶數字 = tl:<tl_num> 數字調 key,非連續 toneless,直接放行不誤殺。
fn matches_continuous_tl_toneless_key(key: &str, record_tl: &str) -> bool {
    let Some(body) = key.strip_prefix("tl:") else {
        return true;
    };
    if body.bytes().any(|b| b.is_ascii_digit()) {
        return true;
    }
    let toneless: String = phonetics::normalize_input(record_tl)
        .chars()
        .filter(|c| !c.is_ascii_digit())
        .collect();
    toneless == body
}

fn record_to_candidate(
    record: DictionaryRecord,
    consumed_span: ConsumedSpan,
    freq_map: &FrequencyMap,
    now_ms: i64,
    coverage_kind: u8,
) -> RawCandidate {
    let DictionaryRecord {
        bitmask,
        frequency,
        syllable_count,
        hanzi,
        tl,
    } = record;
    let mode = derive_mode(hanzi.as_deref());
    // Phase 9 Item 5: keep `roman` = `tl` alongside `display_text`
    // before `hanzi.unwrap_or(tl)` consumes the TL string. `hanji`
    // mirrors `DictionaryRecord.hanzi` verbatim so the proto3
    // `optional` field can preserve the absent-vs-empty distinction.
    // 中文: Item 5 — 在 hanzi.unwrap_or(tl) 移走 tl 之前 clone 一份到 roman 欄位;
    // 中文:   hanji 直接照搬 DictionaryRecord.hanzi,讓 proto optional 保留 None vs Some("")。
    let roman = tl.clone();
    let hanji = hanzi.clone();
    let display_text = hanzi.unwrap_or(tl);
    // Phase 9.3a: look up the candidate's user-frequency snapshot by
    // `display_text` (the same key the platform writes to
    // `user_frequency.db` on commit). Absent entries fall through to
    // `FrequencyData::default()` (count = 0, last_used_ms = 0) →
    // `user_freq_boost(0) = 1.0` and `recency_rank(_, 0) = 1`, which
    // reproduces the cold-start neutral behaviour.
    let freq_data = freq_map.get(&display_text).copied().unwrap_or_default();
    // `FrequencyData.count` is `i32` (legacy `calculate_score` cap
    // domain). Saturate the negative side to 0; the wire builder
    // already saturates the positive side at `i32::MAX`.
    let count_u32 = u32::try_from(freq_data.count).unwrap_or(0);
    let boost = user_freq_boost(count_u32);
    let score = calculate_continuous_score(frequency, syllable_count, boost);
    let recency = recency_rank(now_ms, freq_data.last_used_ms);
    RawCandidate {
        consumed_span,
        syllable_count,
        display_text,
        roman,
        hanji,
        score,
        form: FORM_NOTONE,
        frequency,
        bitmask,
        mode,
        recency_rank: recency,
        coverage_kind,
        // dict.bin FST hit — `source_tier_rank` derives the rank from
        // `bitmask`; `is_custom = false` keeps the kautian/taigitv/…
        // ordering. Only `custom_entry_to_candidate` sets `true`.
        // 中文: dict.bin 命中,rank 由 bitmask 推導;custom 才設 true。
        is_custom: false,
    }
}

/// v3.5.8 Phase 9 Item 12 — synthesize a [`RawCandidate`] from a
/// platform-supplied [`CustomEntry`] (`custom_dictionary.db` row).
///
/// Shape decisions (Codex pre-impl 2026-05-15, D3 / D4):
///
/// - `consumed_span = (0, raw_len)` — custom entries are outside the
///   FST/syllabifier span model, so they commit the whole buffer as
///   one block (final-commit), mirroring the legacy lexicon path's
///   treatment of custom dict and Item 10 partial-prefix Q15.4. With
///   `consumed_span_end == raw_len` the downstream `SortKey.tier` is
///   `0` (full-buffer) — NO forced Tier-1 promotion
///   (`docs/roadmap.md` § Phase 9: "無強制 Tier 1 promotion").
/// - `frequency = 0`, `syllable_count = 1` — `custom_dictionary.db`
///   carries no `dict.bin`-comparable frequency. `is_custom = true`
///   gives `source_tier_rank` rank `0`, which is what governs the
///   `(roman, hanji)` dedupe winner and prior-axis ties; it does NOT
///   globally float custom above `dict.bin` because `SortKey` weighs
///   `score`/`freq` ahead of `source_rank`. This is intentional —
///   Item 12's job is duplicate elimination + custom-wins-collision,
///   not a global custom-priority tier
///   (`docs/engine/continuous-input-ranking.md` §10.10).
/// - `display_text = hanji.unwrap_or(roman)` — identical contract to
///   [`record_to_candidate`] so commit + `user_frequency.db` write
///   keys stay wire-identical between custom and `dict.bin` commits.
/// - `mode = derive_mode(hanji)` — TAILO when `hanji` is `None`.
/// - user-frequency boost / recency are applied identically to
///   `dict.bin` candidates (custom entries can also be user-selected).
// 中文: Item 12 — custom_dictionary.db 一筆 → RawCandidate;full-buffer span (final-commit)、
// 中文:   freq=0 syll=1、is_custom=true (source rank 0,主管去重勝負與前維 tie,不全域置頂)、
// 中文:   display_text/mode/boost/recency 與 record_to_candidate 同契約。
fn custom_entry_to_candidate(
    entry: &CustomEntry,
    raw_len: u32,
    freq_map: &FrequencyMap,
    now_ms: i64,
    coverage_kind: u8,
) -> RawCandidate {
    let roman = entry.roman.clone();
    let hanji = entry.hanji.clone();
    let mode = derive_mode(hanji.as_deref());
    let display_text = hanji.clone().unwrap_or_else(|| roman.clone());
    let freq_data = freq_map.get(&display_text).copied().unwrap_or_default();
    let count_u32 = u32::try_from(freq_data.count).unwrap_or(0);
    let boost = user_freq_boost(count_u32);
    // `frequency = 0` (D4) → `calculate_continuous_score` reduces to
    // the boost-only term; the candidate floats on `source_rank` /
    // dedupe, not raw freq.
    let score = calculate_continuous_score(0, 1, boost);
    let recency = recency_rank(now_ms, freq_data.last_used_ms);
    RawCandidate {
        consumed_span: (0, raw_len),
        syllable_count: 1,
        display_text,
        roman,
        hanji,
        score,
        form: FORM_NOTONE,
        frequency: 0,
        // No `dict.bin` source bits; rank is forced to 0 via
        // `is_custom = true` in `SortKey::new` /
        // `dedupe_by_roman_hanji_span` (`source_tier_rank` short-circuits).
        // 中文: 無 dict.bin source bit;rank 由 is_custom=true 強制為 0。
        bitmask: 0,
        mode,
        recency_rank: recency,
        coverage_kind,
        is_custom: true,
    }
}

/// v3.5.8 Phase 9 Item 12 — `(roman, hanji)` dedupe (Codex pre-impl
/// D1 + D2, 2026-05-15). **v3.5.8 S2: key extended to
/// `(roman, hanji, consumed_span)`** (Codex pre-impl S2 Q1d,
/// 2026-05-16). Runs on the merged `dict.bin` + custom candidate
/// vector BEFORE the `SortKey` sort.
///
/// - **Key**: the triple `(roman, hanji, consumed_span)`. The
///   `(roman, hanji)` pair (D1) keeps romanization variants of the
///   same hanji distinct; adding `consumed_span` keeps the **same
///   word at different spans** distinct — once the whole-sentence
///   walker / path-step candidates exist (S2) the same `(roman,
///   hanji)` legitimately recurs at different spans and must NOT be
///   collapsed (which the pre-S2 `(roman, hanji)`-only key would
///   wrongly do). The Item-12 custom-vs-`dict.bin` collapse is
///   preserved: both the custom synth and its `dict.bin` duplicate
///   are emitted at the **same** full-buffer span `(0, raw_len)`
///   (see the `custom_entry_to_candidate` call sites above), so the
///   span-augmented key still collides and custom still wins by
///   source rank.
/// - **Winner** (D2): the survivor with the lowest
///   `source_tier_rank(bitmask, is_custom)` (custom = rank 0 beats
///   every `dict.bin` tier ≥ 1). On a rank tie the earlier-inserted
///   candidate wins (deterministic; `dict.bin` hits are inserted
///   before custom, so a `dict.bin`-vs-`dict.bin` tie — which
///   `merge_csv.py` already precludes in production — keeps the first
///   FST hit).
/// - Survivor **insertion order is preserved** so the downstream
///   `SortKey.stable_idx` stays deterministic.
// 中文: Item 12 + S2 — (roman,hanji,consumed_span) 三鍵去重,排序前執行;
// 中文:   加 span 讓「同詞不同 span」(walker / path-step) 不被誤併;custom-vs-dict.bin
// 中文:   仍同 (0,raw_len) span 碰撞,custom rank 0 必勝。同 rank 取較早插入者;倖存者保插入序。
fn dedupe_by_roman_hanji_span(out: &mut Vec<RawCandidate>) {
    use std::collections::HashMap;
    // key → (winning source rank, index of winner in `out`).
    let mut best: HashMap<(String, Option<String>, ConsumedSpan), (u8, usize)> =
        HashMap::with_capacity(out.len());
    for (i, c) in out.iter().enumerate() {
        let rank = source_tier_rank(c.bitmask, c.is_custom);
        let key = (c.roman.clone(), c.hanji.clone(), c.consumed_span);
        match best.get(&key) {
            // Strictly lower rank replaces; equal rank keeps the
            // earlier index (no replace) → deterministic tie-break.
            Some(&(best_rank, _)) if rank < best_rank => {
                best.insert(key, (rank, i));
            }
            None => {
                best.insert(key, (rank, i));
            }
            _ => {}
        }
    }
    if best.len() == out.len() {
        return; // no duplicates — common production path, skip rebuild.
    }
    let winners: std::collections::HashSet<usize> = best.values().map(|&(_, idx)| idx).collect();
    let mut idx = 0usize;
    out.retain(|_| {
        let keep = winners.contains(&idx);
        idx += 1;
        keep
    });
}

// ---------------------------------------------------------------------------
// v3.5.8 Phase 9.1 — SortKey
//
// Encodes the eight-dimension lexicographic sort policy pinned in
// `docs/roadmap.md` § Phase 9 (+ 整句 lattice + walker S8). Field
// order in this struct matches `#[derive(Ord)]`'s lexicographic
// comparison; `Reverse<T>` flips individual dimensions whose policy
// is descending. NaN-safe because scores are wrapped in `NonNanF32`
// which coerces NaN to `f32::MIN` at construction.
//
// Order: coverage_kind, tier, recency_rank, -score, -freq,
// -coverage, source_rank, stable_idx. S8 moved `-coverage` from
// dim 3 (above score) down to dim 6 (a weak tiebreak below
// score/freq): the slot-0 whole-sentence walker now owns phrase
// priority, so longest-coverage-first inside a tier only buried the
// short single-syllable first-segment candidate.
// ---------------------------------------------------------------------------

// 中文: Phase 9.1 / Phase 9 Item 10 排序鍵 — 8 維 lexicographic,asc/desc 由 Reverse<T> 控;NaN 在 NonNanF32 內 coerce 成 f32::MIN。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct SortKey {
    /// v3.5.8 Phase 9 Item 10 — leading dim. `0` for full-syllable
    /// (the pre-Item-10 `fetch_candidates_for_keys` path) and `1`
    /// for partial-prefix ([`fetch_partial_prefix_candidates`]).
    /// Sits ahead of [`tier`](Self::tier) because partial-prefix
    /// candidates have `consumed_span_end == raw_len`
    /// (Q15.4 → `tier = 0`); without this dim a partial-prefix
    /// tier-0 candidate would outrank a future full-syllable
    /// tier-1 candidate, violating §15.5 "rank below regardless of
    /// frequency". See `docs/engine/continuous-candidate-display.md`
    /// §15.5.
    // 中文: Item 10 — coverage_kind 必須是最前維;partial-prefix 的 consumed_span_end == raw_len → tier=0,
    // 中文:   若放在 tier 之後,partial 的 tier-0 會反超 full 的 tier-1,違反 §15.5。
    coverage_kind: u8,
    /// `0` = Tier 0 (full-buffer coverage), `1` = Tier 1 (partial).
    /// Roadmap and spec both use the "Tier 0 = full buffer" labelling
    /// (`docs/roadmap.md` § Phase 9 / `docs/engine/continuous-input-
    /// ranking.md` §1.1).
    // 中文: tier — 0 為 Tier 0 (consumed_span_end == raw_len),1 為 Tier 1 (部分覆蓋)。
    tier: u8,
    /// `0` = recent (`last_used_ms` within
    /// `ranking::RECENCY_WINDOW_MS`), `1` = stale, never used, or
    /// clock-skew. Populated by `record_to_candidate` from the
    /// caller-built `FrequencyMap` + `now_ms` (Phase 9.3a). PR-9.1
    /// carried a sentinel `1`; that contract is now lifted.
    // 中文: recency_rank — Phase 9.3a 從 FrequencyMap + now_ms 真正計算;0 = recent,1 = stale/never/clock-skew。
    recency_rank: u8,
    /// Descending: higher `freq × syll_bias × boost` wins.
    // 中文: adjusted_score desc;NaN coerce 成 f32::MIN 於 NonNanF32 內。
    neg_score: Reverse<NonNanF32>,
    /// Descending: raw freq as a secondary tie-break independent of
    /// adjusted_score (only differs when boost ≠ 1.0 once 9.3a lands).
    // 中文: freq desc 作為 adjusted_score 之外的二次 tie-break (9.3a boost 不為 1.0 後才會分歧)。
    neg_freq: Reverse<u32>,
    /// Descending: longer coverage wins — but only as a weak tiebreak
    /// AFTER `neg_score` / `neg_freq`. v3.5.8 整句 lattice + walker S8
    /// relocated this from dim 3 to here. The slot-0 whole-sentence
    /// walker owns phrase priority, so a graded longest-coverage-first
    /// rule inside a tier only buried the short single-syllable
    /// first-segment candidate the user wants for segment-by-segment
    /// selection. Coverage now separates two candidates only when their
    /// score AND freq are equal — aligned with librime's per-segment
    /// menu, which keeps multi-length candidates but never lets a
    /// longer code-length bury a shorter strict match
    /// (`references/librime/src/rime/gear/script_translator.cc`
    /// `kNumExactMatchOnTop`).
    // 中文: coverage 長度 desc,但已降為 score/freq 之後的弱 tiebreak(S8,原 dim 3)。
    // 中文:   整句優先由 slot-0 walker 負責;長覆蓋優先在 tier 內只會埋葬使用者要逐段點選的
    // 中文:   單音節首段。現只在 score+freq 同分時才分 — 對齊 librime per-segment menu
    // 中文:   (保留多長度但不讓長碼蓋短嚴格匹配,kNumExactMatchOnTop)。
    neg_coverage: Reverse<u32>,
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
        // v3.5.8 Phase 9 Item 12 — `is_custom` is now carried on the
        // candidate (`record_to_candidate` → false, dict.bin source
        // bits; `custom_entry_to_candidate` → true, forces rank 0).
        // Before Item 12 this was hardcoded `false` because no caller
        // could produce a custom candidate yet.
        // 中文: Item 12 — is_custom 改由候選帶 (custom=true→rank 0);此前無 caller 能產 custom 故硬編 false。
        let source_rank = source_tier_rank(candidate.bitmask, candidate.is_custom);
        Self {
            coverage_kind: candidate.coverage_kind,
            tier,
            recency_rank: candidate.recency_rank,
            neg_score: Reverse(NonNanF32::new(candidate.score)),
            neg_freq: Reverse(candidate.frequency),
            neg_coverage: Reverse(coverage_bytes),
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
    /// `score = 0.0`, `syllable_count = 1`, `recency_rank = 1` (stale
    /// = the cold-start default in PR-9.3a).
    fn cand(
        span_start: u32,
        span_end: u32,
        score: f32,
        frequency: u32,
        bitmask: u16,
    ) -> RawCandidate {
        cand_with_recency(span_start, span_end, score, frequency, bitmask, 1)
    }

    fn cand_with_recency(
        span_start: u32,
        span_end: u32,
        score: f32,
        frequency: u32,
        bitmask: u16,
        recency_rank: u8,
    ) -> RawCandidate {
        RawCandidate {
            consumed_span: (span_start, span_end),
            syllable_count: 1,
            display_text: String::new(),
            roman: String::new(),
            hanji: None,
            score,
            form: FORM_NOTONE,
            frequency,
            bitmask,
            mode: CandidateMode::Hant,
            recency_rank,
            coverage_kind: COVERAGE_KIND_FULL,
            is_custom: false,
        }
    }

    /// v3.5.8 Phase 9 Item 10 — partial-prefix variant for the new
    /// `coverage_kind` dim. Defaults to recency_rank=1 (stale) so
    /// tests can isolate the coverage_kind axis without mixing in
    /// recency boosts.
    // 中文: Item 10 — partial-prefix 測試 fixture helper;recency_rank 預設 1 (stale),
    // 中文:   讓 coverage_kind 維度可以單獨被驗證,不被 recency 0/1 干擾。
    fn cand_partial(
        span_start: u32,
        span_end: u32,
        score: f32,
        frequency: u32,
        bitmask: u16,
    ) -> RawCandidate {
        let mut c = cand_with_recency(span_start, span_end, score, frequency, bitmask, 1);
        c.coverage_kind = COVERAGE_KIND_PARTIAL_PREFIX;
        c
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
    fn within_tier_higher_score_beats_longer_coverage() {
        // v3.5.8 整句 lattice + walker S8 (was
        // `within_tier_longer_coverage_beats_shorter`, which pinned the
        // pre-S8 policy that caused the `guaikingkahuekhoo` dogfood
        // bug). `-coverage_bytes` is now dim 6, BELOW `-adjusted_score`
        // / `-freq`. A 3-byte coverage with score 1000 must now win
        // over a 6-byte coverage with score 100 when both are Tier 1.
        let raw_len: u32 = 9; // neither span hits full buffer
        let longer = cand(0, 6, 100.0, 100, 0);
        let shorter = cand(0, 3, 1000.0, 1000, 0);

        assert!(SortKey::new(&shorter, raw_len, 0) < SortKey::new(&longer, raw_len, 1));
    }

    #[test]
    fn single_syllable_first_segment_not_buried_by_longer_prefix() {
        // Dogfood bug pin (`guaikingkahuekhoo` → 「我」/Guá buried).
        // When no span-local candidate covers the full buffer (the
        // whole-sentence path is the slot-0 walker, prepended
        // separately), a high-freq single-syllable first-segment
        // candidate must rank ABOVE a longer left-anchored prefix
        // candidate of lower freq — so segment-by-segment selection is
        // fast. Both Tier 1; only score/freq vs coverage differ.
        let raw_len: u32 = 17; // guaikingkahuekhoo; no span hits it
        let single = cand(0, 3, 31281.0, 31281, 0); // gua → 我
        let longer_prefix = cand(0, 6, 1379.0, 1379, 0); // a 2-syll prefix
        assert!(
            SortKey::new(&single, raw_len, 0) < SortKey::new(&longer_prefix, raw_len, 1),
            "high-freq single-syllable first segment must not be buried \
             below a lower-freq longer prefix"
        );
    }

    #[test]
    fn coverage_breaks_tie_only_when_score_and_freq_equal() {
        // S8: `-coverage_bytes` survives as a weak deterministic
        // tiebreak — when score AND freq are identical, the longer
        // coverage still precedes the shorter one (same Tier 1).
        let raw_len: u32 = 9;
        let longer = cand(0, 6, 100.0, 100, 0);
        let shorter = cand(0, 3, 100.0, 100, 0);

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
    fn sort_key_reads_recency_rank_from_candidate() {
        // Phase 9.3a contract: `SortKey::new` reads `candidate
        // .recency_rank` verbatim — no sentinel, no recomputation.
        // The default `cand()` builder seeds rank = 1 (stale), and
        // `cand_with_recency` lets a test explicitly seed rank = 0.
        let raw_len: u32 = 3;
        let stale = SortKey::new(&cand(0, 3, 1.0, 1, 0), raw_len, 0);
        assert_eq!(stale.recency_rank, 1);
        let recent = SortKey::new(&cand_with_recency(0, 3, 1.0, 1, 0, 0), raw_len, 1);
        assert_eq!(recent.recency_rank, 0);
    }

    #[test]
    fn recency_rank_zero_beats_one_when_tier_coverage_equal() {
        // Phase 9.3a headline behaviour: within the same `(tier)`
        // bucket, a recently-used candidate must precede a stale one
        // even if scores otherwise tie. Post-S8, `recency_rank`
        // (dim 3) sits directly above `-adjusted_score` (dim 4), so
        // it triggers reliably here with equal score / freq / coverage
        // / bitmask (coverage is now the weak dim 6 tiebreak).
        let raw_len: u32 = 3;
        let recent = cand_with_recency(0, 3, 100.0, 100, 0, 0);
        let stale = cand_with_recency(0, 3, 100.0, 100, 0, 1);
        assert!(
            SortKey::new(&recent, raw_len, 0) < SortKey::new(&stale, raw_len, 1),
            "recency_rank=0 (recent) must precede recency_rank=1 (stale)"
        );
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

    // ----- v3.5.8 Phase 9 Item 10 — `coverage_kind` SortKey dim -----

    #[test]
    fn coverage_kind_full_beats_partial_regardless_of_other_dims() {
        // Item 10 headline invariant: a partial-prefix candidate with
        // MAX freq + MAX score + recent recency + best source rank
        // must STILL lose to a full-syllable candidate with min freq,
        // min score, stale recency, and worst source rank — the
        // leading `coverage_kind` dim is load-bearing.
        // Both candidates have `end == raw_len` so their `tier`
        // dimension is identical (0) — the regression this guards
        // against is a tier-0 partial beating a tier-1 full when
        // `coverage_kind` is mis-ordered.
        let raw_len: u32 = 3;
        let full_weak = cand_with_recency(0, 3, 0.001, 1, 0, 1);
        let partial_strong = cand_partial(0, 3, f32::MAX, u32::MAX, KAUTIAN_BIT_U16);
        // Sanity: the partial helper sets `coverage_kind = 1` while
        // the full helper leaves it at the default `0`.
        assert_eq!(full_weak.coverage_kind, COVERAGE_KIND_FULL);
        assert_eq!(partial_strong.coverage_kind, COVERAGE_KIND_PARTIAL_PREFIX);
        assert!(
            SortKey::new(&full_weak, raw_len, 0) < SortKey::new(&partial_strong, raw_len, 1),
            "Item 10: full-syllable must precede partial-prefix regardless of other dims"
        );
    }

    #[test]
    fn within_partial_prefix_inner_dims_still_apply() {
        // Inside the `coverage_kind = 1` bucket the
        // `(tier, recency, -score, -freq, -coverage, source, stable_idx)`
        // policy (post-S8 order) still drives ordering — verify with
        // two partials where only `-score` differs.
        let raw_len: u32 = 4;
        let high = cand_partial(0, 4, 100.0, 100, 0);
        let low = cand_partial(0, 4, 1.0, 1, 0);
        assert!(
            SortKey::new(&high, raw_len, 0) < SortKey::new(&low, raw_len, 1),
            "inside coverage_kind=1, higher score still wins"
        );
    }

    /// Bitmask helper for the kautian source bit (1 << 0); declared
    /// here as a local `u16` to keep the test fixture self-contained
    /// without re-importing from `ranking::score::tests`.
    const KAUTIAN_BIT_U16: u16 = 1 << 0;
}

#[cfg(test)]
mod mode_derive_tests {
    //! Hermetic unit tests for the v3.5.8 Phase 9.2 `CandidateMode`
    //! derive (`derive_mode`). Covers the four classification axes
    //! Codex co-decided 2026-05-11:
    //!
    //! 1. `hanzi.is_none()` → TAILO
    //! 2. Plain-ASCII Latin in hanzi → MIXED
    //! 3. NFC-composed Latin (e.g. `ê`) in hanzi → MIXED via NFKD
    //! 4. Fullwidth Latin (e.g. `Ａ`) in hanzi → MIXED via NFKD
    //! 5. Pure CJK → HANT
    //! 6. Digits / punctuation alone do NOT flip MIXED
    //
    // 中文: Phase 9.2 CandidateMode derive hermetic 測試;NFKD 規範後 ASCII 字母命中即 MIXED。
    use super::*;

    #[test]
    fn no_hanzi_means_tailo() {
        // Roman-only entries (`hanzi = None`, `display_text` falls back
        // to the TL field).
        assert_eq!(derive_mode(None), CandidateMode::Tailo);
    }

    #[test]
    fn pure_cjk_is_hant() {
        // Canonical hanji-only display.
        assert_eq!(derive_mode(Some("臺灣台語")), CandidateMode::Hant);
        assert_eq!(derive_mode(Some("珠仔")), CandidateMode::Hant);
        assert_eq!(derive_mode(Some("台")), CandidateMode::Hant);
    }

    #[test]
    fn plain_ascii_latin_in_hanzi_is_mixed() {
        // Real dictionary entries: `hip相`, `iah是`, `ing暗`. The first
        // Latin codepoint is plain ASCII so it would flip MIXED even
        // without NFKD; this test pins the easy path.
        assert_eq!(derive_mode(Some("hip相")), CandidateMode::Mixed);
        assert_eq!(derive_mode(Some("iah是")), CandidateMode::Mixed);
        assert_eq!(derive_mode(Some("ing暗")), CandidateMode::Mixed);
        // Hypothetical "台BAR" — Codex Q-F3 example.
        assert_eq!(derive_mode(Some("台BAR")), CandidateMode::Mixed);
    }

    #[test]
    fn s2_synth_concatenated_hanji_is_mixed_when_any_edge_has_latin() {
        // v3.5.8 S2 (Codex PR #285 P2): the whole-sentence walker
        // synthesizes the slot-0 hanji by concatenating each edge's
        // hanji and classifies the WHOLE joined string through this
        // fn (`composing::continuous::fetch_walker_slot0_inner`). A Latin
        // letter in a NON-first segment (e.g. path `臺灣` + `hip相`)
        // must still flip MIXED — equivalent to the per-edge OR and
        // matching how a single multi-syllable record would classify.
        assert_eq!(derive_mode(Some("臺灣hip相")), CandidateMode::Mixed);
        // All-CJK concatenation stays HANT (the common phrase path).
        assert_eq!(derive_mode(Some("臺灣台語")), CandidateMode::Hant);
    }

    #[test]
    fn composed_latin_in_hanzi_is_mixed_via_nfkd() {
        // Real entries `ê早` (line 22953 of dictionary.csv), `ē得`
        // (22960), `屎î` (42037). The Latin codepoint is NFC-composed
        // (e.g. `ê` = U+00EA, NOT `e` + combining circumflex), so
        // `is_ascii_alphabetic` on the original chars would miss it.
        // NFKD decomposes to base ASCII `e` / `i` + combining mark.
        assert_eq!(derive_mode(Some("ê早")), CandidateMode::Mixed);
        assert_eq!(derive_mode(Some("ē得")), CandidateMode::Mixed);
        assert_eq!(derive_mode(Some("屎î")), CandidateMode::Mixed);
    }

    #[test]
    fn fullwidth_latin_in_hanzi_is_mixed_via_nfkd() {
        // Theoretical: U+FF21..U+FF3A fullwidth Latin folds to ASCII
        // under NFKD (NOT NFD). Pins the "K/D normalization, not just
        // D" choice from Codex F3-c.
        assert_eq!(derive_mode(Some("Ａ字")), CandidateMode::Mixed);
        assert_eq!(derive_mode(Some("字Ｚ")), CandidateMode::Mixed);
    }

    #[test]
    fn digits_or_punctuation_alone_stay_hant() {
        // F3-d: `123` is HANT (no Latin LETTERS). `3Q` is MIXED via the
        // `Q`. Punctuation likewise does not flip MIXED.
        assert_eq!(derive_mode(Some("123")), CandidateMode::Hant);
        assert_eq!(derive_mode(Some("3Q")), CandidateMode::Mixed);
        assert_eq!(derive_mode(Some("。、")), CandidateMode::Hant);
    }

    #[test]
    fn empty_hanzi_string_stays_hant() {
        // Defensive: `hanzi = Some("")` (shouldn't happen but the
        // contract is "any Some without Latin letters = HANT", and an
        // empty NFKD iterator finds no ASCII alphabetic codepoint).
        assert_eq!(derive_mode(Some("")), CandidateMode::Hant);
    }

    #[test]
    fn proto_wire_value_matches_enum_discriminant() {
        // The proto enum (`CandidateMode` in composing.proto) uses
        // exactly UNSPECIFIED=0, HANT=1, TAILO=2, MIXED=3. The cast
        // here pins the wire integers so a future reshuffle of the
        // Rust `repr(u8)` discriminants would fail this test.
        assert_eq!(CandidateMode::Unspecified.to_proto_i32(), 0);
        assert_eq!(CandidateMode::Hant.to_proto_i32(), 1);
        assert_eq!(CandidateMode::Tailo.to_proto_i32(), 2);
        assert_eq!(CandidateMode::Mixed.to_proto_i32(), 3);
    }

    #[test]
    fn local_enum_matches_prost_generated_proto_enum() {
        // Cross-pin: the local `CandidateMode` (in this crate) must agree
        // byte-for-byte with `protos::engine::CandidateMode` (prost-
        // generated from `engine/protos/proto/composing.proto`). If the
        // proto definition is reshuffled the cast in
        // `raw_to_proto_candidate` (which feeds prost via `i32`) would
        // silently misroute; this test fails first.
        //
        // Per Codex post-impl finding #2 (P3, 2026-05-11).
        // 中文: 本 crate 與 prost 產生的 proto enum 對齊;reshuffle 會在這裡先 fail。
        use protos::engine::CandidateMode as ProtoCandidateMode;
        assert_eq!(
            CandidateMode::Unspecified.to_proto_i32(),
            ProtoCandidateMode::Unspecified as i32
        );
        assert_eq!(
            CandidateMode::Hant.to_proto_i32(),
            ProtoCandidateMode::Hant as i32
        );
        assert_eq!(
            CandidateMode::Tailo.to_proto_i32(),
            ProtoCandidateMode::Tailo as i32
        );
        assert_eq!(
            CandidateMode::Mixed.to_proto_i32(),
            ProtoCandidateMode::Mixed as i32
        );
    }
}

#[cfg(test)]
mod record_to_candidate_carrier_tests {
    //! v3.5.8 Phase 9 Item 5 — `record_to_candidate` populates the
    //! `roman` + `hanji` sidechannels alongside `display_text` so the
    //! proto3 wire carries both for dual-line UI render. These tests
    //! pin the field-population rule across the three `CandidateMode`
    //! axes (HANT / TAILO / MIXED).
    // 中文: Item 5 — record_to_candidate 寫 roman + hanji sidechannel 的 hermetic 測試。
    use super::*;

    fn record(tl: &str, hanzi: Option<&str>) -> DictionaryRecord {
        DictionaryRecord {
            bitmask: 0,
            frequency: 0,
            syllable_count: 1,
            hanzi: hanzi.map(str::to_owned),
            tl: tl.to_owned(),
        }
    }

    #[test]
    fn hant_record_emits_roman_and_some_hanji() {
        let cand = record_to_candidate(
            record("tâi-uân", Some("臺灣")),
            (0, 7),
            &FrequencyMap::new(),
            0,
            COVERAGE_KIND_FULL,
        );
        assert_eq!(cand.roman, "tâi-uân");
        assert_eq!(cand.hanji.as_deref(), Some("臺灣"));
        assert_eq!(cand.display_text, "臺灣");
        assert_eq!(cand.mode, CandidateMode::Hant);
    }

    #[test]
    fn tailo_record_emits_roman_and_none_hanji() {
        // `hanzi = None` → TAILO path; `display_text` falls back to TL,
        // `roman` stays equal to TL, `hanji` is wire-absent
        // (proto3 `optional` distinguishes None from Some("")).
        let cand = record_to_candidate(
            record("tāi", None),
            (0, 3),
            &FrequencyMap::new(),
            0,
            COVERAGE_KIND_FULL,
        );
        assert_eq!(cand.roman, "tāi");
        assert_eq!(cand.hanji, None);
        assert_eq!(cand.display_text, "tāi");
        assert_eq!(cand.mode, CandidateMode::Tailo);
    }

    #[test]
    fn mixed_record_emits_roman_and_hanji_with_latin() {
        // MIXED = hanji string contains Latin letters after NFKD.
        // `display_text` keeps the MIXED hanji string verbatim;
        // `roman` still equals the pure TL romanization.
        let cand = record_to_candidate(
            record("hip-siòng", Some("hip相")),
            (0, 9),
            &FrequencyMap::new(),
            0,
            COVERAGE_KIND_FULL,
        );
        assert_eq!(cand.roman, "hip-siòng");
        assert_eq!(cand.hanji.as_deref(), Some("hip相"));
        assert_eq!(cand.display_text, "hip相");
        assert_eq!(cand.mode, CandidateMode::Mixed);
    }
}

#[cfg(test)]
mod item12_custom_dedupe_tests {
    //! v3.5.8 Phase 9 Item 12 — `custom_dictionary.db` synthesis +
    //! `(roman, hanji)` dedupe. Pins Codex pre-impl decisions D1
    //! (dual `(roman, hanji)` key), D2 (lowest `source_tier_rank`
    //! winner, rank-tie → earlier insertion), D3 (full-buffer span),
    //! D4 (`frequency = 0`, `syllable_count = 1`, `is_custom` drives
    //! rank 0). Spec: `docs/engine/continuous-input-ranking.md`
    //! §10.10.
    // 中文: Item 12 — custom 合成 + (roman,hanji) 去重 hermetic 測試;鎖 D1-D4 決議。
    use super::*;

    /// Minimal non-custom `dict.bin`-shaped candidate. `bitmask` picks
    /// the source rank; all sort-noise dims are neutralized so a test
    /// isolates the dedupe / source-rank axis.
    fn dict_cand(roman: &str, hanji: Option<&str>, bitmask: u16) -> RawCandidate {
        RawCandidate {
            consumed_span: (0, 6),
            syllable_count: 1,
            display_text: hanji.unwrap_or(roman).to_owned(),
            roman: roman.to_owned(),
            hanji: hanji.map(str::to_owned),
            score: 1.0,
            form: FORM_NOTONE,
            frequency: 100,
            bitmask,
            mode: derive_mode(hanji),
            recency_rank: 1,
            coverage_kind: COVERAGE_KIND_FULL,
            is_custom: false,
        }
    }

    #[test]
    fn custom_entry_to_candidate_hant_shape() {
        // D3 + D4: full-buffer span, freq 0, syll 1, is_custom true,
        // coverage_kind passed through, display = hanji, mode HANT.
        let c = custom_entry_to_candidate(
            &CustomEntry {
                roman: "tâi-gí".to_owned(),
                hanji: Some("台語".to_owned()),
            },
            9,
            &FrequencyMap::new(),
            0,
            COVERAGE_KIND_FULL,
        );
        assert!(c.is_custom);
        assert_eq!(c.consumed_span, (0, 9));
        assert_eq!(c.frequency, 0);
        assert_eq!(c.syllable_count, 1);
        assert_eq!(c.roman, "tâi-gí");
        assert_eq!(c.hanji.as_deref(), Some("台語"));
        assert_eq!(c.display_text, "台語");
        assert_eq!(c.mode, CandidateMode::Hant);
        assert_eq!(c.coverage_kind, COVERAGE_KIND_FULL);
    }

    #[test]
    fn custom_entry_to_candidate_tailo_when_no_hanji() {
        // hanji None → display falls back to roman, mode TAILO; and
        // the partial-prefix coverage kind is honored (D6).
        let c = custom_entry_to_candidate(
            &CustomEntry {
                roman: "góa".to_owned(),
                hanji: None,
            },
            3,
            &FrequencyMap::new(),
            0,
            COVERAGE_KIND_PARTIAL_PREFIX,
        );
        assert_eq!(c.display_text, "góa");
        assert_eq!(c.hanji, None);
        assert_eq!(c.mode, CandidateMode::Tailo);
        assert_eq!(c.coverage_kind, COVERAGE_KIND_PARTIAL_PREFIX);
        assert!(c.is_custom);
    }

    #[test]
    fn dedupe_custom_wins_over_dict_collision() {
        // D2: same `(roman, hanji)` from a high-freq `dict.bin` entry
        // (kautian, rank 1) and a custom entry (rank 0). The custom
        // survivor wins regardless of the dict entry's higher freq /
        // earlier insertion.
        let mut out = vec![
            dict_cand("tâi-gí", Some("台語"), 1 << 0), // kautian, rank 1
            custom_entry_to_candidate(
                &CustomEntry {
                    roman: "tâi-gí".to_owned(),
                    hanji: Some("台語".to_owned()),
                },
                6,
                &FrequencyMap::new(),
                0,
                COVERAGE_KIND_FULL,
            ),
        ];
        dedupe_by_roman_hanji_span(&mut out);
        assert_eq!(out.len(), 1, "collision must collapse to one");
        assert!(out[0].is_custom, "custom (rank 0) must win the collision");
    }

    #[test]
    fn dedupe_dual_key_preserves_roman_variants() {
        // D1: same hanji, different roman → distinct `(roman, hanji)`
        // keys, both survive (single `display_text` key would wrongly
        // collapse them).
        let mut out = vec![
            dict_cand("tâi-gí", Some("台語"), 1 << 0),
            dict_cand("tâi-gír", Some("台語"), 1 << 0),
        ];
        dedupe_by_roman_hanji_span(&mut out);
        assert_eq!(
            out.len(),
            2,
            "roman variants of same hanji must both survive"
        );
    }

    #[test]
    fn dedupe_rank_tie_keeps_earlier_insertion_and_order() {
        // D2 tie-break: two same-rank non-custom collisions keep the
        // earlier-inserted one; unrelated entries keep insertion order
        // so the downstream `SortKey.stable_idx` stays deterministic.
        let mut first = dict_cand("a", Some("甲"), 1 << 0);
        first.frequency = 10; // earlier insertion, lower freq
        let mut second = dict_cand("a", Some("甲"), 1 << 0);
        second.frequency = 999; // later insertion, higher freq — must lose
        let other = dict_cand("b", Some("乙"), 1 << 0);
        let mut out = vec![first, other.clone(), second];
        dedupe_by_roman_hanji_span(&mut out);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].frequency, 10, "rank tie keeps earlier insertion");
        assert_eq!(out[1].roman, "b", "non-duplicate keeps its position");
    }

    #[test]
    fn dedupe_noop_when_no_duplicates() {
        let mut out = vec![
            dict_cand("a", Some("甲"), 1 << 0),
            dict_cand("b", Some("乙"), 1 << 0),
        ];
        dedupe_by_roman_hanji_span(&mut out);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn dedupe_span_aware_keeps_same_word_at_different_spans() {
        // v3.5.8 S2 (Codex pre-impl Q1d): the same `(roman, hanji)` at
        // DIFFERENT `consumed_span`s must both survive — once the
        // whole-sentence walker / path-step candidates exist the same
        // word legitimately recurs at different spans. The pre-S2
        // `(roman, hanji)`-only key would have wrongly collapsed these.
        let mut a = dict_cand("tâi", Some("台"), 1 << 0);
        a.consumed_span = (0, 3);
        let mut b = dict_cand("tâi", Some("台"), 1 << 0);
        b.consumed_span = (6, 9);
        let mut out = vec![a, b];
        dedupe_by_roman_hanji_span(&mut out);
        assert_eq!(
            out.len(),
            2,
            "same (roman,hanji) at different spans must both survive"
        );
    }

    #[test]
    fn dedupe_span_aware_still_collapses_custom_vs_dict_at_full_buffer() {
        // Regression guard for Codex Q1d: the Item-12 custom-vs-`dict.bin`
        // collapse must NOT regress under the span-augmented key. Both
        // are emitted at the SAME full-buffer span `(0, raw_len)` in
        // production (`fetch_candidates_for_keys`), so the triple key
        // still collides and custom (rank 0) still wins.
        let mut dict = dict_cand("tâi-gí", Some("台語"), 1 << 0);
        dict.consumed_span = (0, 6);
        let custom = custom_entry_to_candidate(
            &CustomEntry {
                roman: "tâi-gí".to_owned(),
                hanji: Some("台語".to_owned()),
            },
            6,
            &FrequencyMap::new(),
            0,
            COVERAGE_KIND_FULL,
        );
        let mut out = vec![dict, custom];
        dedupe_by_roman_hanji_span(&mut out);
        assert_eq!(out.len(), 1, "same-span custom/dict collision collapses");
        assert!(out[0].is_custom, "custom (rank 0) still wins the collision");
    }

    #[test]
    fn is_custom_forces_source_rank_zero_in_sortkey() {
        // The `is_custom` axis must reach `SortKey` via
        // `source_tier_rank(bitmask, is_custom)` — a custom candidate
        // (no source bits) sorts ahead of a default-source dict
        // candidate when every prior dim is equal.
        let custom = custom_entry_to_candidate(
            &CustomEntry {
                roman: "x".to_owned(),
                hanji: Some("某".to_owned()),
            },
            3,
            &FrequencyMap::new(),
            0,
            COVERAGE_KIND_FULL,
        );
        // Default-source dict candidate: same span (tier 0), same
        // score/freq/recency so only `source_rank` differs.
        let mut dict = dict_cand("y", Some("乙"), 0); // no known source bit → rank 5
        dict.consumed_span = (0, 3);
        dict.frequency = 0;
        dict.score = custom.score;
        let k_custom = SortKey::new(&custom, 3, 0);
        let k_dict = SortKey::new(&dict, 3, 1);
        assert!(
            k_custom < k_dict,
            "is_custom → source_rank 0 must outrank default source rank"
        );
    }
}

#[cfg(test)]
mod abbrev_collision_guard_tests {
    //! v3.5.8 — `matches_continuous_tl_toneless_key`: continuous input
    //! must reject `tl_abbrev` acronym collisions that share the `tl:`
    //! FST namespace with a different word's real toneless key. The
    //! motivating bug: typing `ginalangtsiahpngbesai` surfaced 外夷
    //! (`guā-î`, `tl_abbrev == "gi"`) because the first-syllable key
    //! `tl:gi` also indexes the acronym.
    use super::matches_continuous_tl_toneless_key as guard;

    #[test]
    fn genuine_single_syllable_toneless_match_is_kept() {
        // 語 `gí` → normalize "gi2" → toneless "gi" == key body.
        assert!(guard("tl:gi", "gí"));
    }

    #[test]
    fn abbrev_collision_is_rejected() {
        // 外夷 `guā-î` (`tl_abbrev == "gi"`) → toneless "guai" != "gi".
        assert!(!guard("tl:gi", "guā-î"));
        assert!(!guard("tl:gi", "guā-i")); // 外衣, same collision
    }

    #[test]
    fn genuine_multi_syllable_left_anchored_key_is_kept() {
        // 囡仔人 `gín-á-lâng` → toneless "ginalang" == key body.
        assert!(guard("tl:ginalang", "gín-á-lâng"));
    }

    #[test]
    fn no_diacritic_checked_syllable_is_kept() {
        // 鴨 `ah` (tone 4, no diacritic): both sides digitless "ah".
        assert!(guard("tl:ah", "ah"));
    }

    #[test]
    fn numeric_tone_key_skips_guard() {
        // Codex pre-impl BLOCK: a `tl:<tl_num>` key body carries an
        // ASCII digit and is NOT a continuous toneless key — the guard
        // must pass it through so a non-continuous caller is never
        // silently filtered.
        assert!(guard("tl:gua2", "guā-î"));
    }

    #[test]
    fn non_tl_family_passes_through() {
        // `poj:` / `hanzi:` keys are not continuous toneless keys.
        assert!(guard("poj:goa", "goá"));
        assert!(guard("hanzi:外夷", "guā-î"));
    }
}
