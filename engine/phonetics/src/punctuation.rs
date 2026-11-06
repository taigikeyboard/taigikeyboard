//! Punctuation classification for the auto-space "smart punctuation" swap —
//! the one copy every platform asks (`IsAttachingPunctuation`), plus the
//! joiner check iOS and Android ask (`IsJoinerPunctuation`). Behavioural
//! invariant §23 `INVARIANT_AUTO_SPACE_PUNCTUATION_SWAP`.
//!
//! When auto-space is active, every committed word is followed by a trailing
//! space. Typing *attaching* punctuation next must move that space to AFTER
//! the punctuation (`guá ` + `?` → `guá? `), not leave it before (`guá ?`).
//! A *joiner* binds the next syllable to the word, so it takes the space's
//! place and leaves none (`hōo ` + `·` → `hōo·`).

/// Sentence-end + clause separators + CLOSING brackets/quotes. OPENING
/// brackets/quotes (`(（[「『`) are deliberately excluded — they need a
/// LEADING space, not attachment. ASCII straight quotes (`"` `'`) are
/// excluded because the same glyph serves as both opening and closing;
/// attaching them would corrupt `guá "…"` into `guá" …`.
const ATTACHING: [char; 19] = [
    '。', '！', '？', '.', '!', '?', '，', ',', '、', '；', ';', '：', ':', ')', '）', ']', '】',
    '」', '』',
];

/// True when `text` is a single attaching-punctuation character.
pub(crate) fn is_attaching_punctuation(text: &str) -> bool {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => ATTACHING.contains(&c),
        _ => false,
    }
}

/// True when `text` is the khinsiann dot, the one joiner: it stands between
/// two syllables of a phrase, so it never takes a space on either side.
pub(crate) fn is_joiner_punctuation(text: &str) -> bool {
    text == crate::tps::KHINSIANN_DOT
}

#[cfg(test)]
mod tests {
    use super::*;

    // INVARIANT_AUTO_SPACE_PUNCTUATION_SWAP (behavioral-invariants.md §23)
    #[test]
    fn attaching_set_is_the_cross_platform_roster() {
        // trace: the 19 glyphs of `ATTACHING` — the set iOS / Android /
        // macOS `AutoSpacePunctuation` and desktop-core `policies::auto_space`
        // each held before it moved here; openers and straight quotes are
        // outside it.
        for glyph in [
            "。", "！", "？", ".", "!", "?", "，", ",", "、", "；", ";", "：", ":", ")", "）", "]",
            "】", "」", "』",
        ] {
            assert!(is_attaching_punctuation(glyph), "{glyph}");
        }
        for glyph in ["(", "（", "[", "「", "『", "\"", "'"] {
            assert!(!is_attaching_punctuation(glyph), "{glyph}");
        }
    }

    #[test]
    fn only_a_single_character_attaches() {
        // trace: two scalars never match, even when the first is in the set —
        // `?` + U+0301 is one Swift grapheme but not the Character `?`, so
        // every old platform copy answered false too.
        for text in ["", "a", "台", " ", "?!", "? ", "guá?", "?\u{0301}"] {
            assert!(!is_attaching_punctuation(text), "{text:?}");
        }
    }

    #[test]
    fn the_khinsiann_dot_is_the_only_joiner() {
        // trace: `tps::KHINSIANN_DOT` = U+00B7. U+02D9 `˙` is the TPS tone 8
        // mark and `-` composes, neither is a joiner; the dot is not in
        // `ATTACHING`, so a character is never both.
        assert!(is_joiner_punctuation("\u{00b7}"));
        assert!(!is_attaching_punctuation("\u{00b7}"));
        for text in ["", "-", "\u{02d9}", "?", "\u{00b7}\u{00b7}", " \u{00b7}"] {
            assert!(!is_joiner_punctuation(text), "{text:?}");
        }
    }
}
