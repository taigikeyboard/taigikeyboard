//! One recordable key combination, and the keys a binding may never claim.
//! Port of `ComposingKeyChord.swift`.

// 中文: 一個可錄製的按鍵組合,以及絕不可被綁走的按鍵。

use super::intent::ComposingKeyIntent;
use super::slot_key_set::CandidateSlotKeySet;
use super::snapshot::{KeyEventSnapshot, KeyModifiers};

/// A key plus its modifiers, as a composing action can be bound to it.
///
/// The key is stored as the character it types with no modifiers held, not
/// as a key code: a layout that puts `[` somewhere else should bind the key
/// that actually types `[`.
///
/// Constructing one is where the typing keys are defended: [`Self::make`]
/// refuses any chord that would take away a key the user composes with, so a
/// corrupt settings value cannot produce a binding that swallows the letters
/// of a syllable — the classifier never has to re-check.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ComposingKeyChord {
    /// What the key types with no modifiers held, ASCII-lowercased so
    /// `Shift+[` and `[` cannot be recorded as two chords on one key.
    pub key: String,
    pub modifiers: KeyModifiers,
}

/// Why a key could not be recorded, so the recorder can say so rather than
/// silently doing nothing (`ComposingKeyChord.swift:54-84`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChordRejection {
    /// A syllable letter, a digit or the hyphen with no modifier held. The
    /// eight letters no syllable uses are not refused.
    TypesRomanization,
    /// Backspace, Escape, the arrows or the paging keys — reserved whatever
    /// modifiers are held.
    ReservedKey,
    /// An event carrying no character to bind.
    NoKey,
    /// A candidate-slot key: one of the keys the chosen set holds, or
    /// Shift+1…Shift+9 whichever set is chosen.
    CandidateSlotChord,
    /// Global tier only: a chord the system already answers to.
    TakenBySystem,
    /// Global tier only: a press the hotkey registry cannot name.
    NotAGlobalKey,
    /// Global tier only: a bare host-owned combination that would take an
    /// application's own menu command.
    BelongsToHost,
}

/// The letters a TL or POJ syllable can be spelled with. Eight ASCII letters
/// are absent — d f q v w x y z — because neither romanization uses them
/// (`knowledge/taigi-phonetics-reference.md` §2–3), and a key that spells no
/// syllable is exactly the kind a user wants free for a bare binding.
/// CROSS-PLATFORM INVARIANT — mirrors `ComposingKeyChord.swift:256`.
const SYLLABLE_LETTERS: &str = "abceghijklmnoprstu";

/// The AppKit private-use range the Mac spells its arrow keys in. Refused
/// defensively — a raw value naming one of those scalars is a reserved key
/// here too — even though chord raw values are NOT portable between the
/// desktops (see `raw_value`).
const FUNCTION_KEY_RANGE: std::ops::RangeInclusive<u32> = 0xF700..=0xF8FF;

impl ComposingKeyChord {
    /// The chord `key` and `modifiers` name, or why it cannot be one.
    pub fn make(
        raw_key: Option<&str>,
        raw_modifiers: KeyModifiers,
    ) -> Result<Self, ChordRejection> {
        let raw_key = raw_key
            .filter(|key| !key.is_empty())
            .ok_or(ChordRejection::NoKey)?;
        let key = Self::normalized(raw_key);
        if Self::is_never_bindable(&key) {
            return Err(ChordRejection::ReservedKey);
        }
        // Only the four chording modifiers are part of a chord; the snapshot
        // already dropped the rest.
        let modifiers = raw_modifiers;
        // Shift+1…Shift+9 are the `shift` set's slot keys, and no chord on one
        // could be matched under any set — refused as the slot chord it is,
        // ahead of the typing-key rule that would catch the digit with a less
        // true reason.
        if modifiers == KeyModifiers::SHIFT
            && ComposingKeyIntent::direct_selection_slot(Some(&key)).is_some()
        {
            return Err(ChordRejection::CandidateSlotChord);
        }
        // Shift alone does not make a chord out of a typing key: Shift+A is
        // still the letter A, and binding it would cost the user their capitals.
        let first = key.chars().next().ok_or(ChordRejection::NoKey)?;
        if !modifiers.has_host_chord() && Self::is_syllable_typing_key(first) {
            return Err(ChordRejection::TypesRomanization);
        }
        Ok(Self { key, modifiers })
    }

