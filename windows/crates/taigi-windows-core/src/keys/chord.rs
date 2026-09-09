//! One recordable key combination, and the keys a binding may never claim.
//! Port of `ComposingKeyChord.swift`.

use super::intent::ComposingKeyIntent;
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
/// silently doing nothing (`ComposingKeyChord.swift` `Rejection`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChordRejection {
    /// A letter, a digit, the hyphen or `;` with no Ctrl/Alt/Win held — every
    /// key one of the two tone schemes types with or picks a candidate with
    /// (`is_typing_key`). Refused whichever scheme is live, so a chord
    /// recorded under one cannot go inert when the user switches to the
    /// other.
    TypesRomanization,
    /// Backspace, Escape, the arrows or the paging keys — reserved whatever
    /// modifiers are held.
    ReservedKey,
    /// An event carrying no character to bind.
    NoKey,
    /// Global tier only: a chord the system already answers to.
    TakenBySystem,
    /// Global tier only: a press the hotkey registry cannot name.
    NotAGlobalKey,
    /// Global tier only: a bare host-owned combination that would take an
    /// application's own menu command.
    BelongsToHost,
}

/// The number-row virtual-key codes `1`…`9` (`VK_1`…`VK_9` = `0x31`…`0x39`),
/// in digit order. Positions, so the same nine keys on every layout — on
/// AZERTY, where the bare row types `& é " …`, Shift+`&` is still the `1`
/// key. Mirrors `ComposingKeyChord.swift` `numberRowKeyCodes`.
pub(crate) const NUMBER_ROW_KEY_CODES: [u16; 9] =
    [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39];

