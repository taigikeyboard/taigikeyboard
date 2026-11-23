//! Case-transform module — tone-letter casing, candidate raising, nasal
//! marker case adjustment.
//!
//! Rust `str::to_uppercase` / `to_lowercase` keep combining marks
//! (`a̍` → `A̍`, `o̍͘` → `O̍͘`), so every Latin tone letter cases through the
//! stdlib and the result is the same in every input mode. The one
//! non-stdlib pair is the POJ nasal marker `ⁿ` ↔ `ᴺ`, which Unicode gives
//! no case mapping.

use std::borrow::Cow;

/// Three-state shift / case indicator, the wire `LetterCase` (`case.proto`).
/// Android adapts its `(caps, capsLock)` pair in `CaseTransformBridge`
/// (capsLock → CapsLocked; caps → Uppercased; else Lowercased).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LetterCase {
    Lowercased,
    Uppercased,
    CapsLocked,
}

pub(crate) const NASAL_LOWER: char = '\u{207F}'; // ⁿ
pub(crate) const NASAL_UPPER: char = '\u{1D3A}'; // ᴺ

/// Uppercase the first char of `input` and keep the rest as typed
/// (`tsh` → `Tsh`, `a̍` → `A̍`); a lone `ⁿ` becomes `ᴺ`.
pub fn uppercase_tone_char(input: &str) -> String {
    if is_lone(input, NASAL_LOWER) {
        return NASAL_UPPER.to_string();
    }
    crate::api::capitalize_first(input)
}

/// Uppercase every char of `input` (Caps Lock, USER 2026-09-25: "shouldn't
/// caps-lock make it all upper case?" → `óo` → `ÓO`); a lone `ⁿ` becomes `ᴺ`.
pub fn full_uppercase_tone_string(input: &str) -> String {
    if is_lone(input, NASAL_LOWER) {
        return NASAL_UPPER.to_string();
    }
    input.to_uppercase()
}

/// Lowercase every char of `input`; a lone `ᴺ` becomes `ⁿ`.
pub fn lowercase_tone_char(input: &str) -> String {
    if is_lone(input, NASAL_UPPER) {
        return NASAL_LOWER.to_string();
    }
    input.to_lowercase()
}

/// `input` is exactly the one char `marker`.
fn is_lone(input: &str, marker: char) -> bool {
    let mut chars = input.chars();
    chars.next() == Some(marker) && chars.next().is_none()
}