    /// The chord this event would record, or why it cannot be recorded. A
    /// shifted digit is refused by the same rule that makes it pick under the
    /// `shift` set, read off the key code rather than the characters.
    pub fn make_from_event(event: &KeyEventSnapshot) -> Result<Self, ChordRejection> {
        if ComposingKeyIntent::shifted_digit_slot(event).is_some() {
            return Err(ChordRejection::CandidateSlotChord);
        }
        Self::make(event.unmodified_characters(), event.modifiers)
    }

    /// Whether `event` is this chord. Compared on the unmodified characters
    /// for the same reason they are stored: Control rewrites the digits it is
    /// held with, and Alt rewrites much of the keyboard.
    pub fn matches(&self, event: &KeyEventSnapshot) -> bool {
        let Some(characters) = event.unmodified_characters() else {
            return false;
        };
        Self::normalized(characters) == self.key && event.modifiers == self.modifiers
    }

    /// Whether this chord is a key of the set `slot_key_set` puts on the
    /// candidate slots — the half of the slot tier that is a setting.
    pub fn is_candidate_slot_chord(&self, slot_key_set: CandidateSlotKeySet) -> bool {
        slot_key_set
            .slot_for_key(Some(&self.key), self.modifiers)
            .is_some()
    }

    /// The form a key is stored and compared in: ASCII lowercased; the keypad
    /// Enter (`\u{3}`) and the back tab (`\u{19}`) folded onto Return and Tab
    /// — the Mac's spellings, kept so the two chord grammars stay one grammar
    /// even though a shell here never produces either.
    fn normalized(key: &str) -> String {
        match key {
            "\u{3}" => "\r".to_owned(),
            "\u{19}" => "\t".to_owned(),
            other => other.to_ascii_lowercase(),
        }
    }

    /// Backspace, Delete, Escape, and the arrow / paging keys (which the Mac
    /// spells as private-use scalars).
    fn is_never_bindable(key: &str) -> bool {
        matches!(key, "\u{8}" | "\u{7F}" | "\u{1B}")
            || key
                .chars()
                .next()
                .is_some_and(|c| FUNCTION_KEY_RANGE.contains(&(c as u32)))
    }

    /// The letters and the hyphen a syllable is spelled with, plus the digits
    /// that carry its tone.
    fn is_syllable_typing_key(character: char) -> bool {
        SYLLABLE_LETTERS.contains(character)
            || character == '-'
            || ComposingKeyIntent::is_tone_digit(character)
    }

    /// `"<modifiers>|<scalars>"` — modifier letters in a fixed order (`w`
    /// win, `c` control, `a` alt, `s` shift), then the key's scalars in hex.
    /// Hex rather than the character: most bound keys are control characters,
    /// and a settings file holding a raw `\r` is one editor away from
    /// unreadable.
    ///
    /// NAMED DIVERGENCE: the macOS letters are `d` command / `c` control / `o`
    /// option / `s` shift. A chord raw value is platform-LOCAL — the modifiers
    /// it names exist on one keyboard — so neither platform parses the
    /// other's, and a future settings transfer must map `d↔w`, `o↔a` itself
    /// (or, more honestly, carry only the setting names and let each platform
    /// keep its own chords).
    pub fn raw_value(&self) -> String {
        let mut letters = String::new();
        if self.modifiers.win {
            letters.push('w');
        }
        if self.modifiers.control {
            letters.push('c');
        }
        if self.modifiers.alt {
            letters.push('a');
        }
        if self.modifiers.shift {
            letters.push('s');
        }
        let scalars = self
            .key
            .chars()
            .map(|c| format!("{:04X}", c as u32))
            .collect::<Vec<_>>()
            .join(",");
        format!("{letters}|{scalars}")
    }

