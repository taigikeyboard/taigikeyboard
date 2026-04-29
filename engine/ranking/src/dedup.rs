//! Engine + display deduplication.
//!
//! Two complementary passes:
//! - [`remove_duplicates`] runs BEFORE ranking on the merged custom +
//!   system candidate list. Dedup key is `"<roman>|<hanji>"` so two
//!   homophones with different hanji survive (`kau3 / 教` vs `kau3 / 猴`).
//! - [`remove_display_duplicates`] runs AFTER ranking when the keyboard
//!   is in TPS mode. Dedup is hanji-only so visually identical entries
//!   collapse, keeping the highest-ranked one (the caller MUST pre-sort).
//!   Entries with absent / empty hanji always survive — they render as
//!   TPS symbols and are unique by roman.
//!
//! Both functions preserve input order for surviving entries (first-seen
//! wins) and never mutate inputs.

use std::collections::HashSet;

use protos::engine::TaigiWord;

/// Drop entries whose `(roman, hanji)` pair has already been seen.
/// First occurrence wins; ordering of survivors matches input order.
///
/// CROSS-PLATFORM INVARIANT — mirrors `CandidateProcessor.removeDuplicates`
/// on both iOS (`Lexicon/Utils/CandidateProcessor.swift`) and Android
/// (`ime/dictionary/CandidateProcessor.kt`). The pipe-delimited key shape
/// must match exactly so the deduped sets align across platforms.
pub(crate) fn remove_duplicates(words: Vec<TaigiWord>) -> Vec<TaigiWord> {
    let mut seen: HashSet<String> = HashSet::with_capacity(words.len());
    let mut result: Vec<TaigiWord> = Vec::with_capacity(words.len());

    for word in words {
        let key = dedup_key(&word);
        if seen.insert(key) {
            result.push(word);
        }
    }

    result
}

/// Drop entries whose hanji has already been seen, keeping the first
/// occurrence. Words with absent or empty hanji always pass through.
///
/// MUST be called only after sorting so the highest-ranked entry per
/// hanji survives — this matches the platform contract documented in
/// `CandidateProcessor.removeDisplayDuplicates`.
pub(crate) fn remove_display_duplicates(words: Vec<TaigiWord>) -> Vec<TaigiWord> {
    let mut seen_hanji: HashSet<String> = HashSet::with_capacity(words.len());
    let mut result: Vec<TaigiWord> = Vec::with_capacity(words.len());

    for word in words {
        match word.hanji.as_deref() {
            None | Some("") => result.push(word),
            Some(hanji) => {
                if seen_hanji.insert(hanji.to_owned()) {
                    result.push(word);
                }
            }
        }
    }

    result
}

#[inline]
fn dedup_key(word: &TaigiWord) -> String {
    let hanji = word.hanji.as_deref().unwrap_or("");
    let mut key = String::with_capacity(word.roman.len() + 1 + hanji.len());
    key.push_str(&word.roman);
    key.push('|');
    key.push_str(hanji);
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(id: i64, roman: &str, hanji: Option<&str>) -> TaigiWord {
        TaigiWord {
            id,
            roman: roman.to_owned(),
            hanji: hanji.map(str::to_owned),
            length_score: None,
            source_bitmask: None,
        }
    }

    #[test]
    fn remove_duplicates_drops_exact_repeats() {
        let words = vec![
            word(1, "tâi-gí", Some("台語")),
            word(2, "tâi-gí", Some("台語")),
            word(3, "tâi-gí", Some("臺語")),
            word(4, "tâi-gí", None),
            word(5, "tâi-gí", None),
        ];
        let result = remove_duplicates(words);
        assert_eq!(result.len(), 3);
        // Survivors preserve first-seen order.
        assert_eq!(result[0].id, 1);
        assert_eq!(result[1].id, 3);
        assert_eq!(result[2].id, 4);
    }

    #[test]
    fn remove_duplicates_keeps_homophones_with_different_hanji() {
        let words = vec![
            word(1, "kau3", Some("教")),
            word(2, "kau3", Some("猴")),
        ];
        let result = remove_duplicates(words);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn remove_duplicates_keeps_distinct_roman_only_entries() {
        let words = vec![word(1, "hello", None), word(2, "world", None)];
        assert_eq!(remove_duplicates(words).len(), 2);
    }

    #[test]
    fn remove_duplicates_drops_repeat_roman_only_entries() {
        let words = vec![word(1, "hello", None), word(2, "hello", None)];
        assert_eq!(remove_duplicates(words).len(), 1);
    }

    #[test]
    fn remove_duplicates_treats_absent_and_empty_hanji_as_same_key() {
        // Both serialize to "tâi-gí|" so they are the same dedup key.
        // Mirrors Swift `word.hanzi ?? ""` and Kotlin `word.hanzi ?: ""`.
        let words = vec![word(1, "tâi-gí", None), word(2, "tâi-gí", Some(""))];
        let result = remove_duplicates(words);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }

    #[test]
    fn display_dedup_keeps_first_per_hanji() {
        // Pre-sorted so the higher-ranked entry comes first.
        let words = vec![
            word(1, "phuānn-tshiú", Some("伴手")),
            word(2, "phuǎnn-tshiú", Some("伴手")),
        ];
        let result = remove_display_duplicates(words);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id, 1);
    }

    #[test]
    fn display_dedup_keeps_words_without_hanji() {
        let words = vec![
            word(1, "hello", None),
            word(2, "world", None),
            word(3, "guá", Some("我")),
        ];
        let result = remove_display_duplicates(words);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn display_dedup_treats_empty_hanji_as_no_hanji() {
        let words = vec![word(1, "one", Some("")), word(2, "two", Some(""))];
        let result = remove_display_duplicates(words);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn display_dedup_collapses_repeated_hanji_keeps_first() {
        let words = vec![
            word(1, "hello", None),
            word(2, "hello", None),
            word(3, "同", Some("相同")),
            word(4, "x", Some("相同")),
        ];
        let result = remove_display_duplicates(words);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].id, 1);
        assert_eq!(result[1].id, 2);
        assert_eq!(result[2].id, 3);
    }
}
