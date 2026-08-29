//! When a committed word earns the auto-space trailing space, as pure
//! decisions. Port of `AutoSpacePolicy.swift` + `AutoSpacePunctuation.swift`
//! (behavioural invariant §23, `INVARIANT_AUTO_SPACE_PUNCTUATION_SWAP`).

// 中文: 自動空白規則 — 閘門、連字號、附著標點交換、一次寫入的插入增補。

use crate::composing::CandidateScript;

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
/// insertion site and the punctuation swap read.
pub fn is_gate_active(is_auto_space_enabled: bool, wrote_romanization: bool) -> bool {
    is_auto_space_enabled && wrote_romanization
}

/// Whether committing `script` under these settings puts romanization in
/// the document. The one place the 漢羅 key's inversion is written down: a
/// `Primary` commit under 括號標註 writes `tâi-gí (台語)`, which HAS the
/// romanization, while `Alternate` writes one script and never the pair.
pub fn writes_romanization(
    script: CandidateScript,
    is_translate_swapped: bool,
    is_output_both_scripts: bool,
) -> bool {
    match script {
        CandidateScript::Primary => !is_translate_swapped || is_output_both_scripts,
        CandidateScript::Alternate => is_translate_swapped,
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
    fn gate_and_writes_romanization_follow_the_output_mode() {
        // trace: AutoSpacePolicyTests.swift:32-85.
        assert!(!is_gate_active(false, true));
        assert!(!is_gate_active(false, false));
        assert!(is_gate_active(true, true));
        assert!(!is_gate_active(true, false));
        assert!(writes_romanization(CandidateScript::Primary, false, false));
        assert!(!writes_romanization(CandidateScript::Primary, true, false));
        assert!(writes_romanization(CandidateScript::Primary, true, true));
        assert!(writes_romanization(CandidateScript::Alternate, true, false));
        for both in [false, true] {
            assert!(
                !writes_romanization(CandidateScript::Alternate, false, both),
                "both={both}"
            );
        }
    }

    #[test]
    fn a_fresh_install_has_the_gate_on_for_romanization() {
        // trace: AutoSpacePolicyTests — the shipped default is ON (macOS; iOS
        // and Android ship OFF, `keys::IS_AUTO_SPACE_ENABLED` says why).
        let document = crate::settings::SettingsDocument::default();
        let enabled = document.bool(&crate::settings::keys::IS_AUTO_SPACE_ENABLED);
        assert!(is_gate_active(
            enabled,
            writes_romanization(CandidateScript::Primary, false, false)
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
