//! Case-transform module — POJ/TL tone-letter case mapping, candidate
//! capitalization, per-suggestion case transformation, nasal marker case
//! adjustment.
//!
//! Cross-platform canonical for the case-transformation subsystem. Replaces
//! iOS `Input/CaseTransformer.swift` + `Input/ToneUtilities.swift` + the
//! body of `Autocomplete/Services/SuggestionCaseTransformer.swift`, plus
//! Android counterparts `dictionary/ToneUtilities.kt` + body of
//! `dictionary/SuggestionCaseTransformer.kt`.
//!
//! Tone-letter case tables live in `case_tables` (POJ + TL); this module
//! never re-implements them.
//!
//! `adjust_nasal_marker_case` is re-homed here from the former
//! `case_adjust.rs` (formerly `pub(crate)` and called by
//! `Method::NormalizeTone` in-band). The function keeps its existing
//! contract; `case_adjust.rs` is removed in this slice.

use crate::api::InputMode;
use crate::case_tables::{lower_to_upper, upper_to_lower};

/// Three-state shift / case indicator. Mirrors iOS `LetterCase` enum and
/// adapts Android's `(caps: Bool, capsLock: Bool)` pair at the bridge call
/// site (CapsLock=true → CapsLocked; caps=true → Uppercased; else Lowercased).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LetterCase {
    Lowercased,
    Uppercased,
    CapsLocked,
}

pub(crate) const NASAL_LOWER: char = '\u{207F}'; // ⁿ
pub(crate) const NASAL_UPPER: char = '\u{1D3A}'; // ᴺ

// =========================================================================
// Per-char (single grapheme cluster) helpers
// =========================================================================

/// Uppercase a single char/grapheme using mode-aware tone tables. For
/// multi-character inputs (e.g. "tsh") only the first letter is uppercased.
/// Matches Android `ToneUtilities.uppercaseToneLetter` (`replaceFirstChar`)
/// semantics.
pub fn uppercase_tone_char(input: &str, mode: InputMode) -> String {
    uppercase_internal(input, mode, /* all_chars */ false)
}

/// Uppercase ALL characters in `input` using mode-aware tone tables.
/// Matches Android `ToneUtilities.fullUppercaseToneLetter` semantics.
/// Used by Caps Lock paths.
pub fn full_uppercase_tone_string(input: &str, mode: InputMode) -> String {
    uppercase_internal(input, mode, /* all_chars */ true)
}