    /// Back through [`Self::make`], the gate that defends the typing keys, so
    /// a hand-edited value cannot install a binding that swallows the letters
    /// of a syllable.
    ///
    /// NOT the recorder's whole gate: the tier rules — the candidate-slot
    /// keys, `global_rejection`, and the composing tier's Ctrl+Alt refusal —
    /// live in `evaluate_press`, which a value read from `settings.json` does
    /// not pass through. A hand-edited file can therefore hold a chord the
    /// recorder would have refused; only the UI is gated.
    pub fn from_raw(raw: &str) -> Option<Self> {
        let (letters, scalars) = raw.split_once('|')?;
        let mut modifiers = KeyModifiers::NONE;
        for letter in letters.chars() {
            match letter {
                'w' => modifiers.win = true,
                'c' => modifiers.control = true,
                'a' => modifiers.alt = true,
                's' => modifiers.shift = true,
                _ => return None,
            }
        }
        let mut key = String::new();
        for field in scalars.split(',') {
            let value = u32::from_str_radix(field, 16).ok()?;
            key.push(char::from_u32(value)?);
        }
        Self::make(Some(&key), modifiers).ok()
    }

    /// The chord as a keycap label: `Shift+Enter`, `Ctrl+]`, `Space`.
    pub fn display(&self) -> String {
        let mut parts = Vec::new();
        if self.modifiers.win {
            parts.push("Win".to_owned());
        }
        if self.modifiers.control {
            parts.push("Ctrl".to_owned());
        }
        if self.modifiers.alt {
            parts.push("Alt".to_owned());
        }
        if self.modifiers.shift {
            parts.push("Shift".to_owned());
        }
        // A chord WITH modifiers keeps the uppercase keycap legend (`Ctrl+J`);
        // a bare key shows the character it types — an uppercase `Z` on a
        // modifier-less row reads as Shift+Z, a key the row does not hold
        // (USER 2026-08-22; `ShortcutKeyDisplay.swift:509-513`).
        parts.push(match self.key.as_str() {
            "\r" => "Enter".to_owned(),
            "\t" => "Tab".to_owned(),
            " " => "Space".to_owned(),
            other if self.modifiers.is_empty() => other.to_owned(),
            other => other.to_ascii_uppercase(),
        });
        parts.join("+")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chord(key: &str, modifiers: KeyModifiers) -> ComposingKeyChord {
        ComposingKeyChord::make(Some(key), modifiers).expect("bindable")
    }

    #[test]
    fn typing_keys_cannot_be_recorded_bare() {
        // trace: ComposingKeyBindingsTests.swift:15-25 — the whole syllable
        // alphabet plus a capital, the digits and the hyphen.
        for key in SYLLABLE_LETTERS
            .chars()
            .map(String::from)
            .chain(["A", "5", "0", "-"].map(String::from))
        {
            assert_eq!(
                ComposingKeyChord::make(Some(&key), KeyModifiers::NONE),
                Err(ChordRejection::TypesRomanization),
                "{key}"
            );
        }
    }

    #[test]
    fn non_syllable_letters_can_be_recorded_bare_and_capitals_fold() {
        for key in ["d", "f", "q", "v", "w", "x", "y", "z"] {
            let chord = chord(key, KeyModifiers::NONE);
            assert_eq!(chord.key, key);
        }
        let shifted = chord("Z", KeyModifiers::SHIFT);
        assert_eq!(shifted.key, "z");
        assert_eq!(shifted.modifiers, KeyModifiers::SHIFT);
        assert_eq!(
            ComposingKeyChord::make(Some("R"), KeyModifiers::SHIFT),
            Err(ChordRejection::TypesRomanization)
        );
    }

    #[test]
    fn bare_letter_and_its_shifted_twin_do_not_cross_match() {
        let bare = chord("z", KeyModifiers::NONE);
        let shifted = chord("Z", KeyModifiers::SHIFT);
        let bare_event = KeyEventSnapshot::text("z", KeyModifiers::NONE);
        let shifted_event = KeyEventSnapshot::chord(Some("Z"), "z", KeyModifiers::SHIFT);
        assert!(bare.matches(&bare_event));
        assert!(!bare.matches(&shifted_event));
        assert!(shifted.matches(&shifted_event));
        assert!(!shifted.matches(&bare_event));
    }

    #[test]
    fn typing_keys_bind_with_a_host_modifier_but_not_shift_alone() {
        for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT, KeyModifiers::WIN] {
            assert!(ComposingKeyChord::make(Some("a"), modifiers).is_ok());
        }
        assert_eq!(
            ComposingKeyChord::make(Some("a"), KeyModifiers::SHIFT),
            Err(ChordRejection::TypesRomanization)
        );
    }

