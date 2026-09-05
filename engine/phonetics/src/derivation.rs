//! CustomDictionaryDerivation port.
//!
//! Mirrors iOS `Lexicon/Database/CustomDictionaryDerivation.swift`
//! (`generateNotone`, `generateAbbrev`).
//!
//! Whitespace canonical for `derive_abbrev` = `[ \t\n\x0B\f\r-]+` literal
//! (ASCII whitespace + hyphen). Matches Android JVM `Regex("[\\s-]+")`
//! semantics; preserves NBSP (U+00A0) as non-delimiter per Codex v3 §1.
//!
//! Combining-mark / NFD logic for InputNormalizer + ToneRestoration lives
//! in `normalization.rs` — different concerns, different module.

use unicode_normalization::UnicodeNormalization;

/// `Method::DeriveNotone` — strips tone diacritics + digits + hyphens + spaces
/// from the base form of `roman`.
///
/// The base form ([`taigi_unicode_base_form`]) is what makes a POJ display
/// roman and the ASCII a keyboard types agree: it rewrites the nasal marker
/// ⁿ / ᴺ to `nn` and the `o͘` dot U+0358 to `o`, then NFD-decomposes. Both
/// rewrites MUST happen before the strip below — U+0358 is spelling, not tone
/// (POJ `o͘` is TL `oo`), yet it sits inside the `0x0300..=0x036F` block
/// [`is_nonspacing_mark`] treats as tone material, so an unfolded `o͘` would
/// silently collapse onto a bare `o` and put a stored POJ entry under a key no
/// keystroke produces (user report 2026-08-20: `băng-só͘-khó͘`).
pub(crate) fn derive_notone(roman: &str) -> String {
    let decomposed = crate::taigi_unicode_base_form(&roman.to_lowercase());
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
pub(crate) fn derive_abbrev(roman: &str) -> String {
    let lowered = roman.to_lowercase();
    let syllables: Vec<&str> = lowered
        .split([' ', '\t', '\n', '\u{0B}', '\u{0C}', '\r', '-'])
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
    let stripped: String = decomposed
        .chars()
        .filter(|c| !is_nonspacing_mark(*c))
        .collect();
    stripped.nfc().collect()
}

fn is_nonspacing_mark(c: char) -> bool {
    // Unicode Mn category. Stable subset covering combining marks used by
    // POJ / TL tone diacritics.
    matches!(
        c as u32,
        0x0300..=0x036F | 0x1AB0..=0x1AFF | 0x1DC0..=0x1DFF | 0x20D0..=0x20FF | 0xFE20..=0xFE2F
    )
}
