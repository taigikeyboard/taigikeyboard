//! Telex tone keys: the desktop "Telex" scheme edits the pending tail with
//! letters no TL / POJ syllable spells (`docs/reports/desktop-3.6.x-design-notes.md` § Desktop Telex).
//!
//! The buffer stays numeric-tone (`tai5`): a tone letter writes the digit the
//! standard scheme would have typed, so the syllabifier, the literal-roman
//! candidate and auto-space see nothing new.
//!
//! Two keys carry a tone pair, split by the coda (USER 2026-09-19): a
//! syllable that ends in a stop `p t k h` can only carry tone 4 or 8, any
//! other only 1 2 3 5 7 9, so `x` writes 1 / 4 and `v` writes 2 / 8 without
//! ever having to choose. The free letters are `c d f q v w x y z`, and the
//! pairing is what gives the two unmarked tones a key at all.
//!
//! `c` spells the affricate initial and `z` writes tone 6 (USER 2026-11-05),
//! the same in TL and POJ. That costs POJ its spelled-out `ch`: `c` `h` is
//! `chh`, so `cha` reaches `chha`.

use phonetics::InputMode;

/// Every letter the Telex scheme claims, lower case. Platforms classify on
/// this set (uppercase folded) before the engine resolves the edit.
pub const TELEX_KEYS: &str = "vydwxqzfch";

/// What one Telex key means, independent of the buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TelexKey {
    /// The digit written on an open tail and the one written on a tail that
    /// ends in a stop coda `p t k h`. Equal for the keys that carry one tone.
    Tone {
        open: char,
        checked: char,
    },
    /// `c` → the unaspirated affricate initial (`ts` / `ch`); `ch` then
    /// spells the aspirated one because `h` writes itself.
    Affricate {
        uppercase: bool,
    },
    /// `h`, an ordinary letter except after an aspirated affricate, where it
    /// is a slip: POJ's spelled-out `c` `h` `h` has already written `chh` by
    /// the first `h`. Only Telex sends `h` here, so Standard keeps its text.
    Aspiration {
        letter: char,
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
        'x' | 'X' => tone('1', '4'),
        'v' | 'V' => tone('2', '8'),
        'y' | 'Y' => tone('3', '3'),
        'd' | 'D' => tone('5', '5'),
        'w' | 'W' => tone('7', '7'),
        'z' | 'Z' => tone('6', '6'),
        'q' | 'Q' => tone('9', '9'),
        'c' => TelexKey::Affricate { uppercase: false },
        'C' => TelexKey::Affricate { uppercase: true },
        'h' | 'H' => TelexKey::Aspiration { letter },
        'f' | 'F' => TelexKey::Hyphen,
        _ => return None,
    })
}

/// Whether `key` is the hyphen key `f` / `F`. With no tail to append to, the
/// transition types it from Idle as the `-` key: the neutral-tone marker, a
/// document literal (§21).
pub fn is_hyphen_key(key: &str) -> bool {
    resolve(key) == Some(TelexKey::Hyphen)
}

const fn tone(open: char, checked: char) -> TelexKey {
    TelexKey::Tone { open, checked }
}

/// Whether the syllable `letters` ends (no tone digit) closes on a stop coda
/// `p t k h`, the only syllables that carry tone 4 or 8. An initial-only
/// tail (`kh`, `tsh`) reads as checked too: no tone makes it a syllable, so
/// the digit chosen does not matter.
fn ends_in_stop_coda(letters: &str) -> bool {
    letters.chars().last().is_some_and(phonetics::is_stop_coda)
}

/// Whether `letters` already ends in `tsh` / `chh`, which no further `h` can
/// extend: the initial must be followed by a vowel.
fn ends_in_aspirated_affricate(letters: &str) -> bool {
    let lowered = letters.to_ascii_lowercase();
    lowered.ends_with("tsh") || lowered.ends_with("chh")
}

