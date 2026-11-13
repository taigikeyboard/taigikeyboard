//! Compound check for the nailed-prefix join: does a Hanji run resolve to one
//! dictionary word of the given syllable count.

use phonetics::HANJI_KEY_PREFIX;

use crate::dictionary_reader::{DictionaryReader, Filter};
use crate::prefix_index::PrefixIndex;

/// Does the exact hanji `hanji` resolve to a dictionary entry of
/// **exactly `syllable_count` TL syllables**? Used by the composing
/// render/commit join ([`crate`] consumer `composing::nailed::nailed_prefix`)
/// to decide whether a contiguous run of manually-nailed single-syllable
/// segments reconstructs a known n-syllable compound (`紅尾冬` /
/// `âng-bóe-tang`, `查某` / `tsa-bóo`) and therefore render its internal
/// boundaries as hyphens instead of spaces.
///
/// `syllable_count < 2` returns `false` unconditionally (a single
/// syllable is not a "compound" by definition; the caller never queries
/// `n=1`).
///
/// Scans **all** `prefix_index.lookup_exact("hanzi:<hanji>")` rowids —
/// NOT [`best_candidate_for_key_with_barriers`], which returns a single ranking
/// winner and would make a presentation separator depend on score
/// (Codex pre-impl Q2 2026-05-18) — and returns `true` iff some record
/// has `syllable_count == <argument>` **and** `hanji == Some(hanji)`.
/// The `syllable_count` gate is load-bearing: many two-CJK-codepoint
/// dictionary entries are NOT two TL syllables (e.g. `先生 / sin-senn`,
/// `新婦 / sim-pū`); existence alone would over-hyphenate them (Codex
/// pre-impl N2 2026-05-18). The explicit `hanji` re-check guards
/// against wrong rowids / future index drift; it cannot bridge
/// byte-different but visually-equivalent variant forms (an accepted
/// data-level limitation, identical to the toneless-key path).
///
/// **v3.5.9 longest-match extension** — previously fixed at 2 syllables
/// (`§10.2 Option A`), now parameterized so the caller's longest-match
/// loop can ask "is `hanji` an n-syllable compound?" for `n >= 2`. Peer
/// IMEs (khiin-rs `autospace`, librime spelling-algebra DAG, McBopomofo
/// `ReadingGrid`) handle multi-syllable compounds at segmentation /
/// lattice-walk time instead of at commit-join time; this extension is
/// the manual-nail-flow-compatible analog and is intentionally narrower
/// in scope (an isolated UX heuristic, not a general best practice).
///
/// **#772 — word identity, source toggles.** A record counts only when it
/// is the same word the picks spell (Core Principle #6: `(漢字, TL)`):
/// its `tl` must equal `reading` up to separators
/// ([`phonetics::api::roman_reading_eq`], tone-preserving), and it must
/// pass the user's source `filter` (the same [`Filter`] the candidate
/// fetch applies). A compound only a disabled
/// source carries — 俺娘 `án-niâ` with Taihoa off — no longer hyphenates
/// a 俺 + 娘 run the walker itself renders `án niâ`. An empty `reading`
/// matches nothing.
///
/// [`best_candidate_for_key_with_barriers`]: super::best_candidate_for_key_with_barriers
pub fn compound_hanji_exists(
    hanji: &str,
    reading: &str,
    syllable_count: u8,
    filter: &Filter,
    prefix_index: &PrefixIndex,
    dict: &DictionaryReader,
) -> bool {
    if syllable_count < 2 || reading.is_empty() {
        return false;
    }
    let key = format!("{HANJI_KEY_PREFIX}{hanji}");
    prefix_index.lookup_exact(&key).into_iter().any(|rowid| {
        dict.record(rowid).is_some_and(|record| {
            record.syllable_count == syllable_count
                && record.hanji.as_deref() == Some(hanji)
                && DictionaryReader::passes_filter(record.bitmask, record.kautian_subtag, filter)
                && phonetics::api::roman_reading_eq(&record.tl, reading)
        })
    })
}
