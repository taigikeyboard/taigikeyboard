//! A browser `KeyboardEvent` as the desktop core's [`KeyEventSnapshot`]: the
//! `taigi-linux-platform` `key_translation` shape, from `event.key` and
//! `event.code` rather than a keysym and a keycode.
//!
//! `event.key` already follows the OS layout, so it is what a key types. Three
//! things it cannot say are read off `event.code`, the position:
//! - `key_code`, in the Windows virtual-key space the slot and TPS tables are
//!   written in (`chord.rs`);
//! - what the Mac's Option rewrote, which chords need back (`⌃⌥S` reports
//!   `ß`);
//! - which Shift key this is, for the tap.
//!
//! Elsewhere a non-ASCII character under Alt is AltGr text: Windows reports
//! AltGr as Ctrl+Alt, and Polish AltGr+C must type `ć`, not fire Ctrl+Alt+C.

use serde::Deserialize;
use taigi_desktop_core::keys::{
    ComposingKeyChord, KeyEventSnapshot, KeyModifiers, LineEdgeKey, NavigationKey,
    LEFT_SHIFT_SCAN_CODE, RIGHT_SHIFT_SCAN_CODE, SEMICOLON_KEY_CODE,
};
use taigi_desktop_core::platform::DesktopPlatform;

/// The global tier in the core's defaults (`ShortcutAction::default_chord`).
const CORE_TIER: KeyModifiers = KeyModifiers::CONTROL.with(KeyModifiers::ALT);
/// The same tier in the Mac app, whose rows live in Swift
/// (`ShortcutActions.swift`).
const MAC_APP_TIER: KeyModifiers = KeyModifiers::CONTROL.with(KeyModifiers::WIN);

/// A Mac app chord as the core's table spells it, so ⌃⌘C does on the page what
/// it does in the app. `None` for any other press.
pub(crate) fn core_tier_snapshot(
    snapshot: &KeyEventSnapshot,
    platform: DesktopPlatform,
) -> Option<KeyEventSnapshot> {
    (platform == DesktopPlatform::MacOS && snapshot.modifiers == MAC_APP_TIER).then(|| {
        KeyEventSnapshot {
            modifiers: CORE_TIER,
            ..snapshot.clone()
        }
    })
}

/// A press of the core's tier on a Mac, which the app does not have.
pub(crate) fn is_mac_core_tier(snapshot: &KeyEventSnapshot, platform: DesktopPlatform) -> bool {
    platform == DesktopPlatform::MacOS && snapshot.modifiers == CORE_TIER
}

/// A global chord as the visitor presses it: ⌃⌘ on a Mac, as in the app.
pub(crate) fn visitor_chord(
    chord: ComposingKeyChord,
    platform: DesktopPlatform,
) -> ComposingKeyChord {
    if platform == DesktopPlatform::MacOS && chord.modifiers == CORE_TIER {
        ComposingKeyChord {
            modifiers: MAC_APP_TIER,
            ..chord
        }
    } else {
        chord
    }
}

/// The fields of a `KeyboardEvent` the page sends.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub(crate) struct BrowserKey {
    pub key: String,
    pub code: String,
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
    pub repeat: bool,
}

/// Which Shift key `code` is: 0 left, 1 right.
pub(crate) fn shift_index(code: &str) -> Option<usize> {
    match code {
        "ShiftLeft" => Some(0),
        "ShiftRight" => Some(1),
        _ => None,
    }
}

/// The scan code of the Shift key at `index`, as the tap tracker takes it.
pub(crate) const SHIFT_SCAN_CODES: [u32; 2] = [LEFT_SHIFT_SCAN_CODE, RIGHT_SHIFT_SCAN_CODE];

