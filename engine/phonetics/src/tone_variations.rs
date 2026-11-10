//! `Method::GetToneVariations` — init-bulk-pull builder for callout tone
//! variation tables.
//!
//! Owns the callout tone maps formerly built by iOS `buildToneMap(mode:)`; the
//! only source on both mobile platforms (iOS `Callouts/TaigiCallouts+Maps.swift`,
//! Android `PhoneticsBridge.kt` `RustEngineBridge.toneVariations`). Returns the POJ,
//! TL and universal (both scripts, for a custom layout shown in TL and POJ alike)
//! maps in one response so platform caches once at engine init (lazy + idempotent).
//!
//! Pattern matches khiin-rs `loadSettings → AppConfig` cached snapshot.

use crate::tables::TONE_NUM_TO_COMBINING;
use protos::engine::{ToneVariationList, ToneVariationsResult};
use std::collections::HashMap;
use unicode_normalization::UnicodeNormalization;

const TONE_NUMBERS: [&str; 7] = ["2", "3", "5", "6", "7", "8", "9"];
const VOWEL_BASES: [&str; 5] = ["a", "e", "i", "o", "u"];
const CONSONANT_BASES: [&str; 2] = ["n", "m"];

/// Combining mark resolver matching iOS `combiningMark(for:mode:)`.
fn combining_mark(tone: &str, is_tl: bool) -> &'static str {
    if tone == "9" && is_tl {
        // TL tone 9: double acute U+030B (POJ uses breve U+0306).
        "\u{030B}"
    } else {
        TONE_NUM_TO_COMBINING.get(tone).copied().unwrap_or("")
    }
}

/// Build toned variations by inserting combining mark between `base` and
/// `suffix`. Output is NFC-recomposed.
fn build_variations(base: &str, suffix: &str, is_tl: bool) -> Vec<String> {
    TONE_NUMBERS
        .iter()
        .filter_map(|tone| {
            let mark = combining_mark(tone, is_tl);
            if mark.is_empty() {
                return None;
            }
            let raw = format!("{base}{mark}{suffix}");
            Some(raw.nfc().collect::<String>())
        })
        .collect()
}

