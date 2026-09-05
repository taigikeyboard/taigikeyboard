//! Which keys pick a candidate out of the nine slots — the part of the slot
//! contract the user chooses. Port of `CandidateSlotKeySet`
//! (`ComposingKeyBindings.swift:5-107`).

// 九個候選槽位用哪組按鍵選取 — 使用者可選的那部分。

use super::intent::ComposingKeyIntent;
use super::snapshot::{KeyEventSnapshot, KeyModifiers};
use crate::settings::SettingChoice;

/// Every case is a whole key SET: nine slots need nine names. One set is live
/// at a time and it is the ONLY way to pick (USER 2026-08-28). A bare `1`…`9`
/// is never one of them: a bare digit is the TL/POJ tone marker, always.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CandidateSlotKeySet {
    /// Nine bare keys, one per slot — `q w d f z x v y ;`: the eight letters
    /// no TL or POJ syllable spells, plus `;`. The shipped default.
    BareKeys,
    /// Shift+1…Shift+9, read off the number row's key codes since Shift
    /// rewrites the characters.
    Shift,
    /// Ctrl+1…Ctrl+9.
    Control,
    /// Alt+1…Alt+9. Stored as `option` — the Mac's name — so the persisted
    /// value is the same on both desktops; only the label differs.
    Option,
}

impl CandidateSlotKeySet {
    /// The keys `BareKeys` puts on slots 0…8, in slot order. Lowercase, as
    /// they are matched and drawn.
    pub const BARE_KEY_ROW: [&'static str; 9] = ["q", "w", "d", "f", "z", "x", "v", "y", ";"];

    /// The slot `event` picks under this set, or `None` — the classifier's
    /// question, asked of the whole event because the shifted digits are only
    /// knowable from the key code.
    pub fn slot_for_event(self, event: &KeyEventSnapshot) -> Option<usize> {
        if self == Self::Shift {
            return ComposingKeyIntent::shifted_digit_slot(event);
        }
        self.slot_for_key(event.unmodified_characters(), event.modifiers)
    }

    /// The slot `key` picks under this set with exactly `modifiers` held.
    /// `key` is what the key types with no modifiers held. The one rule the
    /// classifier, the recorder's refusal and the window's labels all read.
    pub fn slot_for_key(self, key: Option<&str>, modifiers: KeyModifiers) -> Option<usize> {
        match self.digit_modifier() {
            None => {
                if !modifiers.is_empty() {
                    return None;
                }
                let key = key?.to_ascii_lowercase();
                Self::BARE_KEY_ROW.iter().position(|bare| *bare == key)
            }
            Some((flag, _)) => {
                if modifiers != flag {
                    return None;
                }
                ComposingKeyIntent::direct_selection_slot(key)
            }
        }
    }

    /// The key this set gives the candidate in `slot`, for the nine slots a
    /// page holds — what the candidate window draws beside a cell.
    pub fn label_for_slot(self, slot: usize) -> String {
        match self.digit_modifier() {
            None => Self::BARE_KEY_ROW[slot].to_owned(),
            Some((_, symbol)) => format!("{symbol}{}", slot + 1),
        }
    }

    /// What the shortcut pane's picker offers to choose between — the keys
    /// themselves, so the row reads the same in every display language.
    pub fn menu_label(self) -> String {
        match self.digit_modifier() {
            None => Self::BARE_KEY_ROW.join(" "),
            Some((_, symbol)) => format!("{symbol}1 – {symbol}9"),
        }
    }

    /// The modifier the digit chords are held with, and how it is written
    /// on a keycap. `None` for the bare keys.
    fn digit_modifier(self) -> Option<(KeyModifiers, &'static str)> {
        match self {
            Self::BareKeys => None,
            Self::Shift => Some((KeyModifiers::SHIFT, "⇧")),
            Self::Control => Some((KeyModifiers::CONTROL, "Ctrl+")),
            Self::Option => Some((KeyModifiers::ALT, "Alt+")),
        }
    }
}

impl SettingChoice for CandidateSlotKeySet {
    const ALL: &'static [Self] = &[Self::BareKeys, Self::Shift, Self::Control, Self::Option];
    const DEFAULT: Self = Self::BareKeys;
    fn raw(self) -> &'static str {
        match self {
            Self::BareKeys => "bareKeys",
            Self::Shift => "shift",
            Self::Control => "control",
            Self::Option => "option",
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
            "a bare digit is a tone"
        );
    }

    #[test]
    fn modifier_sets_pick_by_digit_with_exactly_their_modifier() {
        let control = CandidateSlotKeySet::Control;
        assert_eq!(
            control.slot_for_key(Some("3"), KeyModifiers::CONTROL),
            Some(2)
        );
        assert_eq!(
            control.slot_for_key(Some("9"), KeyModifiers::CONTROL),
            Some(8)
        );
        assert_eq!(control.slot_for_key(Some("0"), KeyModifiers::CONTROL), None);
        assert_eq!(control.slot_for_key(Some("3"), KeyModifiers::ALT), None);
        assert_eq!(
            control.slot_for_key(Some("3"), KeyModifiers::CONTROL.with(KeyModifiers::SHIFT)),
            None
        );
        assert_eq!(
            CandidateSlotKeySet::Option.slot_for_key(Some("3"), KeyModifiers::ALT),
            Some(2)
        );
    }

    #[test]
    fn shift_set_reads_the_number_row_key_code() {
        // Shift+3 types `#` on a US layout; VK '3' (0x33) still says which key.
        let event =
            KeyEventSnapshot::chord(Some("#"), "#", KeyModifiers::SHIFT).with_key_code(0x33);
        assert_eq!(CandidateSlotKeySet::Shift.slot_for_event(&event), Some(2));
        let keypad = KeyEventSnapshot::chord(Some("3"), "3", KeyModifiers::SHIFT);
        assert_eq!(
            CandidateSlotKeySet::Shift.slot_for_event(&keypad),
            Some(2),
            "keypad digit keeps its character"
        );
        assert_eq!(CandidateSlotKeySet::Control.slot_for_event(&event), None);
    }

    #[test]
    fn labels_and_menu_labels_name_the_keys() {
        assert_eq!(CandidateSlotKeySet::BareKeys.label_for_slot(0), "q");
        assert_eq!(CandidateSlotKeySet::Shift.label_for_slot(8), "⇧9");
        assert_eq!(CandidateSlotKeySet::Control.label_for_slot(2), "Ctrl+3");
        assert_eq!(CandidateSlotKeySet::Option.menu_label(), "Alt+1 – Alt+9");
        assert_eq!(
            CandidateSlotKeySet::BareKeys.menu_label(),
            "q w d f z x v y ;"
        );
        assert_eq!(
            CandidateSlotKeySet::from_raw("option"),
            Some(CandidateSlotKeySet::Option)
        );
    }
}