/// Apply `letter_case` to a keystroke `text`:
/// - `Lowercased`: lowercase via `lowercase_tone_char`
/// - `Uppercased`: first letter upper (via `uppercase_tone_char`), rest lower
/// - `CapsLocked`: full upper via `full_uppercase_tone_string`
pub fn transform_input_case(text: &str, letter_case: LetterCase) -> String {
    match letter_case {
        LetterCase::CapsLocked => full_uppercase_tone_string(text),
        LetterCase::Uppercased => capitalize_first_letter(text),
        LetterCase::Lowercased => lowercase_tone_char(text),
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
/// `ⁿ` stays as-is (stdlib has no case mapping for it) — the candidate path
/// (`composing::continuous`) re-cases it afterwards via [`apply_nasal_marker_case`].
pub fn raise_case(text: &str, letter_case: LetterCase) -> String {
    match letter_case {
        LetterCase::CapsLocked => full_uppercase_tone_string(text),
        LetterCase::Uppercased => uppercase_first_letter_in_text(text),
        LetterCase::Lowercased => text.to_string(),
    }
}

/// Adjust nasal marker (`ⁿ`/`ᴺ`) case to match the preceding letter.
///
/// - Input without either marker is returned unchanged.
/// - Each marker is rewritten in place to follow the case of the most recent
///   letter scanned. Non-letter characters between the marker and the prior
///   letter (digits, punctuation) do not reset the carried case.
///
/// Called in-band by `apply_nasal_marker_case` (the last step of
/// `api::normalize_tone`), and through it by the candidate casing in
/// `composing::continuous`.
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

/// ⁿ becomes ᴺ in capitals (`behavioral-invariants.md` §53): the marker follows the preceding
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

/// Uppercase the literal first character, lowercase the rest (per char, so a
/// `ᴺ` in the tail becomes `ⁿ`). Used by the `transform_input_case`
/// `Uppercased` path.
fn capitalize_first_letter(text: &str) -> String {
    let mut chars = text.chars();
    let mut result = String::with_capacity(text.len());
    if let Some(first) = chars.next() {
        push_upper(first, &mut result);
    }
    for ch in chars {
        push_lower(ch, &mut result);
    }
    result
}

/// Find the first LETTER in `text` and uppercase it; every other char passes
/// through as stored. DISTINCT from `capitalize_first_letter` above — this
/// skips leading non-letters (e.g. "-gí" → "-Gí" not "-gí") and never lowers
/// the rest.
fn uppercase_first_letter_in_text(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut is_first_letter = true;
    for ch in text.chars() {
        if ch.is_alphabetic() && is_first_letter {
            push_upper(ch, &mut result);
            is_first_letter = false;
        } else {
            result.push(ch);
        }
    }
    result
}

/// Per-letter case matching: the Nth letter of `target` is raised when
/// the Nth letter of `source` is uppercase, otherwise kept as stored
/// (raise-only, see [`raise_case`]). Non-letters pass through unchanged,
/// so a tone placed as a combining mark never shifts the alignment.
pub fn match_case(target: &str, source: &str) -> String {
    let mut source_letters = source.chars().filter(|c| c.is_alphabetic());
    let mut result = String::with_capacity(target.len());
    for ch in target.chars() {
        if !ch.is_alphabetic() {
            result.push(ch);
            continue;
        }
        match source_letters.next() {
            Some(src) if src.is_uppercase() => push_upper(ch, &mut result),
            _ => result.push(ch),
        }
    }
    result
}

/// One char through [`uppercase_tone_char`], appended to `out`.
fn push_upper(ch: char, out: &mut String) {
    if ch == NASAL_LOWER {
        out.push(NASAL_UPPER);
    } else {
        out.extend(ch.to_uppercase());
    }
}

/// One char through [`lowercase_tone_char`], appended to `out`.
fn push_lower(ch: char, out: &mut String) {
    if ch == NASAL_UPPER {
        out.push(NASAL_LOWER);
    } else {
        out.extend(ch.to_lowercase());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn nasal_passthrough_when_no_marker() {
        assert_eq!(adjust_nasal_marker_case(""), "");
        assert_eq!(adjust_nasal_marker_case("abc"), "abc");
        assert_eq!(adjust_nasal_marker_case("ABC"), "ABC");
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

    #[test]
    fn uppercase_tone_char_keeps_combining_marks() {
        assert_eq!(uppercase_tone_char("a̍"), "A̍");
        assert_eq!(uppercase_tone_char("o̍͘"), "O̍͘");
    }

    #[test]
    fn caps_lock_raises_every_letter_of_tl_oo() {
        // trace: stdlib to_uppercase("óo") → "ÓO"
        assert_eq!(full_uppercase_tone_string("óo"), "ÓO");
        assert_eq!(full_uppercase_tone_string("o̍o"), "O̍O");
        assert_eq!(transform_input_case("óo", LetterCase::CapsLocked), "ÓO");
        assert_eq!(raise_case("óo", LetterCase::CapsLocked), "ÓO");
    }

    #[test]
    fn shift_raises_only_the_first_letter_of_tl_oo() {
        assert_eq!(uppercase_tone_char("óo"), "Óo");
        assert_eq!(transform_input_case("óo", LetterCase::Uppercased), "Óo");
    }

    #[test]
    fn caps_lock_tl_oo_lowercases_back() {
        // trace: stdlib to_lowercase("ÓO") → "óo"
        assert_eq!(lowercase_tone_char("ÓO"), "óo");
    }

    #[test]
    fn uppercase_tone_char_first_only_for_multi_char() {
        assert_eq!(uppercase_tone_char("tsh"), "Tsh");
    }

    #[test]
    fn nasal_marker_has_its_own_case_pair() {
        assert_eq!(uppercase_tone_char("\u{207F}"), "\u{1D3A}");
        assert_eq!(lowercase_tone_char("\u{1D3A}"), "\u{207F}");
    }

    #[test]
    fn transform_input_caps_lock_uppercases_all() {
        assert_eq!(transform_input_case("tsh", LetterCase::CapsLocked), "TSH");
    }

    #[test]
    fn transform_input_uppercased_capitalizes_first() {
        assert_eq!(transform_input_case("tsh", LetterCase::Uppercased), "Tsh");
    }

    #[test]
    fn transform_input_lowercased_lowercases() {
        assert_eq!(transform_input_case("Tsh", LetterCase::Lowercased), "tsh");
    }

    // Stored capitals survive (user report 2026-09-19: custom entry
    // `Keng-lâm Su-īⁿ` under abbreviation `klsi`)

    const CUSTOM: &str = "Keng-lâm Su-īⁿ";

    #[test]
    fn raise_case_lowercased_keeps_stored_capitals() {
        assert_eq!(raise_case(CUSTOM, LetterCase::Lowercased), CUSTOM);
        assert_eq!(raise_case("tâi-gí", LetterCase::Lowercased), "tâi-gí");
    }

    #[test]
    fn raise_case_uppercased_raises_first_letter_only() {
        assert_eq!(raise_case(CUSTOM, LetterCase::Uppercased), CUSTOM);
        assert_eq!(raise_case("tâi-gí", LetterCase::Uppercased), "Tâi-gí");
        // First LETTER, not first char.
        assert_eq!(raise_case("-gí", LetterCase::Uppercased), "-Gí");
    }

    #[test]
    fn raise_case_caps_locked_uppercases_all() {
        assert_eq!(raise_case(CUSTOM, LetterCase::CapsLocked), "KENG-LÂM SU-Īⁿ");
    }
}
