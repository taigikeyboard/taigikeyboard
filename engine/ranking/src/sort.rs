//! Stable descending-by-total sort with caller-supplied frequency snapshot.
//!
//! Single source of truth for `sortByScore`; the v3.5.2 ranking slice
//! collapsed both platform mirrors into this crate (Android mirror
//! deleted PR #192, iOS residual is unrelated). Pure: takes a
//! `Vec<TaigiWord>` plus the frequency map and `now_ms`, returns a
//! reordered `Vec<TaigiWord>` paired with each candidate's score
//! breakdown so the orchestrator can optionally serialize breakdowns for
//! debug logging. Stable: equal totals preserve input order, matching
//! Swift `Array.sorted(by:)` and Kotlin `sortedByDescending` semantics.

use protos::engine::{ScoreBreakdown, TaigiWord};

use crate::score::{self, FrequencyMap};

/// Score every input word, sort descending by total, return both the
/// sorted words and their score breakdowns (parallel arrays).
///
/// When `include_breakdown` is `false`, the returned breakdown vector is
/// empty (`Vec::new()`); the sort itself only needs each candidate's
/// total, so release builds skip the per-candidate `Vec<ScoreBreakdown>`
/// allocation entirely. When `true`, the parallel breakdown vector is
/// populated and aligned 1:1 with the sorted words.
///
/// Stable sort: `slice::sort_by` (used here) is guaranteed-stable in
/// std, matching the platform implementations' `sorted` semantics.
pub(crate) fn sort_by_score(
    words: Vec<TaigiWord>,
    normalized_input: &str,
    freq_map: &FrequencyMap,
    now_ms: i64,
    include_breakdown: bool,
) -> (Vec<TaigiWord>, Vec<ScoreBreakdown>) {
    if include_breakdown {
        sort_with_breakdown(words, normalized_input, freq_map, now_ms)
    } else {
        (
            sort_totals_only(words, normalized_input, freq_map, now_ms),
            Vec::new(),
        )
    }
}

fn sort_with_breakdown(
    words: Vec<TaigiWord>,
    normalized_input: &str,
    freq_map: &FrequencyMap,
    now_ms: i64,
) -> (Vec<TaigiWord>, Vec<ScoreBreakdown>) {
    let mut paired: Vec<(TaigiWord, ScoreBreakdown)> = words
        .into_iter()
        .map(|word| {
            let key = display_text_key(&word);
            // R5 pair-key (#7): the `(display_text, canonical_tl)` identity.
            // `word.roman` is the candidate's canonical TL in this crate
            // (see `display_text_key`). Tolerant `get` falls back to the
            // legacy `tl == ""` bucket on an exact miss.
            let freq = freq_map.get(&key, &word.roman);
            let breakdown = score::calculate_score(&word, normalized_input, freq, now_ms);
            (word, breakdown)
        })
        .collect();

    // Descending by total. `sort_by` is stable, so equal-total entries
    // preserve their input order — matches Swift / Kotlin `sorted`.
    paired.sort_by_key(|(_, b)| std::cmp::Reverse(score::total(b)));

    let mut sorted_words = Vec::with_capacity(paired.len());
    let mut breakdowns = Vec::with_capacity(paired.len());
    for (word, breakdown) in paired {
        sorted_words.push(word);
        breakdowns.push(breakdown);
    }
    (sorted_words, breakdowns)
}

/// Hot-path variant: keep only `i32` totals alongside each word, drop
/// the `ScoreBreakdown` after consuming its total. Saves one allocation
/// plus the per-word `ScoreBreakdown` struct copy in release builds where
/// the caller did not request breakdowns.
fn sort_totals_only(
    words: Vec<TaigiWord>,
    normalized_input: &str,
    freq_map: &FrequencyMap,
    now_ms: i64,
) -> Vec<TaigiWord> {
    let mut paired: Vec<(TaigiWord, i32)> = words
        .into_iter()
        .map(|word| {
            let key = display_text_key(&word);
            // R5 pair-key (#7): the `(display_text, canonical_tl)` identity.
            // `word.roman` is the candidate's canonical TL in this crate
            // (see `display_text_key`). Tolerant `get` falls back to the
            // legacy `tl == ""` bucket on an exact miss.
            let freq = freq_map.get(&key, &word.roman);
            let total = score::total(&score::calculate_score(
                &word,
                normalized_input,
                freq,
                now_ms,
            ));
            (word, total)
        })
        .collect();

    paired.sort_by(|(_, a), (_, b)| b.cmp(a));
    paired.into_iter().map(|(word, _)| word).collect()
}