    #[test]
    fn reserved_keys_cannot_be_recorded_at_all() {
        for key in ["\u{1B}", "\u{8}", "\u{7F}", "\u{F702}"] {
            for modifiers in [
                KeyModifiers::NONE,
                KeyModifiers::CONTROL,
                KeyModifiers::WIN.with(KeyModifiers::SHIFT),
            ] {
                assert_eq!(
                    ComposingKeyChord::make(Some(key), modifiers),
                    Err(ChordRejection::ReservedKey),
                    "{key:?}"
                );
            }
        }
        assert_eq!(
            ComposingKeyChord::make(None, KeyModifiers::NONE),
            Err(ChordRejection::NoKey)
        );
        assert_eq!(
            ComposingKeyChord::make(Some(""), KeyModifiers::NONE),
            Err(ChordRejection::NoKey)
        );
    }

    #[test]
    fn shifted_digits_are_slot_chords_and_control_digits_are_ordinary() {
        assert_eq!(
            ComposingKeyChord::make(Some("3"), KeyModifiers::SHIFT),
            Err(ChordRejection::CandidateSlotChord)
        );
        assert!(
            chord("3", KeyModifiers::CONTROL).is_candidate_slot_chord(CandidateSlotKeySet::Control)
        );
        assert!(
            !chord("3", KeyModifiers::CONTROL).is_candidate_slot_chord(CandidateSlotKeySet::Option)
        );
        assert!(!chord("3", KeyModifiers::CONTROL)
            .is_candidate_slot_chord(CandidateSlotKeySet::BareKeys));
        assert!(!chord("0", KeyModifiers::CONTROL)
            .is_candidate_slot_chord(CandidateSlotKeySet::Control));
        assert!(
            chord("z", KeyModifiers::NONE).is_candidate_slot_chord(CandidateSlotKeySet::BareKeys)
        );
    }

    #[test]
    fn raw_values_round_trip_and_stay_stable() {
        // trace: ComposingKeyBindingsTests.swift:163-172 — same shape, Windows
        // modifier letters (w/c/a/s).
        for (key, modifiers) in [
            (" ", KeyModifiers::NONE),
            ("\r", KeyModifiers::SHIFT),
            ("z", KeyModifiers::NONE),
            ("Z", KeyModifiers::SHIFT),
            (
                "]",
                KeyModifiers::WIN
                    .with(KeyModifiers::CONTROL)
                    .with(KeyModifiers::ALT)
                    .with(KeyModifiers::SHIFT),
            ),
        ] {
            let chord = chord(key, modifiers);
            assert_eq!(ComposingKeyChord::from_raw(&chord.raw_value()), Some(chord));
        }
        assert_eq!(chord(" ", KeyModifiers::NONE).raw_value(), "|0020");
        assert_eq!(chord("\r", KeyModifiers::SHIFT).raw_value(), "s|000D");
        assert_eq!(
            chord("]", KeyModifiers::CONTROL.with(KeyModifiers::ALT)).raw_value(),
            "ca|005D"
        );
    }