impl BrowserKey {
    /// The snapshot for a press, or `None` for a press that classifies
    /// nothing: a bare modifier, or a key another input method holds.
    pub(crate) fn snapshot(&self, platform: DesktopPlatform) -> Option<KeyEventSnapshot> {
        let is_dead = self.key == "Dead";
        let is_rewritten = self.alt && (is_dead || !self.key.is_ascii());
        let (modifiers, from_position) = match (is_rewritten, platform) {
            (true, DesktopPlatform::MacOS) => (self.modifiers(), us_character(&self.code)),
            // AltGr text: the two modifiers made the character, they are not
            // a chord.
            (true, _) => (
                KeyModifiers {
                    control: false,
                    alt: false,
                    ..self.modifiers()
                },
                None,
            ),
            (false, _) => (self.modifiers(), None),
        };
        let characters = self.characters()?;
        let navigation_key = navigation_key(&self.key);
        let line_edge_key = line_edge_key(&self.key);
        // A dead key is named so that it ends a composition before the
        // browser starts its own (`ComposingKeyIntent::host_key`).
        let is_named_special_key = is_dead || navigation_key.is_some() || is_named_key(&self.key);
        Some(KeyEventSnapshot {
            characters_ignoring_modifiers: from_position
                .map(str::to_owned)
                .or_else(|| characters.clone()),
            characters,
            key_code: self.key_code(),
            modifiers,
            is_named_special_key,
            navigation_key,
            line_edge_key,
            is_remapped: false,
        })
    }

    /// `Meta` is the core's `win`, the host chord the Mac's ⌘ also maps to.
    fn modifiers(&self) -> KeyModifiers {
        KeyModifiers {
            shift: self.shift,
            control: self.ctrl,
            alt: self.alt,
            win: self.meta,
        }
    }

    /// What the key types: `Some(None)` for a named key that types nothing,
    /// `None` for a key this input method does not read.
    fn characters(&self) -> Option<Option<String>> {
        let fixed = match self.key.as_str() {
            "Enter" => Some("\r"),
            "Tab" => Some("\t"),
            "Escape" => Some("\u{1B}"),
            "Backspace" => Some("\u{8}"),
            "Delete" => Some("\u{7F}"),
            _ => None,
        };
        if let Some(fixed) = fixed {
            return Some(Some(fixed.to_owned()));
        }
        // Every name in the `key` values list is ASCII and longer than one
        // character; anything else is text the layout typed.
        let is_name = self.key.is_ascii() && self.key.len() > 1;
        if !is_name {
            return (!self.key.is_empty()).then(|| Some(self.key.clone()));
        }
        let is_read =
            self.key == "Dead" || navigation_key(&self.key).is_some() || is_named_key(&self.key);
        is_read.then_some(None)
    }

    fn key_code(&self) -> Option<u16> {
        // The chord tables take this code to mean `;`, so a layout that
        // types something else at that position (Dvorak's `s`) has no code.
        if self.code == "Semicolon" {
            return matches!(self.key.as_str(), ";" | ":").then_some(SEMICOLON_KEY_CODE);
        }
        position_key_code(&self.code)
    }
}

/// The number row and the keypad, as `linux` `virtual_key_code` and `macos`
/// `KEY_CODES` spell them. `NumpadEqual` is the Mac's keypad `=`.
fn position_key_code(code: &str) -> Option<u16> {
    let digit = |name: &str| -> Option<u16> {
        let digit = name.parse::<u16>().ok()?;
        (digit <= 9).then_some(digit)
    };
    if let Some(number) = code.strip_prefix("Digit") {
        return digit(number).map(|digit| 0x30 + digit);
    }
    if let Some(number) = code.strip_prefix("Numpad") {
        if let Some(digit) = digit(number) {
            return Some(0x60 + digit);
        }
    }
    match code {
        "NumpadMultiply" => Some(0x6A),
        "NumpadAdd" => Some(0x6B),
        "NumpadSubtract" => Some(0x6D),
        "NumpadDecimal" => Some(0x6E),
        "NumpadDivide" => Some(0x6F),
        "NumpadEqual" => Some(0x92),
        _ => None,
    }
}

