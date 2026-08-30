//! One key-down → one [`KeyEventSnapshot`] (roadmap W5), and the
//! [`RecordedPress`] the shortcut recorder makes of it. The modifier state
//! is sampled ONCE from the keyboard-state array (Codex: `GetKeyState`
//! drifts under autorepeat and re-entrancy); the characters come from
//! `ToUnicodeEx` with flag `0x4` ("do not change keyboard state", Windows
//! 10 1607+) so a dead key is not consumed by the probe; the unmodified
//! characters from a state copy with the Ctrl bits cleared (khiin
//! `key_event.rs:39-74`) — Ctrl+3 must read as `3`, not as Escape. AltGr
//! (Ctrl+Alt) is left intact so international layouts keep their glyphs.
//!
//! Shared: the DLL's key sink reads it through
//! `taigi-windows-tsf/src/key_translation.rs`, and the settings window's
//! shortcut recorder through [`recorded_press`]. One set of rules, so a
//! chord the recorder stores is a chord the classifier matches — the
//! recorder is NOT allowed a weaker copy (roadmap W17).

// 中文: 一個按鍵按下轉成分類器要的快照(以及錄製欄要的 RecordedPress);修飾鍵只取樣一次,字元用 ToUnicodeEx(0x4)。DLL 與設定視窗共用同一份規則。

use taigi_windows_core::keys::{KeyEventSnapshot, KeyModifiers, NavigationKey, RecordedPress};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyboardLayout, GetKeyboardState, ToUnicodeEx, HKL, VK_BACK, VK_CAPITAL, VK_CONTROL,
    VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_F1, VK_F24, VK_HOME, VK_INSERT, VK_LCONTROL, VK_LEFT,
    VK_LMENU, VK_LWIN, VK_MENU, VK_NEXT, VK_NUMLOCK, VK_PACKET, VK_PRIOR, VK_PROCESSKEY,
    VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RWIN, VK_SHIFT, VK_TAB, VK_UP,
};

/// `ToUnicodeEx` flag: do not change the keyboard state (no dead-key
/// consumption). Windows 10 1607+.
const TO_UNICODE_DO_NOT_CHANGE_STATE: u32 = 0x4;
const KEY_IS_DOWN: u8 = 0x80;

/// Keys that are modifiers themselves: never a composing key, and the key
/// sink answers them without building a snapshot.
pub fn is_modifier_key(virtual_key: u16) -> bool {
    [
        VK_SHIFT,
        VK_CONTROL,
        VK_MENU,
        VK_LCONTROL,
        VK_RCONTROL,
        VK_LMENU,
        VK_RMENU,
        VK_LWIN,
        VK_RWIN,
        VK_CAPITAL,
        VK_NUMLOCK,
    ]
    .iter()
    .any(|key| key.0 == virtual_key)
}

fn navigation_key(virtual_key: u16) -> Option<NavigationKey> {
    match virtual_key {
        code if code == VK_LEFT.0 => Some(NavigationKey::LeftArrow),
        code if code == VK_RIGHT.0 => Some(NavigationKey::RightArrow),
        code if code == VK_UP.0 => Some(NavigationKey::UpArrow),
        code if code == VK_DOWN.0 => Some(NavigationKey::DownArrow),
        code if code == VK_PRIOR.0 => Some(NavigationKey::PageUp),
        code if code == VK_NEXT.0 => Some(NavigationKey::PageDown),
        _ => None,
    }
}

/// Keys the platform names rather than types: the navigation keys, Home /
/// End / Insert / forward Delete and the function keys. Return, Tab,
/// Escape and Backspace are NOT named here — they type a control
/// character, which is what the classifier's tiers read.
fn is_named_special_key(virtual_key: u16) -> bool {
    navigation_key(virtual_key).is_some()
        || [VK_HOME, VK_END, VK_INSERT, VK_DELETE]
            .iter()
            .any(|key| key.0 == virtual_key)
        || (VK_F1.0..=VK_F24.0).contains(&virtual_key)
}

/// The control character a key types regardless of layout, so the
/// classifier's tier 2 / tier 6 see the same bytes the Mac sends
/// (`\r`, `\t`, `\u{1B}`, `\u{8}`, `\u{7F}`).
fn fixed_control_character(virtual_key: u16) -> Option<&'static str> {
    match virtual_key {
        code if code == VK_RETURN.0 => Some("\r"),
        code if code == VK_TAB.0 => Some("\t"),
        code if code == VK_ESCAPE.0 => Some("\u{1B}"),
        code if code == VK_BACK.0 => Some("\u{8}"),
        code if code == VK_DELETE.0 => Some("\u{7F}"),
        _ => None,
    }
}

