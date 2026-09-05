//! The seams the composing path reads and writes through. The storage crate
//! implements them over SQLite; tests implement them in memory. `Send + Sync`
//! because the one manager per process lives behind a mutex a TSF host may
//! reach from several thread managers (roadmap W3).

use crate::engine::{AssociationPair, CustomEntry, FrequencyRow};

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

/// `user_association.db`'s write side.
pub trait AssociationSink: Send + Sync {
    /// Writes the learned bigrams. Best-effort; never logs the words.
    fn record(&self, pairs: &[AssociationPair]);
}

/// Milliseconds since the Unix epoch. Injectable because the engine's
/// association window is a comparison against this clock, and a test that
/// cannot move it can only ever exercise one side of it.
pub trait Clock: Send + Sync {
    fn now_ms(&self) -> i64;
}

/// The wall clock.
#[derive(Clone, Copy, Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| {
                i64::try_from(elapsed.as_millis()).unwrap_or(i64::MAX)
            })
    }
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

impl<T: AssociationSink + ?Sized> AssociationSink for std::sync::Arc<T> {
    fn record(&self, pairs: &[AssociationPair]) {
        (**self).record(pairs);
    }
}

/// A store that holds nothing and learns nothing — what a host process that
/// cannot reach `%APPDATA%` (an AppContainer) runs with, and what tests use
/// when learning is not the point.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoStores;

impl FrequencySource for NoStores {
    fn rows_for_words(&self, _words: &[String]) -> Option<Vec<FrequencyRow>> {
        None
    }
    fn record(&self, _word: &str, _tl: &str) {}
}

impl CustomDictionarySource for NoStores {
    fn rows_matching(&self, _family: &str, _form: &str, _key: &str) -> Vec<CustomEntry> {
        Vec::new()
    }
}

impl AssociationSink for NoStores {
    fn record(&self, _pairs: &[AssociationPair]) {}
}
