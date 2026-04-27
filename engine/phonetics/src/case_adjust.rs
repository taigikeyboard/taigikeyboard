//! POJ nasal-marker case agreement.
//!
//! The POJ nasal marker is `ⁿ` (U+207F) after a lowercase letter and
//! `ᴺ` (U+1D3A) after an uppercase letter. Run inline by
//! `Method::NormalizeTone` so its returned string is display-ready.

const NASAL_LOWER: char = '\u{207F}'; // ⁿ
const NASAL_UPPER: char = '\u{1D3A}'; // ᴺ

/// Adjust nasal marker (`ⁿ`/`ᴺ`) case to match the preceding letter.
///
/// - Input without either marker is returned unchanged.
/// - Each marker is rewritten in place to follow the case of the most recent
///   letter scanned. Non-letter characters between the marker and the prior
///   letter (digits, punctuation) do not reset the carried case.
pub fn adjust_nasal_marker_case(text: &str) -> String {
    if !text.contains(NASAL_LOWER) && !text.contains(NASAL_UPPER) {
        return text.to_string();
    }

    let mut result = String::with_capacity(text.len());
    let mut last_letter_uppercase = false;

    for ch in text.chars() {
        if ch == NASAL_LOWER || ch == NASAL_UPPER {
            result.push(if last_letter_uppercase { NASAL_UPPER } else { NASAL_LOWER });
        } else {
            if ch.is_alphabetic() {
                last_letter_uppercase = ch.is_uppercase();
            }
            result.push(ch);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passthrough_when_no_marker() {
        assert_eq!(adjust_nasal_marker_case(""), "");
        assert_eq!(adjust_nasal_marker_case("abc"), "abc");
        assert_eq!(adjust_nasal_marker_case("ABC"), "ABC");
    }

    #[test]
    fn lower_marker_after_upper_letter_promotes() {
        assert_eq!(adjust_nasal_marker_case("AN\u{207F}"), "AN\u{1D3A}");
    }

    #[test]
    fn upper_marker_after_lower_letter_demotes() {
        assert_eq!(adjust_nasal_marker_case("an\u{1D3A}"), "an\u{207F}");
    }

    #[test]
    fn digits_do_not_reset_case() {
        // "AN" is uppercase carrier; "2" between letter and marker keeps upper.
        assert_eq!(adjust_nasal_marker_case("AN2\u{207F}"), "AN2\u{1D3A}");
    }

    #[test]
    fn mixed_case_per_marker() {
        assert_eq!(
            adjust_nasal_marker_case("An\u{207F}-AN\u{207F}"),
            "An\u{207F}-AN\u{1D3A}"
        );
    }

    #[test]
    fn no_letter_yet_defaults_lower() {
        // No carrier letter scanned → defaults to lowercase form.
        assert_eq!(adjust_nasal_marker_case("\u{1D3A}"), "\u{207F}");
    }
}