/// Keys that carry no key of their own: `VK_PACKET` (injected Unicode —
/// touch keyboard, remote desktop, `SendInput`) and `VK_PROCESSKEY` (a key
/// another IME already consumed). Both go straight to the host (DOGFOOD
/// item: touch keyboard + RDP typing).
fn is_synthetic_key(virtual_key: u16) -> bool {
    virtual_key == VK_PACKET.0 || virtual_key == VK_PROCESSKEY.0
}

/// The snapshot for a key-down, or `None` for a modifier or synthetic key.
/// `scan_code` is the low 8 bits of the message's scan code, WITHOUT the
/// extended-key flag — what the key sink has always passed.
pub fn snapshot(virtual_key: u16, scan_code: u32) -> Option<KeyEventSnapshot> {
    if is_modifier_key(virtual_key) || is_synthetic_key(virtual_key) {
        return None;
    }
    let mut state = [0u8; 256];
    // SAFETY: a 256-byte buffer, the size the API requires; a failed read
    // leaves it zeroed, which reads as "no modifier held".
    if unsafe { GetKeyboardState(&mut state) }.is_err() {
        log::debug!("key.state_unavailable");
    }
    let is_down = |key: u16| state[key as usize] & KEY_IS_DOWN != 0;
    let modifiers = KeyModifiers {
        shift: is_down(VK_SHIFT.0),
        control: is_down(VK_CONTROL.0),
        alt: is_down(VK_MENU.0),
        win: is_down(VK_LWIN.0) || is_down(VK_RWIN.0),
    };
    // SAFETY: thread id 0 = the calling thread's layout, which is the
    // focused thread inside a key sink.
    let layout = unsafe { GetKeyboardLayout(0) };

    let characters = fixed_control_character(virtual_key)
        .map(str::to_owned)
        .or_else(|| translate(virtual_key, scan_code, &state, layout));
    let characters_ignoring_modifiers = fixed_control_character(virtual_key)
        .map(str::to_owned)
        .or_else(|| {
            // Ctrl is cleared so Ctrl+3 reads as `3` — EXCEPT under AltGr,
            // whose synthetic Ctrl is part of the glyph on many layouts
            // (`Right Alt` = Ctrl+Alt): there the state is left whole.
            let mut unmodified = state;
            if !modifiers.alt {
                for key in [VK_CONTROL, VK_LCONTROL, VK_RCONTROL] {
                    unmodified[key.0 as usize] = 0;
                }
            }
            translate(virtual_key, scan_code, &unmodified, layout)
        });
    Some(KeyEventSnapshot {
        characters,
        characters_ignoring_modifiers,
        key_code: Some(virtual_key),
        modifiers,
        is_named_special_key: is_named_special_key(virtual_key),
        navigation_key: navigation_key(virtual_key),
    })
}

/// What the key types under `state`, or `None` for a key that types
/// nothing (a dead key's negative return included: nothing to compose with
/// until the next key completes it).
fn translate(virtual_key: u16, scan_code: u32, state: &[u8; 256], layout: HKL) -> Option<String> {
    let mut buffer = [0u16; 8];
    // SAFETY: buffer and state are valid for the lengths passed; the flag
    // keeps the call from mutating the thread's keyboard state.
    let written = unsafe {
        ToUnicodeEx(
            u32::from(virtual_key),
            scan_code,
            state,
            &mut buffer,
            TO_UNICODE_DO_NOT_CHANGE_STATE,
            Some(layout),
        )
    };
    if written <= 0 {
        return None;
    }
    let text = String::from_utf16_lossy(&buffer[..written as usize]);
    (!text.is_empty()).then_some(text)
}

