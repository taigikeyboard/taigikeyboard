//! The Win32 calls that WRITE into a buffer we then read, made through
//! `windows-sys`'s raw bindings rather than the `windows` crate's slice
//! wrappers.
//!
//! WHY THIS MODULE EXISTS. The `windows` crate's ergonomic wrappers take the
//! out-buffer as `&mut [T]` but hand the OS a pointer derived from
//! `as_ptr()` — a READ-ONLY provenance — transmuted to `*mut`:
//!
//! ```text
//! // windows-0.62.2 Win32/UI/Input/KeyboardAndMouse/mod.rs:87-91
//! pub unsafe fn GetKeyboardState(lpkeystate: &mut [u8; 256]) -> Result<()> {
//!     link!("user32.dll" "system" fn GetKeyboardState(lpkeystate: *mut u8) -> BOOL);
//!     unsafe { GetKeyboardState(core::mem::transmute(lpkeystate.as_ptr())).ok() }
//! }
//! ```
//!
//! Writing through a shared-derived pointer is undefined behaviour, and an
//! optimized build acts on it: the caller's own reads of the buffer fold back
//! to its initializer. Measured on the Windows box, release build, one real
//! `Ctrl+Alt+A` press inside `snapshot`, every value from ONE `writeln!`:
//! `state[0x11]` printed `0x81` (a format argument is a REFERENCE, so it
//! loads memory) while `(state[0x11] & 0x80) != 0` computed `false` (a value,
//! folded against `[0u8; 256]`) — and the same call through a raw `*mut`
//! reported the modifiers correctly. Every modifier in every release build
//! read as "not held": no shortcut with a modifier could be recorded, the
//! Ctrl / Alt / Shift candidate-slot sets were dead, and no stored chord ever
//! matched. `ToUnicodeEx` was unaffected only because the kernel returns its
//! answer by value, which is what made the bug read as a contradiction.
//!
//! Same shape in `windows` 0.58 and 0.61.3, so it is not a regression to wait
//! out. Every affected call this repo makes lives here (or, for the COM
//! methods, in `taigi-windows-tsf`'s `com_out_buffer`), and the rule for new
//! code is: a Win32 call that fills a buffer we read goes through a raw
//! binding, never through a `&mut [T]` wrapper.

// 會寫入我們自己 buffer 的 Win32 呼叫,一律走 windows-sys 原始 binding。
// windows crate 的 &mut [T] 包裝用 as_ptr()(唯讀 provenance)transmute 成 *mut 交給 OS 寫,
// 那是 UB;release build 會把呼叫端讀到的值摺回初值 —— modifier 全部讀成「無壓落」。

use windows::Win32::Foundation::{HMODULE, MAX_PATH};
use windows::Win32::System::SystemServices::LOCALE_NAME_MAX_LENGTH;
use windows::Win32::UI::Input::KeyboardAndMouse::HKL;
use windows_sys::Win32::Globalization::GetUserDefaultLocaleName;
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetKeyboardState, ToUnicodeEx};

/// `ToUnicodeEx` flag: do not change the keyboard state (no dead-key
/// consumption). Windows 10 1607+.
const TO_UNICODE_DO_NOT_CHANGE_STATE: u32 = 0x4;

/// Every key's state as the thread's last-read message left it, or `None`
/// when the call failed (the caller decides what "no modifier held" means).
pub fn keyboard_state() -> Option<[u8; 256]> {
    let mut state = [0u8; 256];
    // SAFETY: a 256-byte buffer, the size the API requires, passed by a
    // pointer that may be written through.
    let read = unsafe { GetKeyboardState(state.as_mut_ptr()) };
    (read != 0).then_some(state)
}

/// What `virtual_key` types under `state` on `layout`, or `None` for a key
/// that types nothing (a dead key's negative return included: nothing to
/// compose with until the next key completes it).
pub fn to_unicode(
    virtual_key: u16,
    scan_code: u32,
    state: &[u8; 256],
    layout: HKL,
) -> Option<String> {
    let mut buffer = [0u16; 8];
    // SAFETY: `state` is the 256 bytes the API reads; `buffer` is written
    // through a pointer that may be, and its length is passed alongside.
    let written = unsafe {
        ToUnicodeEx(
            u32::from(virtual_key),
            scan_code,
            state.as_ptr(),
            buffer.as_mut_ptr(),
            buffer.len() as i32,
            TO_UNICODE_DO_NOT_CHANGE_STATE,
            layout.0,
        )
    };
    if written <= 0 {
        return None;
    }
    let text = String::from_utf16_lossy(&buffer[..written as usize]);
    (!text.is_empty()).then_some(text)
}

/// The regional format locale (`zh-TW`, `en-US`, …), or `None` when the
/// system reports none.
pub fn user_default_locale_name() -> Option<String> {
    let mut buffer = [0u16; LOCALE_NAME_MAX_LENGTH as usize];
    // SAFETY: a writable UTF-16 buffer, its length passed alongside.
    let length = unsafe { GetUserDefaultLocaleName(buffer.as_mut_ptr(), buffer.len() as i32) };
    // The count includes the terminating NUL, so 1 is the empty name.
    if length <= 1 {
        return None;
    }
    Some(String::from_utf16_lossy(
        &buffer[..(length as usize).saturating_sub(1)],
    ))
}

/// The full path of the loaded module `module`, or `None` when the call
/// failed or the path did not fit.
pub fn module_file_name(module: HMODULE) -> Option<String> {
    // `MAX_PATH` is enough for an install under Program Files; a longer path
    // is truncated, and the API cannot say how long the real one is.
    let mut buffer = [0u16; MAX_PATH as usize];
    // SAFETY: `module` is a handle the caller holds; the buffer is written
    // through a pointer that may be, and its length is passed alongside.
    let length = unsafe { GetModuleFileNameW(module.0, buffer.as_mut_ptr(), buffer.len() as u32) };
    let length = length as usize;
    // A path that filled the buffer was truncated, and counts as no path.
    if length == 0 || length >= buffer.len() {
        return None;
    }
    Some(String::from_utf16_lossy(&buffer[..length]))
}
