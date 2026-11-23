//! Old table-based case ops, kept only to prove the stdlib rewrite equivalent (R2). Deleted with its proptest.

use once_cell::sync::Lazy;
use std::collections::HashMap;

use crate::api::InputMode;
use crate::case_transform::LetterCase;

/// POJ lowercase → uppercase tone-letter map.
static POJ_LOWER_TO_UPPER: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let pairs: &[(&str, &str)] = &[
        // a
        ("á", "Á"),
        ("à", "À"),
        ("â", "Â"),
        ("ǎ", "Ǎ"),
        ("ā", "Ā"),
        ("a̍", "A̍"),
        ("ă", "Ă"),
        // e
        ("é", "É"),
        ("è", "È"),
        ("ê", "Ê"),
        ("ě", "Ě"),
        ("ē", "Ē"),
        ("e̍", "E̍"),
        ("ĕ", "Ĕ"),
        // i
        ("í", "Í"),
        ("ì", "Ì"),
        ("î", "Î"),
        ("ǐ", "Ǐ"),
        ("ī", "Ī"),
        ("i̍", "I̍"),
        ("ĭ", "Ĭ"),
        // o
        ("ó", "Ó"),
        ("ò", "Ò"),
        ("ô", "Ô"),
        ("ǒ", "Ǒ"),
        ("ō", "Ō"),
        ("o̍", "O̍"),
        ("ŏ", "Ŏ"),
        // o͘ (POJ combining U+0358)
        ("ó͘", "Ó͘"),
        ("ò͘", "Ò͘"),
        ("ô͘", "Ô͘"),
        ("ǒ͘", "Ǒ͘"),
        ("ō͘", "Ō͘"),
        ("o̍͘", "O̍͘"),
        ("ŏ͘", "Ŏ͘"),
        // u
        ("ú", "Ú"),
        ("ù", "Ù"),
        ("û", "Û"),
        ("ǔ", "Ǔ"),
        ("ū", "Ū"),
        ("u̍", "U̍"),
        ("ŭ", "Ŭ"),
        // n
        ("ń", "Ń"),
        ("ǹ", "Ǹ"),
        ("n̂", "N̂"),
        ("ň", "Ň"),
        ("n̄", "N̄"),
        ("n̍", "N̍"),
        ("n̋", "N̋"),
        // m
        ("ḿ", "Ḿ"),
        ("m̀", "M̀"),
        ("m̂", "M̂"),
        ("m̌", "M̌"),
        ("m̄", "M̄"),
        ("m̍", "M̍"),
        ("m̋", "M̋"),
    ];
    pairs.iter().copied().collect()
});

/// TL lowercase → uppercase tone-letter map. Differs from POJ on the
/// 8th-tone diacritic (`̍` U+030D unchanged but tone-9 `̋` U+030B added)
/// and on `oo` (TL doubles instead of POJ's `o͘`).
static TL_LOWER_TO_UPPER: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let pairs: &[(&str, &str)] = &[
        // a
        ("á", "Á"),
        ("à", "À"),
        ("â", "Â"),
        ("ǎ", "Ǎ"),
        ("ā", "Ā"),
        ("a̍", "A̍"),
        ("a̋", "A̋"),
        // e
        ("é", "É"),
        ("è", "È"),
        ("ê", "Ê"),
        ("ě", "Ě"),
        ("ē", "Ē"),
        ("e̍", "E̍"),
        ("e̋", "E̋"),
        // i
        ("í", "Í"),
        ("ì", "Ì"),
        ("î", "Î"),
        ("ǐ", "Ǐ"),
        ("ī", "Ī"),
        ("i̍", "I̍"),
        ("i̋", "I̋"),
        // o
        ("ó", "Ó"),
        ("ò", "Ò"),
        ("ô", "Ô"),
        ("ǒ", "Ǒ"),
        ("ō", "Ō"),
        ("o̍", "O̍"),
        ("ő", "Ő"),
        // oo (TL — doubled vowel form)
        ("óo", "Óo"),
        ("òo", "Òo"),
        ("ôo", "Ôo"),
        ("ǒo", "Ǒo"),
        ("ōo", "Ōo"),
        ("o̍o", "O̍o"),
        ("őo", "Őo"),
        // u
        ("ú", "Ú"),
        ("ù", "Ù"),
        ("û", "Û"),
        ("ǔ", "Ǔ"),
        ("ū", "Ū"),
        ("u̍", "U̍"),
        ("ű", "Ű"),
        // n
        ("ń", "Ń"),
        ("ǹ", "Ǹ"),
        ("n̂", "N̂"),
        ("ň", "Ň"),
        ("n̄", "N̄"),
        ("n̍", "N̍"),
        ("n̋", "N̋"),
        // m
        ("ḿ", "Ḿ"),
        ("m̀", "M̀"),
        ("m̂", "M̂"),
        ("m̌", "M̌"),
        ("m̄", "M̄"),
        ("m̍", "M̍"),
        ("m̋", "M̋"),
    ];
    pairs.iter().copied().collect()
});

