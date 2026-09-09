//! Which keys pick a candidate out of the nine slots. Port of
//! `CandidateSlotKeySet` (`ComposingKeyBindings.swift`).

use super::chord::{NUMBER_ROW_KEY_CODES, SEMICOLON_KEY_CODE};
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
/// with the picker that chose them (USER 2026-09-08).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CandidateSlotKeySet {
    /// Nine bare keys, one per slot — `q w d f z x v y ;`: the eight letters
    /// no TL or POJ syllable spells, plus `;`. The set under `Standard`.
    BareKeys,
    /// Bare `1`…`9`, the set under `Telex`, where the letters above type the
    /// tones and a digit no longer can — so the digit is free to pick the way
    /// the system Zhuyin input method's is.
    Digits,
}

impl CandidateSlotKeySet {
    /// Both sets, for anything measured against every form a slot can be
    /// drawn in (`CandidateIndexLabel::widest_label_forms`). Not a
    /// `SettingChoice`: nothing stores a set any more.
    pub const ALL: [Self; 2] = [Self::BareKeys, Self::Digits];

    /// The keys `BareKeys` puts on slots 0…8, in slot order. Lowercase, as
    /// they are matched and drawn.
    pub const BARE_KEY_ROW: [&'static str; 9] = ["q", "w", "d", "f", "z", "x", "v", "y", ";"];

    /// The slot `event` picks under this set, or `None` — the classifier's
    /// question.
    pub fn slot_for_event(self, event: &KeyEventSnapshot) -> Option<usize> {
        self.slot_for_key(event.unmodified_characters(), event.modifiers)
    }

    /// The slot `key` picks under this set with exactly `modifiers` held.
    /// `key` is what the key types with no modifiers held. Both sets are bare
    /// keys, so any chording modifier makes the key miss: Shift+Q is the
    /// capital the composition takes as text, and Ctrl+3 is the host's. The
    /// one rule the classifier and the window's labels both read, so a key
    /// drawn beside a candidate is the key that picks it.
    pub fn slot_for_key(self, key: Option<&str>, modifiers: KeyModifiers) -> Option<usize> {
        if !modifiers.is_empty() {
            return None;
        }
        match self {
            Self::BareKeys => {
                let key = key?.to_ascii_lowercase();
                Self::BARE_KEY_ROW.iter().position(|bare| *bare == key)
            }
            Self::Digits => ComposingKeyIntent::direct_selection_slot(key),
        }
    }

    /// The slot `event` names with exactly Shift held — the 漢羅 commit
    /// aimed at a slot (`ComposingKeyIntent::SelectCandidateSlot { flip }`),
    /// or `None`.
    ///
    /// Resolved off the key CODE for the digits and `;`, because the
    /// unmodified characters keep Shift (`key_translation.rs`
    /// `unmodified_state`): Shift+3 reads `#` and Shift+`;` reads `:`, and
    /// only the key's position still says which key was pressed — the same
    /// reading the recorder refuses those presses by
    /// (`ComposingKeyChord::make_from_press`). The letters read as their
    /// capital, which the case fold already handles. Port of
    /// `CandidateSlotKeySet.shiftedSlot(for:)`.
    pub fn shifted_slot_for_event(self, event: &KeyEventSnapshot) -> Option<usize> {
        if event.modifiers != KeyModifiers::SHIFT {
            return None;
        }
        match self {
            Self::BareKeys => {
                if event.key_code == Some(SEMICOLON_KEY_CODE) {
                    return Some(Self::BARE_KEY_ROW.len() - 1);
                }
                self.slot_for_key(event.unmodified_characters(), KeyModifiers::NONE)
            }
            Self::Digits => {
                let key_code = event.key_code?;
                NUMBER_ROW_KEY_CODES
                    .iter()
                    .position(|code| *code == key_code)
            }
        }
    }

    /// The key this set gives the candidate in `slot`, for the nine slots a
    /// page holds — what the candidate window draws beside a cell.
    pub fn label_for_slot(self, slot: usize) -> String {
        match self {
            Self::BareKeys => Self::BARE_KEY_ROW[slot].to_owned(),
            Self::Digits => (slot + 1).to_string(),
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
    fn digits_pick_by_bare_digit_only() {
        // trace: CandidateSlotKeySet.swift `digitSlot` — `1`…`9` → 0…8, `0`
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
        // trace: CandidateSlotKeyTests.swift `testEveryBareKey_underShift_flipsItsSlot`
        // + `testAShiftedDigit_flipsItsSlot_underTelex…` — letters by their
        // capital, `;` and the digits by key code (US layout types `:` / `#`).
        let bare = CandidateSlotKeySet::BareKeys;
        let shift_q = KeyEventSnapshot::chord(Some("Q"), "Q", KeyModifiers::SHIFT);
        assert_eq!(bare.shifted_slot_for_event(&shift_q), Some(0));
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
}