fn uppercase_internal(input: &str, mode: InputMode, all_chars: bool) -> String {
    // Nasal marker shortcut (mode-independent).
    if input == "\u{207F}" {
        return "\u{1D3A}".to_string();
    }

    // Mode-specific tone-letter table lookup.
    if let Some(map) = lower_to_upper(mode) {
        if let Some(mapped) = map.get(input) {
            return (*mapped).to_string();
        }
    }

    // Stdlib fallback.
    if all_chars {
        input.to_uppercase()
    } else {
        // Char count via grapheme-naive iteration is acceptable here because
        // the table lookup above already handled all multi-codepoint
        // sequences with combining marks; surviving inputs are ASCII or
        // simple precomposed letters.
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

/// Lowercase a single char/grapheme using mode-aware tone tables. Matches
/// Android `ToneUtilities.lowercaseToneLetter`.
pub fn lowercase_tone_char(input: &str, mode: InputMode) -> String {
    // Nasal marker shortcut (mode-independent).
    if input == "\u{1D3A}" {
        return "\u{207F}".to_string();
    }

    // Mode-specific tone-letter table lookup.
    if let Some(map) = upper_to_lower(mode) {
        if let Some(mapped) = map.get(input) {
            return (*mapped).to_string();
        }
    }

    input.to_lowercase()
}

// =========================================================================
// Per-string compound transforms
// =========================================================================

/// Apply `letter_case` to `text` per `CaseTransformer.transformForInput`
/// semantics:
/// - `Lowercased`: lowercase via `lowercase_tone_char`
/// - `Uppercased`: first letter upper (via `uppercase_tone_char`), rest lower
/// - `CapsLocked`: full upper via `full_uppercase_tone_string`
pub fn transform_input_case(text: &str, letter_case: LetterCase, mode: InputMode) -> String {
    match letter_case {
        LetterCase::CapsLocked => full_uppercase_tone_string(text, mode),
        LetterCase::Uppercased => capitalize_first_letter(text, mode),
        LetterCase::Lowercased => lowercase_tone_char(text, mode),
    }
}

/// Gate on `auto_cap_enabled` + `input` first char's case. If both true and
/// `text` starts with a letter, uppercase the first letter via tone tables;
/// otherwise return `text` as-is. Matches `CaseTransformer.capitalizeCandidate`.
pub fn capitalize_candidate(
    text: &str,
    input: &str,
    auto_cap_enabled: bool,
    mode: InputMode,
) -> String {
    if !auto_cap_enabled {
        return text.to_string();
    }

    let Some(first_input) = input.chars().next() else {
        return text.to_string();
    };
    if !first_input.is_uppercase() {
        return text.to_string();
    }

    let Some(first_text) = text.chars().next() else {
        return text.to_string();
    };
    if !first_text.is_alphabetic() {
        return text.to_string();
    }

    let capitalized = uppercase_tone_char(&first_text.to_string(), mode);
    let rest: String = text.chars().skip(1).collect();
    capitalized + &rest
}

/// Apply the SuggestionCaseTransformer per-word case transformation:
/// - `CapsLocked`: full upper
/// - else with non-empty `composing_text`: split typed-portion (matchCase
///   to composing) + remaining-portion (`Uppercased` → first upper /
///   `Lowercased` → lower)
/// - empty composing: original returned as-is
///
/// Output is post-processed via `adjust_nasal_marker_case` so the engine
/// returns the final-form string ready for display. Suggestion skip rules
/// (iOS `additionalInfo` flags, Android `id` markers) stay platform-side
/// — only transform-eligible items reach this op.
pub fn transform_suggestion(
    original_text: &str,
    composing_text: &str,
    letter_case: LetterCase,
    mode: InputMode,
) -> String {
    let inner = transform_suggestion_inner(original_text, composing_text, letter_case, mode);
    adjust_nasal_marker_case(&inner)
}

fn transform_suggestion_inner(
    original_text: &str,
    composing_text: &str,
    letter_case: LetterCase,
    mode: InputMode,
) -> String {
    if matches!(letter_case, LetterCase::CapsLocked) {
        return to_uppercase_per_char(original_text, mode);
    }

    let typed_letter_count = count_letters(composing_text);
    if typed_letter_count == 0 {
        return original_text.to_string();
    }

    let original_letter_count = count_letters(original_text);
    if typed_letter_count >= original_letter_count {
        return match_case(original_text, composing_text, mode);
    }

    let (typed_portion, remaining_portion) =
        split_by_letter_count(original_text, typed_letter_count);
    let preserved_typed = match_case(&typed_portion, composing_text, mode);
    let transformed_remaining = match letter_case {
        // SuggestionCaseTransformer.capitalizeFirstLetter — finds first
        // LETTER (not first char), uppercases it, lowercases subsequent
        // letters; non-letters pass through. Distinct from
        // `capitalize_first_letter` below which uppercases the literal
        // first char (used by transform_input_case .uppercased path).
        LetterCase::Uppercased => capitalize_first_letter_in_text(&remaining_portion, mode),
        _ => to_lowercase_per_char(&remaining_portion, mode),
    };
    preserved_typed + &transformed_remaining
}

// =========================================================================
// Nasal marker
// =========================================================================

/// Adjust nasal marker (`ⁿ`/`ᴺ`) case to match the preceding letter.
///
/// - Input without either marker is returned unchanged.
/// - Each marker is rewritten in place to follow the case of the most recent
///   letter scanned. Non-letter characters between the marker and the prior
///   letter (digits, punctuation) do not reset the carried case.
///
/// Re-homed from the former `case_adjust.rs::adjust_nasal_marker_case`.
/// Called in-band by `Method::NormalizeTone` (engine remains source-of-truth)
/// AND by `transform_suggestion` (post-process).
pub fn adjust_nasal_marker_case(text: &str) -> String {
    if !text.contains(NASAL_LOWER) && !text.contains(NASAL_UPPER) {
        return text.to_string();
    }

    let mut result = String::with_capacity(text.len());
    let mut last_letter_uppercase = false;

    for ch in text.chars() {
        if ch == NASAL_LOWER || ch == NASAL_UPPER {
            result.push(if last_letter_uppercase {
                NASAL_UPPER
            } else {
                NASAL_LOWER
            });
        } else {
            if ch.is_alphabetic() {
                last_letter_uppercase = ch.is_uppercase();
            }
            result.push(ch);
        }
    }
    result
}

// =========================================================================
// Internal helpers (mirror SuggestionCaseTransformer private funcs)
// =========================================================================

/// Uppercase the literal first character (via `uppercase_tone_char`),
/// lowercase the rest (per-char). Matches
/// `CaseTransformer.capitalizeFirstLetter` (engine-side input pipeline).
/// Used by `transform_input_case` `.uppercased` path.
fn capitalize_first_letter(text: &str, mode: InputMode) -> String {
    let mut chars = text.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let first_upper = uppercase_tone_char(&first.to_string(), mode);

    // The "rest lowercase" applies per-char so combining-mark sequences in
    // the tail also pass through the table.
    let rest: String = chars.collect();
    let rest_lower = to_lowercase_per_char(&rest, mode);
    first_upper + &rest_lower
}

/// Find the first LETTER in `text` and uppercase it (via
/// `uppercase_tone_char`); lowercase subsequent letters; non-letters pass
/// through. Matches `SuggestionCaseTransformer.capitalizeFirstLetter`
/// (per-suggestion remainder transform). DISTINCT from
/// `capitalize_first_letter` above — this skips leading non-letters
/// (e.g. "-gí" → "-Gí" not "-gí").
fn capitalize_first_letter_in_text(text: &str, mode: InputMode) -> String {
    let mut result = String::with_capacity(text.len());
    let mut is_first_letter = true;
    for ch in text.chars() {
        if ch.is_alphabetic() {
            let ch_str = ch.to_string();
            if is_first_letter {
                result.push_str(&uppercase_tone_char(&ch_str, mode));
                is_first_letter = false;
            } else {
                result.push_str(&lowercase_tone_char(&ch_str, mode));
            }
        } else {
            result.push(ch);
        }
    }
    result
}

fn count_letters(text: &str) -> usize {
    text.chars().filter(|c| c.is_alphabetic()).count()
}

/// Split `text` into (first part containing exactly `letter_count` letters,
/// rest). Non-letter characters in the leading region are kept with the
/// first part; the split index lands immediately after the Nth letter.
fn split_by_letter_count(text: &str, letter_count: usize) -> (String, String) {
    let mut count = 0usize;
    let mut split_byte_idx = text.len();
    for (idx, ch) in text.char_indices() {
        if ch.is_alphabetic() {
            count += 1;
            if count == letter_count {
                split_byte_idx = idx + ch.len_utf8();
                break;
            }
        }
    }
    let (first, second) = text.split_at(split_byte_idx);
    (first.to_string(), second.to_string())
}

/// Per-letter case matching. For each letter in `target`, look at the
/// corresponding letter in `source` and apply that case. Non-letters in
/// `target` pass through unchanged. Mirrors
/// `SuggestionCaseTransformer.matchCase`.
fn match_case(target: &str, source: &str, mode: InputMode) -> String {
    let mut source_letters: Vec<char> = source.chars().filter(|c| c.is_alphabetic()).collect();
    source_letters.reverse(); // pop_back == take next from front efficiently

    let mut result = String::with_capacity(target.len());
    for ch in target.chars() {
        if ch.is_alphabetic() {
            if let Some(src) = source_letters.pop() {
                let ch_str = ch.to_string();
                let mapped = if src.is_uppercase() {
                    uppercase_tone_char(&ch_str, mode)
                } else {
                    lowercase_tone_char(&ch_str, mode)
                };
                result.push_str(&mapped);
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }
    result
}

fn to_uppercase_per_char(text: &str, mode: InputMode) -> String {
    text.chars()
        .map(|ch| uppercase_tone_char(&ch.to_string(), mode))
        .collect()
}

fn to_lowercase_per_char(text: &str, mode: InputMode) -> String {
    text.chars()
        .map(|ch| lowercase_tone_char(&ch.to_string(), mode))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------
    // adjust_nasal_marker_case — moved from case_adjust.rs verbatim
    // -----------------------------------------------------------------

    #[test]
    fn nasal_passthrough_when_no_marker() {
        assert_eq!(adjust_nasal_marker_case(""), "");
        assert_eq!(adjust_nasal_marker_case("abc"), "abc");
        assert_eq!(adjust_nasal_marker_case("ABC"), "ABC");
    }

    #[test]
    fn nasal_lower_after_upper_letter_promotes() {
        assert_eq!(adjust_nasal_marker_case("AN\u{207F}"), "AN\u{1D3A}");
    }

    #[test]
    fn nasal_upper_after_lower_letter_demotes() {
        assert_eq!(adjust_nasal_marker_case("an\u{1D3A}"), "an\u{207F}");
    }

    #[test]
    fn nasal_digits_do_not_reset_case() {
        assert_eq!(adjust_nasal_marker_case("AN2\u{207F}"), "AN2\u{1D3A}");
    }

    #[test]
    fn nasal_mixed_case_per_marker() {
        assert_eq!(
            adjust_nasal_marker_case("An\u{207F}-AN\u{207F}"),
            "An\u{207F}-AN\u{1D3A}"
        );
    }

    #[test]
    fn nasal_no_letter_yet_defaults_lower() {
        assert_eq!(adjust_nasal_marker_case("\u{1D3A}"), "\u{207F}");
    }

    // -----------------------------------------------------------------
    // Per-char helpers
    // -----------------------------------------------------------------

    #[test]
    fn uppercase_tone_char_uses_tone_table_for_combining_mark() {
        assert_eq!(uppercase_tone_char("a̍", InputMode::Poj), "A̍");
        assert_eq!(uppercase_tone_char("o̍͘", InputMode::Poj), "O̍͘");
    }

    #[test]
    fn full_uppercase_tone_string_uppercases_all() {
        assert_eq!(full_uppercase_tone_string("tsh", InputMode::Tl), "TSH");
    }

    #[test]
    fn uppercase_tone_char_first_only_for_multi_char() {
        assert_eq!(uppercase_tone_char("tsh", InputMode::Tl), "Tsh");
    }

    #[test]
    fn lowercase_tone_char_uses_tone_table() {
        assert_eq!(lowercase_tone_char("Á", InputMode::Poj), "á");
    }

    #[test]
    fn nasal_marker_shortcut_is_mode_independent() {
        assert_eq!(uppercase_tone_char("\u{207F}", InputMode::Tl), "\u{1D3A}");
        assert_eq!(lowercase_tone_char("\u{1D3A}", InputMode::Poj), "\u{207F}");
    }

    // -----------------------------------------------------------------
    // transform_input_case
    // -----------------------------------------------------------------

    #[test]
    fn transform_input_caps_lock_uppercases_all() {
        assert_eq!(
            transform_input_case("tsh", LetterCase::CapsLocked, InputMode::Tl),
            "TSH"
        );
    }

    #[test]
    fn transform_input_uppercased_capitalizes_first() {
        assert_eq!(
            transform_input_case("tsh", LetterCase::Uppercased, InputMode::Tl),
            "Tsh"
        );
    }

    #[test]
    fn transform_input_lowercased_passthrough() {
        assert_eq!(
            transform_input_case("Tsh", LetterCase::Lowercased, InputMode::Tl),
            "tsh"
        );
    }

    // -----------------------------------------------------------------
    // capitalize_candidate
    // -----------------------------------------------------------------

    #[test]
    fn capitalize_candidate_disabled_passthrough() {
        assert_eq!(
            capitalize_candidate("góa", "Goa", false, InputMode::Poj),
            "góa"
        );
    }

    #[test]
    fn capitalize_candidate_lowercase_input_passthrough() {
        assert_eq!(
            capitalize_candidate("góa", "goa", true, InputMode::Poj),
            "góa"
        );
    }

    #[test]
    fn capitalize_candidate_uppercase_input_capitalizes() {
        assert_eq!(
            capitalize_candidate("góa", "Goa", true, InputMode::Poj),
            "Góa"
        );
    }

    #[test]
    fn capitalize_candidate_empty_inputs_passthrough() {
        assert_eq!(capitalize_candidate("", "G", true, InputMode::Poj), "");
        assert_eq!(capitalize_candidate("góa", "", true, InputMode::Poj), "góa");
    }

    // -----------------------------------------------------------------
    // transform_suggestion
    // -----------------------------------------------------------------

    #[test]
    fn transform_suggestion_caps_lock_uppercases_all() {
        assert_eq!(
            transform_suggestion("góa", "G", LetterCase::CapsLocked, InputMode::Poj),
            "GÓA"
        );
    }

    #[test]
    fn transform_suggestion_empty_composing_passthrough() {
        assert_eq!(
            transform_suggestion("góa", "", LetterCase::Lowercased, InputMode::Poj),
            "góa"
        );
    }

    #[test]
    fn transform_suggestion_match_case_when_typed_covers_original() {
        // composing has 3 letters "GOA" upper, original "góa" 3 letters → match
        assert_eq!(
            transform_suggestion("góa", "GOA", LetterCase::Lowercased, InputMode::Poj),
            "GÓA"
        );
    }

    #[test]
    fn transform_suggestion_split_typed_remaining() {
        // composing has 1 letter "G" upper, original "góa" 3 letters
        // → typed "g" → "G" (matchCase to "G"), remaining "óa" lowered
        assert_eq!(
            transform_suggestion("góa", "G", LetterCase::Lowercased, InputMode::Poj),
            "Góa"
        );
    }

    #[test]
    fn transform_suggestion_uppercased_remaining_first_upper() {
        // composing "G", uppercased mode → typed "g"→"G", remaining "óa"
        // → "Óa" (first remaining upper, rest lower)
        assert_eq!(
            transform_suggestion("góa", "G", LetterCase::Uppercased, InputMode::Poj),
            "GÓa"
        );
    }

    #[test]
    fn transform_suggestion_runs_nasal_adjust_post_process() {
        // composing "AN" forces both letters upper → "AN" + nasal marker
        // should auto-promote ⁿ → ᴺ via adjust_nasal_marker_case
        assert_eq!(
            transform_suggestion("an\u{207F}", "AN", LetterCase::Lowercased, InputMode::Poj),
            "AN\u{1D3A}"
        );
    }
}