    #[test]
    fn raw_values_that_would_take_a_typing_key_do_not_parse() {
        assert_eq!(ComposingKeyChord::from_raw("|0061"), None, "bare a");
        assert_eq!(ComposingKeyChord::from_raw("s|0035"), None, "Shift+5");
        assert_eq!(
            ComposingKeyChord::from_raw("c|F702"),
            None,
            "Ctrl+← is still an arrow"
        );
        assert_eq!(ComposingKeyChord::from_raw("garbage"), None);
        assert_eq!(
            ComposingKeyChord::from_raw("x|0020"),
            None,
            "unknown modifier letter"
        );
    }

    #[test]
    fn shifted_digit_is_refused_from_an_event_under_any_slot_set() {
        // The key-code path: Shift+3 types `#`, so the characters alone would
        // slip past the digit rule — `make_from_event` reads the row.
        let shift_three =
            KeyEventSnapshot::chord(Some("#"), "#", KeyModifiers::SHIFT).with_key_code(0x33);
        assert_eq!(
            ComposingKeyChord::make_from_event(&shift_three),
            Err(ChordRejection::CandidateSlotChord)
        );
        let keypad_three = KeyEventSnapshot::chord(Some("3"), "3", KeyModifiers::SHIFT);
        assert_eq!(
            ComposingKeyChord::make_from_event(&keypad_three),
            Err(ChordRejection::CandidateSlotChord)
        );
        let ctrl_three = KeyEventSnapshot::chord(Some("\u{1B}"), "3", KeyModifiers::CONTROL);
        assert!(
            ComposingKeyChord::make_from_event(&ctrl_three).is_ok(),
            "Ctrl+3 is an ordinary chord to record"
        );
    }

    #[test]
    fn keypad_enter_matches_a_return_chord_and_bare_row_has_negative_controls() {
        let enter = chord("\r", KeyModifiers::NONE);
        assert!(
            enter.matches(&KeyEventSnapshot::text("\u{3}", KeyModifiers::NONE)),
            "keypad Enter is Return"
        );
        assert!(!enter.matches(&KeyEventSnapshot::text("\r", KeyModifiers::SHIFT)));
        for (slot, key) in CandidateSlotKeySet::BARE_KEY_ROW.iter().enumerate() {
            assert!(
                chord(key, KeyModifiers::NONE)
                    .is_candidate_slot_chord(CandidateSlotKeySet::BareKeys),
                "{key}"
            );
            assert_eq!(
                CandidateSlotKeySet::BareKeys.slot_for_key(Some(key), KeyModifiers::NONE),
                Some(slot)
            );
        }
        for key in [",", ".", "'", "/"] {
            assert!(
                !chord(key, KeyModifiers::NONE)
                    .is_candidate_slot_chord(CandidateSlotKeySet::BareKeys),
                "{key}"
            );
        }
    }

    #[test]
    fn keypad_enter_and_back_tab_fold_onto_their_key() {
        assert_eq!(chord("\u{3}", KeyModifiers::NONE).key, "\r");
        assert_eq!(chord("\u{19}", KeyModifiers::SHIFT).key, "\t");
        assert_eq!(chord("\r", KeyModifiers::SHIFT).display(), "Shift+Enter");
        assert_eq!(chord("]", KeyModifiers::CONTROL).display(), "Ctrl+]");
        assert_eq!(chord(" ", KeyModifiers::NONE).display(), "Space");
        assert_eq!(chord("z", KeyModifiers::NONE).display(), "z");
        assert_eq!(chord("z", KeyModifiers::CONTROL).display(), "Ctrl+Z");
    }
}
