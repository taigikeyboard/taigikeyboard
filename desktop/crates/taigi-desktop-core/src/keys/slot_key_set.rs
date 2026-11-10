//! Which keys pick a candidate out of the nine slots.

use super::chord::{KEYPAD_DIGIT_KEY_CODES, NUMBER_ROW_KEY_CODES, SEMICOLON_KEY_CODE};
use super::intent::ComposingKeyIntent;
use super::snapshot::{KeyEventSnapshot, KeyModifiers};

/// Every case is a whole key SET: nine slots need nine names. One set is live
/// at a time and it is the ONLY way to pick (USER 2026-08-28). The set is
/// DERIVED from the tone scheme, not chosen on its own
/// (`ToneInputScheme::slot_key_set`): the letters and the digits are the same
/// keys under both schemes, with the two jobs swapped, so whichever keys type
/// the tones leaves the others free to pick. A bare digit is the TL/POJ tone
/// marker (`tai5`) only under `Standard` — which is why the digits can sit on
/// the slots under Telex and not there. The Shift / Ctrl / Alt digit sets went
/// with the picker that chose them (USER 2026-09-08). Type - with f takes `f`
/// off the Standard row (`BareKeysWithoutF`, USER 2026-11-10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CandidateSlotKeySet {
    /// Nine bare keys, one per slot — `q w d f z x v y ;`: the eight letters
    /// no TL or POJ syllable spells, plus `;`. The set under `Standard`.
    BareKeys,
    /// `BareKeys` without `f` — `q w d z x v y ;` on slots 0…7, the keys after
    /// `f` moved one slot left and the ninth slot unkeyed (USER 2026-11-10).
    /// The set under `Standard` while Type - with f is on, where `f` types
    /// the hyphen and must not pick.
    BareKeysWithoutF,
    /// Bare `1`…`9`, the set under `Telex`, where the letters above type the
    /// tones and a digit no longer can — so the digit is free to pick the way
    /// the system Zhuyin input method's is.
    Digits,
    /// `1`…`9` of the number row and of the keypad, the set under TPS. The
    /// number row types glyphs while the user types (`tps_layout.rs`); it
    /// picks only once the window is up, which under TPS opens on demand
    /// (desktop TPS roadmap D7) — the classifier asks this set only then.
    /// Read by key code, so a layout's other characters on those keys pick
    /// by position, and Shift+`1` stays the ㆠ it types.
    TpsDigits,
}

impl CandidateSlotKeySet {
    /// Every set, for anything measured against every form a slot can be
    /// drawn in (`CandidateIndexLabel::widest_label_forms`). Not a
    /// `SettingChoice`: nothing stores a set any more.
    pub const ALL: [Self; 4] = [
        Self::BareKeys,
        Self::BareKeysWithoutF,
        Self::Digits,
        Self::TpsDigits,
    ];

