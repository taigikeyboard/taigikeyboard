//! CustomDictionaryDerivation port + InputNormalizer port + ToneRestoration port.
//!
//! Mirrors:
//! - iOS `Lexicon/Database/CustomDictionaryDerivation.swift` (`generateNotone`,
//!   `generateAbbrev`).
//! - iOS `Lexicon/Trie/InputNormalizer.swift` (`normalize`, `hasToneMarks`,
//!   `normalizeSyllable`).
//! - iOS `Phonetics/ToneRestoration.swift` (`restore`).
//!
//! Whitespace canonical for `derive_abbrev` = `[ \t\n\x0B\f\r-]+` literal
//! (ASCII whitespace + hyphen). Matches Android JVM `Regex("[\\s-]+")`
//! semantics; preserves NBSP (U+00A0) as non-delimiter per Codex v3 §1.

use crate::api::to_tone_number;
use crate::parser::{is_stop_tone, normalize_to_tl, split_initial_final};
use crate::tps;
use crate::tables::COMBINING_TO_TONE_NUM;
use unicode_normalization::UnicodeNormalization;

// ===========================================================================
// CustomDictionaryDerivation
// ===========================================================================

/// `Method::DeriveNotone` — strips tone diacritics + digits + hyphens + spaces,
/// after lowercase + nasal marker conversion (ⁿ U+207F / ᴺ U+1D3A → nn).
pub fn derive_notone(roman: &str) -> String {
    let with_nasal_converted = roman
        .to_lowercase()
        .replace('\u{207F}', "nn")
        .replace('\u{1D3A}', "nn");
    let decomposed: String = with_nasal_converted.nfd().collect();
    let mut result = String::new();
    for ch in decomposed.chars() {
        if is_nonspacing_mark(ch) {
            continue;
        }
        // ASCII digit, hyphen, space — drop.
        let cp = ch as u32;
        if (0x30..=0x39).contains(&cp) {
            continue;
        }
        if ch == '-' || ch == ' ' {
            continue;
        }
        result.push(ch);
    }
    result.nfc().collect::<String>()
}

/// `Method::DeriveAbbrev` — first char per syllable, diacritics stripped.
/// Returns "" when fewer than 2 syllables.
///
/// Whitespace split = ASCII `[ \t\n\x0B\f\r-]+` literal (matches Android JVM
/// behavior; NBSP U+00A0 stays a non-delimiter).
pub fn derive_abbrev(roman: &str) -> String {
    let lowered = roman.to_lowercase();
    let syllables: Vec<&str> = lowered
        .split(|c: char| matches!(c, ' ' | '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | '-'))
        .filter(|s| !s.is_empty())
        .collect();
    if syllables.len() < 2 {
        return String::new();
    }
    syllables
        .iter()
        .map(|s| {
            let first: String = s.chars().take(1).collect();
            strip_diacritics(&first)
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Internal helper used by `derive_abbrev`. NOT exposed as an op (Codex v1
/// Decision 4 — only dedicated derivation ops are exposed; primitives stay
/// internal so platform cannot rebuild custom-dict semantics).
fn strip_diacritics(s: &str) -> String {
    let decomposed: String = s.nfd().collect();
    let stripped: String = decomposed.chars().filter(|c| !is_nonspacing_mark(*c)).collect();
    stripped.nfc().collect()
}

fn is_nonspacing_mark(c: char) -> bool {
    // Unicode Mn category. Stable subset covering combining marks used by
    // POJ / TL tone diacritics.
    matches!(c as u32, 0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F)
}

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
    for syl in lowered.split(|c: char| c == '-' || c == ' ') {
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
    let with_nasal = text.replace('\u{207f}', "nn").replace('\u{1d3a}', "nn");
    let decomposed: String = with_nasal.nfd().collect();
    // o + combining dot above right (U+0358) → "oo"
    decomposed.replace("o\u{0358}", "oo").replace("O\u{0358}", "Oo")
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

// Re-export so `api.rs` test target keeps existing helpers reachable
// without depending on `dispatch.rs`.
pub use crate::api::{to_tone_marks as _alias_to_tone_marks};
#[allow(unused_imports)]
use _alias_to_tone_marks as _;

// Suppress unused-import warning when split_initial_final / is_stop_tone
// only feed normalize_syllable which is module-private.
#[allow(dead_code)]
fn _keep_imports_alive() {
    let _ = split_initial_final;
    let _ = is_stop_tone;
    let _ = normalize_to_tl;
    let _ = to_tone_number;
}
