//! InputNormalizer + ToneRestoration + lookup-NFD ports.
//!
//! Mirrors:
//! - iOS `Lexicon/Trie/InputNormalizer.swift` (`normalize`, `hasToneMarks`,
//!   `normalizeSyllable`).
//! - iOS `Phonetics/ToneRestoration.swift` (`restore`).
//! - iOS / Android `TaigiUnicode.nfdPreprocessed` (lookup-side NFD prep —
//!   exposed as `taigi_unicode_base_form` for the `engine/ranking` crate
//!   and the `Method::NfdPreprocessForLookup` op).
//!
//! Shares NFD + `COMBINING_TO_TONE_NUM` mechanics across all ports. The
//! two lookup-NFD helpers (`trie_key_unicode_form` and
//! `taigi_unicode_base_form`) are intentionally distinct algorithms —
//! one collapses literal `o\u{0358}` adjacency into `oo` for trie keys,
//! the other replaces every standalone `\u{0358}` codepoint with `o`
//! for ranking comparison bases.

use crate::tables::COMBINING_TO_TONE_NUM;
use crate::tps;
use unicode_normalization::UnicodeNormalization;

// ===========================================================================
// InputNormalizer (full pipeline → trie-query key)
// ===========================================================================

/// `Method::NormalizeInput` — full pipeline:
/// 1. TPS preprocess (if input contains TPS) via `from_zhuyin`.
/// 2. Lowercase.
/// 3. Detect whether input contains tone-mark diacritics (decides
///    `addDefaultTone`).
/// 4. Split by `-` or ` `.
/// 5. Per-syllable: nasal preprocess (ⁿ→nn, o͘→oo) + extract tone digit
///    + checked-ending heuristic.
///
/// Replaces both platforms' `InputNormalizer.normalize`.
pub fn normalize_input(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }
    let processed = if tps::is_zhuyin(input) {
        tps::from_zhuyin(input)
    } else {
        input.to_string()
    };
    let lowered = processed.to_lowercase();
    let should_add_default_tones = has_tone_marks(&lowered);
    let mut result = String::new();
    for syl in lowered.split(['-', ' ']) {
        result.push_str(&normalize_syllable(syl, should_add_default_tones));
    }
    result
}

/// `Method::HasToneMarks` — true if `text` (after NFD) contains any combining
/// tone mark recognised by `COMBINING_TO_TONE_NUM`.
pub(crate) fn has_tone_marks(text: &str) -> bool {
    text.nfd().any(|c| COMBINING_TO_TONE_NUM.contains_key(&c))
}

fn normalize_syllable(syllable: &str, add_default_tone: bool) -> String {
    if syllable.is_empty() {
        return String::new();
    }
    let with_oo = trie_key_unicode_form(syllable);
    if let Some(last) = with_oo.chars().last() {
        if last.is_ascii_digit() {
            return with_oo;
        }
    }
    let mut tone_number = String::new();
    let mut without_tone = String::new();
    for ch in with_oo.chars() {
        if let Some(t) = COMBINING_TO_TONE_NUM.get(&ch) {
            tone_number = (*t).to_string();
        } else {
            without_tone.push(ch);
        }
    }
    if add_default_tone && tone_number.is_empty() {
        if let Some(last_char) = without_tone.chars().last() {
            tone_number = if matches!(last_char, 'p' | 't' | 'k' | 'h') {
                "4".to_string()
            } else {
                "1".to_string()
            };
        }
    }
    let mut out = without_tone;
    out.push_str(&tone_number);
    out
}

/// Mirrors iOS `TaigiUnicode.nfdPreprocessed`: NFD-decompose, then convert
/// `ⁿ` → `nn` and `o͘` → `oo` so the trie key uses ASCII-only forms.
fn trie_key_unicode_form(text: &str) -> String {
    let with_nasal = text.replace(['\u{207f}', '\u{1d3a}'], "nn");
    let decomposed: String = with_nasal.nfd().collect();
    // o + combining dot above right (U+0358) → "oo"
    decomposed
        .replace("o\u{0358}", "oo")
        .replace("O\u{0358}", "Oo")
}

// ===========================================================================
// Lookup NFD preprocessing (taigi_unicode_base_form)
// ===========================================================================

