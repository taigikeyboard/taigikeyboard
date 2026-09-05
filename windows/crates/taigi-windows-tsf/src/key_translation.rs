//! The key sink's `WM_KEYDOWN` / `WM_KEYUP` message, unpacked for the shared
//! translation. The rules themselves live in
//! `taigi_windows_platform::key_translation` (roadmap W5) so the settings
//! window's shortcut recorder reads a key exactly as this classifier does.

// 把 WM_KEYDOWN/WM_KEYUP 的 WPARAM/LPARAM 拆開,規則本身在 platform crate,與設定視窗的錄製欄共用。

use taigi_windows_core::keys::KeyEventSnapshot;
use windows::Win32::Foundation::{LPARAM, WPARAM};

pub use taigi_windows_platform::key_translation::is_other_modifier_held_at_shift_press;

/// The virtual-key code a key message is about.
pub fn virtual_key(wparam: WPARAM) -> u16 {
    (wparam.0 & 0xFFFF) as u16
}

/// `lParam` bits 16–23. The extended-key flag (bit 24) is deliberately left
/// out, as it always has been — the two Shift keys already differ in these
/// eight bits (left `0x2A`, right `0x36`), which is what tells them apart.
pub fn scan_code(lparam: LPARAM) -> u32 {
    ((lparam.0 as u32) >> 16) & 0xFF
}

/// `lParam` bit 30, the previous key state: set means the key was already
/// down, so the message is auto-repeat rather than a fresh press.
pub fn is_repeat(lparam: LPARAM) -> bool {
    const PREVIOUS_KEY_STATE: isize = 1 << 30;
    lparam.0 & PREVIOUS_KEY_STATE != 0
}

/// The snapshot for a key-down, or `None` for a modifier or synthetic key.
pub fn snapshot(wparam: WPARAM, lparam: LPARAM) -> Option<KeyEventSnapshot> {
    taigi_windows_platform::key_translation::snapshot(virtual_key(wparam), scan_code(lparam))
}
