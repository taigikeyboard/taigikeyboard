//! Taigi-specific Unicode preprocessing for ranking.
//!
//! CROSS-PLATFORM INVARIANT — byte-exact mirror of iOS
//! `Lexicon/Utils/TaigiUnicode.swift::nfdPreprocessed` and Android
//! `ime/dictionary/TaigiUnicode.kt::nfdPreprocessed`. Both platform copies
//! are retained for JVM unit-test compatibility (see PR #187 +
//! `feedback_jvm_test_jni_compat.md`); the production code path on both
//! platforms routes through ranking via `RustEngineBridge.processCandidates`,
//! which calls this helper internally.
//!
//! Note: `phonetics::derivation::nfd_preprocessed` uses different semantics
//! (literal `o\u{0358}` → `oo` adjacency-substitution for trie keys); that
//! variant ignores combining-class reordering so it is NOT interchangeable
//! with this helper. Keep the two side-by-side until we have a single
//! authoritative caller (post v3.5.2 cleanup, no version commitment).

use unicode_normalization::UnicodeNormalization;

/// Apply Taigi-specific Unicode preprocessing:
/// 1. POJ nasal markers `ⁿ` (U+207F) / `ᴺ` (U+1D3A) → `"nn"`
/// 2. NFD decompose (combining marks become individually accessible)
/// 3. Replace each standalone `\u{0358}` (POJ `o͘` combining mark) → `"o"`
///    so any decomposed `o\u{0358}` collapses to `"oo"` regardless of the
///    canonical combining-class reorder.
///
/// Step 3 matches iOS / Android single-codepoint replacement exactly —
/// it does NOT require `o` adjacency, which is critical when other
/// combining marks (e.g. acute, U+0301) sit between `o` and `\u{0358}`
/// after NFD canonical reorder.
pub fn nfd_preprocessed(input: &str) -> String {
    let with_nasal = input.replace(['\u{207f}', '\u{1d3a}'], "nn");
    let decomposed: String = with_nasal.nfd().collect();
    decomposed.replace('\u{0358}', "o")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nasal_marker_superscript_n_to_nn() {
        // U+207F SUPERSCRIPT LATIN SMALL LETTER N
        assert_eq!(nfd_preprocessed("sa\u{207f}"), "sann");
    }

    #[test]
    fn nasal_marker_modifier_n_to_nn() {
        // U+1D3A MODIFIER LETTER CAPITAL N
        assert_eq!(nfd_preprocessed("sa\u{1d3a}"), "sann");
    }

    #[test]
    fn poj_o_dot_collapses_to_oo() {
        // "ho͘" — o + U+0358. After NFD it stays as `o\u{0358}` (no further
        // decomposition), then \u{0358} → "o" gives "hoo".
        assert_eq!(nfd_preprocessed("ho\u{0358}"), "hoo");
    }

    #[test]
    fn poj_o_dot_with_acute_handles_ccc_reorder() {
        // "hó͘" composed: NFC `h ó ͘`. NFD reorders to `h, o, combining_acute,
        // combining_dot_above` (CCC 230 before CCC 232). Replacing
        // \u{0358} (the dot) with "o" gives `h, o, combining_acute, o`,
        // visually "hóo".
        let input = "h\u{00f3}\u{0358}";
        let result = nfd_preprocessed(input);
        let expected = "ho\u{0301}o";
        assert_eq!(result, expected);
    }

    #[test]
    fn no_special_chars_just_nfd_decompose() {
        // Plain "tâi-gí" with NFC accents: NFD decomposes the diacritics.
        let input = "t\u{00e2}i-g\u{00ed}";
        let result = nfd_preprocessed(input);
        let expected = "ta\u{0302}i-gi\u{0301}";
        assert_eq!(result, expected);
    }

    #[test]
    fn empty_string_is_fixed_point() {
        assert_eq!(nfd_preprocessed(""), "");
    }

    #[test]
    fn ascii_only_passes_through() {
        assert_eq!(nfd_preprocessed("hello"), "hello");
    }
}
