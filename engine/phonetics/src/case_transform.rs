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
//! `case_adjust.rs` (formerly `pub(crate)`). The function keeps its existing
//! contract; `case_adjust.rs` is removed in this slice.

use std::borrow::Cow;

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

    // Mode-specific tone-letter table lookup. Table values are first-letter
    // upper (TL `óo` → `Óo`); Caps Lock raises the rest too (`ÓO`, USER
    // 2026-09-25 「caps-lock 不是應該都大寫嗎」, behavioral-invariants.md
    // "CapsLock → all upper").
    if let Some(map) = lower_to_upper(mode) {
        if let Some(mapped) = map.get(input) {
            return if all_chars {
                mapped.to_uppercase()
            } else {
                (*mapped).to_string()
            };
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

/// Case a fetched candidate roman the way the raw keystrokes ask,
/// **only ever raising** letters: `CapsLocked` → full upper, `Uppercased`
/// → first letter upper, `Lowercased` → untouched.
///
/// Candidate-side counterpart of [`transform_input_case`], which acts on
/// the keystroke itself and does lowercase. `dict.bin` romans are
/// canonical lowercase, so on those the two agree; a custom dictionary
/// entry carries the user's own capitals (`Keng-lâm Su-īⁿ` for `klsi`),
/// and lowering it to the keystroke's case threw those away (user report
/// 2026-09-19). Every candidate-casing path ends here; only keystrokes
/// go through [`transform_input_case`]. Under `CapsLocked` the POJ nasal
/// `ⁿ` stays as-is (no uppercase hook in POJ) — [`transform_suggestion`]
/// re-cases it afterwards via [`adjust_nasal_marker_case`].
pub fn raise_case(text: &str, letter_case: LetterCase, mode: InputMode) -> String {
    match letter_case {
        LetterCase::CapsLocked => full_uppercase_tone_string(text, mode),
        LetterCase::Uppercased => uppercase_first_letter_in_text(text, mode),
        LetterCase::Lowercased => text.to_string(),
    }
}

/// Apply the SuggestionCaseTransformer per-word case transformation:
/// - `CapsLocked`: full upper
/// - else with non-empty `composing_text`: split typed-portion (matchCase
///   to composing) + remaining-portion (`Uppercased` → first upper /
///   `Lowercased` → untouched)
/// - empty composing: original returned as-is
///
/// Raise-only, see [`raise_case`].
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
        return raise_case(original_text, letter_case, mode);
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
    let transformed_remaining = raise_case(&remaining_portion, letter_case, mode);
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
/// Called in-band by `apply_nasal_marker_case` (the last step of
/// `api::normalize_tone`) AND by `transform_suggestion` (post-process).
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

/// ⁿ大本字 (`behavioral-invariants.md` §53): the marker follows the preceding
/// letter's case ([`adjust_nasal_marker_case`]), or with `force_lowercase`
/// (`AppConfig.force_lowercase_nasal_marker`, the switch OFF) it is always
/// `ⁿ` ([`lowercase_nasal_markers`]). Borrowed when nothing would change.
pub fn apply_nasal_marker_case(text: &str, force_lowercase: bool) -> Cow<'_, str> {
    let rewrites = if force_lowercase {
        text.contains(NASAL_UPPER)
    } else {
        text.contains(NASAL_LOWER) || text.contains(NASAL_UPPER)
    };
    if !rewrites {
        return Cow::Borrowed(text);
    }
    Cow::Owned(if force_lowercase {
        lowercase_nasal_markers(text)
    } else {
        adjust_nasal_marker_case(text)
    })
}

/// Every `ᴺ` U+1D3A written as `ⁿ` U+207F; the letters keep their case.
pub fn lowercase_nasal_markers(text: &str) -> String {
    text.replace(NASAL_UPPER, "\u{207F}")
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
/// `uppercase_tone_char`); every other char passes through as stored.
/// DISTINCT from `capitalize_first_letter` above — this skips leading
/// non-letters (e.g. "-gí" → "-Gí" not "-gí") and never lowers the rest.
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

/// Per-letter case matching: the Nth letter of `target` is raised when
/// the Nth letter of `source` is uppercase, otherwise kept as stored
/// (raise-only, see [`raise_case`]). Non-letters pass through unchanged,
/// so a tone placed as a combining mark never shifts the alignment.
pub fn match_case(target: &str, source: &str, mode: InputMode) -> String {
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

fn to_lowercase_per_char(text: &str, mode: InputMode) -> String {
    text.chars()
        .map(|ch| lowercase_tone_char(&ch.to_string(), mode))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------
    #[test]
    fn apply_nasal_marker_case_follows_the_switch_and_borrows_when_unchanged() {
        assert_eq!(
            apply_nasal_marker_case("SI\u{c2}\u{1d3a}", true),
            "SI\u{c2}\u{207f}"
        );
        assert_eq!(
            apply_nasal_marker_case("SI\u{c2}\u{1d3a}-Si\u{e2}\u{207f}", true),
            "SI\u{c2}\u{207f}-Si\u{e2}\u{207f}"
        );
        assert_eq!(
            apply_nasal_marker_case("SI\u{c2}\u{207f}", false),
            "SI\u{c2}\u{1d3a}"
        );
        // Nothing to rewrite: no allocation either way.
        assert!(matches!(
            apply_nasal_marker_case("T\u{c2}I-G\u{cd}", false),
            Cow::Borrowed(_)
        ));
        assert!(matches!(
            apply_nasal_marker_case("si\u{e2}\u{207f}", true),
            Cow::Borrowed(_)
        ));
    }

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
    fn caps_lock_raises_every_letter_of_a_tl_oo_table_entry() {
        // trace: table "óo"→"Óo" (first-letter upper), all_chars → to_uppercase → "ÓO"
        assert_eq!(full_uppercase_tone_string("óo", InputMode::Tl), "ÓO");
        assert_eq!(full_uppercase_tone_string("o̍o", InputMode::Tl), "O̍O");
        assert_eq!(
            transform_input_case("óo", LetterCase::CapsLocked, InputMode::Tl),
            "ÓO"
        );
        assert_eq!(
            raise_case("óo", LetterCase::CapsLocked, InputMode::Tl),
            "ÓO"
        );
    }

    #[test]
    fn shift_keeps_a_tl_oo_table_entry_first_letter_only() {
        assert_eq!(uppercase_tone_char("óo", InputMode::Tl), "Óo");
        assert_eq!(
            transform_input_case("óo", LetterCase::Uppercased, InputMode::Tl),
            "Óo"
        );
    }

    #[test]
    fn caps_lock_tl_oo_lowercases_back() {
        // trace: "ÓO" misses the reverse table ("Óo" only) → stdlib lowercase
        assert_eq!(lowercase_tone_char("ÓO", InputMode::Tl), "óo");
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

    // -----------------------------------------------------------------
    // Stored capitals survive (user report 2026-09-19: custom entry
    // `Keng-lâm Su-īⁿ` under abbreviation `klsi`)
    // -----------------------------------------------------------------

    const CUSTOM: &str = "Keng-lâm Su-īⁿ";

    #[test]
    fn raise_case_lowercased_keeps_stored_capitals() {
        assert_eq!(
            raise_case(CUSTOM, LetterCase::Lowercased, InputMode::Poj),
            CUSTOM
        );
        assert_eq!(
            raise_case("tâi-gí", LetterCase::Lowercased, InputMode::Poj),
            "tâi-gí"
        );
    }

    #[test]
    fn raise_case_uppercased_raises_first_letter_only() {
        assert_eq!(
            raise_case(CUSTOM, LetterCase::Uppercased, InputMode::Poj),
            CUSTOM
        );
        assert_eq!(
            raise_case("tâi-gí", LetterCase::Uppercased, InputMode::Poj),
            "Tâi-gí"
        );
        // First LETTER, not first char.
        assert_eq!(
            raise_case("-gí", LetterCase::Uppercased, InputMode::Poj),
            "-Gí"
        );
    }

    #[test]
    fn raise_case_caps_locked_uppercases_all() {
        assert_eq!(
            raise_case(CUSTOM, LetterCase::CapsLocked, InputMode::Poj),
            "KENG-LÂM SU-Īⁿ"
        );
    }

    #[test]
    fn transform_suggestion_abbreviation_keeps_stored_capitals() {
        // `klsi` (4 letters) aligns positionally with `Keng`; lowercase
        // keystrokes must not lower `K` nor the untyped `Su`.
        assert_eq!(
            transform_suggestion(CUSTOM, "klsi", LetterCase::Lowercased, InputMode::Poj),
            CUSTOM
        );
        // Shift on the first key, keyboard already back to lowercase.
        assert_eq!(
            transform_suggestion(CUSTOM, "Klsi", LetterCase::Lowercased, InputMode::Poj),
            CUSTOM
        );
        // Shift still held: the remainder's first letter is raised, the
        // stored `Su` stays.
        assert_eq!(
            transform_suggestion(CUSTOM, "Klsi", LetterCase::Uppercased, InputMode::Poj),
            "Keng-Lâm Su-īⁿ"
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