/// Apply one Telex key to the pending tail `raw`. `None` means the key
/// changes nothing (unknown key, a tone on an empty or hyphen-ended tail, the
/// same tone twice, a hyphen on an empty tail, an `h` after `tsh` / `chh`) —
/// the transition answers with a no-op, except for an idle `f`, which it
/// types as the `-` key ([`is_hyphen_key`]). While composing the platform
/// swallows such a key; only from Idle does a tone letter reach the host as
/// text.
///
/// A tone on a tail that already ends in a different digit replaces it;
/// the same digit is left alone so a held key cannot flip-flop (Backspace
/// removes the digit). A paired key reads the coda of the letters under the
/// digit, so `sit8` + `x` → `sit4`. Nailed segments are never part of `raw`.
pub fn apply_telex_key(raw: &str, key: &str, mode: InputMode) -> Option<String> {
    match resolve(key)? {
        TelexKey::Tone { open, checked } => {
            let last = raw.chars().last()?;
            if last == '-' {
                return None;
            }
            let letters = raw
                .strip_suffix(|c: char| c.is_ascii_digit())
                .unwrap_or(raw);
            let digit = if ends_in_stop_coda(letters) {
                checked
            } else {
                open
            };
            if last == digit {
                return None;
            }
            Some(format!("{letters}{digit}"))
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
        TelexKey::Aspiration { letter } => {
            if ends_in_aspirated_affricate(raw) {
                return None;
            }
            Some(format!("{raw}{letter}"))
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
    // INVARIANT_TELEX_KEY_TABLE (behavioral-invariants.md §62)
    use super::*;

    #[test]
    fn tone_letters_append_their_digit() {
        // trace: open tail "te" — x v y d z w q → 1 2 3 5 6 7 9
        for (key, digit) in [
            ("x", '1'),
            ("v", '2'),
            ("y", '3'),
            ("d", '5'),
            ("z", '6'),
            ("w", '7'),
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
    fn paired_keys_write_the_checked_tone_on_a_stop_coda() {
        // trace: x = 1 / 4, v = 2 / 8; the coda is the last letter, any of
        // p t k h in either case; the single-tone keys ignore the coda.
        for tail in ["sit", "tsap", "bak", "ah", "annh", "SIT"] {
            assert_eq!(
                apply_telex_key(tail, "x", InputMode::Tl),
                Some(format!("{tail}4")),
                "{tail}"
            );
            assert_eq!(
                apply_telex_key(tail, "v", InputMode::Tl),
                Some(format!("{tail}8")),
                "{tail}"
            );
            assert_eq!(
                apply_telex_key(tail, "y", InputMode::Tl),
                Some(format!("{tail}3")),
                "{tail}"
            );
        }
        // Open codas — vowel, nasal `nn`, `ng`, `m` — take the open tone.
        for tail in ["tai", "tinn", "kang", "m", "oo"] {
            assert_eq!(
                apply_telex_key(tail, "x", InputMode::Tl),
                Some(format!("{tail}1")),
                "{tail}"
            );
            assert_eq!(
                apply_telex_key(tail, "v", InputMode::Tl),
                Some(format!("{tail}2")),
                "{tail}"
            );
        }
    }

    #[test]
    fn paired_key_reads_the_coda_under_an_existing_digit() {
        // trace: strip the digit, "sit" ends in t → checked: 8 → 4, 4 → 4
        // (same digit, no-op); "tai5" → open: 5 → 1.
        assert_eq!(
            apply_telex_key("sit8", "x", InputMode::Tl),
            Some("sit4".into())
        );
        assert_eq!(apply_telex_key("sit4", "x", InputMode::Tl), None);
        assert_eq!(
            apply_telex_key("sit4", "v", InputMode::Tl),
            Some("sit8".into())
        );
        assert_eq!(apply_telex_key("sit8", "v", InputMode::Tl), None);
        assert_eq!(
            apply_telex_key("tai5", "x", InputMode::Tl),
            Some("tai1".into())
        );
    }

    #[test]
    fn initial_only_tail_reads_as_checked() {
        // trace: "kh" ends in h → 4; no tone makes `kh` a syllable either way.
        assert_eq!(
            apply_telex_key("kh", "x", InputMode::Tl),
            Some("kh4".into())
        );
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
    fn z_is_a_single_tone_key_on_any_syllable() {
        // trace: `tone('6', '6')` ignores the coda, as `y d w q` do; a second
        // `z` is the same digit, another tone letter replaces it.
        assert_eq!(
            apply_telex_key("sit", "z", InputMode::Tl),
            Some("sit6".into())
        );
        assert_eq!(
            apply_telex_key("Te", "Z", InputMode::Poj),
            Some("Te6".into())
        );
        assert_eq!(apply_telex_key("te6", "z", InputMode::Tl), None);
        assert_eq!(
            apply_telex_key("te2", "z", InputMode::Tl),
            Some("te6".into())
        );
        assert_eq!(
            apply_telex_key("te6", "y", InputMode::Tl),
            Some("te3".into())
        );
        assert_eq!(apply_telex_key("", "z", InputMode::Tl), None);
    }

    #[test]
    fn c_expands_by_mode_and_case() {
        assert_eq!(apply_telex_key("", "c", InputMode::Tl), Some("ts".into()));
        assert_eq!(apply_telex_key("", "C", InputMode::Tl), Some("Ts".into()));
        assert_eq!(apply_telex_key("", "c", InputMode::Poj), Some("ch".into()));
        assert_eq!(apply_telex_key("", "C", InputMode::Poj), Some("Ch".into()));
        assert_eq!(apply_telex_key("a", "c", InputMode::Tl), Some("ats".into()));
        assert_eq!(
            apply_telex_key("a", "c", InputMode::Poj),
            Some("ach".into())
        );
    }

    #[test]
    fn h_writes_itself_except_after_an_aspirated_affricate() {
        // trace: `ends_in_aspirated_affricate` — `ts` / `ch` take the `h` that
        // aspirates them; `tsh` / `chh` take no further one, in either mode or
        // case. Every other tail keeps the letter as typed.
        for (tail, next) in [
            ("", "h"),
            ("t", "th"),
            ("ts", "tsh"),
            ("ch", "chh"),
            ("Ch", "Chh"),
            ("a", "ah"),
            ("tsh5", "tsh5h"),
            ("tsha", "tshah"),
        ] {
            assert_eq!(
                apply_telex_key(tail, "h", InputMode::Tl),
                Some(next.into()),
                "{tail}"
            );
        }
        assert_eq!(
            apply_telex_key("ts", "H", InputMode::Tl),
            Some("tsH".into())
        );
        for tail in ["tsh", "chh", "Tsh", "Chh", "CHH", "atsh"] {
            for mode in [InputMode::Tl, InputMode::Poj] {
                assert_eq!(apply_telex_key(tail, "h", mode), None, "{tail}");
                assert_eq!(apply_telex_key(tail, "H", mode), None, "{tail}");
            }
        }
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
        for key in ["a", "t", "1", "", "vv", "-"] {
            assert_eq!(
                apply_telex_key("te", key, InputMode::Tl),
                None,
                "key {key:?}"
            );
        }
    }
}
