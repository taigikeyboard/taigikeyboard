//! One recordable key combination, and the keys a binding may never claim.

use super::intent::ComposingKeyIntent;
use super::snapshot::{KeyEventSnapshot, KeyModifiers};
use crate::platform::DesktopPlatform;

/// A key plus its modifiers, as a composing action can be bound to it.
///
/// The key is stored as the character it types with no modifiers held, not
/// as a key code: a layout that puts `[` somewhere else should bind the key
/// that actually types `[`.
///
/// The grammar is per desktop ([`DesktopPlatform`]): which keys are
/// reserved, how a key is case-folded, the modifier letters of a stored
/// value and how a chord reads on screen. Every function that builds, reads,
/// stores or draws a chord takes the platform, so the four agree.
///
/// Constructing one is where the typing keys are defended: [`Self::make`]
/// refuses any chord that would take away a key the user composes with, so a
/// corrupt settings value cannot produce a binding that swallows the letters
/// of a syllable — the classifier never has to re-check.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ComposingKeyChord {
    /// What the key types with no modifiers held, lowercased (`normalized`)
    /// so `Shift+[` and `[` cannot be recorded as two chords on one key.
    pub key: String,
    pub modifiers: KeyModifiers,
}

/// Why a key could not be recorded, so the recorder can say so rather than
/// silently doing nothing. macOS keeps a Swift twin of the reasons:
/// `ComposingKeyChord.swift` `Rejection`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChordRejection {
    /// A letter, a digit, the hyphen or `;` with no Ctrl/Alt/Win held — every
    /// key one of the two tone schemes types with or picks a candidate with
    /// (`is_typing_key`). Refused whichever scheme is live, so a chord
    /// recorded under one cannot go inert when the user switches to the
    /// other.
    TypesRomanization,
    /// Backspace, Escape, the arrows or the paging keys — reserved whatever
    /// modifiers are held — and a bare Home / End, the caret's while composing.
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
/// key.
pub(crate) const NUMBER_ROW_KEY_CODES: [u16; 9] =
    [0x31, 0x32, 0x33, 0x34, 0x35, 0x36, 0x37, 0x38, 0x39];

/// `VK_NUMPAD1`…`VK_NUMPAD9`, the keypad digits that pick under TPS
/// (`CandidateSlotKeySet::TpsDigits`). Windows fills them natively; the Linux and
/// macOS key translations map theirs.
pub(crate) const KEYPAD_DIGIT_KEY_CODES: [u16; 9] =
    [0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69];

/// Whether `code` is any keypad key: `VK_NUMPAD0`…`VK_DIVIDE` (0x60–0x6F),
/// or `VK_OEM_NEC_EQUAL` (0x92), the code the macOS translation gives the
/// keypad `=`. A keypad key types the same character as its main-block twin
/// but is never a TPS layout key (`tps_layout.rs`).
pub(crate) fn is_keypad_key_code(code: u16) -> bool {
    (0x60..=0x6F).contains(&code) || code == 0x92
}

/// `VK_OEM_1`, the `;` key on a US layout — refused under Shift for the same
/// reason as the number row: it is the ninth slot key, and Shift+`;` aims
/// the Hanji/romanization commit at it (`CandidateSlotKeySet::shifted_slot_for_event`)
/// even though the layout types `:` for it. Layout-dependent by Microsoft's
/// own word, the trade the US-position number row already makes.
pub const SEMICOLON_KEY_CODE: u16 = 0xBA;

/// The AppKit private-use range the Mac spells its arrow and function keys
/// in. Windows and Linux reserve all of it: the Windows recorder spells
/// every key that types nothing — the arrows, Insert, Home, End, the paging
/// keys, F1–F24 — as one of these scalars so it is refused as reserved
/// (`taigi-windows-platform` `key_translation::named_key_scalar`).
const FUNCTION_KEY_RANGE: std::ops::RangeInclusive<u32> = 0xF700..=0xF8FF;

/// What the Mac reserves, as whole keys: the six navigation keys (←, →, ↑,
/// ↓, Page Up, Page Down), Backspace, Delete and Escape. Every other
/// function key binds — ⌃Home is a chord there.
const MAC_NEVER_BINDABLE: [&str; 9] = [
    "\u{F702}",
    "\u{F703}",
    "\u{F700}",
    "\u{F701}",
    "\u{F72C}",
    "\u{F72D}",
    EDITING_KEYS[0],
    EDITING_KEYS[1],
    EDITING_KEYS[2],
];

/// The Mac's Home / End (`NSHomeFunctionKey` / `NSEndFunctionKey`): reserved
/// bare, because a composition reads them as its start / end
/// (`ComposingKeyIntent` tier 0); ⌃Home still binds. Windows and Linux
/// reserve them whole through [`FUNCTION_KEY_RANGE`].
const MAC_LINE_EDGE_KEYS: [&str; 2] = ["\u{F729}", "\u{F72B}"];

/// Backspace, Delete and Escape — reserved on every desktop.
const EDITING_KEYS: [&str; 3] = ["\u{8}", "\u{7F}", "\u{1B}"];

