//! The DLL's own module handle, saved by `DllMain`, and the paths derived
//! from it: the install directory holds the dictionaries, the fonts and the
//! settings exe. Resolved from the DLL — never from the host exe (roadmap
//! W2, Codex).

// 中文: DllMain 存下的模組把手,以及由它推出的安裝目錄(辭典、字型、設定程式都在那裡)。

use std::path::PathBuf;
use std::sync::atomic::{AtomicIsize, Ordering};
use windows::Win32::Foundation::{HINSTANCE, HMODULE, MAX_PATH};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;

static INSTANCE: AtomicIsize = AtomicIsize::new(0);

pub fn remember_instance(instance: HINSTANCE) {
    INSTANCE.store(instance.0 as isize, Ordering::Release);
}

pub fn instance() -> HINSTANCE {
    HINSTANCE(INSTANCE.load(Ordering::Acquire) as *mut _)
}

/// The DLL's full path, e.g. `C:\Program Files\TaigiKeyboard\TaigiKeyboard.dll`.
pub fn module_path() -> Option<PathBuf> {
    let module = HMODULE(instance().0);
    if module.is_invalid() {
        return None;
    }
    // MAX_PATH is enough for an install under Program Files; a longer path
    // is truncated and reads as "no module path" via the length check.
    let mut buffer = [0u16; MAX_PATH as usize];
    // SAFETY: `buffer` is a valid, writable UTF-16 buffer of the length
    // passed; the module handle is this DLL's own, stored by DllMain.
    let length = unsafe { GetModuleFileNameW(Some(module), &mut buffer) } as usize;
    if length == 0 || length >= buffer.len() {
        return None;
    }
    Some(PathBuf::from(String::from_utf16_lossy(&buffer[..length])))
}

/// The directory the DLL was loaded from.
pub fn install_directory() -> Option<PathBuf> {
    module_path().and_then(|path| path.parent().map(PathBuf::from))
}
