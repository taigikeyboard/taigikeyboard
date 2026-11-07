//! Hanji detection — whether a text holds a CJK ideograph.
//!
//! - `is_hanji` — the Tab3 search short-circuit predicate (lexicon proto
//!   dispatch, re-exported as `lexicon::classification`), the Continuous
//!   fetch's Hanji check, and the Learning Records page's Add to Custom
//!   Dictionary rule (`userdata`). Replaces iOS
//!   `CandidateProcessor.isHanji` and Android `DictionarySearchViewModel`'s
//!   inline 16-bit `Char.code` check (parity correction — that inline check
//!   silently missed Extensions B/C/D/E because Kotlin `Char.code` tops out
//!   at 0xFFFF).
//! - `is_roman_only_spelling` — whether the spelling in a word's Hanji column
//!   is written only in romanization (`lán`), so the word has no Hanji: the
//!   custom dictionary's write boundary (`userdata`), the rule the dictionary
//!   pipeline applies to the shipped rows.
//!
//! Pure functions — no I/O, no engine handle. INVARIANT contracts live in
//! `docs/architecture/behavioral-invariants.md` under the umbrella label
//! `INVARIANT_LEX_INPUT_CLASSIFICATION`.

/// CJK ideograph ranges, inclusive — the same table as the dictionary
/// pipeline's `dictionary/common/cjk.py` `CJK_RANGES`, kept equal by the
/// `cjk_ranges_parity` test. `0x20000–0x3134F` spans Extensions B–G and I,
/// the Compatibility Ideographs Supplement and the unassigned gaps between
/// them; Extension H (`0x31350–0x323AF`) and later stay out.
pub const CJK_RANGES: &[(u32, u32)] = &[
    (0x4E00, 0x9FFF),   // Unified Ideographs
    (0x3400, 0x4DBF),   // Extension A
    (0x20000, 0x3134F), // Extensions B–G, I + Compatibility Supplement
    (0xF900, 0xFAFF),   // Compatibility Ideographs
];

/// Returns `true` iff `text` contains at least one codepoint in
/// [`CJK_RANGES`] (`INVARIANT_LEX_INPUT_CLASSIFICATION_HANJI_RANGE`).
pub fn is_hanji(text: &str) -> bool {
    text.chars().any(|c| {
        let cp = u32::from(c);
        CJK_RANGES
            .iter()
            .any(|&(low, high)| (low..=high).contains(&cp))
    })
}

/// The non-ASCII characters of the pipeline's romanization class that NFD
/// leaves standing: superscript ⁿ and the combining grave, macron and caron
/// the class spells out (its precomposed letters decompose onto ASCII).
const ROMAN_MARKS: [char; 4] = ['\u{207F}', '\u{0300}', '\u{0304}', '\u{030C}'];

/// Hiragana or Katakana block, the prolonged sound mark included — the
/// pipeline's `cjk.py` `is_kana`.
fn is_kana(c: char) -> bool {
    ('\u{3040}'..='\u{30FF}').contains(&c)
}

/// Returns `true` iff the spelling `text` — what a word's Hanji column holds —
/// is written only in romanization: a Latin letter, no ideograph, no kana
/// (`lán`, `a-lú-mih`; not the Han-Lo `白phau-phau`, not `にんじん`). Such a
/// word has no Hanji. The port of
/// the dictionary pipeline's `dictionary/common/cleanup.py`
/// `is_roman_only_hanzi`, kept equal by the `roman_only_spelling_parity` test;
/// read in NFD, so `ê` counts by its `e`.
pub fn is_roman_only_spelling(text: &str) -> bool {
    use unicode_normalization::UnicodeNormalization;
    let mut has_roman = false;
    for c in text.nfd() {
        if is_hanji(c.encode_utf8(&mut [0; 4])) || is_kana(c) {
            return false;
        }
        has_roman |= c.is_ascii_alphabetic() || ROMAN_MARKS.contains(&c);
    }
    has_roman
}

#[cfg(test)]
mod tests {
    use super::*;

    // INVARIANT_LEX_INPUT_CLASSIFICATION_HANJI_RANGE
    #[test]
    fn is_hanji_unified_block() {
        assert!(is_hanji("我"));
    }
    #[test]
    fn is_hanji_extension_a_lower_bound() {
        assert!(is_hanji("\u{3400}"));
    }
    #[test]
    fn is_hanji_extension_a_upper_bound() {
        assert!(is_hanji("\u{4DBF}"));
    }
    #[test]
    fn is_hanji_extension_b_lower_bound() {
        assert!(is_hanji("\u{20000}"));
    }
    #[test]
    fn is_hanji_extension_c_lower_bound() {
        assert!(is_hanji("\u{2A700}"));
    }
    #[test]
    fn is_hanji_extension_d_lower_bound() {
        assert!(is_hanji("\u{2B740}"));
    }
    #[test]
    fn is_hanji_extension_e_lower_bound() {
        assert!(is_hanji("\u{2B820}"));
    }
    #[test]
    fn is_hanji_extension_e_upper_bound() {
        assert!(is_hanji("\u{2CEAF}"));
    }
    #[test]
    fn is_hanji_extension_f_lower_bound() {
        assert!(is_hanji("\u{2CEB0}"));
    }
    #[test]
    fn is_hanji_extension_i_lower_bound() {
        assert!(is_hanji("\u{2EBF0}"));
    }
    #[test]
    fn is_hanji_compatibility_supplement() {
        // U+2F801 丸 — a `dictionary.csv` row (丸/huân).
        assert!(is_hanji("\u{2F801}"));
    }
    #[test]
    fn is_hanji_extension_g_dictionary_char() {
        // U+308FB 𰣻 — a `dictionary.csv` row (𰣻/ko).
        assert!(is_hanji("\u{308FB}"));
    }
    #[test]
    fn is_hanji_extension_g_upper_bound() {
        assert!(is_hanji("\u{3134F}"));
    }
    #[test]
    fn is_hanji_extension_h_excluded() {
        // 0x31350 opens Extension H, past `cjk.py`'s last range.
        assert!(!is_hanji("\u{31350}"));
    }
    #[test]
    fn is_hanji_compatibility_ideographs_bounds() {
        assert!(is_hanji("\u{F900}"));
        assert!(is_hanji("\u{FAFF}"));
    }
    #[test]
    fn is_hanji_around_compatibility_ideographs_no_match() {
        // 0xF8FF closes the Private Use Area; 0xFB00 opens Alphabetic
        // Presentation Forms (ﬀ).
        assert!(!is_hanji("\u{F8FF}"));
        assert!(!is_hanji("\u{FB00}"));
    }
    #[test]
    fn is_hanji_roman_letters_no_match() {
        assert!(!is_hanji("gua"));
    }
    #[test]
    fn is_hanji_empty_no_match() {
        assert!(!is_hanji(""));
    }
    #[test]
    fn is_hanji_mixed_substring_match() {
        assert!(is_hanji("a好b"));
    }
    #[test]
    fn is_hanji_just_below_unified_block_no_match() {
        // 0x4DFF is in Yijing Hexagram Symbols block, NOT CJK
        assert!(!is_hanji("\u{4DFF}"));
    }
}