/// Display-text component of the user-frequency PAIR key. Mirrors
/// `TaigiWord.displayText` on both platforms (`hanji` if non-empty else
/// `roman`). The full identity is `(display_text_key, word.roman)` —
/// `word.roman` is the candidate's canonical TL in this crate, so the
/// pair distinguishes 一字多音 (#7). NOTE: the non-Continuous
/// `process_candidates` path that feeds `sort_by_score` is test-only on
/// both platforms (no production caller); production user-frequency
/// scoring flows through `lexicon::continuous`, which keys on
/// `RawCandidate.canonical_tl`. Keep `word.roman` canonical-TL if this
/// path is ever revived so the read key matches the platform write key.
pub(crate) fn display_text_key(word: &TaigiWord) -> String {
    match word.hanji.as_deref() {
        Some(h) if !h.is_empty() => h.to_owned(),
        _ => word.roman.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::score::FrequencyData;

    fn word(id: i64, roman: &str, hanji: Option<&str>, length_score: Option<i32>) -> TaigiWord {
        TaigiWord {
            id,
            roman: roman.to_owned(),
            hanji: hanji.map(str::to_owned),
            length_score,
            source_bitmask: None,
        }
    }

    #[test]
    fn sort_honors_caller_supplied_frequency_data() {
        // Two homophones, both with completion penalty (input "gua" vs
        // candidates "gua" — actually both exact). One has user-frequency
        // hits, must lead.
        let cold = word(1, "gua", Some("瓜"), Some(100));
        let learned = word(2, "gua", Some("我"), Some(10));
        let mut freq_map = FrequencyMap::new();
        // Pair key: display_text "我" + canonical TL = candidate roman "gua".
        freq_map.insert(
            "我".to_owned(),
            "gua".to_owned(),
            FrequencyData {
                count: 5,
                last_used_ms: 0,
            },
        );
        let now = 10_000_000_000;

        let (sorted, _) = sort_by_score(vec![cold, learned], "gua", &freq_map, now, false);
        assert_eq!(sorted[0].id, 2, "learned word must lead");
        assert_eq!(sorted[1].id, 1, "cold word trails");
    }

    #[test]
    fn sort_kautian_beats_itaigi_at_comparable_frequency() {
        // Direct port of Android `test_kautian_beats_itaigi_at_comparable_frequency`.
        // Kautian lengthScore=100 → baseFreqScore = 15.
        // Itaigi lengthScore=110 → baseFreqScore = 11.
        // Kautian wins despite slightly lower raw frequency.
        let kautian = TaigiWord {
            id: 1,
            roman: "tl-a".to_owned(),
            hanji: Some("A".to_owned()),
            length_score: Some(100),
            source_bitmask: Some(1 << 0), // kautian
        };
        let itaigi = TaigiWord {
            id: 2,
            roman: "tl-b".to_owned(),
            hanji: Some("B".to_owned()),
            length_score: Some(110),
            source_bitmask: Some(1 << 2), // itaigi
        };
        let (sorted, _) = sort_by_score(vec![itaigi, kautian], "z", &FrequencyMap::new(), 0, false);
        assert_eq!(sorted[0].hanji.as_deref(), Some("A"));
    }

    #[test]
    fn sort_is_stable_for_equal_totals() {
        // Two identical-score entries — relative order must match input.
        let a = word(1, "x", None, Some(50));
        let b = word(2, "x", None, Some(50));
        let (sorted, _) = sort_by_score(vec![a, b], "y", &FrequencyMap::new(), 0, false);
        assert_eq!(sorted[0].id, 1);
        assert_eq!(sorted[1].id, 2);
    }

    #[test]
    fn display_text_key_prefers_hanji_when_nonempty() {
        let with_hanji = word(0, "abc", Some("漢"), None);
        let empty_hanji = word(0, "abc", Some(""), None);
        let no_hanji = word(0, "abc", None, None);
        assert_eq!(display_text_key(&with_hanji), "漢");
        assert_eq!(display_text_key(&empty_hanji), "abc");
        assert_eq!(display_text_key(&no_hanji), "abc");
    }

    #[test]
    fn breakdowns_are_aligned_with_sorted_words_when_requested() {
        let high = word(1, "x", None, Some(100));
        let low = word(2, "y", None, Some(0));
        let (sorted, breakdowns) =
            sort_by_score(vec![low, high], "z", &FrequencyMap::new(), 0, true);
        // High base_freq_score must lead; breakdowns parallel.
        assert_eq!(sorted[0].id, 1);
        assert_eq!(breakdowns.len(), sorted.len());
        assert!(breakdowns[0].base_freq_score > breakdowns[1].base_freq_score);
    }

    #[test]
    fn breakdowns_are_omitted_when_not_requested() {
        let a = word(1, "x", None, Some(100));
        let b = word(2, "y", None, Some(0));
        let (sorted, breakdowns) = sort_by_score(vec![b, a], "z", &FrequencyMap::new(), 0, false);
        // Sort still runs; breakdowns vector is empty (release-path
        // allocation skipped).
        assert_eq!(sorted[0].id, 1);
        assert!(breakdowns.is_empty());
    }
}
