//! Spacing for a literal appended by the commit-and-insert transition.

use phonetics::api::{is_attaching_punctuation, is_spaced_punctuation};

use crate::commit_text::should_append_space;

/// The literal and spacing appended to a finalized composition.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AugmentedInsert {
    pub text: String,
    /// True when the insertion leaves the auto space immediately before the
    /// caret, so the next attaching-punctuation key may swap with it.
    pub leaves_trailing_auto_space: bool,
}

/// Rewrites the external text a mid-composition punctuation key inserts:
/// attaching punctuation lands before the space (`guá? `) — a full-width
/// mark in place of it (`guá？`), arming nothing — everything else after it
/// (`guá (`). A typed space is left alone (the engine already
/// appends it) but still arms the swap. `composing_display_text` is the
/// preedit about to be committed — its trailing hyphen is the committed
/// text's trailing hyphen.
pub(crate) fn augment_insert(
    text: &str,
    composing_display_text: &str,
    gate_active: bool,
) -> AugmentedInsert {
    if !gate_active || text.is_empty() {
        return AugmentedInsert {
            text: text.to_owned(),
            leaves_trailing_auto_space: false,
        };
    }
    if text.chars().any(char::is_whitespace) {
        return AugmentedInsert {
            text: text.to_owned(),
            leaves_trailing_auto_space: text == " ",
        };
    }
    if !should_append_space(composing_display_text) {
        return AugmentedInsert {
            text: text.to_owned(),
            leaves_trailing_auto_space: false,
        };
    }
    if is_attaching_punctuation(text) && !is_spaced_punctuation(text) {
        AugmentedInsert {
            text: text.to_owned(),
            leaves_trailing_auto_space: false,
        }
    } else if is_attaching_punctuation(text) {
        AugmentedInsert {
            text: format!("{text} "),
            leaves_trailing_auto_space: true,
        }
    } else {
        AugmentedInsert {
            text: format!(" {text}"),
            leaves_trailing_auto_space: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn augment_insert_places_the_space_around_the_character() {
        // trace: `augment_insert` — attaching → "? " (arms the swap), a
        // full-width attaching mark → "？" (arms nothing), other →
        // " (", a typed space passes through but arms it, a trailing hyphen
        // or the gate off writes the text as is.
        let attaching = augment_insert("?", "guá", true);
        assert_eq!(attaching.text, "? ");
        assert!(attaching.leaves_trailing_auto_space);
        let full_width = augment_insert("？", "guá", true);
        assert_eq!(full_width.text, "？");
        assert!(!full_width.leaves_trailing_auto_space);
        let opening = augment_insert("(", "guá", true);
        assert_eq!(opening.text, " (");
        assert!(!opening.leaves_trailing_auto_space);
        let space = augment_insert(" ", "guá", true);
        assert_eq!(space.text, " ", "a typed space is not doubled");
        assert!(space.leaves_trailing_auto_space, "but still arms the swap");
        let hyphen = augment_insert("?", "guá-", true);
        assert_eq!(hyphen.text, "?");
        assert!(!hyphen.leaves_trailing_auto_space);
        let off = augment_insert("?", "guá", false);
        assert_eq!(off.text, "?");
        assert!(!off.leaves_trailing_auto_space);
        assert_eq!(augment_insert("", "guá", true).text, "");
    }
}
