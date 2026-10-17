//! Maps typed half-width punctuation to its full-width form for hanji-first
//! output, applied only while the
//! Hanji/Romanization Swap has Hanji coming first (the MOE rule: full-width in Hanji mode, TL mode
//! half-width); the caller reads the mode, and the auto-space swap is read first
//! and wins. The mode is the only switch: no chord types the other width
//! (USER 2026-10-07). TPS is full width only; there Ctrl on a key of this map
//! types its mark (`ComposingKeyIntent::tps_punctuation_chord`), except the
//! keys in [`HOST_CHORD_KEYS`], and the swap attaches the full-width glyph.
//! macOS keeps a Swift twin: `macos/.../Policies/FullWidthPunctuation.swift`.

/// The MOE manual's symbol shortcut table, minus what this input method must keep
/// half-width: digits (tone markers), the hyphen (syllable separator),
/// letters, and the straight double quote (one glyph serves both sides). Plus
/// `~` (⇧ on the backtick key): the rest of the shifted number row is here, so
/// its first key is too (user report 2026-09-28), and backslash as an additional
/// key for the ideographic comma; apostrophe keeps the MOE shortcut.
const MAP: [(char, char); 26] = [
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
    ('\\', '、'),
    ('~', '～'),
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

/// Mapped keys whose Ctrl chord stays the host's even under TPS:
/// Ctrl+Shift+` (`~`) is VS Code's New Terminal on every desktop.
const HOST_CHORD_KEYS: [char; 1] = ['~'];

/// Whether Ctrl on the key that typed `text` is the TPS punctuation chord: a
/// mapped key outside [`HOST_CHORD_KEYS`].
pub fn is_punctuation_chord_key(text: &str) -> bool {
    full_width_mapped(text).is_some() && !text.chars().any(|c| HOST_CHORD_KEYS.contains(&c))
}

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
        // trace: the 26 rows of `MAP`, the MOE table plus `~` and backslash.
        for (half, full) in MAP {
            assert_eq!(
                full_width_mapped(&half.to_string()).as_deref(),
                Some(full.to_string().as_str())
            );
        }
        assert_eq!(full_width_mapped(","), Some("，".into()));
        assert_eq!(full_width_mapped("'"), Some("、".into()));
        assert_eq!(full_width_mapped("\\"), Some("、".into()));
        assert_eq!(full_width_mapped("~"), Some("～".into()));
        assert_eq!(MAP.len(), 26);
    }

    #[test]
    fn ctrl_shift_backtick_stays_the_hosts_while_tilde_still_maps() {
        // trace: `~` is in `MAP` and in `HOST_CHORD_KEYS` → mapped, no chord.
        assert_eq!(full_width_mapped("~").as_deref(), Some("～"));
        assert!(!is_punctuation_chord_key("~"));
        assert!(is_punctuation_chord_key(","));
        assert!(is_punctuation_chord_key("<"));
        assert!(!is_punctuation_chord_key("5"));
    }

    #[test]
    fn tones_syllable_characters_hyphen_quote_space_and_multichar_never_map() {
        for text in ["5", "0", "a", "n", "-", "\"", " ", "?!", "", "台"] {
            assert_eq!(full_width_mapped(text), None, "{text:?}");
        }
    }
}