/// `VK_OEM_1`, the `;` key on a US layout — refused under Shift for the same
/// reason as the number row: it is the ninth slot key, and Shift+`;` aims
/// the 漢羅 commit at it (`CandidateSlotKeySet::shifted_slot_for_event`)
/// even though the layout types `:` for it. Layout-dependent by Microsoft's
/// own word, the trade the US-position number row already makes. Mirrors
/// `ComposingKeyChord.swift` `semicolonKeyCode`.
pub(crate) const SEMICOLON_KEY_CODE: u16 = 0xBA;

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
        // Shift alone does not make a chord out of a typing key: Shift+A is
        // still the letter A, and binding it would cost the user their capitals.
        let first = key.chars().next().ok_or(ChordRejection::NoKey)?;
        if !modifiers.has_host_chord() && Self::is_typing_key(first) {
            return Err(ChordRejection::TypesRomanization);
        }
        Ok(Self { key, modifiers })
    }

    /// The chord this event would record, or why it cannot be recorded.
    ///
    /// A shifted number-row key is refused as the digit it is: Shift alone
    /// does not make a chord out of a typing key (`make`), and Shift+3 is the
    /// `3` key even though a US layout types `#` for it. Read off the key
    /// code, because that is the one thing Shift does not rewrite — and it is
    /// what keeps this path and the settings-file path (`translate_raw`,
    /// which is handed the unmodified `3`) refusing the same press. Every
    /// other shifted key records as the character it types, which is what its
    /// stored chords already hold. Mirrors `ComposingKeyChord.make(_:)`.
    pub fn make_from_event(event: &KeyEventSnapshot) -> Result<Self, ChordRejection> {
        Self::make_from_press(
            event.unmodified_characters(),
            event.modifiers,
            event.key_code,
        )
    }

    /// `make` with the key code beside the characters — the one question the
    /// recorder (`RecordedPress`) and a live event both have to ask, so the
    /// shifted number row is refused on both.
    pub fn make_from_press(
        key: Option<&str>,
        modifiers: KeyModifiers,
        key_code: Option<u16>,
    ) -> Result<Self, ChordRejection> {
        if modifiers == KeyModifiers::SHIFT
            && key_code.is_some_and(|code| {
                NUMBER_ROW_KEY_CODES.contains(&code) || code == SEMICOLON_KEY_CODE
            })
        {
            return Err(ChordRejection::TypesRomanization);
        }
        Self::make(key, modifiers)
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

    /// The keys a composition is typed or picked with, under either tone
    /// scheme: all 26 ASCII letters, the digits, the hyphen and `;`.
    ///
    /// All 26 rather than the eighteen a TL or POJ syllable is spelled with,
    /// because the other eight are not free either: under Telex `v y d w x q
    /// z f` type the tones, and under Standard those eight and `;` are the
    /// candidate slots (`CandidateSlotKeySet::BARE_KEY_ROW`). One rule for
    /// both schemes, so a chord recorded under one cannot go inert when the
    /// user switches — which is also what lets `ComposingKeyBindings` skip
    /// any pass against the slot tier. Asked of the normalized key, so the
    /// case fold is `normalized`'s.
    /// CROSS-PLATFORM INVARIANT — mirrors `ComposingKeyChord.swift` `isTypingKey`.
    fn is_typing_key(character: char) -> bool {
        character.is_ascii_alphabetic()
            || ComposingKeyIntent::is_tone_digit(character)
            || character == '-'
            || character == ';'
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
    /// NOT the recorder's whole gate: the tier rules — `global_rejection`
    /// and the composing tier's Ctrl+Alt refusal — live in `evaluate_press`,
    /// which a value read from `settings.json` does not pass through. A
    /// hand-edited file can therefore hold a chord the recorder would have
    /// refused; only the UI is gated.
    pub fn from_raw(raw: &str) -> Option<Self> {
        Self::translate_raw(raw).and_then(Result::ok)
    }

    /// [`Self::from_raw`] with the gate's refusal kept: `None` for a value
    /// the grammar cannot read at all, `Some(Err)` for a well-formed value
    /// naming a chord the gate refuses. The launch pass reads WHY a stored
    /// global row fails to translate, because a row on a typing key is one
    /// the recorder would refuse today and the preserved key would still be
    /// dispatched first (`ShortcutActions.swift` `translation(of:)`). Kept to
    /// the key contract: `ShortcutAction::translation_in` is its only caller.
    pub(super) fn translate_raw(raw: &str) -> Option<Result<Self, ChordRejection>> {
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
        Some(Self::make(Some(&key), modifiers))
    }

    /// The modifier names a chord label leads with, in the order the system
    /// prints them: `Win`, `Ctrl`, `Alt`, `Shift`.
    pub fn modifier_labels(modifiers: KeyModifiers) -> impl Iterator<Item = String> {
        [
            (modifiers.win, "Win"),
            (modifiers.control, "Ctrl"),
            (modifiers.alt, "Alt"),
            (modifiers.shift, "Shift"),
        ]
        .into_iter()
        .filter(|(held, _)| *held)
        .map(|(_, name)| name.to_owned())
    }

    /// The chord as a keycap label: `Shift+Enter`, `Ctrl+]`, `Space`.
    pub fn display(&self) -> String {
        let mut parts: Vec<String> = Self::modifier_labels(self.modifiers).collect();
        // A chord WITH modifiers keeps the uppercase keycap legend (`Ctrl+J`);
        // a bare key shows the character it types — an uppercase `Z` on a
        // modifier-less row reads as Shift+Z, a key the row does not hold
        // (USER 2026-08-22; `ShortcutKeyDisplay` in `ShortcutKeyRecorder.swift`).
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
    use crate::keys::CandidateSlotKeySet;

    fn chord(key: &str, modifiers: KeyModifiers) -> ComposingKeyChord {
        ComposingKeyChord::make(Some(key), modifiers).expect("bindable")
    }

    #[test]
    fn typing_keys_cannot_be_recorded_bare() {
        // trace: ComposingKeyBindingsTests.swift — every ASCII letter (both
        // schemes' keys), a capital, the digits, the hyphen and `;`.
        for key in ('a'..='z')
            .map(String::from)
            .chain(["A", "V", "5", "0", "-", ";"].map(String::from))
        {
            assert_eq!(
                ComposingKeyChord::make(Some(&key), KeyModifiers::NONE),
                Err(ChordRejection::TypesRomanization),
                "{key}"
            );
        }
        // The Telex keys and the bare slot row are typing keys under one
        // rule, so a chord recorded under either scheme stays live.
        for key in crate::keys::ToneInputScheme::TELEX_KEYS
            .chars()
            .map(String::from)
            .chain(CandidateSlotKeySet::BARE_KEY_ROW.map(String::from))
        {
            assert_eq!(
                ComposingKeyChord::make(Some(&key), KeyModifiers::NONE),
                Err(ChordRejection::TypesRomanization),
                "{key}"
            );
        }
    }

    #[test]
    fn punctuation_can_be_recorded_bare_and_capitals_fold() {
        for key in [",", ".", "'", "/", "[", "]", "`"] {
            let chord = chord(key, KeyModifiers::NONE);
            assert_eq!(chord.key, key);
        }
        let shifted = chord("Z", KeyModifiers::CONTROL);
        assert_eq!(shifted.key, "z");
        assert_eq!(shifted.modifiers, KeyModifiers::CONTROL);
        assert_eq!(
            ComposingKeyChord::make(Some("Z"), KeyModifiers::SHIFT),
            Err(ChordRejection::TypesRomanization),
            "Shift alone does not make a chord out of a letter"
        );
    }

    #[test]
    fn bare_key_and_its_shifted_twin_do_not_cross_match() {
        let bare = chord("[", KeyModifiers::NONE);
        let shifted = chord("{", KeyModifiers::SHIFT);
        let bare_event = KeyEventSnapshot::text("[", KeyModifiers::NONE);
        let shifted_event = KeyEventSnapshot::chord(Some("{"), "{", KeyModifiers::SHIFT);
        assert!(bare.matches(&bare_event));
        assert!(!bare.matches(&shifted_event));
        assert!(shifted.matches(&shifted_event));
        assert!(!shifted.matches(&bare_event));
    }

    #[test]
    fn typing_keys_bind_with_a_host_modifier_but_not_shift_alone() {
        for key in ["a", "v", "z", "3", ";"] {
            for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT, KeyModifiers::WIN] {
                assert!(
                    ComposingKeyChord::make(Some(key), modifiers).is_ok(),
                    "{key} {modifiers:?}"
                );
            }
            assert_eq!(
                ComposingKeyChord::make(Some(key), KeyModifiers::SHIFT),
                Err(ChordRejection::TypesRomanization),
                "{key}"
            );
        }
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
    fn raw_values_round_trip_and_stay_stable() {
        // trace: ComposingKeyBindingsTests.swift:163-172 — same shape, Windows
        // modifier letters (w/c/a/s).
        for (key, modifiers) in [
            (" ", KeyModifiers::NONE),
            ("\r", KeyModifiers::SHIFT),
            ("[", KeyModifiers::NONE),
            ("z", KeyModifiers::CONTROL),
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
        assert_eq!(ComposingKeyChord::from_raw("|007A"), None, "bare z");
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
        // The launch pass reads the refusal apart from a value the grammar
        // cannot read.
        assert_eq!(
            ComposingKeyChord::translate_raw("|007A"),
            Some(Err(ChordRejection::TypesRomanization))
        );
        assert_eq!(
            ComposingKeyChord::translate_raw("s|0033"),
            Some(Err(ChordRejection::TypesRomanization)),
            "Shift+3 stored as the 3 it is"
        );
        assert_eq!(ComposingKeyChord::translate_raw("garbage"), None);
        assert_eq!(
            ComposingKeyChord::translate_raw("c|007A"),
            Some(Ok(chord("z", KeyModifiers::CONTROL)))
        );
    }

    #[test]
    fn shifted_number_row_key_is_refused_from_an_event_and_from_the_raw_digit() {
        // The key-code path: Shift+3 types `#`, so the characters alone would
        // slip past the digit rule — `make_from_event` reads the row.
        let shift_three =
            KeyEventSnapshot::chord(Some("#"), "#", KeyModifiers::SHIFT).with_key_code(0x33);
        assert_eq!(
            ComposingKeyChord::make_from_event(&shift_three),
            Err(ChordRejection::TypesRomanization)
        );
        // The raw path, handed the unmodified `3` with Shift held.
        assert_eq!(
            ComposingKeyChord::make(Some("3"), KeyModifiers::SHIFT),
            Err(ChordRejection::TypesRomanization)
        );
        let keypad_three = KeyEventSnapshot::chord(Some("3"), "3", KeyModifiers::SHIFT);
        assert_eq!(
            ComposingKeyChord::make_from_event(&keypad_three),
            Err(ChordRejection::TypesRomanization)
        );
        // A `#` reached without the number row (a layout with a `#` key)
        // still records as `#`.
        let hash_key = KeyEventSnapshot::chord(Some("#"), "#", KeyModifiers::SHIFT);
        assert_eq!(
            ComposingKeyChord::make_from_event(&hash_key),
            Ok(chord("#", KeyModifiers::SHIFT))
        );
        // Shift plus a host modifier on the number row is an ordinary chord.
        let ctrl_shift_three = KeyEventSnapshot::chord(
            Some("#"),
            "3",
            KeyModifiers::CONTROL.with(KeyModifiers::SHIFT),
        )
        .with_key_code(0x33);
        assert_eq!(
            ComposingKeyChord::make_from_event(&ctrl_shift_three),
            Ok(chord("3", KeyModifiers::CONTROL.with(KeyModifiers::SHIFT)))
        );
        let ctrl_three = KeyEventSnapshot::chord(Some("\u{1B}"), "3", KeyModifiers::CONTROL);
        assert!(
            ComposingKeyChord::make_from_event(&ctrl_three).is_ok(),
            "Ctrl+3 is an ordinary chord to record"
        );
        // Shift+`;` is the ninth slot key's 漢羅 chord, refused by its key
        // code the same way; a `:` reached without that key still records.
        let shift_semicolon = KeyEventSnapshot::chord(Some(":"), ":", KeyModifiers::SHIFT)
            .with_key_code(SEMICOLON_KEY_CODE);
        assert_eq!(
            ComposingKeyChord::make_from_event(&shift_semicolon),
            Err(ChordRejection::TypesRomanization)
        );
        let colon_key = KeyEventSnapshot::chord(Some(":"), ":", KeyModifiers::SHIFT);
        assert_eq!(
            ComposingKeyChord::make_from_event(&colon_key),
            Ok(chord(":", KeyModifiers::SHIFT))
        );
    }

    #[test]
    fn keypad_enter_matches_a_return_chord() {
        let enter = chord("\r", KeyModifiers::NONE);
        assert!(
            enter.matches(&KeyEventSnapshot::text("\u{3}", KeyModifiers::NONE)),
            "keypad Enter is Return"
        );
        assert!(!enter.matches(&KeyEventSnapshot::text("\r", KeyModifiers::SHIFT)));
    }

    #[test]
    fn keypad_enter_and_back_tab_fold_onto_their_key() {
        assert_eq!(chord("\u{3}", KeyModifiers::NONE).key, "\r");
        assert_eq!(chord("\u{19}", KeyModifiers::SHIFT).key, "\t");
        assert_eq!(chord("\r", KeyModifiers::SHIFT).display(), "Shift+Enter");
        assert_eq!(chord("]", KeyModifiers::CONTROL).display(), "Ctrl+]");
        assert_eq!(chord(" ", KeyModifiers::NONE).display(), "Space");
        assert_eq!(chord("[", KeyModifiers::NONE).display(), "[");
        assert_eq!(chord("z", KeyModifiers::CONTROL).display(), "Ctrl+Z");
    }
}
