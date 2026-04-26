//! Syllable parsing — ported from `taigi-converter/src/phonetics.js`.

use crate::tables::{COMBINING_TO_TONE_NUM, TL_FINALS, TL_INITIALS};
use unicode_normalization::UnicodeNormalization;

/// Strip the tone mark from `text`, returning `(bare NFC text, tone digit)`.
/// Recognises both NFD combining marks and trailing ASCII digits 1..=9.
/// `tone` is the empty string when no mark is present.
pub fn strip_tone_mark(text: &str) -> (String, String) {
    // Fast path: pure-ASCII input cannot carry combining marks. NFD/NFC are
    // no-ops on ASCII, so skip the allocation. This is the common case for
    // raw IME keystrokes (e.g. "ka2", "tshiu7", "hello").
    if text.is_ascii() {
        if let Some(last) = text.chars().last() {
            if let Some(d) = last.to_digit(10) {
                if (1..=9).contains(&d) {
                    return (text[..text.len() - 1].to_string(), d.to_string());
                }
            }
        }
        return (text.to_string(), String::new());
    }

    let decomposed: String = text.nfd().collect();
    if let Some((idx, ch)) = decomposed
        .char_indices()
        .find(|(_, c)| COMBINING_TO_TONE_NUM.contains_key(c))
    {
        let tone = COMBINING_TO_TONE_NUM[&ch];
        let mut bare = String::with_capacity(decomposed.len() - ch.len_utf8());
        bare.push_str(&decomposed[..idx]);
        bare.push_str(&decomposed[idx + ch.len_utf8()..]);
        return (bare.nfc().collect(), tone.to_string());
    }

    // No combining mark — check for trailing 1..=9 digit on the original input.
    if let Some(last) = text.chars().last() {
        if let Some(d) = last.to_digit(10) {
            if (1..=9).contains(&d) {
                let bare: String = text.chars().take(text.chars().count() - 1).collect();
                return (bare.nfc().collect(), d.to_string());
            }
        }
    }

    (text.nfc().collect(), String::new())
}

/// Lowercase, then map POJ-style spellings into TL spellings.
/// Order is meaningful: `oonn` collapses into `onn` only after `oo` substitutions
/// have already happened, mirroring the JS source.
pub fn normalize_to_tl(text: &str) -> String {
    text.replace("ch", "ts")
        .replace("ou", "oo")
        .replace("o\u{0358}", "oo")
        .replace(['\u{207f}', '\u{1d3a}'], "nn")
        .replace("oa", "ua")
        .replace("oe", "ue")
        .replace("eng", "ing")
        .replace("ek", "ik")
        .replace("oonn", "onn")
}

/// True when the final ends with a stop consonant (p, t, k, h), ignoring trailing
/// nasal `nn`. `kah4` → true; `kann2` → false.
pub fn is_stop_tone(final_str: &str) -> bool {
    let cleaned = final_str.to_lowercase().replace("nn", "");
    cleaned.ends_with('p')
        || cleaned.ends_with('t')
        || cleaned.ends_with('k')
        || cleaned.ends_with('h')
}

/// Split `text` into `(initial, final)` by iterating prefixes against the TL
/// initial / final tables. `text` must already be lowercase + TL-normalised.
pub fn split_initial_final(text: &str) -> Option<(String, String)> {
    for i in 0..=text.len() {
        if !text.is_char_boundary(i) {
            continue;
        }
        let initial = &text[..i];
        if TL_INITIALS.contains(initial) {
            let final_str = &text[i..];
            if TL_FINALS.contains(final_str) {
                return Some((initial.to_string(), final_str.to_string()));
            }
        }
    }
    None
}

/// Parse a syllable into `(initial, final, tone)`. Returns `None` when the
/// syllable cannot be split. Inferred tones: `4` for stop finals, `1` otherwise.
pub fn parse_syllable(text: &str) -> Option<(String, String, String)> {
    let (bare, tone) = strip_tone_mark(text);
    let normalized = normalize_to_tl(&bare.to_lowercase());
    let (initial, final_str) = split_initial_final(&normalized)?;
    let final_tone = if tone.is_empty() {
        if is_stop_tone(&final_str) { "4" } else { "1" }.to_string()
    } else {
        tone
    };
    Some((initial, final_str, final_tone))
}