    /// The keys `BareKeys` puts on slots 0…8, in slot order. Lowercase, as
    /// they are matched and drawn.
    pub const BARE_KEY_ROW: [&'static str; 9] = ["q", "w", "d", "f", "z", "x", "v", "y", ";"];

    /// The keys `BareKeysWithoutF` puts on slots 0…7, in slot order.
    pub const BARE_KEY_ROW_WITHOUT_F: [&'static str; 8] = ["q", "w", "d", "z", "x", "v", "y", ";"];

    /// The bare-key row a bare-key set picks with, or `None` for the digit
    /// sets.
    fn bare_key_row(self) -> Option<&'static [&'static str]> {
        match self {
            Self::BareKeys => Some(&Self::BARE_KEY_ROW),
            Self::BareKeysWithoutF => Some(&Self::BARE_KEY_ROW_WITHOUT_F),
            Self::Digits | Self::TpsDigits => None,
        }
    }

    /// The slot `event` picks under this set, or `None` — the classifier's
    /// question.
    pub fn slot_for_event(self, event: &KeyEventSnapshot) -> Option<usize> {
        match self {
            Self::BareKeys | Self::BareKeysWithoutF | Self::Digits => {
                self.slot_for_key(event.unmodified_characters(), event.modifiers)
            }
            Self::TpsDigits => Self::number_row_slot(event).or_else(|| Self::keypad_slot(event)),
        }
    }

    /// A bare number-row `1`…`9`, by key code.
    fn number_row_slot(event: &KeyEventSnapshot) -> Option<usize> {
        if !event.modifiers.is_empty() {
            return None;
        }
        NUMBER_ROW_KEY_CODES
            .iter()
            .position(|code| Some(*code) == event.key_code)
    }

    /// A bare keypad digit — the key code says keypad, and the key typed its
    /// digit: with Num Lock off a Linux keypad key keeps its code but types
    /// nothing, and must not pick.
    fn keypad_slot(event: &KeyEventSnapshot) -> Option<usize> {
        if !event.modifiers.is_empty() {
            return None;
        }
        let slot = KEYPAD_DIGIT_KEY_CODES
            .iter()
            .position(|code| Some(*code) == event.key_code)?;
        (ComposingKeyIntent::direct_selection_slot(event.unmodified_characters()) == Some(slot))
            .then_some(slot)
    }

    /// The slot `key` picks under this set with exactly `modifiers` held.
    /// `key` is what the key types with no modifiers held. Both sets are bare
    /// keys, so any chording modifier makes the key miss: Shift+Q is the
    /// capital the composition takes as text, and Ctrl+3 is the host's. The
    /// one rule the classifier and the window's labels both read, so a key
    /// drawn beside a candidate is the key that picks it. `TpsDigits` answers
    /// `None`: it reads key codes, which only the event carries
    /// (`slot_for_event`).
    pub fn slot_for_key(self, key: Option<&str>, modifiers: KeyModifiers) -> Option<usize> {
        if !modifiers.is_empty() {
            return None;
        }
        match self {
            Self::BareKeys | Self::BareKeysWithoutF => {
                let key = key?.to_ascii_lowercase();
                self.bare_key_row()?.iter().position(|bare| *bare == key)
            }
            Self::Digits => ComposingKeyIntent::direct_selection_slot(key),
            Self::TpsDigits => None,
        }
    }

    /// The slot `event` names with exactly Shift held — the Hanji/romanization commit
    /// aimed at a slot (`ComposingKeyIntent::SelectCandidateSlot { flip }`),
    /// or `None`.
    ///
    /// Resolved off the key CODE for the digits and `;`, because the
    /// unmodified characters keep Shift (`key_translation.rs`
    /// `unmodified_state`): Shift+3 reads `#` and Shift+`;` reads `:`, and
    /// only the key's position still says which key was pressed — the same
    /// reading the recorder refuses those presses by
    /// (`ComposingKeyChord::make_from_press`). The letters read as their
    /// capital, which the case fold already handles.
    pub fn shifted_slot_for_event(self, event: &KeyEventSnapshot) -> Option<usize> {
        if event.modifiers != KeyModifiers::SHIFT {
            return None;
        }
        match self {
            Self::BareKeys | Self::BareKeysWithoutF => {
                if event.key_code == Some(SEMICOLON_KEY_CODE) {
                    return self.bare_key_row().map(|row| row.len() - 1);
                }
                self.slot_for_key(event.unmodified_characters(), KeyModifiers::NONE)
            }
            Self::Digits => {
                let key_code = event.key_code?;
                NUMBER_ROW_KEY_CODES
                    .iter()
                    .position(|code| *code == key_code)
            }
            // Under TPS a commit is always the Hanji; there is no other
            // script to aim at a slot.
            Self::TpsDigits => None,
        }
    }

    /// The key this set gives the candidate in `slot`, for the nine slots a
    /// page holds — what the candidate window draws beside a cell; `""` for
    /// a slot the set leaves unkeyed.
    pub fn label_for_slot(self, slot: usize) -> String {
        match self.bare_key_row() {
            Some(row) => row.get(slot).copied().unwrap_or_default().to_owned(),
            None => (slot + 1).to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_keys_pick_by_letter_with_no_modifier() {
        // trace: CandidateSlotKeyTests — `z` is slot 4, `;` slot 8, case-folded.
        let set = CandidateSlotKeySet::BareKeys;
        assert_eq!(set.slot_for_key(Some("z"), KeyModifiers::NONE), Some(4));
        assert_eq!(set.slot_for_key(Some("Z"), KeyModifiers::NONE), Some(4));
        assert_eq!(set.slot_for_key(Some(";"), KeyModifiers::NONE), Some(8));
        assert_eq!(set.slot_for_key(Some("z"), KeyModifiers::SHIFT), None);
        assert_eq!(set.slot_for_key(Some("a"), KeyModifiers::NONE), None);
        assert_eq!(
            set.slot_for_key(Some("1"), KeyModifiers::NONE),
            None,
            "a bare digit is a tone under Standard"
        );
        for (slot, key) in CandidateSlotKeySet::BARE_KEY_ROW.iter().enumerate() {
            assert_eq!(set.slot_for_key(Some(key), KeyModifiers::NONE), Some(slot));
        }
    }

    #[test]
    fn without_f_the_row_shifts_left_and_f_picks_nothing() {
        // trace: `BARE_KEY_ROW_WITHOUT_F` — `d` stays slot 2, `z` moves from
        // slot 4 to 3, `;` from 8 to 7; slot 8 has no key and no label.
        let set = CandidateSlotKeySet::BareKeysWithoutF;
        assert_eq!(set.slot_for_key(Some("d"), KeyModifiers::NONE), Some(2));
        assert_eq!(set.slot_for_key(Some("z"), KeyModifiers::NONE), Some(3));
        assert_eq!(set.slot_for_key(Some(";"), KeyModifiers::NONE), Some(7));
        for key in ["f", "F"] {
            assert_eq!(
                set.slot_for_key(Some(key), KeyModifiers::NONE),
                None,
                "{key}"
            );
        }
        let labels: Vec<String> = (0..9).map(|slot| set.label_for_slot(slot)).collect();
        assert_eq!(labels, ["q", "w", "d", "z", "x", "v", "y", ";", ""]);
        let shift_semicolon = KeyEventSnapshot::chord(Some(":"), ":", KeyModifiers::SHIFT)
            .with_key_code(SEMICOLON_KEY_CODE);
        assert_eq!(set.shifted_slot_for_event(&shift_semicolon), Some(7));
        let shift_f = KeyEventSnapshot::chord(Some("F"), "F", KeyModifiers::SHIFT);
        assert_eq!(set.shifted_slot_for_event(&shift_f), None);
        let shift_z = KeyEventSnapshot::chord(Some("Z"), "Z", KeyModifiers::SHIFT);
        assert_eq!(set.shifted_slot_for_event(&shift_z), Some(3));
    }

    #[test]
    fn digits_pick_by_bare_digit_only() {
        // trace: `slot_for_key` under `Digits` — `1`…`9` → 0…8, `0`
        // names none, any chording modifier misses.
        let set = CandidateSlotKeySet::Digits;
        assert_eq!(set.slot_for_key(Some("1"), KeyModifiers::NONE), Some(0));
        assert_eq!(set.slot_for_key(Some("9"), KeyModifiers::NONE), Some(8));
        assert_eq!(set.slot_for_key(Some("0"), KeyModifiers::NONE), None);
        assert_eq!(set.slot_for_key(Some("q"), KeyModifiers::NONE), None);
        assert_eq!(set.slot_for_key(Some("3"), KeyModifiers::SHIFT), None);
        assert_eq!(set.slot_for_key(Some("3"), KeyModifiers::CONTROL), None);
        assert_eq!(set.slot_for_key(None, KeyModifiers::NONE), None);
        let keypad = KeyEventSnapshot::chord(Some("3"), "3", KeyModifiers::NONE);
        assert_eq!(set.slot_for_event(&keypad), Some(2));
        let shifted =
            KeyEventSnapshot::chord(Some("#"), "#", KeyModifiers::SHIFT).with_key_code(0x33);
        assert_eq!(set.slot_for_event(&shifted), None, "Shift+3 is not a slot");
    }

    #[test]
    fn shift_on_a_slot_key_names_the_same_slot_for_the_flip() {
        // trace: `shifted_slot_for_event` — letters by their capital, `;` and
        // the digits by key code (US layout types `:` / `#`).
        let bare = CandidateSlotKeySet::BareKeys;
        // Every letter of `BARE_KEY_ROW` by its capital.
        for (slot, key) in CandidateSlotKeySet::BARE_KEY_ROW.iter().enumerate() {
            if *key == ";" {
                continue;
            }
            let capital = key.to_uppercase();
            let shifted = KeyEventSnapshot::chord(Some(&capital), &capital, KeyModifiers::SHIFT);
            assert_eq!(bare.shifted_slot_for_event(&shifted), Some(slot), "{key}");
        }
        let shift_q = KeyEventSnapshot::chord(Some("Q"), "Q", KeyModifiers::SHIFT);
        assert_eq!(
            bare.slot_for_event(&shift_q),
            None,
            "the bare path still misses"
        );
        let shift_semicolon = KeyEventSnapshot::chord(Some(":"), ":", KeyModifiers::SHIFT)
            .with_key_code(SEMICOLON_KEY_CODE);
        assert_eq!(bare.shifted_slot_for_event(&shift_semicolon), Some(8));
        let colon_elsewhere = KeyEventSnapshot::chord(Some(":"), ":", KeyModifiers::SHIFT);
        assert_eq!(bare.shifted_slot_for_event(&colon_elsewhere), None);
        let ctrl_shift_q = KeyEventSnapshot::chord(
            Some("Q"),
            "Q",
            KeyModifiers::CONTROL.with(KeyModifiers::SHIFT),
        );
        assert_eq!(
            bare.shifted_slot_for_event(&ctrl_shift_q),
            None,
            "exactly Shift"
        );

        let digits = CandidateSlotKeySet::Digits;
        let shift_three =
            KeyEventSnapshot::chord(Some("#"), "#", KeyModifiers::SHIFT).with_key_code(0x33);
        assert_eq!(digits.shifted_slot_for_event(&shift_three), Some(2));
        assert_eq!(
            digits.shifted_slot_for_event(&shift_q),
            None,
            "letters are tones under Telex"
        );
        assert_eq!(digits.shifted_slot_for_event(&shift_semicolon), None);
        assert_eq!(
            bare.shifted_slot_for_event(&shift_three),
            None,
            "digits are tones under Standard"
        );
        let hash_elsewhere = KeyEventSnapshot::chord(Some("#"), "#", KeyModifiers::SHIFT);
        assert_eq!(
            digits.shifted_slot_for_event(&hash_elsewhere),
            None,
            "no key code, no slot"
        );
    }

    #[test]
    fn labels_name_the_keys() {
        assert_eq!(CandidateSlotKeySet::BareKeys.label_for_slot(0), "q");
        assert_eq!(CandidateSlotKeySet::BareKeys.label_for_slot(8), ";");
        assert_eq!(CandidateSlotKeySet::Digits.label_for_slot(0), "1");
        assert_eq!(CandidateSlotKeySet::Digits.label_for_slot(8), "9");
    }

    #[test]
    fn tps_digits_pick_by_number_row_or_keypad_code() {
        // trace: KEYPAD_DIGIT_KEY_CODES — VK_NUMPAD3 (0x63) typing `3` is slot 2;
        // NUMBER_ROW_KEY_CODES — the number row's `3` (0x33) is slot 2 too,
        // whatever it types; Shift+`1` (ㆠ) misses; a keypad key typing
        // nothing (Num Lock off on Linux) misses; Shift never flips.
        let set = CandidateSlotKeySet::TpsDigits;
        let keypad = |digit: &str, code: u16, modifiers| {
            KeyEventSnapshot::chord(Some(digit), digit, modifiers).with_key_code(code)
        };
        assert_eq!(
            set.slot_for_event(&keypad("3", 0x63, KeyModifiers::NONE)),
            Some(2)
        );
        assert_eq!(
            set.slot_for_event(&keypad("1", 0x61, KeyModifiers::NONE)),
            Some(0)
        );
        assert_eq!(
            set.slot_for_event(&keypad("9", 0x69, KeyModifiers::NONE)),
            Some(8)
        );
        assert_eq!(
            set.slot_for_event(&keypad("3", 0x33, KeyModifiers::NONE)),
            Some(2)
        );
        assert_eq!(
            set.slot_for_event(&keypad("\"", 0x33, KeyModifiers::NONE)),
            Some(2),
            "by position on any layout"
        );
        assert_eq!(
            set.slot_for_event(&keypad("!", 0x31, KeyModifiers::SHIFT)),
            None,
            "Shift+1 types ㆠ"
        );
        assert_eq!(
            set.slot_for_event(&keypad("0", 0x30, KeyModifiers::NONE)),
            None
        );
        assert_eq!(
            set.slot_for_event(&keypad("3", 0x63, KeyModifiers::CONTROL)),
            None
        );
        assert_eq!(
            set.shifted_slot_for_event(&keypad("3", 0x63, KeyModifiers::SHIFT)),
            None
        );
        let num_lock_off = KeyEventSnapshot::named_special(KeyModifiers::NONE).with_key_code(0x63);
        assert_eq!(set.slot_for_event(&num_lock_off), None);
        assert_eq!(set.slot_for_key(Some("3"), KeyModifiers::NONE), None);
        assert_eq!(set.label_for_slot(0), "1");
    }
}
