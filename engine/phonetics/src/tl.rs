//! TL (Tâi-lô) assembly — ported from `taigi-converter/src/tl.js`.

use crate::tables::tl_tone_mark;
use unicode_normalization::UnicodeNormalization;

/// Assemble a TL syllable from `initial + final + tone`. Output is NFC.
pub fn to_tl(initial: &str, final_str: &str, tone: &str) -> String {
    let mark = tl_tone_mark(tone);
    let marked = place_tl_tone_mark(final_str, mark);
    let mut combined = String::with_capacity(initial.len() + marked.len());
    combined.push_str(initial);
    combined.push_str(&marked);
    combined.nfc().collect()
}

/// Vowel priority for TL: `a > oo > ere > e > o > ui→i > iu→u > iri > i > u > ng > m`.
/// Mirrors `placeTlToneMark` in `tl.js`.
fn place_tl_tone_mark(final_str: &str, mark: &str) -> String {
    if mark.is_empty() {
        return final_str.to_string();
    }
    if final_str.contains('a') {
        return final_str.replacen('a', &format!("a{mark}"), 1);
    }
    if final_str.contains("oo") {
        return final_str.replacen("oo", &format!("o{mark}o"), 1);
    }
    if final_str.contains("ere") {
        return final_str.replacen("ere", &format!("ere{mark}"), 1);
    }
    if final_str.contains('e') {
        return final_str.replacen('e', &format!("e{mark}"), 1);
    }
    if final_str.contains('o') {
        return final_str.replacen('o', &format!("o{mark}"), 1);
    }
    if final_str.contains("ui") {
        return final_str.replacen('i', &format!("i{mark}"), 1);
    }
    if final_str.contains("iu") {
        return final_str.replacen('u', &format!("u{mark}"), 1);
    }
    if final_str.contains("iri") {
        return final_str.replacen("iri", &format!("iri{mark}"), 1);
    }
    if final_str.contains('i') {
        return final_str.replacen('i', &format!("i{mark}"), 1);
    }
    if final_str.contains('u') {
        return final_str.replacen('u', &format!("u{mark}"), 1);
    }
    if final_str.contains("ng") {
        return final_str.replacen("ng", &format!("n{mark}g"), 1);
    }
    if final_str.contains('m') {
        return final_str.replacen('m', &format!("m{mark}"), 1);
    }
    final_str.to_string()
}