/// What the key at `code` types on a US layout, for a character Option
/// rewrote.
fn us_character(code: &str) -> Option<&'static str> {
    const LETTERS: &str = "abcdefghijklmnopqrstuvwxyz";
    const DIGITS: &str = "0123456789";
    if let Some(letter) = code.strip_prefix("Key") {
        let index = LETTERS.find(&letter.to_ascii_lowercase())?;
        return LETTERS.get(index..=index);
    }
    if let Some(digit) = code.strip_prefix("Digit") {
        let index = DIGITS.find(digit)?;
        return DIGITS.get(index..=index);
    }
    match code {
        "Backquote" => Some("`"),
        "Minus" => Some("-"),
        "Equal" => Some("="),
        "BracketLeft" => Some("["),
        "BracketRight" => Some("]"),
        "Backslash" => Some("\\"),
        "Semicolon" => Some(";"),
        "Quote" => Some("'"),
        "Comma" => Some(","),
        "Period" => Some("."),
        "Slash" => Some("/"),
        _ => None,
    }
}

fn navigation_key(key: &str) -> Option<NavigationKey> {
    match key {
        "ArrowLeft" => Some(NavigationKey::LeftArrow),
        "ArrowRight" => Some(NavigationKey::RightArrow),
        "ArrowUp" => Some(NavigationKey::UpArrow),
        "ArrowDown" => Some(NavigationKey::DownArrow),
        "PageUp" => Some(NavigationKey::PageUp),
        "PageDown" => Some(NavigationKey::PageDown),
        _ => None,
    }
}

fn line_edge_key(key: &str) -> Option<LineEdgeKey> {
    match key {
        "Home" => Some(LineEdgeKey::Home),
        "End" => Some(LineEdgeKey::End),
        _ => None,
    }
}

/// Keys the platform names rather than types, past navigation: the Linux
/// `is_named_special_keysym` list. `Clear` is `Begin`, the keypad 5 with Num
/// Lock off.
fn is_named_key(key: &str) -> bool {
    matches!(
        key,
        "Home" | "End" | "Insert" | "Delete" | "Clear" | "ContextMenu" | "Pause" | "PrintScreen"
    ) || key
        .strip_prefix('F')
        .and_then(|number| number.parse::<u16>().ok())
        .is_some_and(|number| (1..=35).contains(&number))
}

#[cfg(test)]
mod tests {
    use super::*;
    use taigi_desktop_core::keys::{CandidateSlotKeySet, ComposingKeyChord, ShortcutAction};
    use taigi_desktop_core::settings::SettingsDocument;

    const LINUX: DesktopPlatform = DesktopPlatform::Linux;
    const MAC: DesktopPlatform = DesktopPlatform::MacOS;

    fn press(key: &str, code: &str) -> BrowserKey {
        BrowserKey {
            key: key.to_owned(),
            code: code.to_owned(),
            ..BrowserKey::default()
        }
    }

    fn snapshot(key: BrowserKey) -> KeyEventSnapshot {
        key.snapshot(LINUX).expect("a snapshot")
    }

    #[test]
    fn a_letter_keeps_what_the_layout_typed() {
        let shifted = BrowserKey {
            shift: true,
            ..press("A", "KeyA")
        };
        let snapshot = snapshot(shifted);
        assert_eq!(snapshot.characters.as_deref(), Some("A"));
        assert_eq!(snapshot.characters_ignoring_modifiers.as_deref(), Some("A"));
        assert_eq!(snapshot.modifiers, KeyModifiers::SHIFT);
        assert_eq!(snapshot.key_code, None);
        assert!(!snapshot.is_named_special_key);
    }

    #[test]
    fn keys_the_classifier_never_reads_answer_nothing() {
        for key in [
            "Shift",
            "Control",
            "Alt",
            "Meta",
            "AltGraph",
            "CapsLock",
            "NumLock",
            "Process",
            "Unidentified",
            "AudioVolumeUp",
            "",
        ] {
            assert_eq!(press(key, "").snapshot(LINUX), None, "{key}");
        }
    }

    #[test]
    fn the_control_characters_are_spelled_as_on_the_desktops() {
        // trace: Linux `fixed_control_character`.
        for (key, character) in [
            ("Enter", "\r"),
            ("Tab", "\t"),
            ("Escape", "\u{1B}"),
            ("Backspace", "\u{8}"),
            ("Delete", "\u{7F}"),
        ] {
            let snapshot = snapshot(press(key, key));
            assert_eq!(snapshot.characters.as_deref(), Some(character), "{key}");
        }
        assert!(snapshot(press("Delete", "Delete")).is_named_special_key);
        assert!(!snapshot(press("Enter", "NumpadEnter")).is_named_special_key);
    }

