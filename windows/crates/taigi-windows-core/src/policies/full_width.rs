//! Maps typed half-width punctuation to its full-width form for hanji-first
//! output. Port of `FullWidthPunctuation.swift`: applied only while the
//! 漢羅對調 swap has Hanji coming first (the MOE rule 漢字模式全形, 臺羅模式
//! 半形); the caller reads the mode, and the auto-space swap is read first
//! and wins.

// 中文: 半形→全形標點對照(教育部表);只在漢字優先模式套用,呼叫端判斷模式。

/// The MOE manual's 符號快捷鍵對照表, minus what this input method must keep
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

#[cfg(test)]
mod tests {
    use super::*;

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
