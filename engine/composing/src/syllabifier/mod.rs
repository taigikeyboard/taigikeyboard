//! Pure-function syllable boundary scanners for v3.5.8 continuous input Phase 3.
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
//! `lexicon::fetch_candidates_for_keys_with_barriers` directly. The scanners never
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

pub mod tl;
pub mod tps;

use lexicon::SyllableInventory;
use phonetics::InputMode;

/// The family-correct scanner for `mode`, with the stripped-separator
/// barriers: no single-syllable link crosses one in either family. TPS (§35) also
/// expands ambiguity families and marks the glyph before a barrier
/// Final-only; TL / POJ (§52) only refuse the crossing — a typed `-` is
/// the user's syllable boundary (`khi--ah` is never read `khiah`);
/// English ignores them. An empty slice leaves every scanner
/// byte-identical to its barrier-free form.
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
            let barriers = if typed_hyphen_is_boundary(mode) {
                barriers
            } else {
                &[]
            };
            tl::valid_span_endings_lowered(lowered, pos, inv, mode, max_syllables, barriers)
        }
    }
}

/// A barrier strictly inside `cur..end`: the one cut no single syllable
/// may span in either family (TPS §35 part (a), TL / POJ §52). Chains
/// still meet AT a barrier.
pub(super) fn crosses_barrier(barriers: &[usize], cur: usize, end: usize) -> bool {
    barriers.iter().any(|&b| cur < b && b < end)
}

/// §52 policy, authored once: TL / POJ read a typed `-` as a syllable
/// boundary (scanner, OOV readings, lookup pin); TPS has its own §35
/// barrier contract in its scanner and keeps its OOV readings blind;
/// English never pins a boundary, so a one-layer split would surface
/// nothing.
pub(crate) fn typed_hyphen_is_boundary(mode: InputMode) -> bool {
    matches!(mode, InputMode::Tl | InputMode::Poj)
}
