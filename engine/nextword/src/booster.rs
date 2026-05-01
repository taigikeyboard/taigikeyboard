//! Pure first-character partition reorder. Mirrors iOS
//! `AutocompleteContextBooster.boost` and Android
//! `AutocompleteContextBooster.boost`. Engine state untouched.

use std::collections::HashSet;

/// Reorder `words` so entries whose first Unicode scalar is in
/// `predicted_first_chars` float to the top, preserving original order
/// within both partitions. Empty `predicted_first_chars` short-circuits to
/// the original list (matches iOS / Android behavior).
pub(crate) fn boost_words(words: Vec<String>, predicted_first_chars: Vec<String>) -> Vec<String> {
    if predicted_first_chars.is_empty() {
        return words;
    }
    let predicted: HashSet<String> = predicted_first_chars.into_iter().collect();
    let mut boosted: Vec<String> = Vec::with_capacity(words.len());
    let mut rest: Vec<String> = Vec::with_capacity(words.len());
    for word in words {
        let first_char = word
            .chars()
            .next()
            .map(|c| c.to_string())
            .unwrap_or_default();
        if !first_char.is_empty() && predicted.contains(&first_char) {
            boosted.push(word);
        } else {
            rest.push(word);
        }
    }
    boosted.extend(rest);
    boosted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_predicted_short_circuits() {
        let words = vec!["a".to_owned(), "b".to_owned(), "c".to_owned()];
        let result = boost_words(words.clone(), vec![]);
        assert_eq!(result, words);
    }

    #[test]
    fn boosted_partition_floats_to_top_preserving_order() {
        let words = vec!["abc", "xyz", "alpha", "yes"]
            .into_iter()
            .map(String::from)
            .collect();
        let predicted = vec!["a".to_owned()];
        let result = boost_words(words, predicted);
        assert_eq!(result, vec!["abc", "alpha", "xyz", "yes"]);
    }

    #[test]
    fn unicode_cjk_first_char() {
        let words = vec!["九份", "甲級", "九龍", "alpha"]
            .into_iter()
            .map(String::from)
            .collect();
        let predicted = vec!["九".to_owned()];
        let result = boost_words(words, predicted);
        assert_eq!(result, vec!["九份", "九龍", "甲級", "alpha"]);
    }

    #[test]
    fn empty_words_returns_empty() {
        let result = boost_words(vec![], vec!["a".to_owned()]);
        assert!(result.is_empty());
    }
}
