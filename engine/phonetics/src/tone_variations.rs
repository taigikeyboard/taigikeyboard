//! `Method::GetToneVariations` — init-bulk-pull builder for callout tone
//! variation tables.
//!
//! Mirrors iOS `Callouts/Callouts+TaigiCalloutMaps.swift` `TaigiToneMaps`
//! `buildToneMap(mode:)` static initializer. Returns POJ + TL maps in one
//! response so platform caches once at engine init (lazy + idempotent).
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

/// Insert a two-letter base (`oo`, `ng`) under three keys: lowercase (`oo`),
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

    // POJ + TL: append "ⁿ" (U+207F) to existing "n" entry.
    let n_entry = mapping.entry("n".to_string()).or_default();
    n_entry.push("\u{207f}".to_string());

    mapping
        .into_iter()
        .map(|(k, v)| (k, ToneVariationList { variations: v }))
        .collect()
}

/// Returns POJ + TL tone-variation maps in one response.
pub(crate) fn build() -> ToneVariationsResult {
    let poj_variations = build_mode_map(false);
    let tl_variations = build_mode_map(true);
    log::info!(
        "tone_variations init bulk-pull (poj={} bases, tl={} bases)",
        poj_variations.len(),
        tl_variations.len()
    );
    ToneVariationsResult {
        poj_variations,
        tl_variations,
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
}