fn uppercase_first_only(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Insert a two-letter base (`oo`, `ng`, `er` …) under three keys: lowercase (`oo`),
/// Shift (`Oo`, first letter upper) and Caps Lock (`OO`, all upper).
fn insert_digraph(
    mapping: &mut HashMap<String, Vec<String>>,
    base: &str,
    suffix: &str,
    is_tl: bool,
) {
    let variations = build_variations(base, suffix, is_tl);
    let shift_key = uppercase_first_only(&format!("{base}{suffix}"));
    let shift_variations = variations.iter().map(|s| uppercase_first_only(s)).collect();
    let caps_variations = variations.iter().map(|s| s.to_uppercase()).collect();
    mapping.insert(format!("{base}{suffix}").to_uppercase(), caps_variations);
    mapping.insert(shift_key, shift_variations);
    mapping.insert(format!("{base}{suffix}"), variations);
}

fn build_mode_map(is_tl: bool) -> HashMap<String, ToneVariationList> {
    let mut mapping: HashMap<String, Vec<String>> = HashMap::new();

    // Vowel bases (a, e, i, o, u) — single-char, full uppercase trivially.
    for base in VOWEL_BASES {
        let variations = build_variations(base, "", is_tl);
        let upper_variations: Vec<String> = variations.iter().map(|s| s.to_uppercase()).collect();
        mapping.insert(base.to_string(), variations);
        mapping.insert(base.to_uppercase(), upper_variations);
    }

    if is_tl {
        // TL: oo (double o, tone mark on first o).
        insert_digraph(&mut mapping, "o", "o", is_tl);
    } else {
        // POJ: o͘ = o + combining dot above right (U+0358).
        let suffix = "\u{0358}";
        let variations = build_variations("o", suffix, false);
        let upper_variations: Vec<String> = variations.iter().map(|s| s.to_uppercase()).collect();
        mapping.insert(format!("o{suffix}"), variations);
        mapping.insert(format!("O{suffix}"), upper_variations);
    }

    // Syllabic consonants n, m (appended to existing entries, base may
    // already exist if the loop above touched it — n/m are NOT vowels so
    // they fall in this branch).
    for base in CONSONANT_BASES {
        let variations = build_variations(base, "", is_tl);
        let upper_variations: Vec<String> = variations.iter().map(|s| s.to_uppercase()).collect();
        let entry = mapping.entry(base.to_string()).or_default();
        entry.extend(variations);
        let upper_entry = mapping.entry(base.to_uppercase()).or_default();
        upper_entry.extend(upper_variations);
    }

    // ng — tone mark on n, g is suffix.
    insert_digraph(&mut mapping, "n", "g", is_tl);

    // Dialectal vowels er / ir / or (`tables::TL_FINALS`; engine POJ spells them the
    // same) — tone mark on the vowel, r is suffix (`tl::place_tl_tone_mark`).
    for vowel in ["e", "i", "o"] {
        insert_digraph(&mut mapping, vowel, "r", is_tl);
    }

    // POJ + TL: append "ⁿ" (U+207F) to existing "n" entry.
    let n_entry = mapping.entry("n".to_string()).or_default();
    n_entry.push("\u{207f}".to_string());

    mapping
        .into_iter()
        .map(|(k, v)| (k, ToneVariationList { variations: v }))
        .collect()
}

/// Every key of `tl` and `poj`, its two lists interleaved entry by entry, each entry once.
/// The lists are index-aligned (same tones in the same order, only tone 9's mark differs),
/// so each tone-9 pair sits together: `a` → … a̍ a̋ ă; `n` → … n̋ n̆ ⁿ. A key in one map
/// only (`oo` TL, `o͘` POJ) keeps its own list.
fn build_universal_map(
    tl: &HashMap<String, ToneVariationList>,
    poj: &HashMap<String, ToneVariationList>,
) -> HashMap<String, ToneVariationList> {
    let empty = Vec::new();
    tl.keys()
        .chain(poj.keys().filter(|key| !tl.contains_key(*key)))
        .map(|key| {
            let tl_list = tl.get(key).map_or(&empty, |l| &l.variations);
            let poj_list = poj.get(key).map_or(&empty, |l| &l.variations);
            let mut merged: Vec<String> = Vec::new();
            for i in 0..tl_list.len().max(poj_list.len()) {
                for entry in [tl_list.get(i), poj_list.get(i)].into_iter().flatten() {
                    if !merged.contains(entry) {
                        merged.push(entry.clone());
                    }
                }
            }
            (key.clone(), ToneVariationList { variations: merged })
        })
        .collect()
}

/// Returns the POJ, TL and universal tone-variation maps in one response.
pub(crate) fn build() -> ToneVariationsResult {
    let poj_variations = build_mode_map(false);
    let tl_variations = build_mode_map(true);
    let universal_variations = build_universal_map(&tl_variations, &poj_variations);
    log::info!(
        "tone_variations init bulk-pull (poj={} bases, tl={} bases, universal={} bases)",
        poj_variations.len(),
        tl_variations.len(),
        universal_variations.len()
    );
    ToneVariationsResult {
        poj_variations,
        tl_variations,
        universal_variations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variations(map: &HashMap<String, ToneVariationList>, key: &str) -> Vec<String> {
        map.get(key)
            .unwrap_or_else(|| panic!("no callout entry for {key:?}"))
            .variations
            .clone()
    }

    /// Pins every lowercase key's full list in order: Android lost its JSON
    /// mirror of this table, so these lists are what both keyboards show.
    #[test]
    fn full_lists_per_base_both_modes() {
        // trace: TONE_NUMBERS 2 3 5 6 7 8 9 → U+0301 U+0300 U+0302 U+030C U+0304
        // U+030D, then tone 9 = U+030B (TL) / U+0306 (POJ); NFC recomposes where a
        // precomposed letter exists; `n` ends with ⁿ; er / ir / or carry it on the vowel.
        let tl = build_mode_map(true);
        let poj = build_mode_map(false);
        let expected_tl: [(&str, &[&str]); 12] = [
            ("a", &["á", "à", "â", "ǎ", "ā", "a\u{30D}", "a\u{30B}"]),
            ("e", &["é", "è", "ê", "ě", "ē", "e\u{30D}", "e\u{30B}"]),
            ("i", &["í", "ì", "î", "ǐ", "ī", "i\u{30D}", "i\u{30B}"]),
            ("o", &["ó", "ò", "ô", "ǒ", "ō", "o\u{30D}", "ő"]),
            ("u", &["ú", "ù", "û", "ǔ", "ū", "u\u{30D}", "ű"]),
            ("oo", &["óo", "òo", "ôo", "ǒo", "ōo", "o\u{30D}o", "őo"]),
            (
                "m",
                &[
                    "ḿ", "m\u{300}", "m\u{302}", "m\u{30C}", "m\u{304}", "m\u{30D}", "m\u{30B}",
                ],
            ),
            (
                "n",
                &[
                    "ń", "ǹ", "n\u{302}", "ň", "n\u{304}", "n\u{30D}", "n\u{30B}", "ⁿ",
                ],
            ),
            (
                "ng",
                &[
                    "ńg",
                    "ǹg",
                    "n\u{302}g",
                    "ňg",
                    "n\u{304}g",
                    "n\u{30D}g",
                    "n\u{30B}g",
                ],
            ),
            (
                "er",
                &["ér", "èr", "êr", "ěr", "ēr", "e\u{30D}r", "e\u{30B}r"],
            ),
            (
                "ir",
                &["ír", "ìr", "îr", "ǐr", "īr", "i\u{30D}r", "i\u{30B}r"],
            ),
            ("or", &["ór", "òr", "ôr", "ǒr", "ōr", "o\u{30D}r", "őr"]),
        ];
        let expected_poj: [(&str, &[&str]); 12] = [
            ("a", &["á", "à", "â", "ǎ", "ā", "a\u{30D}", "ă"]),
            ("e", &["é", "è", "ê", "ě", "ē", "e\u{30D}", "ĕ"]),
            ("i", &["í", "ì", "î", "ǐ", "ī", "i\u{30D}", "ĭ"]),
            ("o", &["ó", "ò", "ô", "ǒ", "ō", "o\u{30D}", "ŏ"]),
            ("u", &["ú", "ù", "û", "ǔ", "ū", "u\u{30D}", "ŭ"]),
            (
                "o\u{358}",
                &[
                    "ó\u{358}",
                    "ò\u{358}",
                    "ô\u{358}",
                    "ǒ\u{358}",
                    "ō\u{358}",
                    "o\u{30D}\u{358}",
                    "ŏ\u{358}",
                ],
            ),
            (
                "m",
                &[
                    "ḿ", "m\u{300}", "m\u{302}", "m\u{30C}", "m\u{304}", "m\u{30D}", "m\u{306}",
                ],
            ),
            (
                "n",
                &[
                    "ń", "ǹ", "n\u{302}", "ň", "n\u{304}", "n\u{30D}", "n\u{306}", "ⁿ",
                ],
            ),
            (
                "ng",
                &[
                    "ńg",
                    "ǹg",
                    "n\u{302}g",
                    "ňg",
                    "n\u{304}g",
                    "n\u{30D}g",
                    "n\u{306}g",
                ],
            ),
            ("er", &["ér", "èr", "êr", "ěr", "ēr", "e\u{30D}r", "ĕr"]),
            ("ir", &["ír", "ìr", "îr", "ǐr", "īr", "i\u{30D}r", "ĭr"]),
            ("or", &["ór", "òr", "ôr", "ǒr", "ōr", "o\u{30D}r", "ŏr"]),
        ];
        for (map, expected) in [(&tl, &expected_tl), (&poj, &expected_poj)] {
            for (key, list) in expected.iter() {
                assert_eq!(variations(map, key), *list, "key {key:?}");
                // Shift capitalises the first letter, Caps Lock every letter (one key for a
                // single letter); ⁿ rides the lowercase `n` list only.
                let tones = list.iter().filter(|s| **s != "\u{207f}");
                let shift: Vec<String> = tones.clone().map(|s| uppercase_first_only(s)).collect();
                let caps: Vec<String> = tones.map(|s| s.to_uppercase()).collect();
                assert_eq!(
                    variations(map, &uppercase_first_only(key)),
                    shift,
                    "Shift {key:?}"
                );
                assert_eq!(variations(map, &key.to_uppercase()), caps, "Caps {key:?}");
            }
            let lowercase_keys = map.keys().filter(|k| k.to_lowercase() == **k).count();
            assert_eq!(lowercase_keys, expected.len());
        }
    }

    #[test]
    fn tl_oo_callouts_shift_first_letter_caps_lock_all_letters() {
        let tl = build_mode_map(true);
        // trace: build_variations("o","o") tone 2 → "o"+U+0301+"o" → NFC "óo"
        assert_eq!(variations(&tl, "oo")[0], "óo");
        assert_eq!(variations(&tl, "Oo")[0], "Óo");
        assert_eq!(variations(&tl, "OO")[0], "ÓO");
        assert_eq!(variations(&tl, "OO").len(), variations(&tl, "oo").len());
    }

    #[test]
    fn ng_callouts_shift_first_letter_caps_lock_all_letters_both_modes() {
        for is_tl in [true, false] {
            let map = build_mode_map(is_tl);
            // trace: build_variations("n","g") tone 2 → "n"+U+0301+"g" → NFC "ńg"
            assert_eq!(variations(&map, "ng")[0], "ńg");
            assert_eq!(variations(&map, "Ng")[0], "Ńg");
            assert_eq!(variations(&map, "NG")[0], "ŃG");
        }
    }

    /// TL and POJ lists hold the same tones in the same order; only tone 9 (the 7th entry)
    /// differs. The universal interleave relies on it.
    #[test]
    fn tl_and_poj_lists_differ_only_at_tone_nine() {
        let tl = build_mode_map(true);
        let poj = build_mode_map(false);
        for (key, tl_list) in &tl {
            let Some(poj_list) = poj.get(key) else {
                continue;
            };
            let (tl_list, poj_list) = (&tl_list.variations, &poj_list.variations);
            assert_eq!(tl_list.len(), poj_list.len(), "key {key:?}");
            for (i, (t, p)) in tl_list.iter().zip(poj_list).enumerate() {
                assert_eq!(t == p, i != 6, "key {key:?} entry {i}");
            }
        }
    }

    #[test]
    fn universal_lists_hold_both_tone_nine_marks() {
        let tl = build_mode_map(true);
        let poj = build_mode_map(false);
        let universal = build_universal_map(&tl, &poj);
        // trace: TL tone list (2 3 5 6 7 8 9=U+030B) interleaved with POJ's (9=U+0306);
        // tones 2–8 are equal and kept once, then TL 9, POJ 9, then `n`'s ⁿ.
        let expected: [(&str, &[&str]); 8] = [
            ("a", &["á", "à", "â", "ǎ", "ā", "a\u{30D}", "a\u{30B}", "ă"]),
            ("A", &["Á", "À", "Â", "Ǎ", "Ā", "A\u{30D}", "A\u{30B}", "Ă"]),
            ("o", &["ó", "ò", "ô", "ǒ", "ō", "o\u{30D}", "ő", "ŏ"]),
            (
                "n",
                &[
                    "ń", "ǹ", "n\u{302}", "ň", "n\u{304}", "n\u{30D}", "n\u{30B}", "n\u{306}", "ⁿ",
                ],
            ),
            (
                "Ng",
                &[
                    "Ńg",
                    "Ǹg",
                    "N\u{302}g",
                    "Ňg",
                    "N\u{304}g",
                    "N\u{30D}g",
                    "N\u{30B}g",
                    "N\u{306}g",
                ],
            ),
            (
                "er",
                &["ér", "èr", "êr", "ěr", "ēr", "e\u{30D}r", "e\u{30B}r", "ĕr"],
            ),
            // One script only: its own list.
            ("oo", &["óo", "òo", "ôo", "ǒo", "ōo", "o\u{30D}o", "őo"]),
            (
                "o\u{358}",
                &[
                    "ó\u{358}",
                    "ò\u{358}",
                    "ô\u{358}",
                    "ǒ\u{358}",
                    "ō\u{358}",
                    "o\u{30D}\u{358}",
                    "ŏ\u{358}",
                ],
            ),
        ];
        for (key, list) in expected {
            assert_eq!(variations(&universal, key), list, "key {key:?}");
        }
        let keys = |m: &HashMap<String, ToneVariationList>| {
            m.keys().cloned().collect::<std::collections::HashSet<_>>()
        };
        assert_eq!(keys(&universal), &keys(&tl) | &keys(&poj));
    }
}
