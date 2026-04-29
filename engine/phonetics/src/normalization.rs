//! InputNormalizer port + ToneRestoration port.
//!
//! Mirrors:
//! - iOS `Lexicon/Trie/InputNormalizer.swift` (`normalize`, `hasToneMarks`,
//!   `normalizeSyllable`).
//! - iOS `Phonetics/ToneRestoration.swift` (`restore`).
//!
//! Shares NFD + `COMBINING_TO_TONE_NUM` mechanics across both ports — every
//! function in this file works against combining-tone-mark tables, distinct
//! from `derivation.rs` which deals in custom-dictionary string transforms
//! (whitespace splitting, ASCII filtering).

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
pub fn has_tone_marks(text: &str) -> bool {
    text.nfd().any(|c| COMBINING_TO_TONE_NUM.contains_key(&c))
}

fn normalize_syllable(syllable: &str, add_default_tone: bool) -> String {
    if syllable.is_empty() {
        return String::new();
    }
    let with_oo = nfd_preprocessed(syllable);
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
fn nfd_preprocessed(text: &str) -> String {
    let with_nasal = text.replace(['\u{207f}', '\u{1d3a}'], "nn");
    let decomposed: String = with_nasal.nfd().collect();
    // o + combining dot above right (U+0358) → "oo"
    decomposed
        .replace("o\u{0358}", "oo")
        .replace("O\u{0358}", "Oo")
}

// ===========================================================================
// ToneRestoration
// ===========================================================================

/// `Method::RestoreTone` — find the LAST combining tone mark in NFD-decomposed
/// `text`, remove it, and NFC-recompose. Returns `None` if no tone mark
/// found. Replaces both platforms' `ToneRestoration.restore`.
pub fn restore_tone(text: &str) -> Option<String> {
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