/// Reverse maps — populated by inverting the lowercase→uppercase map at
/// first access. POJ and TL each have a 1:1 inverse since the entries are
/// unique grapheme cluster strings.
static POJ_UPPER_TO_LOWER: Lazy<HashMap<&'static str, &'static str>> =
    Lazy::new(|| POJ_LOWER_TO_UPPER.iter().map(|(k, v)| (*v, *k)).collect());

static TL_UPPER_TO_LOWER: Lazy<HashMap<&'static str, &'static str>> =
    Lazy::new(|| TL_LOWER_TO_UPPER.iter().map(|(k, v)| (*v, *k)).collect());

/// Returns the mode-specific lowercase→uppercase map, or `None` for English /
/// unsupported modes (in which case the caller falls back to stdlib casing).
fn lower_to_upper(mode: InputMode) -> Option<&'static HashMap<&'static str, &'static str>> {
    match mode {
        InputMode::Poj => Some(&POJ_LOWER_TO_UPPER),
        InputMode::Tl => Some(&TL_LOWER_TO_UPPER),
        // v3.5.9 D / C-3b — TPS Bopomofo has no upper/lower case axis;
        // table absent so callers fall through to identity (matches English).
        InputMode::Tps | InputMode::English => None,
    }
}

/// Returns the mode-specific uppercase→lowercase map, or `None` for English /
/// unsupported modes (in which case the caller falls back to stdlib casing).
fn upper_to_lower(mode: InputMode) -> Option<&'static HashMap<&'static str, &'static str>> {
    match mode {
        InputMode::Poj => Some(&POJ_UPPER_TO_LOWER),
        InputMode::Tl => Some(&TL_UPPER_TO_LOWER),
        // v3.5.9 D / C-3b — TPS Bopomofo has no upper/lower case axis.
        InputMode::Tps | InputMode::English => None,
    }
}

pub(crate) fn uppercase_tone_char(input: &str, mode: InputMode) -> String {
    uppercase_internal(input, mode, false)
}

pub(crate) fn full_uppercase_tone_string(input: &str, mode: InputMode) -> String {
    uppercase_internal(input, mode, true)
}

fn uppercase_internal(input: &str, mode: InputMode, all_chars: bool) -> String {
    if input == "\u{207F}" {
        return "\u{1D3A}".to_string();
    }
    if let Some(map) = lower_to_upper(mode) {
        if let Some(mapped) = map.get(input) {
            return if all_chars {
                mapped.to_uppercase()
            } else {
                (*mapped).to_string()
            };
        }
    }
    if all_chars {
        input.to_uppercase()
    } else {
        let chars: Vec<char> = input.chars().collect();
        if chars.len() > 1 {
            let mut s = String::with_capacity(input.len());
            let mut iter = chars.into_iter();
            if let Some(first) = iter.next() {
                s.extend(first.to_uppercase());
            }
            for c in iter {
                s.push(c);
            }
            s
        } else {
            input.to_uppercase()
        }
    }
}

pub(crate) fn lowercase_tone_char(input: &str, mode: InputMode) -> String {
    if input == "\u{1D3A}" {
        return "\u{207F}".to_string();
    }
    if let Some(map) = upper_to_lower(mode) {
        if let Some(mapped) = map.get(input) {
            return (*mapped).to_string();
        }
    }
    input.to_lowercase()
}

pub(crate) fn transform_input_case(text: &str, letter_case: LetterCase, mode: InputMode) -> String {
    match letter_case {
        LetterCase::CapsLocked => full_uppercase_tone_string(text, mode),
        LetterCase::Uppercased => capitalize_first_letter(text, mode),
        LetterCase::Lowercased => lowercase_tone_char(text, mode),
    }
}

pub(crate) fn raise_case(text: &str, letter_case: LetterCase, mode: InputMode) -> String {
    match letter_case {
        LetterCase::CapsLocked => full_uppercase_tone_string(text, mode),
        LetterCase::Uppercased => uppercase_first_letter_in_text(text, mode),
        LetterCase::Lowercased => text.to_string(),
    }
}

