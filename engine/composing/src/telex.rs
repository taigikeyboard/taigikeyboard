//! Telex tone keys: the desktop "Telex" scheme edits the pending tail with
//! letters no TL / POJ syllable spells (`docs/roadmap.md` § Desktop Telex).
//!
//! The buffer stays numeric-tone (`tai5`): a tone letter writes the digit the
//! standard scheme would have typed, so the syllabifier, the literal-roman
//! candidate and auto-space see nothing new.

use phonetics::InputMode;

/// Every letter the Telex scheme claims, lower case. Platforms classify on
/// this set (uppercase folded) before the engine resolves the edit.
pub const TELEX_KEYS: &str = "vydwxqzf";

/// What one Telex key means, independent of the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TelexKey {
    Tone(char),
    /// `z` → the unaspirated affricate initial (`ts` / `ch`); `zh` then
    /// spells the aspirated one because `h` is an ordinary letter.
    Affricate {
        uppercase: bool,
    },
    Hyphen,
}

fn resolve(key: &str) -> Option<TelexKey> {
    let mut chars = key.chars();
    let letter = chars.next()?;
    if chars.next().is_some() {
        return None;
    }
    Some(match letter {
        'v' | 'V' => TelexKey::Tone('2'),
        'y' | 'Y' => TelexKey::Tone('3'),
        'd' | 'D' => TelexKey::Tone('5'),
        'w' | 'W' => TelexKey::Tone('7'),
        'x' | 'X' => TelexKey::Tone('8'),
        'q' | 'Q' => TelexKey::Tone('9'),
        'z' => TelexKey::Affricate { uppercase: false },
        'Z' => TelexKey::Affricate { uppercase: true },
        'f' | 'F' => TelexKey::Hyphen,
        _ => return None,
    })
}

/// Apply one Telex key to the pending tail `raw`. `None` means the key
/// changes nothing (unknown key, a tone on an empty or hyphen-ended tail, the
/// same tone twice, a hyphen on an empty tail) — the transition answers with
/// a no-op. While composing the platform swallows such a key; only from Idle
/// does a tone letter or `f` reach the host as text.
///
/// A tone on a tail that already ends in a different digit replaces it;
/// the same digit is left alone so a held key cannot flip-flop (Backspace
/// removes the digit). Nailed segments are never part of `raw`.
pub fn apply_telex_key(raw: &str, key: &str, mode: InputMode) -> Option<String> {
    match resolve(key)? {
        TelexKey::Tone(digit) => {
            let last = raw.chars().last()?;
            if last == '-' {
                return None;
            }
            if last.is_ascii_digit() {
                if last == digit {
                    return None;
                }
                let mut next = raw[..raw.len() - 1].to_string();
                next.push(digit);
                return Some(next);
            }
            Some(format!("{raw}{digit}"))
        }
        TelexKey::Affricate { uppercase } => {
            let initial = match (mode, uppercase) {
                (InputMode::Poj, false) => "ch",
                (InputMode::Poj, true) => "Ch",
                (_, false) => "ts",
                (_, true) => "Ts",
            };
            Some(format!("{raw}{initial}"))
        }
        TelexKey::Hyphen => {
            if raw.is_empty() {
                return None;
            }
            Some(format!("{raw}-"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tone_letters_append_their_digit() {
        // trace: v y d w x q → 2 3 5 7 8 9
        for (key, digit) in [
            ("v", '2'),
            ("y", '3'),
            ("d", '5'),
            ("w", '7'),
            ("x", '8'),
            ("q", '9'),
        ] {
            assert_eq!(
                apply_telex_key("te", key, InputMode::Tl),
                Some(format!("te{digit}")),
                "key {key}"
            );
        }
    }

    #[test]
    fn uppercase_tone_letter_carries_the_same_tone() {
        assert_eq!(
            apply_telex_key("Te", "V", InputMode::Tl),
            Some("Te2".into())
        );
    }

    #[test]
    fn different_tone_replaces_same_tone_is_noop() {
        assert_eq!(
            apply_telex_key("te2", "y", InputMode::Tl),
            Some("te3".into())
        );
        assert_eq!(apply_telex_key("te2", "v", InputMode::Tl), None);
    }

    #[test]
    fn tone_on_empty_or_hyphen_tail_is_noop() {
        assert_eq!(apply_telex_key("", "v", InputMode::Tl), None);
        assert_eq!(apply_telex_key("tai-", "v", InputMode::Tl), None);
    }

    #[test]
    fn tone_applies_to_the_last_unhyphenated_chunk() {
        assert_eq!(
            apply_telex_key("taigi", "v", InputMode::Tl),
            Some("taigi2".into())
        );
        assert_eq!(
            apply_telex_key("tai5-gi", "v", InputMode::Tl),
            Some("tai5-gi2".into())
        );
    }

    #[test]
    fn z_expands_by_mode_and_case() {
        assert_eq!(apply_telex_key("", "z", InputMode::Tl), Some("ts".into()));
        assert_eq!(apply_telex_key("", "Z", InputMode::Tl), Some("Ts".into()));
        assert_eq!(apply_telex_key("", "z", InputMode::Poj), Some("ch".into()));
        assert_eq!(apply_telex_key("", "Z", InputMode::Poj), Some("Ch".into()));
        assert_eq!(apply_telex_key("a", "z", InputMode::Tl), Some("ats".into()));
    }

    #[test]
    fn f_appends_hyphen_after_content_only() {
        assert_eq!(
            apply_telex_key("tai5", "f", InputMode::Tl),
            Some("tai5-".into())
        );
        assert_eq!(apply_telex_key("", "f", InputMode::Tl), None);
    }

    #[test]
    fn telex_keys_constant_matches_resolve() {
        for letter in TELEX_KEYS.chars() {
            assert!(resolve(&letter.to_string()).is_some(), "{letter}");
            assert!(
                resolve(&letter.to_uppercase().to_string()).is_some(),
                "{letter}"
            );
        }
        for letter in 'a'..='z' {
            assert_eq!(
                resolve(&letter.to_string()).is_some(),
                TELEX_KEYS.contains(letter),
                "{letter}"
            );
        }
    }

    #[test]
    fn unknown_or_multi_char_keys_are_ignored() {
        for key in ["a", "c", "1", "", "vv", "-"] {
            assert_eq!(
                apply_telex_key("te", key, InputMode::Tl),
                None,
                "key {key:?}"
            );
        }
    }
}
