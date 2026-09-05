//! The DLL's own module handle, saved by `DllMain`, and the paths derived
//! from it: the install directory holds the dictionaries, the fonts and the
//! settings exe. Resolved from the DLL — never from the host exe (roadmap
//! W2, Codex).

use std::path::PathBuf;
use std::sync::atomic::{AtomicIsize, Ordering};
use taigi_windows_platform::os_out_buffer;
use windows::Win32::Foundation::{HINSTANCE, HMODULE};

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
    // The handle is this DLL's own, stored by DllMain. `MAX_PATH` is enough
    // for an install under Program Files; a longer path reads as "no module
    // path" rather than as a truncated one (`os_out_buffer`).
    os_out_buffer::module_file_name(module).map(PathBuf::from)
}

/// The directory the DLL was loaded from.
pub fn install_directory() -> Option<PathBuf> {
    module_path().and_then(|path| path.parent().map(PathBuf::from))
}
