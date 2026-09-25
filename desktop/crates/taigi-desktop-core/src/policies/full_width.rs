//! Maps typed half-width punctuation to its full-width form for hanji-first
//! output. Port of `FullWidthPunctuation.swift`: applied only while the
//! Hanji/Romanization Swap has Hanji coming first (the MOE rule: full-width in Hanji mode, TL mode
//! half-width); the caller reads the mode, and the auto-space swap is read first
//! and wins. The mode is a default, not a wall: Ctrl on any key of this map
//! types the other width once (`ComposingKeyIntent::width_flip_character`).

/// The MOE manual's symbol shortcut table, minus what this input method must keep
/// half-width: digits (tone markers), the hyphen (syllable separator),
/// letters, and the straight double quote (one glyph serves both sides).
const MAP: [(char, char); 24] = [
    (',', '，'),
    ('.', '。'),
    ('?', '？'),
    ('!', '！'),
    (';', '；'),
    (':', '：'),
    ('(', '（'),
    (')', '）'),
    ('[', '「'),
    (']', '」'),
    ('{', '『'),
    ('}', '』'),
    ('<', '《'),
    ('>', '》'),
    ('\'', '、'),
    ('@', '＠'),
    ('#', '＃'),
    ('$', '＄'),
    ('%', '％'),
    ('^', '＾'),
    ('&', '＆'),
    ('*', '＊'),
    ('_', '＿'),
    ('+', '＋'),
];

/// The full-width form of one typed character, or `None` when the key is
/// not punctuation this policy maps. Multi-character strings are never
/// mapped: a key event carries one typed character.
pub fn full_width_mapped(text: &str) -> Option<String> {
    let mut chars = text.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return None;
    };
    MAP.iter()
        .find(|(half, _)| *half == c)
        .map(|(_, full)| full.to_string())
}

/// The punctuation the input method writes for `text`, or `None` when the
/// host should write it: the full-width form when the mode types full-width
/// marks, and under the width-flip chord the OTHER width. A flipped key is
/// never `None` — the host would read the chord as a shortcut, so even its
/// half-width form is written by the input method
/// (`FullWidthPunctuation.swift` `documentPunctuation`).
pub fn document_punctuation(
    text: &str,
    is_full_width_mode: bool,
    is_width_flip: bool,
) -> Option<String> {
    if is_full_width_mode != is_width_flip {
        return full_width_mapped(text);
    }
    is_width_flip.then(|| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_flip_chord_types_the_other_width_and_the_bare_key_the_modes() {
        // trace: FullWidthPunctuationTests.swift
        // `testDocumentPunctuation_flipTypesTheOtherWidthAndTheBareKeyTheModes`.
        assert_eq!(
            document_punctuation(",", true, false).as_deref(),
            Some("，")
        );
        assert_eq!(document_punctuation(",", true, true).as_deref(), Some(","));
        assert_eq!(document_punctuation(",", false, false), None);
        assert_eq!(
            document_punctuation(",", false, true).as_deref(),
            Some("，")
        );
        // A key the map does not carry is the host's under the mode; the
        // classifier never reports it as a flip.
        assert_eq!(document_punctuation("5", true, false), None);
    }

    #[test]
    fn every_mapped_pair_follows_the_moe_table() {
        // trace: FullWidthPunctuationTests.swift:13-29.
        for (half, full) in MAP {
            assert_eq!(
                full_width_mapped(&half.to_string()).as_deref(),
                Some(full.to_string().as_str())
            );
        }
        assert_eq!(full_width_mapped(","), Some("，".into()));
        assert_eq!(full_width_mapped("'"), Some("、".into()));
        assert_eq!(MAP.len(), 24);
    }

    #[test]
    fn tones_syllable_characters_hyphen_quote_space_and_multichar_never_map() {
        for text in ["5", "0", "a", "n", "-", "\"", " ", "?!", "", "台"] {
            assert_eq!(full_width_mapped(text), None, "{text:?}");
        }
    }
}