impl ComposingKeyChord {
    /// The chord `key` and `modifiers` name, or why it cannot be one.
    pub fn make(
        raw_key: Option<&str>,
        raw_modifiers: KeyModifiers,
        platform: DesktopPlatform,
    ) -> Result<Self, ChordRejection> {
        let raw_key = raw_key
            .filter(|key| !key.is_empty())
            .ok_or(ChordRejection::NoKey)?;
        let key = Self::normalized(raw_key, platform);
        if Self::is_never_bindable(&key, platform) {
            return Err(ChordRejection::ReservedKey);
        }
        // Only the four chording modifiers are part of a chord; the snapshot
        // already dropped the rest.
        let modifiers = raw_modifiers;
        if platform == DesktopPlatform::MacOS
            && modifiers.is_empty()
            && MAC_LINE_EDGE_KEYS.contains(&key.as_str())
        {
            return Err(ChordRejection::ReservedKey);
        }
        // Shift alone does not make a chord out of a typing key: Shift+A is
        // still the letter A, and binding it would cost the user their capitals.
        let first = key.chars().next().ok_or(ChordRejection::NoKey)?;
        if !modifiers.has_host_chord() && Self::is_typing_key(first) {
            return Err(ChordRejection::TypesRomanization);
        }
        Ok(Self { key, modifiers })
    }

    /// The chord this event would record, or why it cannot be recorded.
    #[cfg(test)]
    fn make_from_event(
        event: &KeyEventSnapshot,
        platform: DesktopPlatform,
    ) -> Result<Self, ChordRejection> {
        Self::make_from_press(
            event.unmodified_characters(),
            event.modifiers,
            event.key_code,
            platform,
        )
    }

    /// `make` with the key code beside the characters — the one question the
    /// recorder (`RecordedPress`) and a live event both have to ask, so the
    /// shifted number row is refused on both.
    ///
    /// A shifted number-row key is refused as the digit it is: Shift alone
    /// does not make a chord out of a typing key (`make`), and Shift+3 is the
    /// `3` key even though a US layout types `#` for it. Read off the key
    /// code, because that is the one thing Shift does not rewrite — and it is
    /// what keeps this path and the settings-file path (`translate_raw`,
    /// which is handed the unmodified `3`) refusing the same press. Every
    /// other shifted key records as the character it types, which is what its
    /// stored chords already hold.
    pub fn make_from_press(
        key: Option<&str>,
        modifiers: KeyModifiers,
        key_code: Option<u16>,
        platform: DesktopPlatform,
    ) -> Result<Self, ChordRejection> {
        if modifiers == KeyModifiers::SHIFT
            && key_code.is_some_and(|code| {
                NUMBER_ROW_KEY_CODES.contains(&code) || code == SEMICOLON_KEY_CODE
            })
        {
            return Err(ChordRejection::TypesRomanization);
        }
        Self::make(key, modifiers, platform)
    }

    /// Whether `event` is this chord. Compared on the unmodified characters
    /// for the same reason they are stored: Control rewrites the digits it is
    /// held with, and Alt rewrites much of the keyboard.
    pub fn matches(&self, event: &KeyEventSnapshot, platform: DesktopPlatform) -> bool {
        let Some(characters) = event.unmodified_characters() else {
            return false;
        };
        Self::normalized(characters, platform) == self.key && event.modifiers == self.modifiers
    }

    /// The form a key is stored and compared in: lowercased — ASCII only on
    /// Windows and Linux, the whole of Unicode on the Mac
    /// (`str::to_lowercase`), so ⌃⇧Ñ stores `ñ`
    /// there and `Ñ` here; the keypad Enter (`\u{3}`) and the back tab
    /// (`\u{19}`) folded onto Return and Tab — the Mac's spellings, folded on
    /// every desktop even though the Windows and Linux shells never produce
    /// either.
    fn normalized(key: &str, platform: DesktopPlatform) -> String {
        match key {
            "\u{3}" => "\r".to_owned(),
            "\u{19}" => "\t".to_owned(),
            other => match platform {
                DesktopPlatform::Windows | DesktopPlatform::Linux => other.to_ascii_lowercase(),
                DesktopPlatform::MacOS => other.to_lowercase(),
            },
        }
    }

