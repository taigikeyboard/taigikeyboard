//! An egui key event as the recorder's `RecordedPress`: the character the
//! key types with no modifier held (what the composing registry stores),
//! spelled the way the shared chord gate expects — AppKit's function-key
//! scalars for the navigation keys, so `ComposingKeyChord::make` refuses
//! them as reserved exactly as on the Mac.

// 中文: egui 按鍵 → 錄製欄的 RecordedPress;導覽鍵用 AppKit 的功能鍵碼位,讓共用閘門一致拒絕。

use egui::{Key, Modifiers};
use taigi_windows_core::keys::{KeyModifiers, RecordedPress};

/// The unmodified character `key` types, or `None` for a key that types
/// nothing the chord gate could name (a function key). egui's `Key` is the
/// logical key under the active layout, so letters follow the keymap;
/// the shifted punctuation names are folded back onto their base key,
/// as `charactersIgnoringModifiers` does. NAMED LIMITATION (roadmap W15
/// dogfood): the folds are the US layout's — on a layout where `:` or `+`
/// sit on another base key, a CHORD on such punctuation records a key the
/// DLL's classifier will not match; a bare press is exact (the recorder
/// takes the typed text for it). The keypad's `+` reads as `=`.
pub fn chord_key(key: Key) -> Option<&'static str> {
    Some(match key {
        Key::A => "a",
        Key::B => "b",
        Key::C => "c",
        Key::D => "d",
        Key::E => "e",
        Key::F => "f",
        Key::G => "g",
        Key::H => "h",
        Key::I => "i",
        Key::J => "j",
        Key::K => "k",
        Key::L => "l",
        Key::M => "m",
        Key::N => "n",
        Key::O => "o",
        Key::P => "p",
        Key::Q => "q",
        Key::R => "r",
        Key::S => "s",
        Key::T => "t",
        Key::U => "u",
        Key::V => "v",
        Key::W => "w",
        Key::X => "x",
        Key::Y => "y",
        Key::Z => "z",
        Key::Num0 => "0",
        Key::Num1 | Key::Exclamationmark => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::Num4 => "4",
        Key::Num5 => "5",
        Key::Num6 => "6",
        Key::Num7 => "7",
        Key::Num8 => "8",
        Key::Num9 => "9",
        Key::Space => " ",
        Key::Enter => "\r",
        Key::Tab => "\t",
        Key::Backspace => "\u{8}",
        Key::Delete => "\u{7F}",
        Key::Escape => "\u{1B}",
        Key::Backtick => "`",
        Key::Minus => "-",
        Key::Equals | Key::Plus => "=",
        Key::OpenBracket | Key::OpenCurlyBracket => "[",
        Key::CloseBracket | Key::CloseCurlyBracket => "]",
        Key::Semicolon | Key::Colon => ";",
        Key::Quote => "'",
        Key::Comma => ",",
        Key::Period => ".",
        Key::Slash | Key::Questionmark => "/",
        Key::Backslash | Key::Pipe => "\\",
        // AppKit's private-use scalars (`NSUpArrowFunctionKey` …): the
        // shared gate's reserved range (`ComposingKeyChord.swift:71`).
        Key::ArrowUp => "\u{F700}",
        Key::ArrowDown => "\u{F701}",
        Key::ArrowLeft => "\u{F702}",
        Key::ArrowRight => "\u{F703}",
        Key::Insert => "\u{F727}",
        Key::Home => "\u{F729}",
        Key::End => "\u{F72B}",
        Key::PageUp => "\u{F72C}",
        Key::PageDown => "\u{F72D}",
        _ => return None,
    })
}

/// The chording modifiers egui reports. There is no Win key in egui's
/// model, so a Win chord cannot be recorded here — the global gate refuses
/// it anyway (`global_rejection`). `command` is Ctrl on Windows.
pub fn modifiers_of(modifiers: Modifiers) -> KeyModifiers {
    KeyModifiers {
        shift: modifiers.shift,
        control: modifiers.ctrl || modifiers.command,
        alt: modifiers.alt,
        win: false,
    }
}

pub fn press_from_event(key: Key, modifiers: Modifiers, is_repeat: bool) -> RecordedPress {
    RecordedPress {
        key: chord_key(key).map(str::to_owned),
        modifiers: modifiers_of(modifiers),
        is_repeat,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taigi_windows_core::keys::{ChordRejection, ComposingKeyChord};

    #[test]
    fn letters_digits_and_punctuation_fold_onto_their_unmodified_character() {
        assert_eq!(chord_key(Key::Z), Some("z"));
        assert_eq!(chord_key(Key::Exclamationmark), Some("1"));
        assert_eq!(chord_key(Key::Colon), Some(";"));
        assert_eq!(chord_key(Key::Backtick), Some("`"));
        assert_eq!(chord_key(Key::F5), None);
    }

    #[test]
    fn navigation_keys_reach_the_shared_gate_as_reserved() {
        // trace: 0xF700 is inside FUNCTION_KEY_RANGE → ReservedKey.
        let press = press_from_event(Key::ArrowUp, Modifiers::NONE, false);
        assert_eq!(
            ComposingKeyChord::make(press.key.as_deref(), press.modifiers),
            Err(ChordRejection::ReservedKey)
        );
    }

    #[test]
    fn ctrl_and_command_both_read_as_control_and_win_is_never_reported() {
        let press = press_from_event(Key::S, Modifiers::COMMAND, false);
        assert!(press.modifiers.control);
        assert!(!press.modifiers.win);
        let press = press_from_event(Key::S, Modifiers::CTRL | Modifiers::SHIFT, true);
        assert!(press.modifiers.control && press.modifiers.shift && press.is_repeat);
    }
}
