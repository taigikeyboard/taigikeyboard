//! TL syllabifier — BFS over a `SyllableInventory` returning every span
//! ending reachable from `pos` by a chain of 1..=`max_syllables` valid
//! TL syllables.
//!
//! Why BFS rather than longest-match: `tsua` typed by the user must surface
//! both `珠 (tsu, span=3)` and `紙 / 珠仔 (tsua, span=4)` as Phase 5 candidate
//! sources — pure longest-match (khiin-rs `references/khiin-rs/khiin/src/data/segmenter.rs:122`)
//! would commit to `tsua` and lose the `珠` candidate. Global lattice
//! (librime `src/rime/algo/syllabifier.cc`) is over-built for our scope.
//! BFS with depth cap = canonical middle ground per `docs/releases/v3.5.8/plan.md`
//! § Phase 3 — pure-function syllabifier design decision.

use std::collections::{BTreeSet, VecDeque};

use lexicon::SyllableInventory;
use phonetics::InputMode;

/// Upper bound on a single TL syllable's byte length: max initial
/// `tsh` (3) + max final `uainnh` / `iaunnh` (6) + optional ASCII tone
/// digit (1) = 10 bytes. Sourced from `engine/phonetics/src/tables.rs:13`
/// (TL_INITIALS) and `tables.rs:21-33` (TL_FINALS); revisit if either
/// table grows.
const MAX_SYLLABLE_BYTES: usize = 10;

/// Return every byte offset `e > pos` reachable from `pos` by a chain
/// of 1..=`max_syllables` syllables, where each chain link
/// `input[cur..end]` (after ASCII lowercasing) is a member of the
/// `mode`-family branch of the v3.5.9 B-1 tagged-single-FST syllable
/// inventory (`tl:` for `InputMode::Tl` / `InputMode::English`,
/// `poj:` for `InputMode::Poj`).
///
/// Contract:
/// - Returns ascending, deduplicated byte offsets.
/// - Returns empty `Vec` when `pos > input.len()`, `pos == input.len()`,
///   `max_syllables == 0`, or `pos` is not on a UTF-8 char boundary.
/// - Caller must pass input canonicalized for the chosen `mode`
///   (TL: lowercase ASCII, POJ→TL fold already applied via
///   `phonetics::canonicalize_syllable`; POJ: POJ ASCII via
///   `phonetics::canonicalize_poj_syllable`). Non-matching bytes won't
///   land in the inventory and silently produce no endings — no panic.
///
/// Algorithm: FIFO BFS using `endings` itself as the visited set —
/// `BTreeSet::insert` returns `true` only on first arrival, which under
/// unit edge costs is also the minimum depth. Time: O(n × MAX_SYLLABLE_BYTES)
/// FST lookups, each O(syllable_len). Allocation: one
/// `to_ascii_lowercase` pass on `input`.
pub fn valid_span_endings(
    input: &str,
    pos: usize,
    inv: &SyllableInventory,
    mode: InputMode,
    max_syllables: usize,
) -> Vec<usize> {
    // Guard on the raw `input` before allocating the lowercased copy
    // (`to_ascii_lowercase` is byte-length + UTF-8-boundary
    // preserving, so the guard is equivalent either side, but
    // checking first skips the alloc on the early-out paths).
    if max_syllables == 0 || pos >= input.len() || !input.is_char_boundary(pos) {
        return Vec::new();
    }
    valid_span_endings_lowered(&input.to_ascii_lowercase(), pos, inv, mode, max_syllables)
}

/// Pre-lowered variant of [`valid_span_endings`]: the caller has
/// already ASCII-lowercased `lowered`, so this skips the per-call
/// `to_ascii_lowercase()` allocation. The lattice builder calls this
/// once per reachable BFS start against a single lowercased shadow
/// (the public wrapper would otherwise re-lower the whole buffer for
/// every start — Codex PR #284 P1, `r3252344518`). Behavior is
/// identical to [`valid_span_endings`] on already-lowercase ASCII
/// input; the same early guard is repeated here because this is a
/// `pub(crate)` entry point (BFS callers pass `pos == lowered.len()`
/// at chain ends and rely on the guard to terminate).
pub(crate) fn valid_span_endings_lowered(
    lowered: &str,
    pos: usize,
    inv: &SyllableInventory,
    mode: InputMode,
    max_syllables: usize,
) -> Vec<usize> {
    if max_syllables == 0 || pos >= lowered.len() || !lowered.is_char_boundary(pos) {
        return Vec::new();
    }

    let bytes = lowered.as_bytes();
    let mut endings: BTreeSet<usize> = BTreeSet::new();
    let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
    queue.push_back((pos, 0));

    while let Some((cur, depth)) = queue.pop_front() {
        if depth >= max_syllables {
            continue;
        }
        let upper = (cur + MAX_SYLLABLE_BYTES).min(lowered.len());
        for end in (cur + 1)..=upper {
            if !lowered.is_char_boundary(end) {
                continue;
            }
            if inv.contains_in(mode, &lowered[cur..end])
                && !is_false_toneless_boundary(bytes, end)
                && endings.insert(end)
            {
                queue.push_back((end, depth + 1));
            }
        }
    }

    endings.into_iter().collect()
}

/// True when the FST hit at `..end` is a toneless syllable match that
/// sits immediately before an ASCII tone digit `1..=9`. Such matches
/// are false boundaries — the tone digit belongs to the matched
/// syllable, and the FST will also contain the longer numeric form
/// (e.g. `tai` followed by `5` is really `tai5`). Per
/// `phonetics::syllable.rs:18-20` only digits 1..=9 carry tone
/// meaning; '0' is not a tone marker.
fn is_false_toneless_boundary(bytes: &[u8], end: usize) -> bool {
    let last_is_tone_digit =
        matches!(bytes.get(end - 1), Some(b) if b.is_ascii_digit() && *b != b'0');
    if last_is_tone_digit {
        return false;
    }
    matches!(bytes.get(end), Some(b) if b.is_ascii_digit() && *b != b'0')
}
