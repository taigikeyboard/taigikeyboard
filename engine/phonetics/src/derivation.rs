//! CustomDictionaryDerivation port.
//!
//! Mirrors iOS `Lexicon/Database/CustomDictionaryDerivation.swift`
//! (`generateNotone`, `generateAbbrev` → the legacy first-letter face), and
//! owns the index abbreviation face ([`derive_abbrev`], one leading
//! spelling unit per syllable) the dictionary build and the custom-dict
//! search keys share.
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

/// `Method::DeriveAbbrev` — the LEGACY custom-dictionary `abbrev` column:
/// first char per syllable, diacritics stripped, "" when fewer than 2
/// syllables. Kept at first-letter semantics because that column is a
/// stored rollback contract on iOS / Android
/// (`INVARIANT_abbrev_key_is_one_char_per_syllable`) and no query reads it
/// any more; the index face is [`derive_abbrev`].
///
/// Whitespace split = ASCII `[ \t\n\x0B\f\r-]+` literal (matches Android JVM
/// behavior; NBSP U+00A0 stays a non-delimiter).
pub fn derive_abbrev_first_letter(roman: &str) -> String {
    let lowered = roman.to_lowercase();
    let syllables: Vec<&str> = lowered
        .split(SYLLABLE_DELIMITERS)
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

/// The abbreviation face of a romanized reading — the runtime mirror of
/// `dictionary/common/abbrev.py::extract_abbrev`, which fills the
/// `tl_abbrev` / `poj_abbrev` CSV columns `create_fst.py` indexes under
/// `tl-abbrev:` / `poj-abbrev:`, and the face `custom_search` stores for a
/// user's own words. Parity with the shipped CSV is pinned by
/// `lexicon/tests/roman_num_face_parity.rs`.
///
/// One **leading spelling unit** per syllable: the longest prefix of the
/// lowercased, diacritic-stripped syllable that is a TL or POJ initial
/// ([`ABBREV_INITIALS`] — `knowledge/taigi-phonetics-reference.md` §2, so
/// `ph` / `th` / `kh` / `tsh` / `chh` / `ng` stay whole), or the first
/// letter when no initial matches (a zero-initial syllable, `âng` → `a`).
/// This is what TPS has always done with its one-glyph initials (披頭巾
/// `ㄆㄊㄍ`), and what a typist means by `phthk` for `phi-thâu-kin`
/// (USER 2026-09-18: "aspirates (p/ph, t/th, k/kh, ts/tsh) must be handled on their own"). Longest
/// match, not the syllable parser's shortest-first split, so syllabic 黃
/// `ng` keeps `ng` and `nng` gives `n`. "" when fewer than 2 syllables.
///
/// Whitespace split = ASCII `[ \t\n\x0B\f\r-]+` literal (matches Android JVM
/// behavior; NBSP U+00A0 stays a non-delimiter).
pub fn derive_abbrev(roman: &str) -> String {
    let lowered = roman.to_lowercase();
    let syllables: Vec<&str> = lowered
        .split(SYLLABLE_DELIMITERS)
        .filter(|s| !s.is_empty())
        .collect();
    if syllables.len() < 2 {
        return String::new();
    }
    syllables
        .iter()
        .map(|s| leading_unit(&strip_diacritics(s)))
        .collect::<String>()
}

/// ASCII whitespace + hyphen, the syllable delimiters of both faces.
const SYLLABLE_DELIMITERS: [char; 7] = [' ', '\t', '\n', '\u{0B}', '\u{0C}', '\r', '-'];

/// TL and POJ initials, longest first so a prefix scan takes `tsh` before
/// `ts` before `t`, `chh` before `ch`, `ng` before `n`
/// (`knowledge/taigi-phonetics-reference.md` §2: TL `ts` / `tsh` ↔ POJ
/// `ch` / `chh`, the rest shared). One table for both scripts: `ch` never
/// starts a TL syllable, and a traditional-POJ `ts…` spelling reads as the
/// same `ts` unit it would in TL, so the union changes no verdict.
const ABBREV_INITIALS: [&str; 19] = [
    "tsh", "chh", "ts", "ch", "ph", "th", "kh", "ng", "p", "m", "b", "t", "n", "l", "k", "g", "s",
    "j", "h",
];

/// The leading spelling unit of one bare (lowercased, diacritic-stripped)
/// syllable: its initial, or its first char when it has none.
fn leading_unit(syllable: &str) -> String {
    ABBREV_INITIALS
        .iter()
        .find(|initial| syllable.starts_with(*initial))
        .map(|initial| initial.to_string())
        .unwrap_or_else(|| syllable.chars().take(1).collect())
}

/// The `poj_abbrev` face of a canonical TL reading — the abbreviation of its
/// POJ display (`tsia̍h-pn̄g` → `chia̍h-pn̄g` → `chp`), as
/// `dictionary/common/stages/abbrev.py` derives it from the `poj` column.
/// Sibling of [`crate::tps_abbrev_from_tl`] for the third family.
pub fn poj_abbrev_from_tl(tl: &str) -> String {
    derive_abbrev(&crate::api::tl_display_to_poj_display(tl))
}

/// Internal helper used by both abbreviation faces. NOT exposed as an op (Codex v1
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

#[cfg(test)]
mod tests {
    use super::*;

    // trace: one leading spelling unit per syllable — aspirates and
    // affricates stay whole (§2 initials), a zero-initial syllable gives
    // its first letter, longest match keeps syllabic `ng` whole and reads
    // `nng` as `n` + `ng`.
    #[test]
    fn derive_abbrev_takes_the_leading_unit_of_each_syllable() {
        assert_eq!(derive_abbrev("phi-thâu-kin"), "phthk", "披頭巾");
        assert_eq!(derive_abbrev("tshut-khì"), "tshkh", "出去");
        assert_eq!(derive_abbrev("tsia̍h-pn̄g"), "tsp", "食飯 (TL)");
        assert_eq!(derive_abbrev("chia̍h-pn̄g"), "chp", "食飯 (POJ)");
        assert_eq!(derive_abbrev("chhut-khì"), "chhkh", "出去 (POJ)");
        assert_eq!(derive_abbrev("só-sî"), "ss", "鎖匙");
        assert_eq!(derive_abbrev("âng-enn-á"), "aea", "紅嬰仔");
        assert_eq!(derive_abbrev("io̍k-iù-īnn"), "iii", "育幼院");
        assert_eq!(derive_abbrev("m̄-sī"), "ms", "毋是");
        assert_eq!(derive_abbrev("n̂g-sng"), "ngs", "黃酸");
        assert_eq!(derive_abbrev("nn̄g-á"), "na", "卵仔");
        assert_eq!(derive_abbrev("ngiau-ti"), "ngt");
        assert_eq!(derive_abbrev("ji̍t-thâu"), "jth", "日頭");
        assert_eq!(derive_abbrev("Guá SĪ"), "gs", "case + space delimiter");
        assert_eq!(derive_abbrev("tâi"), "", "single syllable");
        assert_eq!(derive_abbrev(""), "");
    }

    // trace: the legacy column stays first-letter (`ph` → `p`).
    #[test]
    fn derive_abbrev_first_letter_is_the_legacy_column_face() {
        assert_eq!(derive_abbrev_first_letter("phi-thâu-kin"), "ptk");
        assert_eq!(derive_abbrev_first_letter("só-sî"), "ss");
        assert_eq!(derive_abbrev_first_letter("tâi"), "");
    }

    #[test]
    fn poj_abbrev_from_tl_uses_the_poj_spelling() {
        assert_eq!(poj_abbrev_from_tl("tsia̍h-pn̄g"), "chp");
        assert_eq!(poj_abbrev_from_tl("tshut-khì"), "chhkh");
        assert_eq!(poj_abbrev_from_tl("só-sî"), "ss");
    }
}
