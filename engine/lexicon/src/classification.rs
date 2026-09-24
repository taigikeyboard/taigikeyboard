//! Input classification primitives.
//!
//! - `is_hanzi` — the Tab3 search short-circuit predicate (lexicon proto
//!   dispatch) and the Continuous fetch's Hanji check. Replaces iOS
//!   `CandidateProcessor.isHanzi` and Android `DictionarySearchViewModel`'s
//!   inline 16-bit `Char.code` check (parity correction — that inline check
//!   silently missed Extensions B/C/D/E because Kotlin `Char.code` tops out
//!   at 0xFFFF).
//!
//! Pure functions — no I/O, no engine handle. INVARIANT contracts live in
//! `docs/architecture/behavioral-invariants.md` under the umbrella label
//! `INVARIANT_LEX_INPUT_CLASSIFICATION`.

/// Returns `true` iff `text` contains at least one CJK Unified Ideograph
/// (Unified block + Extensions A–E).
///
/// Range coverage matches the canonical platform set captured by
/// `INVARIANT_LEX_INPUT_CLASSIFICATION_HANZI_RANGE`. Extensions F/G/H/I/J
/// are intentionally excluded — including them would be a behavior
/// expansion beyond the v3.5.7 parity correction scope.
pub fn is_hanzi(text: &str) -> bool {
    text.chars().any(|c| {
        let cp = c as u32;
        (0x4E00..=0x9FFF).contains(&cp)        // CJK Unified Ideographs
            || (0x3400..=0x4DBF).contains(&cp)  // Extension A
            || (0x20000..=0x2A6DF).contains(&cp) // Extension B
            || (0x2A700..=0x2B73F).contains(&cp) // Extension C
            || (0x2B740..=0x2B81F).contains(&cp) // Extension D
            || (0x2B820..=0x2CEAF).contains(&cp) // Extension E
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // INVARIANT_LEX_INPUT_CLASSIFICATION_HANZI_RANGE
    #[test]
    fn is_hanzi_unified_block() {
        assert!(is_hanzi("我"));
    }
    #[test]
    fn is_hanzi_extension_a_lower_bound() {
        assert!(is_hanzi("\u{3400}"));
    }
    #[test]
    fn is_hanzi_extension_a_upper_bound() {
        assert!(is_hanzi("\u{4DBF}"));
    }
    #[test]
    fn is_hanzi_extension_b_lower_bound() {
        assert!(is_hanzi("\u{20000}"));
    }
    #[test]
    fn is_hanzi_extension_c_lower_bound() {
        assert!(is_hanzi("\u{2A700}"));
    }
    #[test]
    fn is_hanzi_extension_d_lower_bound() {
        assert!(is_hanzi("\u{2B740}"));
    }
    #[test]
    fn is_hanzi_extension_e_lower_bound() {
        assert!(is_hanzi("\u{2B820}"));
    }
    #[test]
    fn is_hanzi_extension_e_upper_bound() {
        assert!(is_hanzi("\u{2CEAF}"));
    }
    #[test]
    fn is_hanzi_extension_f_excluded() {
        // 0x2CEB0 is the start of Extension F — must NOT match (out of slice scope)
        assert!(!is_hanzi("\u{2CEB0}"));
    }
    #[test]
    fn is_hanzi_roman_letters_no_match() {
        assert!(!is_hanzi("gua"));
    }
    #[test]
    fn is_hanzi_empty_no_match() {
        assert!(!is_hanzi(""));
    }
    #[test]
    fn is_hanzi_mixed_substring_match() {
        assert!(is_hanzi("a好b"));
    }
    #[test]
    fn is_hanzi_just_below_unified_block_no_match() {
        // 0x4DFF is in Yijing Hexagram Symbols block, NOT CJK
        assert!(!is_hanzi("\u{4DFF}"));
    }
}