    #[test]
    fn every_position_maps_into_the_virtual_key_space() {
        // trace: `chord.rs` NUMBER_ROW_KEY_CODES, KEYPAD_DIGIT_KEY_CODES and
        // `is_keypad_key_code`; Linux `virtual_key_code` for 0x30 and the
        // keypad operators; macOS KEY_CODES for 0x92.
        let cases = [
            ("Digit0", 0x30),
            ("Digit1", 0x31),
            ("Digit2", 0x32),
            ("Digit3", 0x33),
            ("Digit4", 0x34),
            ("Digit5", 0x35),
            ("Digit6", 0x36),
            ("Digit7", 0x37),
            ("Digit8", 0x38),
            ("Digit9", 0x39),
            ("Numpad0", 0x60),
            ("Numpad1", 0x61),
            ("Numpad2", 0x62),
            ("Numpad3", 0x63),
            ("Numpad4", 0x64),
            ("Numpad5", 0x65),
            ("Numpad6", 0x66),
            ("Numpad7", 0x67),
            ("Numpad8", 0x68),
            ("Numpad9", 0x69),
            ("NumpadMultiply", 0x6A),
            ("NumpadAdd", 0x6B),
            ("NumpadSubtract", 0x6D),
            ("NumpadDecimal", 0x6E),
            ("NumpadDivide", 0x6F),
            ("NumpadEqual", 0x92),
        ];
        for (code, expected) in cases {
            assert_eq!(position_key_code(code), Some(expected), "{code}");
        }
        for code in [
            "KeyA",
            "Numpad",
            "Numpad10",
            "NumpadEnter",
            "Digit10",
            "Space",
        ] {
            assert_eq!(position_key_code(code), None, "{code}");
        }
    }

    #[test]
    fn a_shifted_digit_is_still_its_key() {
        // trace: Shift+3 types `#`; `shifted_slot_for_event` reads the code.
        let key = BrowserKey {
            shift: true,
            ..press("#", "Digit3")
        };
        assert_eq!(
            CandidateSlotKeySet::Digits.shifted_slot_for_event(&snapshot(key)),
            Some(2)
        );
    }

    #[test]
    fn the_semicolon_code_needs_a_semicolon() {
        // trace: Dvorak's `s` sits at `Semicolon`; with that code Shift+S
        // would read as slot 8 (`shifted_slot_for_event`, BareKeys).
        assert_eq!(snapshot(press(";", "Semicolon")).key_code, Some(0xBA));
        let colon = BrowserKey {
            shift: true,
            ..press(":", "Semicolon")
        };
        assert_eq!(snapshot(colon).key_code, Some(0xBA));
        let dvorak = BrowserKey {
            shift: true,
            ..press("S", "Semicolon")
        };
        assert_eq!(snapshot(dvorak).key_code, None);
    }

    #[test]
    fn a_keypad_key_without_num_lock_is_the_key_it_names() {
        let end = snapshot(press("End", "Numpad1"));
        assert_eq!(end.line_edge_key, Some(LineEdgeKey::End));
        assert_eq!(end.characters, None);
        assert_eq!(end.key_code, Some(0x61));
        assert!(end.is_named_special_key);
    }

    #[test]
    fn navigation_and_function_keys_type_nothing() {
        let left = snapshot(press("ArrowLeft", "ArrowLeft"));
        assert_eq!(left.navigation_key, Some(NavigationKey::LeftArrow));
        assert_eq!(left.characters, None);
        assert!(left.is_named_special_key);
        assert!(snapshot(press("F12", "F12")).is_named_special_key);
        assert_eq!(press("F36", "F36").snapshot(LINUX), None);
    }

    #[test]
    fn a_bare_dead_key_is_named_so_a_composition_ends_first() {
        let dead = snapshot(press("Dead", "Quote"));
        assert_eq!(dead.characters, None);
        assert!(dead.is_named_special_key);
    }