/// The AppKit private-use scalar for a key that types nothing — the
/// spelling the shared chord gate reserves (`ComposingKeyChord`'s
/// `FUNCTION_KEY_RANGE`, `ComposingKeyChord.swift:71`), so an arrow or a
/// function key is refused as a RESERVED key rather than as no key at all.
/// Keys with a fixed control character (Return, Tab, Escape, Backspace,
/// forward Delete) never reach here — they type, and the recorder's rules
/// read what they type.
fn named_key_scalar(virtual_key: u16) -> Option<char> {
    /// `NSUpArrowFunctionKey`; Down / Left / Right follow it.
    const UP_ARROW: u32 = 0xF700;
    /// `NSF1FunctionKey`; F2… follow it.
    const F1: u32 = 0xF704;
    const INSERT: u32 = 0xF727;
    const HOME: u32 = 0xF729;
    const END: u32 = 0xF72B;
    const PAGE_UP: u32 = 0xF72C;
    const PAGE_DOWN: u32 = 0xF72D;
    let scalar = match virtual_key {
        code if code == VK_UP.0 => UP_ARROW,
        code if code == VK_DOWN.0 => UP_ARROW + 1,
        code if code == VK_LEFT.0 => UP_ARROW + 2,
        code if code == VK_RIGHT.0 => UP_ARROW + 3,
        code if code == VK_INSERT.0 => INSERT,
        code if code == VK_HOME.0 => HOME,
        code if code == VK_END.0 => END,
        code if code == VK_PRIOR.0 => PAGE_UP,
        code if code == VK_NEXT.0 => PAGE_DOWN,
        code if (VK_F1.0..=VK_F24.0).contains(&code) => F1 + u32::from(code - VK_F1.0),
        _ => return None,
    };
    char::from_u32(scalar)
}

/// The press a recording shortcut row makes of one key-down: the key named
/// by what it types with NO modifier held — the layout's own answer, via
/// `ToUnicodeEx`, not a table of one layout's punctuation — or the
/// reserved scalar for a key that types nothing. `None` for a modifier or
/// a synthetic key, which the recorder never records.
pub fn recorded_press(virtual_key: u16, scan_code: u32, is_repeat: bool) -> Option<RecordedPress> {
    let snapshot = snapshot(virtual_key, scan_code)?;
    let key = snapshot
        .unmodified_characters()
        .map(str::to_owned)
        .or_else(|| named_key_scalar(virtual_key).map(String::from));
    Some(RecordedPress {
        key,
        modifiers: snapshot.modifiers,
        is_repeat,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use taigi_windows_core::keys::{ChordRejection, ComposingKeyChord, KeyModifiers};

    /// Every key the recorder cannot name by what it types must still be
    /// refused as RESERVED, never as "no key" — the message the user reads.
    fn is_reserved(virtual_key: u16) -> bool {
        let key = named_key_scalar(virtual_key).map(String::from);
        ComposingKeyChord::make(key.as_deref(), KeyModifiers::default())
            == Err(ChordRejection::ReservedKey)
    }

    #[test]
    fn the_named_keys_land_inside_the_shared_gates_reserved_range() {
        // trace: 0xF700..=0xF8FF is FUNCTION_KEY_RANGE, and the scalars are
        // AppKit's, so a Windows arrow reads as the Mac's arrow.
        for key in [
            VK_UP, VK_DOWN, VK_LEFT, VK_RIGHT, VK_INSERT, VK_HOME, VK_END, VK_PRIOR, VK_NEXT,
        ] {
            assert!(is_reserved(key.0), "{key:?} must be refused as reserved");
        }
        assert_eq!(named_key_scalar(VK_UP.0), Some('\u{F700}'));
        assert_eq!(named_key_scalar(VK_RIGHT.0), Some('\u{F703}'));
        assert_eq!(named_key_scalar(VK_NEXT.0), Some('\u{F72D}'));
    }

    #[test]
    fn the_function_keys_number_from_f1_with_no_off_by_one() {
        assert_eq!(named_key_scalar(VK_F1.0), Some('\u{F704}'));
        assert_eq!(named_key_scalar(VK_F1.0 + 11), Some('\u{F70F}'), "F12");
        assert_eq!(named_key_scalar(VK_F24.0), Some('\u{F71B}'));
        assert!(is_reserved(VK_F1.0) && is_reserved(VK_F24.0));
    }

    #[test]
    fn a_key_that_types_is_named_by_what_it_types_not_by_a_scalar() {
        // trace: the letters, the digits and every punctuation key have
        // `unmodified_characters`, so `named_key_scalar` is never asked —
        // and a key with a fixed control character (Tab, Delete) types too.
        for typing in [b'A' as u16, b'Z' as u16, b'0' as u16, VK_TAB.0, VK_DELETE.0] {
            assert_eq!(named_key_scalar(typing), None);
        }
    }
}