    /// Backspace, Delete, Escape and the navigation keys, whatever modifiers
    /// are held: on Windows and Linux any key starting with a private-use
    /// function-key scalar ([`FUNCTION_KEY_RANGE`]), on the Mac exactly
    /// [`MAC_NEVER_BINDABLE`].
    fn is_never_bindable(key: &str, platform: DesktopPlatform) -> bool {
        match platform {
            DesktopPlatform::Windows | DesktopPlatform::Linux => {
                EDITING_KEYS.contains(&key)
                    || key
                        .chars()
                        .next()
                        .is_some_and(|c| FUNCTION_KEY_RANGE.contains(&(c as u32)))
            }
            DesktopPlatform::MacOS => MAC_NEVER_BINDABLE.contains(&key),
        }
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
    ///
    /// Read off the first SCALAR, so a letter carrying a combining mark (`İ`
    /// folds to `i̇`) is still the letter it types — E6 in
    /// `macos-desktop-core-roadmap.md`, settled P11b.
    /// CROSS-PLATFORM INVARIANT — one gate for macOS, Windows and Linux, owned here.
    fn is_typing_key(character: char) -> bool {
        character.is_ascii_alphabetic()
            || ComposingKeyIntent::is_tone_digit(character)
            || character == '-'
            || character == ';'
    }

    /// `"<modifiers>|<scalars>"` — modifier letters in a fixed order
    /// ([`Self::modifier_letters`]), then the key's scalars in hex. Hex
    /// rather than the character: most bound keys are control characters,
    /// and a settings file holding a raw `\r` is one editor away from
    /// unreadable.
    ///
    /// A raw value is platform-LOCAL — the modifiers it names exist on one
    /// keyboard — so a future settings transfer must map `d↔w`, `o↔a` itself
    /// (or, more honestly, carry only the setting names and let each platform
    /// keep its own chords).
    pub fn raw_value(&self, platform: DesktopPlatform) -> String {
        let [win, control, alt, shift] = Self::modifier_letters(platform);
        let letters: String = [
            (self.modifiers.win, win),
            (self.modifiers.control, control),
            (self.modifiers.alt, alt),
            (self.modifiers.shift, shift),
        ]
        .into_iter()
        .filter_map(|(held, letter)| held.then_some(letter))
        .collect();
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
    /// NOT the recorder's whole gate: the global tier's rules
    /// (`global_rejection`) live in `evaluate_press`,
    /// which a value read from `settings.json` does not pass through. A
    /// hand-edited file can therefore hold a chord the recorder would have
    /// refused; only the UI is gated.
    pub fn from_raw(raw: &str, platform: DesktopPlatform) -> Option<Self> {
        Self::translate_raw(raw, platform).and_then(Result::ok)
    }

    /// [`Self::from_raw`] with the gate's refusal kept: `None` for a value
    /// the grammar cannot read at all, `Some(Err)` for a well-formed value
    /// naming a chord the gate refuses. The launch pass reads WHY a stored
    /// global row fails to translate, because a row on a typing key is one
    /// the recorder would refuse today and the preserved key would still be
    /// dispatched first. Kept to the key contract:
    /// `ShortcutAction::translation_in` is its only caller. macOS keeps a
    /// Swift twin for its global tier: `ShortcutActions.swift` `translation(of:)`.
    ///
    /// An empty hex field (`c|0041,,0042`) refuses the whole value on every
    /// desktop — E7 in `macos-desktop-core-roadmap.md`, settled P11a.
    pub(super) fn translate_raw(
        raw: &str,
        platform: DesktopPlatform,
    ) -> Option<Result<Self, ChordRejection>> {
        let (letters, scalars) = raw.split_once('|')?;
        let [win, control, alt, shift] = Self::modifier_letters(platform);
        let mut modifiers = KeyModifiers::NONE;
        for letter in letters.chars() {
            match letter {
                _ if letter == win => modifiers.win = true,
                _ if letter == control => modifiers.control = true,
                _ if letter == alt => modifiers.alt = true,
                _ if letter == shift => modifiers.shift = true,
                _ => return None,
            }
        }
        let mut key = String::new();
        for field in scalars.split(',') {
            let value = u32::from_str_radix(field, 16).ok()?;
            key.push(char::from_u32(value)?);
        }
        Some(Self::make(Some(&key), modifiers, platform))
    }

    /// The letters a raw value spells `win`, `control`, `alt`, `shift` with,
    /// in that order: `w c a s` on Windows and Linux, the Mac's `d` command /
    /// `c` control / `o` option / `s` shift.
    fn modifier_letters(platform: DesktopPlatform) -> [char; 4] {
        match platform {
            DesktopPlatform::Windows | DesktopPlatform::Linux => ['w', 'c', 'a', 's'],
            DesktopPlatform::MacOS => ['d', 'c', 'o', 's'],
        }
    }

    /// The modifier names a chord label leads with, in the order the system
    /// prints them: `Win`, `Ctrl`, `Alt`, `Shift` on Windows and Linux; the
    /// Mac's `⌃⌥⇧⌘` (the order of KeyboardShortcuts'
    /// `ks_symbolicRepresentation`).
    fn modifier_labels(
        modifiers: KeyModifiers,
        platform: DesktopPlatform,
    ) -> impl Iterator<Item = String> {
        let labels = match platform {
            DesktopPlatform::Windows | DesktopPlatform::Linux => [
                (modifiers.win, "Win"),
                (modifiers.control, "Ctrl"),
                (modifiers.alt, "Alt"),
                (modifiers.shift, "Shift"),
            ],
            DesktopPlatform::MacOS => [
                (modifiers.control, "⌃"),
                (modifiers.alt, "⌥"),
                (modifiers.shift, "⇧"),
                (modifiers.win, "⌘"),
            ],
        };
        labels
            .into_iter()
            .filter(|(held, _)| *held)
            .map(|(_, name)| name.to_owned())
    }

    /// What joins a label's modifiers and key: `Ctrl+]` on Windows and Linux,
    /// `⌃]` on the Mac.
    fn label_separator(platform: DesktopPlatform) -> &'static str {
        match platform {
            DesktopPlatform::Windows | DesktopPlatform::Linux => "+",
            DesktopPlatform::MacOS => "",
        }
    }

    /// The chord as a keycap label: `Shift+Enter`, `Ctrl+]`, `Space` on
    /// Windows and Linux; `⇧↩`, `⌃]`, `Space` on the Mac.
    pub fn display(&self, platform: DesktopPlatform) -> String {
        let is_mac = platform == DesktopPlatform::MacOS;
        // A chord WITH modifiers keeps the uppercase keycap legend (`Ctrl+J`);
        // a bare key shows the character it types — an uppercase `Z` on a
        // modifier-less row reads as Shift+Z, a key the row does not hold
        // (USER 2026-08-22).
        let keycap = match self.key.as_str() {
            " " => "Space".to_owned(),
            "\r" if is_mac => "↩".to_owned(),
            "\t" if is_mac => "⇥".to_owned(),
            "\r" => "Enter".to_owned(),
            "\t" => "Tab".to_owned(),
            other if self.modifiers.is_empty() => other.to_owned(),
            other if is_mac => other.to_uppercase(),
            other => other.to_ascii_uppercase(),
        };
        Self::modifier_labels(self.modifiers, platform)
            .chain([keycap])
            .collect::<Vec<_>>()
            .join(Self::label_separator(platform))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keys::CandidateSlotKeySet;

    use crate::platform::test_support::{ALL_PLATFORMS, WINDOWS_AND_LINUX};

    const ALL_MODIFIERS: KeyModifiers = KeyModifiers::WIN
        .with(KeyModifiers::CONTROL)
        .with(KeyModifiers::ALT)
        .with(KeyModifiers::SHIFT);
    const MAC: DesktopPlatform = DesktopPlatform::MacOS;

    fn chord_on(
        key: &str,
        modifiers: KeyModifiers,
        platform: DesktopPlatform,
    ) -> ComposingKeyChord {
        ComposingKeyChord::make(Some(key), modifiers, platform).expect("bindable")
    }

    #[test]
    fn typing_keys_cannot_be_recorded_bare() {
        for platform in ALL_PLATFORMS {
            // trace: `is_typing_key` — every ASCII letter (both
            // schemes' keys), a capital, the digits, the hyphen and `;`.
            for key in ('a'..='z')
                .map(String::from)
                .chain(["A", "V", "5", "0", "-", ";"].map(String::from))
            {
                assert_eq!(
                    ComposingKeyChord::make(Some(&key), KeyModifiers::NONE, platform),
                    Err(ChordRejection::TypesRomanization),
                    "{key}"
                );
            }
            // The Telex keys and the bare slot row are typing keys under one
            // rule, so a chord recorded under either scheme stays live.
            for key in dispatch::TELEX_KEYS
                .chars()
                .map(String::from)
                .chain(CandidateSlotKeySet::BARE_KEY_ROW.map(String::from))
            {
                assert_eq!(
                    ComposingKeyChord::make(Some(&key), KeyModifiers::NONE, platform),
                    Err(ChordRejection::TypesRomanization),
                    "{key}"
                );
            }
        }
    }

    #[test]
    fn punctuation_can_be_recorded_bare_and_capitals_fold() {
        for platform in ALL_PLATFORMS {
            for key in [",", ".", "'", "/", "[", "]", "`"] {
                let chord = chord_on(key, KeyModifiers::NONE, platform);
                assert_eq!(chord.key, key);
            }
            let shifted = chord_on("Z", KeyModifiers::CONTROL, platform);
            assert_eq!(shifted.key, "z");
            assert_eq!(shifted.modifiers, KeyModifiers::CONTROL);
            assert_eq!(
                ComposingKeyChord::make(Some("Z"), KeyModifiers::SHIFT, platform),
                Err(ChordRejection::TypesRomanization),
                "Shift alone does not make a chord out of a letter"
            );
        }
    }

    #[test]
    fn bare_key_and_its_shifted_twin_do_not_cross_match() {
        for platform in ALL_PLATFORMS {
            let bare = chord_on("[", KeyModifiers::NONE, platform);
            let shifted = chord_on("{", KeyModifiers::SHIFT, platform);
            let bare_event = KeyEventSnapshot::text("[", KeyModifiers::NONE);
            let shifted_event = KeyEventSnapshot::chord(Some("{"), "{", KeyModifiers::SHIFT);
            assert!(bare.matches(&bare_event, platform));
            assert!(!bare.matches(&shifted_event, platform));
            assert!(shifted.matches(&shifted_event, platform));
            assert!(!shifted.matches(&bare_event, platform));
        }
    }

    #[test]
    fn typing_keys_bind_with_a_host_modifier_but_not_shift_alone() {
        for platform in ALL_PLATFORMS {
            for key in ["a", "v", "z", "3", ";"] {
                for modifiers in [KeyModifiers::CONTROL, KeyModifiers::ALT, KeyModifiers::WIN] {
                    assert!(
                        ComposingKeyChord::make(Some(key), modifiers, platform).is_ok(),
                        "{key} {modifiers:?}"
                    );
                }
                assert_eq!(
                    ComposingKeyChord::make(Some(key), KeyModifiers::SHIFT, platform),
                    Err(ChordRejection::TypesRomanization),
                    "{key}"
                );
            }
        }
    }

    #[test]
    fn reserved_keys_cannot_be_recorded_at_all() {
        for platform in ALL_PLATFORMS {
            for key in ["\u{1B}", "\u{8}", "\u{7F}", "\u{F702}"] {
                for modifiers in [
                    KeyModifiers::NONE,
                    KeyModifiers::CONTROL,
                    KeyModifiers::WIN.with(KeyModifiers::SHIFT),
                ] {
                    assert_eq!(
                        ComposingKeyChord::make(Some(key), modifiers, platform),
                        Err(ChordRejection::ReservedKey),
                        "{key:?}"
                    );
                }
            }
            assert_eq!(
                ComposingKeyChord::make(None, KeyModifiers::NONE, platform),
                Err(ChordRejection::NoKey)
            );
            assert_eq!(
                ComposingKeyChord::make(Some(""), KeyModifiers::NONE, platform),
                Err(ChordRejection::NoKey)
            );
        }
    }

    #[test]
    fn raw_values_round_trip_and_stay_stable() {
        for platform in WINDOWS_AND_LINUX {
            // trace: `raw_value` with the Windows and Linux modifier letters
            // (w/c/a/s).
            for (key, modifiers) in [
                (" ", KeyModifiers::NONE),
                ("\r", KeyModifiers::SHIFT),
                ("[", KeyModifiers::NONE),
                ("z", KeyModifiers::CONTROL),
                ("]", ALL_MODIFIERS),
            ] {
                let chord = chord_on(key, modifiers, platform);
                assert_eq!(
                    ComposingKeyChord::from_raw(&chord.raw_value(platform), platform),
                    Some(chord)
                );
            }
            assert_eq!(
                chord_on(" ", KeyModifiers::NONE, platform).raw_value(platform),
                "|0020"
            );
            assert_eq!(
                chord_on("\r", KeyModifiers::SHIFT, platform).raw_value(platform),
                "s|000D"
            );
            assert_eq!(
                chord_on("]", KeyModifiers::CONTROL.with(KeyModifiers::ALT), platform)
                    .raw_value(platform),
                "ca|005D"
            );
        }
    }

    #[test]
    fn raw_values_that_would_take_a_typing_key_do_not_parse() {
        for platform in WINDOWS_AND_LINUX {
            assert_eq!(
                ComposingKeyChord::from_raw("|0061", platform),
                None,
                "bare a"
            );
            assert_eq!(
                ComposingKeyChord::from_raw("|007A", platform),
                None,
                "bare z"
            );
            assert_eq!(
                ComposingKeyChord::from_raw("s|0035", platform),
                None,
                "Shift+5"
            );
            assert_eq!(
                ComposingKeyChord::from_raw("c|F702", platform),
                None,
                "Ctrl+← is still an arrow"
            );
            assert_eq!(ComposingKeyChord::from_raw("garbage", platform), None);
            assert_eq!(
                ComposingKeyChord::from_raw("x|0020", platform),
                None,
                "unknown modifier letter"
            );
            // The launch pass reads the refusal apart from a value the grammar
            // cannot read.
            assert_eq!(
                ComposingKeyChord::translate_raw("|007A", platform),
                Some(Err(ChordRejection::TypesRomanization))
            );
            assert_eq!(
                ComposingKeyChord::translate_raw("s|0033", platform),
                Some(Err(ChordRejection::TypesRomanization)),
                "Shift+3 stored as the 3 it is"
            );
            assert_eq!(ComposingKeyChord::translate_raw("garbage", platform), None);
            assert_eq!(
                ComposingKeyChord::translate_raw("c|007A", platform),
                Some(Ok(chord_on("z", KeyModifiers::CONTROL, platform)))
            );
        }
    }

    #[test]
    fn shifted_number_row_key_is_refused_from_an_event_and_from_the_raw_digit() {
        for platform in ALL_PLATFORMS {
            // The key-code path: Shift+3 types `#`, so the characters alone would
            // slip past the digit rule — `make_from_event` reads the row.
            let shift_three =
                KeyEventSnapshot::chord(Some("#"), "#", KeyModifiers::SHIFT).with_key_code(0x33);
            assert_eq!(
                ComposingKeyChord::make_from_event(&shift_three, platform),
                Err(ChordRejection::TypesRomanization)
            );
            // The raw path, handed the unmodified `3` with Shift held.
            assert_eq!(
                ComposingKeyChord::make(Some("3"), KeyModifiers::SHIFT, platform),
                Err(ChordRejection::TypesRomanization)
            );
            let keypad_three = KeyEventSnapshot::chord(Some("3"), "3", KeyModifiers::SHIFT);
            assert_eq!(
                ComposingKeyChord::make_from_event(&keypad_three, platform),
                Err(ChordRejection::TypesRomanization)
            );
            // A `#` reached without the number row (a layout with a `#` key)
            // still records as `#`.
            let hash_key = KeyEventSnapshot::chord(Some("#"), "#", KeyModifiers::SHIFT);
            assert_eq!(
                ComposingKeyChord::make_from_event(&hash_key, platform),
                Ok(chord_on("#", KeyModifiers::SHIFT, platform))
            );
            // Shift plus a host modifier on the number row is an ordinary chord.
            let ctrl_shift_three = KeyEventSnapshot::chord(
                Some("#"),
                "3",
                KeyModifiers::CONTROL.with(KeyModifiers::SHIFT),
            )
            .with_key_code(0x33);
            assert_eq!(
                ComposingKeyChord::make_from_event(&ctrl_shift_three, platform),
                Ok(chord_on(
                    "3",
                    KeyModifiers::CONTROL.with(KeyModifiers::SHIFT),
                    platform
                ))
            );
            let ctrl_three = KeyEventSnapshot::chord(Some("\u{1B}"), "3", KeyModifiers::CONTROL);
            assert!(
                ComposingKeyChord::make_from_event(&ctrl_three, platform).is_ok(),
                "Ctrl+3 is an ordinary chord to record"
            );
            // Shift+`;` is the ninth slot key's Hanji/romanization chord, refused by its key
            // code the same way; a `:` reached without that key still records.
            let shift_semicolon = KeyEventSnapshot::chord(Some(":"), ":", KeyModifiers::SHIFT)
                .with_key_code(SEMICOLON_KEY_CODE);
            assert_eq!(
                ComposingKeyChord::make_from_event(&shift_semicolon, platform),
                Err(ChordRejection::TypesRomanization)
            );
            let colon_key = KeyEventSnapshot::chord(Some(":"), ":", KeyModifiers::SHIFT);
            assert_eq!(
                ComposingKeyChord::make_from_event(&colon_key, platform),
                Ok(chord_on(":", KeyModifiers::SHIFT, platform))
            );
        }
    }

    #[test]
    fn keypad_enter_matches_a_return_chord() {
        for platform in WINDOWS_AND_LINUX {
            let enter = chord_on("\r", KeyModifiers::NONE, platform);
            assert!(
                enter.matches(
                    &KeyEventSnapshot::text("\u{3}", KeyModifiers::NONE),
                    platform
                ),
                "keypad Enter is Return"
            );
            assert!(!enter.matches(&KeyEventSnapshot::text("\r", KeyModifiers::SHIFT), platform));
        }
    }

    #[test]
    fn keypad_enter_and_back_tab_fold_onto_their_key() {
        for platform in WINDOWS_AND_LINUX {
            assert_eq!(chord_on("\u{3}", KeyModifiers::NONE, platform).key, "\r");
            assert_eq!(chord_on("\u{19}", KeyModifiers::SHIFT, platform).key, "\t");
            assert_eq!(
                chord_on("\r", KeyModifiers::SHIFT, platform).display(platform),
                "Shift+Enter"
            );
            assert_eq!(
                chord_on("]", KeyModifiers::CONTROL, platform).display(platform),
                "Ctrl+]"
            );
            assert_eq!(
                chord_on(" ", KeyModifiers::NONE, platform).display(platform),
                "Space"
            );
            assert_eq!(
                chord_on("[", KeyModifiers::NONE, platform).display(platform),
                "["
            );
            assert_eq!(
                chord_on("z", KeyModifiers::CONTROL, platform).display(platform),
                "Ctrl+Z"
            );
        }
    }

    #[test]
    fn windows_and_linux_reserve_the_whole_function_key_range_by_its_first_scalar() {
        // trace: FUNCTION_KEY_RANGE = F700..=F8FF on the first scalar (K4) —
        // the range's two ends, a key past each end, and a multi-scalar
        // string that starts inside it.
        for platform in WINDOWS_AND_LINUX {
            for key in ["\u{F700}", "\u{F729}", "\u{F8FF}", "\u{F729}x"] {
                assert_eq!(
                    ComposingKeyChord::make(Some(key), KeyModifiers::CONTROL, platform),
                    Err(ChordRejection::ReservedKey),
                    "{key:?} {platform:?}"
                );
            }
            for key in ["\u{F6FF}", "\u{F900}"] {
                assert!(
                    ComposingKeyChord::make(Some(key), KeyModifiers::CONTROL, platform).is_ok(),
                    "{key:?} {platform:?}"
                );
            }
        }
    }

    #[test]
    fn windows_and_linux_fold_ascii_only() {
        // trace: `to_ascii_lowercase` leaves Ñ alone (K6); `to_ascii_uppercase`
        // leaves ñ alone in the label.
        for platform in WINDOWS_AND_LINUX {
            assert_eq!(chord_on("Ñ", KeyModifiers::CONTROL, platform).key, "Ñ");
            assert_eq!(
                chord_on("ñ", KeyModifiers::CONTROL, platform).display(platform),
                "Ctrl+ñ"
            );
            let event = KeyEventSnapshot::text("ñ", KeyModifiers::CONTROL);
            assert!(!chord_on("Ñ", KeyModifiers::CONTROL, platform).matches(&event, platform));
        }
    }

    #[test]
    fn each_desktop_reads_only_its_own_modifier_letters() {
        // trace: w/c/a/s on Windows and Linux, d/c/o/s on the Mac
        // (`modifier_letters`); `c` and `s` are shared.
        for platform in WINDOWS_AND_LINUX {
            assert_eq!(ComposingKeyChord::from_raw("d|005D", platform), None);
            assert_eq!(ComposingKeyChord::from_raw("o|005D", platform), None);
        }
        assert_eq!(ComposingKeyChord::from_raw("w|005D", MAC), None);
        assert_eq!(ComposingKeyChord::from_raw("a|005D", MAC), None);
        assert_eq!(
            ComposingKeyChord::from_raw("cs|005D", MAC),
            ComposingKeyChord::from_raw("cs|005D", DesktopPlatform::Windows),
        );
    }

    #[test]
    fn the_mac_spells_command_d_and_option_o() {
        // trace: `raw_value` / `modifier_letters` — `d` command (win), `c` control, `o` option
        // (alt), `s` shift, in that order, then `%04X` scalars joined by `,`.
        assert_eq!(
            chord_on("]", ALL_MODIFIERS, MAC).raw_value(MAC),
            "dcos|005D"
        );
        assert_eq!(
            chord_on("\r", KeyModifiers::ALT, MAC).raw_value(MAC),
            "o|000D"
        );
        assert_eq!(
            ComposingKeyChord::from_raw("o|000D", MAC),
            Some(chord_on("\r", KeyModifiers::ALT, MAC)),
            "the stored ⌥↩ of inventory K2"
        );
        // trace: `normalized` + `is_typing_key` — `Z` under ⇧⌃ is
        // stored folded, `` ` `` is a chord bare and under ⇧ (not a typing key).
        for (key, modifiers) in [
            (" ", KeyModifiers::NONE),
            ("\r", KeyModifiers::SHIFT),
            ("`", KeyModifiers::NONE),
            ("`", KeyModifiers::SHIFT),
            ("Z", KeyModifiers::SHIFT.with(KeyModifiers::CONTROL)),
            ("z", KeyModifiers::WIN),
            ("]", ALL_MODIFIERS),
        ] {
            let chord = chord_on(key, modifiers, MAC);
            assert_eq!(
                ComposingKeyChord::from_raw(&chord.raw_value(MAC), MAC),
                Some(chord)
            );
        }
        // trace: `raw_value` — the Mac's stored spellings.
        for (key, modifiers, raw) in [
            (" ", KeyModifiers::NONE, "|0020"),
            ("\r", KeyModifiers::SHIFT, "s|000D"),
            (
                "]",
                KeyModifiers::CONTROL.with(KeyModifiers::ALT),
                "co|005D",
            ),
        ] {
            assert_eq!(chord_on(key, modifiers, MAC).raw_value(MAC), raw);
        }
        // trace: `translate_raw` through `make` — bare `a`, ⇧5,
        // ⌃← (F702 is one of the Mac's nine), garbage, an unknown letter.
        for raw in ["|0061", "|007A", "s|0035", "c|F702", "garbage", "x|0020"] {
            assert_eq!(ComposingKeyChord::from_raw(raw, MAC), None, "{raw}");
        }
        // trace: `normalized` folds `\u{19}` → `\t` — a back tab stored
        // before the fold reads back as ⇧⇥.
        assert_eq!(
            ComposingKeyChord::from_raw("s|0019", MAC),
            Some(chord_on("\t", KeyModifiers::SHIFT, MAC))
        );
    }

    #[test]
    fn the_mac_matches_exact_modifiers_on_the_unmodified_key() {
        // trace: `matches` compares the unmodified characters and the exact
        // modifiers — ⌥↩ matches only ⌥↩; ⌥J arrives as `∆` and matches
        // through the unmodified `j`.
        let option_return = chord_on("\r", KeyModifiers::ALT, MAC);
        assert!(option_return.matches(&KeyEventSnapshot::text("\r", KeyModifiers::ALT), MAC));
        assert!(!option_return.matches(&KeyEventSnapshot::text("\r", KeyModifiers::NONE), MAC));
        assert!(!option_return.matches(
            &KeyEventSnapshot::text("\r", KeyModifiers::ALT.with(KeyModifiers::SHIFT)),
            MAC
        ));
        let option_j = chord_on("j", KeyModifiers::ALT, MAC);
        assert!(option_j.matches(
            &KeyEventSnapshot::chord(Some("∆"), "j", KeyModifiers::ALT),
            MAC
        ));
        // The keypad Enter matches a Return chord and AppKit's back tab a ⇧⇥
        // one (the folds themselves: `the_mac_folds_the_whole_of_unicode`).
        assert!(chord_on("\r", KeyModifiers::NONE, MAC)
            .matches(&KeyEventSnapshot::text("\u{3}", KeyModifiers::NONE), MAC));
        let back_tab = chord_on("\t", KeyModifiers::SHIFT, MAC);
        assert!(back_tab.matches(&KeyEventSnapshot::text("\u{19}", KeyModifiers::SHIFT), MAC));
        assert_eq!(back_tab.display(MAC), "⇧⇥");
    }

    /// Roadmap E6, settled P11b: the gate asks the first scalar of the fold
    /// (`İ` → `i̇`, first scalar `i`) and refuses it bare.
    #[test]
    fn e6_a_fold_that_adds_a_combining_mark_is_a_typing_key_on_the_mac() {
        // trace: str::to_lowercase("İ") = "i\u{307}" (SpecialCasing), first
        // char 'i' is ASCII alphabetic → TypesRomanization unless a host
        // modifier is held.
        for modifiers in [KeyModifiers::NONE, KeyModifiers::SHIFT] {
            assert_eq!(
                ComposingKeyChord::make(Some("İ"), modifiers, MAC),
                Err(ChordRejection::TypesRomanization),
                "{modifiers:?}"
            );
        }
        assert_eq!(chord_on("İ", KeyModifiers::CONTROL, MAC).key, "i\u{307}");
        assert_eq!(
            ComposingKeyChord::translate_raw("s|0069,0307", MAC),
            Some(Err(ChordRejection::TypesRomanization)),
            "a ⇧İ row recorded before P11b"
        );
        // Negative controls: `Ñ` folds to a
        // non-ASCII `ñ` and binds bare; the Kelvin sign folds to an ASCII
        // `k` and is refused.
        assert_eq!(chord_on("Ñ", KeyModifiers::NONE, MAC).key, "ñ");
        assert_eq!(
            ComposingKeyChord::make(Some("\u{212A}"), KeyModifiers::NONE, MAC),
            Err(ChordRejection::TypesRomanization)
        );
    }

    /// Roadmap E7, settled P11a: a hand-edited value with an empty hex field
    /// is refused whole, on every desktop.
    #[test]
    fn e7_a_stored_chord_with_an_empty_hex_field_does_not_parse() {
        for platform in ALL_PLATFORMS {
            // trace: `"".split(',')` yields one empty field and
            // `u32::from_str_radix("", 16)` is an error → `None`.
            for raw in ["c|0041,,0042", "c|,005D", "c|005D,", "c|"] {
                assert_eq!(
                    ComposingKeyChord::translate_raw(raw, platform),
                    None,
                    "{raw} {platform:?}"
                );
            }
            assert_eq!(
                ComposingKeyChord::translate_raw("c|005D", platform),
                Some(Ok(chord_on("]", KeyModifiers::CONTROL, platform))),
                "negative control"
            );
        }
    }

    #[test]
    fn the_mac_reserves_nine_whole_keys_and_binds_every_other_function_key() {
        // trace: `MAC_NEVER_BINDABLE` = the six navigation keys
        // (F702 F703 F700 F701 F72C F72D) + Backspace, Delete, Escape —
        // matched as whole strings, so ⌃Home (F729), F1 (F704) and a string
        // that only starts with an arrow scalar bind.
        for key in MAC_NEVER_BINDABLE {
            for modifiers in [KeyModifiers::NONE, KeyModifiers::CONTROL] {
                assert_eq!(
                    ComposingKeyChord::make(Some(key), modifiers, MAC),
                    Err(ChordRejection::ReservedKey),
                    "{key:?}"
                );
            }
        }
        // Bare Home / End are the composition's start / end, so reserved;
        // Shift+Home still records.
        for key in MAC_LINE_EDGE_KEYS {
            assert_eq!(
                ComposingKeyChord::make(Some(key), KeyModifiers::NONE, MAC),
                Err(ChordRejection::ReservedKey),
                "{key:?}"
            );
            assert!(ComposingKeyChord::make(Some(key), KeyModifiers::SHIFT, MAC).is_ok());
        }
        // ⌃Home, ⌃End, ⌃Help — the scalars the macOS bridge hands the gate
        // (`CrossTierShortcutConflictTests.swift` `testHomeEndAndHelp_*`).
        for key in ["\u{F729}", "\u{F72B}", "\u{F746}"] {
            assert!(
                ComposingKeyChord::make(Some(key), KeyModifiers::CONTROL, MAC).is_ok(),
                "{key:?}"
            );
        }
        assert!(ComposingKeyChord::make(Some("\u{F704}"), KeyModifiers::NONE, MAC).is_ok());
        assert!(ComposingKeyChord::make(Some("\u{F702}x"), KeyModifiers::CONTROL, MAC).is_ok());
    }

    #[test]
    fn the_mac_folds_the_whole_of_unicode() {
        // trace: `str::to_lowercase` in `normalized`, `str::to_uppercase` in
        // `display` — ⌃⇧Ñ stores `ñ` (inventory K6), matches an `Ñ` event,
        // and labels as `⌃Ñ`.
        let chord = chord_on("Ñ", KeyModifiers::CONTROL, MAC);
        assert_eq!(chord.key, "ñ");
        assert!(chord.matches(&KeyEventSnapshot::text("Ñ", KeyModifiers::CONTROL), MAC));
        assert_eq!(chord.display(MAC), "⌃Ñ");
        assert_eq!(chord_on("\u{3}", KeyModifiers::NONE, MAC).key, "\r");
        assert_eq!(chord_on("\u{19}", KeyModifiers::SHIFT, MAC).key, "\t");
        assert_eq!(
            ComposingKeyChord::make(Some("Z"), KeyModifiers::SHIFT, MAC),
            Err(ChordRejection::TypesRomanization)
        );
    }

    #[test]
    fn the_mac_labels_in_glyphs_with_no_separator() {
        // trace: `display` = `modifier_labels` (⌃ ⌥ ⇧ ⌘, in that order) +
        // keycap, no separator; Space / ↩ / ⇥ by name, a bare key as typed,
        // uppercased under a modifier.
        for (key, modifiers, label) in [
            ("j", ALL_MODIFIERS, "⌃⌥⇧⌘J"),
            ("\t", KeyModifiers::NONE, "⇥"),
            ("[", KeyModifiers::NONE, "["),
            ("z", KeyModifiers::ALT, "⌥Z"),
            (" ", KeyModifiers::NONE, "Space"),
            ("\r", KeyModifiers::NONE, "↩"),
            ("\r", KeyModifiers::SHIFT, "⇧↩"),
            ("]", KeyModifiers::NONE, "]"),
            ("j", KeyModifiers::CONTROL.with(KeyModifiers::ALT), "⌃⌥J"),
            ("`", KeyModifiers::NONE, "`"),
            ("Z", KeyModifiers::SHIFT.with(KeyModifiers::CONTROL), "⌃⇧Z"),
        ] {
            assert_eq!(chord_on(key, modifiers, MAC).display(MAC), label, "{key:?}");
        }
    }

    #[test]
    fn windows_and_linux_label_every_modifier_by_name() {
        for platform in WINDOWS_AND_LINUX {
            assert_eq!(
                chord_on("j", ALL_MODIFIERS, platform).display(platform),
                "Win+Ctrl+Alt+Shift+J"
            );
            assert_eq!(
                chord_on("\t", KeyModifiers::NONE, platform).display(platform),
                "Tab"
            );
        }
    }
}