/// Apply Taigi-specific Unicode preprocessing for external dictionary
/// lookup + ranking comparison-base form:
/// 1. Replace POJ nasal markers `ⁿ` (U+207F) / `ᴺ` (U+1D3A) with `nn`.
/// 2. NFD-decompose so combining marks become individually accessible.
/// 3. Replace every standalone `\u{0358}` (POJ `o͘` combining dot)
///    with `o` so the original `o\u{0358}` collapses into `oo`
///    regardless of the canonical combining-class reorder that NFD
///    applies when other diacritics sit between `o` and `\u{0358}`.
///
/// CROSS-PLATFORM INVARIANT — byte-exact mirror of iOS
/// `Lexicon/Utils/TaigiUnicode.swift::nfdPreprocessed` and Android
/// `ime/dictionary/TaigiUnicode.kt::nfdPreprocessed` until v3.5.3
/// follow-up (PR #192) removed those platform copies. Production routes:
/// - `engine/ranking::score::roman_to_base` (candidate scoring)
/// - `Method::NfdPreprocessForLookup` (URL builder phonetic prep)
///
/// Distinct from [`trie_key_unicode_form`] — see module-level docs.
pub fn taigi_unicode_base_form(input: &str) -> String {
    let with_nasal = input.replace(['\u{207f}', '\u{1d3a}'], "nn");
    let decomposed: String = with_nasal.nfd().collect();
    decomposed.replace('\u{0358}', "o")
}

// ===========================================================================
// ToneRestoration
// ===========================================================================

/// `Method::RestoreTone` — find the LAST combining tone mark in NFD-decomposed
/// `text`, remove it, and NFC-recompose. Returns `None` if no tone mark
/// found. Replaces both platforms' `ToneRestoration.restore`.
pub(crate) fn restore_tone(text: &str) -> Option<String> {
    if text.is_empty() {
        return None;
    }
    let decomposed: Vec<char> = text.nfd().collect();
    for i in (0..decomposed.len()).rev() {
        if COMBINING_TO_TONE_NUM.contains_key(&decomposed[i]) {
            let restored: String = decomposed
                .iter()
                .enumerate()
                .filter_map(|(j, c)| if j == i { None } else { Some(*c) })
                .collect();
            return Some(restored.nfc().collect::<String>());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // taigi_unicode_base_form — moved from engine/ranking/src/nfd.rs in
    // v3.5.3 follow-up (PR #192) — consolidates the helper that backs
    // both ranking and Method::NfdPreprocessForLookup. Test cases
    // preserved verbatim from the original ranking-side module.

    #[test]
    fn nasal_marker_superscript_n_becomes_nn() {
        assert_eq!(taigi_unicode_base_form("sa\u{207f}"), "sann");
    }

    #[test]
    fn nasal_marker_modifier_n_becomes_nn() {
        assert_eq!(taigi_unicode_base_form("sa\u{1d3a}"), "sann");
    }

    #[test]
    fn poj_o_dot_collapses_to_oo() {
        // "ho͘" — o + U+0358. After NFD it stays as `o\u{0358}` (no further
        // decomposition), then \u{0358} → "o" gives "hoo".
        assert_eq!(taigi_unicode_base_form("ho\u{0358}"), "hoo");
    }

    #[test]
    fn poj_o_dot_with_acute_handles_ccc_reorder() {
        // "hó͘" composed: NFC `h ó ͘`. NFD reorders to `h, o, combining_acute,
        // combining_dot_above` (CCC 230 before CCC 232). Replacing
        // \u{0358} with "o" gives `h, o, combining_acute, o`, visually "hóo".
        let input = "h\u{00f3}\u{0358}";
        let result = taigi_unicode_base_form(input);
        let expected = "ho\u{0301}o";
        assert_eq!(result, expected);
    }

    #[test]
    fn no_special_chars_preserves_nfd_decompose() {
        // Plain "tâi-gí" with NFC accents: NFD decomposes the diacritics.
        let input = "t\u{00e2}i-g\u{00ed}";
        let result = taigi_unicode_base_form(input);
        let expected = "ta\u{0302}i-gi\u{0301}";
        assert_eq!(result, expected);
    }

    #[test]
    fn empty_string_is_fixed_point() {
        assert_eq!(taigi_unicode_base_form(""), "");
    }

    #[test]
    fn ascii_only_passes_through() {
        assert_eq!(taigi_unicode_base_form("hello"), "hello");
    }

    /// Pin behaviour against URL-builder use case: a tone-marked Taigi
    /// syllable from MOE / Chhoe Taigi keeps its tone diacritics and only
    /// the lookup-style preprocessing (nasal substitution + dot collapse)
    /// applies. The caller (`ExternalLookupURLBuilder`) feeds the result
    /// into `Method::StripTone` to peel the diacritic.
    #[test]
    fn url_builder_pipeline_keeps_tone_diacritic_intact() {
        // "tāi" — NFD `t a U+0304 i`. base_form keeps the macron because
        // U+0304 is not in {U+207F, U+1D3A, U+0358}.
        let input = "t\u{0101}i";
        let result = taigi_unicode_base_form(input);
        let expected = "ta\u{0304}i";
        assert_eq!(result, expected);
    }
}