    #[test]
    fn option_rewrites_are_read_back_by_position_on_the_mac() {
        // trace: `default_chord` ToggleTps = Ctrl+Alt+P; Option+P reports `π`,
        // Option+E a dead key.
        let settings = SettingsDocument::default();
        let chord = |key: &str, code: &str| BrowserKey {
            ctrl: true,
            alt: true,
            ..press(key, code)
        };
        let toggle = chord("π", "KeyP").snapshot(MAC).unwrap();
        assert_eq!(
            ShortcutAction::matching(&toggle, &settings, MAC),
            Some(ShortcutAction::ToggleTps)
        );
        let comma = chord("≤", "Comma").snapshot(MAC).unwrap();
        assert_eq!(
            ShortcutAction::matching(&comma, &settings, MAC),
            Some(ShortcutAction::ShowSymbolPicker)
        );
        let dead = chord("Dead", "KeyE").snapshot(MAC).unwrap();
        assert_eq!(dead.characters_ignoring_modifiers.as_deref(), Some("e"));
        assert_eq!(
            dead.modifiers,
            KeyModifiers::CONTROL.with(KeyModifiers::ALT)
        );
    }

    #[test]
    fn altgr_text_is_text_on_windows_and_linux() {
        // trace: Polish AltGr+C types `ć`; as Ctrl+Alt with `c` it would match
        // ToggleRomanization.
        let key = BrowserKey {
            ctrl: true,
            alt: true,
            ..press("ć", "KeyC")
        };
        let text = key.snapshot(DesktopPlatform::Windows).unwrap();
        assert_eq!(text.modifiers, KeyModifiers::NONE);
        assert_eq!(text.characters.as_deref(), Some("ć"));
        assert_eq!(
            ShortcutAction::matching(&text, &SettingsDocument::default(), LINUX),
            None
        );
    }

    #[test]
    fn an_ascii_key_under_control_and_alt_is_the_chord_it_reads() {
        // trace: Dvorak types `s` at `Semicolon`; `key` decides, not the position.
        let key = BrowserKey {
            ctrl: true,
            alt: true,
            ..press("s", "Semicolon")
        };
        assert_eq!(
            ShortcutAction::matching(&snapshot(key), &SettingsDocument::default(), LINUX),
            Some(ShortcutAction::OpenLastSettingsPane)
        );
    }

    #[test]
    fn meta_is_the_host_chord() {
        let key = BrowserKey {
            meta: true,
            ..press("c", "KeyC")
        };
        let snapshot = snapshot(key);
        assert_eq!(snapshot.modifiers, KeyModifiers::WIN);
        assert!(snapshot.modifiers.has_host_chord());
    }

    #[test]
    fn every_default_chord_is_reachable_from_a_browser_event() {
        // trace: `default_chord`: Ctrl+Alt + s c p h , / j, bare `.
        let settings = SettingsDocument::default();
        for action in ShortcutAction::ALL {
            let ComposingKeyChord { key, modifiers } = action.default_chord();
            let event = BrowserKey {
                ctrl: modifiers.control,
                alt: modifiers.alt,
                ..press(&key, "")
            };
            assert_eq!(
                ShortcutAction::matching(&snapshot(event), &settings, LINUX),
                Some(action),
                "{key}"
            );
        }
    }

    #[test]
    fn only_the_macs_control_command_reads_as_the_core_tier() {
        let event = BrowserKey {
            ctrl: true,
            meta: true,
            ..press("c", "KeyC")
        };
        let rewritten = core_tier_snapshot(&event.snapshot(MAC).unwrap(), MAC).unwrap();
        assert_eq!(rewritten.modifiers, CORE_TIER);
        assert!(core_tier_snapshot(&event.snapshot(LINUX).unwrap(), LINUX).is_none());
        let plain_command = BrowserKey {
            meta: true,
            ..press("c", "KeyC")
        };
        assert!(core_tier_snapshot(&plain_command.snapshot(MAC).unwrap(), MAC).is_none());
    }

    #[test]
    fn the_shift_keys_tell_each_other_apart() {
        assert_eq!(shift_index("ShiftLeft"), Some(0));
        assert_eq!(shift_index("ShiftRight"), Some(1));
        assert_eq!(shift_index("KeyA"), None);
    }
}