fn capitalize_first_letter(text: &str, mode: InputMode) -> String {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let first_upper = uppercase_tone_char(&first.to_string(), mode);
    let rest: String = chars.collect();
    let rest_lower: String = rest
        .chars()
        .map(|ch| lowercase_tone_char(&ch.to_string(), mode))
        .collect();
    first_upper + &rest_lower
}

fn uppercase_first_letter_in_text(text: &str, mode: InputMode) -> String {
    let mut result = String::with_capacity(text.len());
    let mut is_first_letter = true;
    for ch in text.chars() {
        if ch.is_alphabetic() && is_first_letter {
            result.push_str(&uppercase_tone_char(&ch.to_string(), mode));
            is_first_letter = false;
        } else {
            result.push(ch);
        }
    }
    result
}

pub(crate) fn match_case(target: &str, source: &str, mode: InputMode) -> String {
    let mut source_letters = source.chars().filter(|c| c.is_alphabetic());
    let mut result = String::with_capacity(target.len());
    for ch in target.chars() {
        if !ch.is_alphabetic() {
            result.push(ch);
            continue;
        }
        match source_letters.next() {
            Some(src) if src.is_uppercase() => {
                result.push_str(&uppercase_tone_char(&ch.to_string(), mode));
            }
            _ => result.push(ch),
        }
    }
    result
}

/// Every key and value of the four tables.
fn table_strings() -> Vec<&'static str> {
    [&*POJ_LOWER_TO_UPPER, &*TL_LOWER_TO_UPPER]
        .into_iter()
        .flat_map(|map| map.iter().flat_map(|(k, v)| [*k, *v]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::case_transform as new;
    use proptest::prelude::*;

    const MODES: [InputMode; 4] = [
        InputMode::Poj,
        InputMode::Tl,
        InputMode::Tps,
        InputMode::English,
    ];
    const CASES: [LetterCase; 3] = [
        LetterCase::Lowercased,
        LetterCase::Uppercased,
        LetterCase::CapsLocked,
    ];

    fn assert_equivalent(input: &str, source: &str) {
        for mode in MODES {
            assert_eq!(
                uppercase_tone_char(input, mode),
                new::uppercase_tone_char(input),
                "{input:?} {mode:?}"
            );
            assert_eq!(
                full_uppercase_tone_string(input, mode),
                new::full_uppercase_tone_string(input),
                "{input:?} {mode:?}"
            );
            assert_eq!(
                lowercase_tone_char(input, mode),
                new::lowercase_tone_char(input),
                "{input:?} {mode:?}"
            );
            assert_eq!(
                match_case(input, source, mode),
                new::match_case(input, source),
                "{input:?} {source:?} {mode:?}"
            );
            for case in CASES {
                assert_eq!(
                    transform_input_case(input, case, mode),
                    new::transform_input_case(input, case),
                    "{input:?} {case:?} {mode:?}"
                );
                assert_eq!(
                    raise_case(input, case, mode),
                    new::raise_case(input, case),
                    "{input:?} {case:?} {mode:?}"
                );
            }
        }
    }

    #[test]
    fn every_table_entry_matches_stdlib() {
        let strings = table_strings();
        assert_eq!(strings.len(), 224);
        for s in &strings {
            assert_equivalent(s, s);
            assert_equivalent(s, &s.to_uppercase());
        }
        for edge in [
            "",
            "ß",
            "ŉ",
            "ǆ",
            "Ǆ",
            "\u{207F}",
            "\u{1D3A}",
            "a\u{207F}",
            "A\u{1D3A}",
        ] {
            assert_equivalent(edge, edge);
            assert_equivalent(edge, "AAAA");
        }
    }

    fn atom() -> impl Strategy<Value = String> {
        let tables: Vec<String> = table_strings().into_iter().map(String::from).collect();
        prop_oneof![
            proptest::sample::select(tables),
            "[a-zA-Z0-9 \\-ßŉǆ\u{00C0}-\u{017F}\u{0300}-\u{036F}\u{0358}\u{207F}\u{1D3A}]{1,3}",
        ]
    }

    fn text() -> impl Strategy<Value = String> {
        proptest::collection::vec(atom(), 0..5).prop_map(|parts| parts.concat())
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(4096))]
        #[test]
        fn random_strings_match_stdlib(input in text(), source in text()) {
            assert_equivalent(&input, &source);
        }
    }
}
