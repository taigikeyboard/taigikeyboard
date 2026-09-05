//! When a committed word earns the auto-space trailing space, as pure
//! decisions. Port of `AutoSpacePolicy.swift` + `AutoSpacePunctuation.swift`
//! (behavioural invariant §23, `INVARIANT_AUTO_SPACE_PUNCTUATION_SWAP`).

// 自動空白規則 — 閘門、連字號、附著標點交換、一次寫入的插入增補。

use crate::settings::InputMode;

/// Sentence-end + clause separators + CLOSING brackets/quotes. OPENING
/// brackets/quotes are deliberately excluded (they need a LEADING space),
/// and the ASCII straight quotes because one glyph serves both sides.
/// CROSS-PLATFORM INVARIANT — mirrors `ios/.../Input/AutoSpacePunctuation.swift`,
/// `android/.../ime/text/AutoSpacePunctuation.kt`, `macos/.../AutoSpacePunctuation.swift`.
const ATTACHING: [char; 19] = [
    '。', '！', '？', '.', '!', '?', '，', ',', '、', '；', ';', '：', ':', ')', '）', ']', '】',
    '」', '』',
];

/// True when `text` is a single attaching-punctuation character.
pub fn is_attaching_punctuation(text: &str) -> bool {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => ATTACHING.contains(&c),
        _ => false,
    }
}

/// True when this commit earns a trailing space at all — the same gate every
/// insertion site and the punctuation swap read. `wrote_romanization` comes
/// from whatever resolved the string: `composing::resolved_commit` for a
/// candidate, [`raw_preedit_writes_romanization`] for the preedit itself.
pub fn is_gate_active(is_auto_space_enabled: bool, wrote_romanization: bool) -> bool {
    is_auto_space_enabled && wrote_romanization
}

/// Whether committing the preedit AS TYPED writes romanization — the
/// literal-commit chord and the mid-composition punctuation commit, neither
/// of which goes through a candidate.
///
/// A `match` over a two-variant enum rather than `true`, so that adding a
/// non-romanized layout (TPS composes Bopomofo, which takes no spacing)
/// fails to compile here instead of silently spacing 注音.
/// CROSS-PLATFORM INVARIANT — mirrors `macos/.../AutoSpacePolicy.swift`
/// `rawPreeditWritesRomanization(inputMode:)`.
pub fn raw_preedit_writes_romanization(input_mode: InputMode) -> bool {
    match input_mode {
        InputMode::Tl | InputMode::Poj => true,
    }
}

/// Whether the text a commit just wrote should be followed by a space —
/// judged on the actual document string; a trailing hyphen (a syllable
/// about to be continued) suppresses it.
pub fn should_append_space(document_text: &str) -> bool {
    !document_text.is_empty() && !document_text.ends_with('-')
}

/// What `commit_then_insert` should hand the engine, so one keystroke stays
/// one document mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AugmentedInsert {
    pub text: String,
    /// True when the insertion leaves the auto space immediately before the
    /// caret, so the next attaching-punctuation key may swap with it.
    pub leaves_trailing_auto_space: bool,
}

/// Rewrites the external text a mid-composition punctuation key inserts:
/// attaching punctuation lands before the space (`guá? `), everything else
/// after it (`guá (`). A typed space is left alone (the engine already
/// appends it) but still arms the swap. `composing_display_text` is the
/// preedit about to be committed — its trailing hyphen is the committed
/// text's trailing hyphen.
pub fn augment_insert(
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
    if is_attaching_punctuation(text) {
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
    fn attaching_set_matches_the_cross_platform_roster() {
        // trace: AutoSpacePolicyTests.swift:10-30 — the 19 glyphs.
        for glyph in [
            "。", "！", "？", ".", "!", "?", "，", ",", "、", "；", ";", "：", ":", ")", "）", "]",
            "】", "」", "』",
        ] {
            assert!(is_attaching_punctuation(glyph), "{glyph}");
        }
        for glyph in ["(", "（", "[", "「", "『", "\"", "'"] {
            assert!(!is_attaching_punctuation(glyph), "{glyph}");
        }
        for text in ["a", "台", " ", "", "?!", "guá?"] {
            assert!(!is_attaching_punctuation(text), "{text:?}");
        }
    }

    #[test]
    fn the_gate_follows_whether_the_commit_wrote_romanization() {
        // trace: AutoSpacePolicyTests.swift:32-48.
        assert!(!is_gate_active(false, true));
        assert!(!is_gate_active(false, false));
        assert!(is_gate_active(true, true));
        assert!(!is_gate_active(true, false));
    }

    #[test]
    fn a_fresh_install_has_the_gate_off_for_romanization() {
        // trace: AutoSpacePolicyTests — the shipped default is OFF on every
        // platform (`keys::IS_AUTO_SPACE_ENABLED` says why).
        let document = crate::settings::SettingsDocument::default();
        let enabled = document.bool(&crate::settings::keys::IS_AUTO_SPACE_ENABLED);
        assert!(!is_gate_active(
            enabled,
            raw_preedit_writes_romanization(InputMode::Tl)
        ));
    }

    #[test]
    fn committed_word_earns_the_space_but_a_trailing_hyphen_suppresses_it() {
        assert!(should_append_space("guá"));
        assert!(!should_append_space("guá-"));
        assert!(!should_append_space(""));
    }

    #[test]
    fn augment_insert_places_the_space_around_the_character() {
        // trace: AutoSpacePolicyTests.swift:96-143.
        let attaching = augment_insert("?", "guá", true);
        assert_eq!(attaching.text, "? ");
        assert!(attaching.leaves_trailing_auto_space);
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
