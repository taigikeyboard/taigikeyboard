//! The seams the composing path reads and writes through. The stores in this
//! crate implement them over SQLite; tests implement them in memory. `Send +
//! Sync` because the one manager per process lives behind a mutex a TSF host
//! may reach from several thread managers (windows-roadmap W3).

use crate::types::{CustomEntry, FrequencyRow, LearnedPhrase};

/// `user_frequency.db`, as the keystroke path sees it.
pub trait FrequencySource: Send + Sync {
    /// The learned rows for `words` (display-text keys), or `None` when the
    /// store could not answer right now — a closed store, a busy database.
    /// `None` and an empty list both degrade to the neutral ranking; they are
    /// kept apart only so a caller can log the difference.
    fn rows_for_words(&self, words: &[String]) -> Option<Vec<FrequencyRow>>;
    /// Counts one commit of the `(word, canonical TL)` pair. Best-effort.
    fn record(&self, word: &str, tl: &str);
}

/// `custom_dictionary.db`, as the keystroke path sees it.
pub trait CustomDictionarySource: Send + Sync {
    /// The rows whose search key under `family` / `form` starts with `key`.
    fn rows_matching(&self, family: &str, form: &str, key: &str) -> Vec<CustomEntry>;
}

/// `learned_phrases.db` (§50), as the keystroke path sees it — learning
/// data, not the user's dictionary (USER 2026-09-21).
pub trait LearnedPhraseSource: Send + Sync {
    /// The phrases whose search key under `family` / `form` EQUALS `key` —
    /// the whole buffer, never a prefix.
    fn rows_matching(&self, family: &str, form: &str, key: &str) -> Vec<LearnedPhrase>;
    /// Records one phrase a final commit taught. Best-effort; never logs the words.
    fn learn_phrase(&self, hanzi: &str, canonical_tl: &str);
    /// Bumps a phrase the user just picked whole. Best-effort.
    fn touch_phrase(&self, hanzi: &str, canonical_tl: &str);
}

// A shared store is the store: the shell hands one `Arc` to the manager and
// keeps another for the settings window's pages.
impl<T: FrequencySource + ?Sized> FrequencySource for std::sync::Arc<T> {
    fn rows_for_words(&self, words: &[String]) -> Option<Vec<FrequencyRow>> {
        (**self).rows_for_words(words)
    }

    fn record(&self, word: &str, tl: &str) {
        (**self).record(word, tl);
    }
}

impl<T: CustomDictionarySource + ?Sized> CustomDictionarySource for std::sync::Arc<T> {
    fn rows_matching(&self, family: &str, form: &str, key: &str) -> Vec<CustomEntry> {
        (**self).rows_matching(family, form, key)
    }
}

impl<T: LearnedPhraseSource + ?Sized> LearnedPhraseSource for std::sync::Arc<T> {
    fn rows_matching(&self, family: &str, form: &str, key: &str) -> Vec<LearnedPhrase> {
        (**self).rows_matching(family, form, key)
    }
    fn learn_phrase(&self, hanzi: &str, canonical_tl: &str) {
        (**self).learn_phrase(hanzi, canonical_tl);
    }
    fn touch_phrase(&self, hanzi: &str, canonical_tl: &str) {
        (**self).touch_phrase(hanzi, canonical_tl);
    }
}
