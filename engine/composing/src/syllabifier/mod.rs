//! Pure-function syllable boundary scanners for v3.5.8 連續輸入 Phase 3.
//!
//! TL and TPS share one segmenter shape: BFS over a `SyllableInventory`
//! where each chain link `lowered[cur..end]` is accepted only if the
//! mode-family branch of the inventory contains it. Caller must supply
//! input already canonicalized for the chosen mode (TL ASCII for
//! TL/English via `phonetics::canonicalize_syllable`; POJ ASCII for
//! POJ via `phonetics::canonicalize_poj_syllable`; Bopomofo for TPS via
//! `phonetics::canonicalize_tps_syllable` at build time). v3.5.9 B-1
//! added the POJ axis and B-2 made the inventory mode-aware via
//! `SyllableInventory::contains_in(mode, ..)`.
//!
//! - `tl::valid_span_endings_lowered` — used for `Tl` / `Poj` /
//!   `English`. Probes the `tl:` or `poj:` family per call.
//! - `tps::valid_span_endings_lowered` — used for `Tps`. Probes the
//!   `tps:` family; the toneless / numeric-tone dual-emit comes from
//!   `dictionary/build/create_syllables_fst.py`.
//!
//! Both functions return `Vec<usize>` of ascending, deduplicated byte
//! offsets. In production these endings flow into
//! `composing::continuous::assemble_candidates`, which builds the
//! mode-aware `<prefix>:<toneless>` keys and calls
//! `lexicon::fetch_candidates_for_keys` directly. The scanners never
//! panic on partial UTF-8 or out-of-range `pos`; see each entry's
//! `pos` validation contract.
//!
//! Direction-first alignment (CLAUDE.md Core Principle #6 + Codex
//! pre-impl 2026-05-27): TL+POJ+TPS now share one segmenter shape,
//! matching librime DAG (`references/librime/src/rime/algo/syllabifier.cc`),
//! khiin-rs DP over known words
//! (`references/khiin-rs/khiin/src/data/segmenter.rs`), and McBopomofo
//! ReadingGrid unigram-backed spans
//! (`references/McBopomofo/Source/Engine/gramambular2/reading_grid.cpp`).
//! The pre-fix TPS structural pre-scan ("next initial seen") could not
//! split toneless `ㄉㄞ|ㄨㄢ` because medial `ㄨ` is not a TPS initial.

// 中文: v3.5.8 連續輸入 Phase 3 的純函式音節邊界掃描器,TL / TPS 共用 inv-driven BFS 同型實作。
// 中文: 修復前 TPS 是 structural pre-scan,無法切 ㄉㄞ|ㄨㄢ;改成 inv-driven 後對齊 TL,
// 中文:   亦對齊 librime / khiin-rs / McBopomofo。

pub mod tl;
pub mod tps;

use lexicon::SyllableInventory;
use phonetics::InputMode;

/// v3.5.9 D / C-3b — unified mode-aware syllabifier entry point.
///
/// Dispatches `valid_span_endings_lowered` to the family-correct scanner:
/// - `Tl | Poj | English` → [`tl::valid_span_endings_lowered`]
///   (BFS over the `tl:` / `poj:` family).
/// - `Tps` → [`tps::valid_span_endings_lowered`] (BFS over the `tps:`
///   family — same inv-driven shape as TL post-fix 2026-05-27, replacing
///   the legacy structural pre-scan that could not segment medial-led
///   second syllables).
///
/// All callers downstream of [`crate::lattice::build_lattice`] (lattice
/// BFS, walker greedy / min-syllable derivations) route through here so
/// the syllabification family stays tied to the same `mode` parameter
/// that drives `composing::shadow::mode_key_prefix` and the emitted
/// `<prefix>:<toneless>` FST key — one mode parameter, one family, no
/// drift.
// 中文: 統一 mode-aware syllabifier 入口。TL/POJ/English 與 TPS 共用 inv-driven BFS。
// 中文: 所有 lattice 下游(BFS、walker greedy / min-syllable)都透過本入口路由,
// 中文:   shadow::mode_key_prefix 與 FST 家族鍵共用同一 mode,單一參數不分歧。
pub(crate) fn valid_span_endings_lowered(
    lowered: &str,
    pos: usize,
    inv: &SyllableInventory,
    mode: InputMode,
    max_syllables: usize,
) -> Vec<usize> {
    valid_span_endings_lowered_with_barriers(lowered, pos, inv, mode, max_syllables, &[])
}

/// [`valid_span_endings_lowered`] plus the stripped-separator barriers
/// (§35): the TPS scanner refuses any single-syllable link that crosses
/// a barrier and expands ambiguity families otherwise. The TL scanner
/// ignores barriers entirely — TL / POJ / English never strip a
/// separator, callers always pass an empty slice, and the TL primitive
/// is untouched (§18 fix-location: the shared TL path stays
/// byte-identical).
// 中文: valid_span_endings_lowered + barrier(§35)。TPS 掃描器拒絕跨 barrier 的單音節
// 中文:   並展開歧義家族;TL 端完全忽略(呼叫端恆傳空)且原函式一字不動(§18 教訓)。
pub(crate) fn valid_span_endings_lowered_with_barriers(
    lowered: &str,
    pos: usize,
    inv: &SyllableInventory,
    mode: InputMode,
    max_syllables: usize,
    barriers: &[usize],
) -> Vec<usize> {
    match mode {
        InputMode::Tps => {
            tps::valid_span_endings_lowered(lowered, pos, inv, mode, max_syllables, barriers)
        }
        InputMode::Tl | InputMode::Poj | InputMode::English => {
            tl::valid_span_endings_lowered(lowered, pos, inv, mode, max_syllables)
        }
    }
}
