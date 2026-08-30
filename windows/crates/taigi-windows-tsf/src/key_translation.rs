//! The key sink's `WM_KEYDOWN` message, unpacked for the shared
//! translation. The rules themselves live in
//! `taigi_windows_platform::key_translation` (roadmap W5) so the settings
//! window's shortcut recorder reads a key exactly as this classifier does.

// 中文: 把 WM_KEYDOWN 的 WPARAM/LPARAM 拆開,規則本身在 platform crate,與設定視窗的錄製欄共用。

use taigi_windows_core::keys::KeyEventSnapshot;
use windows::Win32::Foundation::{LPARAM, WPARAM};

/// The snapshot for a key-down, or `None` for a modifier or synthetic key.
/// `lParam` bits 16–23 are the scan code; the extended-key flag (bit 24)
/// is deliberately left out, as it always has been.
pub fn snapshot(wparam: WPARAM, lparam: LPARAM) -> Option<KeyEventSnapshot> {
    let virtual_key = (wparam.0 & 0xFFFF) as u16;
    let scan_code = ((lparam.0 as u32) >> 16) & 0xFF;
    taigi_windows_platform::key_translation::snapshot(virtual_key, scan_code)
}
