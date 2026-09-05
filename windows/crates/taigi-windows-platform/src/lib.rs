//! The few Win32 calls the DLL and the settings window reach for — the
//! user's locale, opening a URL, the alert sound, what the system says about
//! appearance — behind plain functions. Most are wanted by both; a couple
//! belong to one caller and live here because it cannot hold them
//! ([`preload_library`] is the settings exe's, which is `unsafe_code =
//! forbid`, and the DLL must never load what it maps).
//! On a non-Windows host (the macOS build gate, `make check`) each answers
//! the neutral value, so the callers compile and test natively.
//!
//! Two of them are big enough to be their own modules: [`key_translation`]
//! (one key-down → the snapshot the classifier reads and the press the
//! shortcut recorder records, roadmap W5) and [`keyboard_hook`] (the
//! thread-scoped `WH_KEYBOARD` the recorder listens through, W17-B1).

/// Windows-only: it IS the Win32 keyboard API. The macOS gate type-checks
/// it for the gnu target and `make check-box` runs its tests on the box.
#[cfg(windows)]
pub mod key_translation;
pub mod keyboard_hook;
/// Windows-only, and the reason it exists is a `windows` crate wrapper that
/// cannot be used for a buffer we read back — see the module.
#[cfg(windows)]
pub mod os_out_buffer;

// DLL 與設定視窗共用的少量 Win32 呼叫;非 Windows 主機給中性值,讓呼叫端在 macOS 上可測。

/// The user's preferred UI language, e.g. `zh-TW` / `ja-JP` / `en-US`, for
/// the `system` display language. Empty when the platform cannot say.
///
/// The UI language list (`GetUserPreferredUILanguages`), NOT the regional
/// format locale (`GetUserDefaultLocaleName`): the Mac reads
/// `Locale.preferredLanguages.first` (`DisplayLanguageStore.swift:122-128`),
/// and an English-UI machine set to a Taiwan region must draw English here
/// too, not Hanji. The format locale is only the fallback for a machine whose
/// UI-language list cannot be read.
#[cfg(windows)]
pub fn system_locale() -> String {
    use windows::core::PWSTR;
    use windows::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};
    let mut count = 0u32;
    let mut length = 0u32;
    // SAFETY: the documented size query — no buffer, the required length
    // comes back in `length` (UTF-16 units, double-NUL-terminated list).
    let sized =
        unsafe { GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, &mut count, None, &mut length) };
    if sized.is_ok() && length > 0 {
        let mut buffer = vec![0u16; length as usize];
        // SAFETY: a writable buffer of exactly the length the query asked for.
        let filled = unsafe {
            GetUserPreferredUILanguages(
                MUI_LANGUAGE_NAME,
                &mut count,
                Some(PWSTR(buffer.as_mut_ptr())),
                &mut length,
            )
        };
        if filled.is_ok() {
            // The first entry of the multi-string is the top preference.
            let first_end = buffer.iter().position(|&unit| unit == 0).unwrap_or(0);
            if first_end > 0 {
                return String::from_utf16_lossy(&buffer[..first_end]);
            }
        }
    }
    crate::os_out_buffer::user_default_locale_name().unwrap_or_default()
}

#[cfg(not(windows))]
pub fn system_locale() -> String {
    String::new()
}

/// Opens `url` in the user's browser; `false` when nothing could handle it
/// (the settings window then says so rather than doing nothing —
/// `ExternalLinkButton.swift:775-780`).
#[cfg(windows)]
pub fn open_url(url: &str) -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let operation: Vec<u16> = "open\0".encode_utf16().collect();
    let target: Vec<u16> = url.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: both strings are NUL-terminated buffers alive for the call;
    // no window, no parameters, no directory.
    let instance = unsafe {
        ShellExecuteW(
            None,
            PCWSTR(operation.as_ptr()),
            PCWSTR(target.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        )
    };
    // Documented: the HINSTANCE is really an INT_PTR — above 32 is
    // success, at or below an error code.
    let code = instance.0 as isize;
    let opened = code > 32;
    if !opened {
        log::warn!("platform.open_url_failed code={code}");
    }
    opened
}

#[cfg(not(windows))]
pub fn open_url(url: &str) -> bool {
    log::info!("platform.open_url_stub url={url}");
    false
}

/// The default system alert sound (`NSSound.beep()`).
/// The window a file dialog must be modal to, in the shape `rfd` asks for.
/// Reactor hands out no HWND, so it is read from the thread inside the
/// message handler that opens the dialog — which is the UI thread, whose
/// active window is the settings window. An unowned dialog would be free
/// to fall behind that window.
pub struct DialogOwner(#[allow(dead_code)] std::num::NonZeroIsize);

#[cfg(windows)]
impl raw_window_handle::HasWindowHandle for DialogOwner {
    fn window_handle(
        &self,
    ) -> Result<raw_window_handle::WindowHandle<'_>, raw_window_handle::HandleError> {
        let handle = raw_window_handle::RawWindowHandle::Win32(
            raw_window_handle::Win32WindowHandle::new(self.0),
        );
        // SAFETY: the HWND is a window of this thread, read moments ago,
        // and the returned borrow is tied to `&self` — it cannot outlive
        // the owner. `rfd` copies the raw handle out to own the dialog.
        Ok(unsafe { raw_window_handle::WindowHandle::borrow_raw(handle) })
    }
}

#[cfg(windows)]
impl raw_window_handle::HasDisplayHandle for DialogOwner {
    fn display_handle(
        &self,
    ) -> Result<raw_window_handle::DisplayHandle<'_>, raw_window_handle::HandleError> {
        Ok(raw_window_handle::DisplayHandle::windows())
    }
}

/// The window this thread has active, or `None` when it has none.
#[cfg(windows)]
pub fn dialog_owner() -> Option<DialogOwner> {
    use windows::Win32::UI::Input::KeyboardAndMouse::GetActiveWindow;
    // SAFETY: one read of this thread's window-manager state.
    let window = unsafe { GetActiveWindow() };
    std::num::NonZeroIsize::new(window.0 as isize).map(DialogOwner)
}

/// The host has no window manager; the caller is Windows-only.
#[cfg(not(windows))]
pub fn dialog_owner() -> Option<DialogOwner> {
    None
}

/// Whether the window the user is typing into belongs to THIS thread.
/// The shortcut recorder asks: Reactor exposes no activation event, so a
/// row left recording when the user switches app is ended by the window's
/// own beat instead (`ShortcutKeyRecorder`'s `WindowFocused(false)` on the
/// egui side).
#[cfg(windows)]
pub fn is_foreground_thread() -> bool {
    use windows::Win32::System::Threading::GetCurrentThreadId;
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    // SAFETY: two reads of window-manager state; a null foreground window
    // (a lock screen, a switch in progress) reads as "not ours".
    unsafe {
        let foreground = GetForegroundWindow();
        if foreground.is_invalid() {
            return false;
        }
        GetWindowThreadProcessId(foreground, None) == GetCurrentThreadId()
    }
}

/// The host has no window manager; the callers are Windows-only.
#[cfg(not(windows))]
pub fn is_foreground_thread() -> bool {
    true
}

#[cfg(windows)]
pub fn beep() {
    use windows::Win32::System::Diagnostics::Debug::MessageBeep;
    use windows::Win32::UI::WindowsAndMessaging::MB_OK;
    // SAFETY: a fire-and-forget system call with a constant.
    let _ = unsafe { MessageBeep(MB_OK) };
}

#[cfg(not(windows))]
pub fn beep() {}

/// Claims the named mutex for this process's lifetime; answers `false`
/// when another process of ours already holds it. The host stub always
/// answers `true`.
///
/// The claim lives as long as the process that WON it: the winner's handle
/// is deliberately leaked (it IS the claim) and a loser closes its own
/// right away, so the name frees when the winner exits and not before. It
/// is an atomic "am I first" across the processes alive right now, NOT a
/// mark that survives the logon session — callers that want once-per-session
/// must say what a second run costs.
#[cfg(windows)]
pub fn acquire_named_claim(name: &str) -> bool {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ALREADY_EXISTS};
    use windows::Win32::System::Threading::CreateMutexW;
    let wide: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    // SAFETY: a NUL-terminated name alive for the call.
    let created = unsafe { CreateMutexW(None, false, PCWSTR(wide.as_ptr())) };
    match created {
        Ok(handle) => {
            // SAFETY: the last-error read right after the call that set it.
            let already_existed = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
            if already_existed {
                // A loser's handle is not the claim, and holding it would
                // keep the name alive past the winner's exit — closing it
                // is what makes the claim end with the process that won.
                // SAFETY: a handle this call just returned, used nowhere else.
                let _ = unsafe { CloseHandle(handle) };
            }
            !already_existed
        }
        Err(error) => {
            log::warn!("platform.named_claim_failed error={error}");
            true
        }
    }
}

#[cfg(not(windows))]
pub fn acquire_named_claim(_name: &str) -> bool {
    true
}

/// Maps `path` as an executable image, the way the loader would, and keeps
/// it mapped for this process's life — see `taigi-windows-settings`'s
/// `prewarm` module for why keeping it is the point.
#[cfg(windows)]
pub fn preload_library(path: &std::path::Path) {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::PCWSTR;
    use windows::Win32::System::LibraryLoader::LoadLibraryW;
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: a NUL-terminated absolute path alive for the call. The
    // returned handle is intentionally leaked: the mapping is the point,
    // and the process exits moments later.
    if let Err(error) = unsafe { LoadLibraryW(PCWSTR(wide.as_ptr())) } {
        log::warn!(
            "platform.preload_library_failed path={} error={error}",
            path.display()
        );
    }
}

#[cfg(not(windows))]
pub fn preload_library(_path: &std::path::Path) {}

/// A debug-build logger to the debugger's output window
/// (`OutputDebugStringW`; DebugView shows it). Release builds install
/// nothing: no log leaves a release build (`security-rules.md`).
#[cfg(all(windows, debug_assertions))]
pub fn install_debug_logger() {
    struct DebugOutput;

    impl log::Log for DebugOutput {
        fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            use windows::core::PCWSTR;
            use windows::Win32::System::Diagnostics::Debug::OutputDebugStringW;
            let line = format!("[taigi:{}] {}\n", record.level(), record.args());
            let wide: Vec<u16> = line.encode_utf16().chain(std::iter::once(0)).collect();
            // SAFETY: a NUL-terminated buffer alive for the call.
            unsafe { OutputDebugStringW(PCWSTR(wide.as_ptr())) };
        }

        fn flush(&self) {}
    }

    if log::set_logger(&DebugOutput).is_ok() {
        log::set_max_level(log::LevelFilter::Debug);
    }
}

#[cfg(not(all(windows, debug_assertions)))]
pub fn install_debug_logger() {}

/// Today's date on the user's clock as `yyyy-MM-dd` — what an export file
/// is named by (`UserDataFilePanels.exportFileName`, local time like
/// `DateFormatter`). The host stub answers UTC.
#[cfg(windows)]
pub fn local_date() -> String {
    use windows::Win32::System::SystemInformation::GetLocalTime;
    // SAFETY: a plain query returning a struct by value.
    let now = unsafe { GetLocalTime() };
    format!("{:04}-{:02}-{:02}", now.wYear, now.wMonth, now.wDay)
}

#[cfg(not(windows))]
pub fn local_date() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let days = (seconds / 86_400) as i64;
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// The directory the running executable lives in — the install directory
/// for the settings window (the DLL resolves its own from its `HMODULE`).
pub fn executable_directory() -> Option<std::path::PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(std::path::Path::to_path_buf)
}

/// An sRGB colour as the system reports it.
pub type Rgb = (u8, u8, u8);

/// `HKCU\…\Themes\Personalize\AppsUseLightTheme` = 0 → dark. Missing (older
/// Windows) → light. Read by the candidate window for its 自動 mode; the
/// settings window lets egui/winit read the same value.
#[cfg(windows)]
pub fn system_prefers_dark() -> bool {
    use windows::core::w;
    use windows::Win32::Foundation::ERROR_SUCCESS;
    use windows::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
    let mut value: u32 = 1;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: a DWORD read into a local of the size passed.
    let status = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("AppsUseLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some(&mut value as *mut u32 as *mut _),
            Some(&mut size),
        )
    };
    status == ERROR_SUCCESS && value == 0
}

#[cfg(not(windows))]
pub fn system_prefers_dark() -> bool {
    false
}

/// The DWM colorization colour (`0xAARRGGBB`, alpha dropped — it describes
/// the frame blend, not a colour a highlight can carry), the closest thing
/// to the user's accent an in-proc DLL can read without WinRT. `None` when
/// DWM is not composing (a remote session).
#[cfg(windows)]
pub fn system_accent() -> Option<Rgb> {
    use windows::core::BOOL;
    use windows::Win32::Graphics::Dwm::DwmGetColorizationColor;
    let mut color = 0u32;
    let mut opaque = BOOL(0);
    // SAFETY: out-pointers to locals.
    unsafe { DwmGetColorizationColor(&mut color, &mut opaque) }.ok()?;
    Some((
        ((color >> 16) & 0xFF) as u8,
        ((color >> 8) & 0xFF) as u8,
        (color & 0xFF) as u8,
    ))
}

#[cfg(not(windows))]
pub fn system_accent() -> Option<Rgb> {
    None
}

/// The colours a high-contrast theme dictates. When one is on, every
/// surface of ours must draw with these and nothing of its own — the whole
/// point of the theme is that the user chose them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HighContrastColors {
    pub window: Rgb,
    pub window_text: Rgb,
    pub highlight: Rgb,
    pub highlight_text: Rgb,
    pub gray_text: Rgb,
}

/// `SPI_GETHIGHCONTRAST`: the system colours when a high-contrast theme is
/// on, `None` when it is off. Re-read on `WM_THEMECHANGED` /
/// `WM_SETTINGCHANGE`, which Windows sends when the theme flips.
#[cfg(windows)]
pub fn high_contrast_colors() -> Option<HighContrastColors> {
    use windows::Win32::Graphics::Gdi::{
        GetSysColor, COLOR_GRAYTEXT, COLOR_HIGHLIGHT, COLOR_HIGHLIGHTTEXT, COLOR_WINDOW,
        COLOR_WINDOWTEXT,
    };
    use windows::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
    use windows::Win32::UI::WindowsAndMessaging::{
        SystemParametersInfoW, SPI_GETHIGHCONTRAST, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS,
    };
    let mut info = HIGHCONTRASTW {
        cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32,
        ..Default::default()
    };
    // SAFETY: the documented query — `cbSize` set, the struct's own size
    // passed, written in place.
    let queried = unsafe {
        SystemParametersInfoW(
            SPI_GETHIGHCONTRAST,
            info.cbSize,
            Some(&mut info as *mut HIGHCONTRASTW as *mut _),
            SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        )
    };
    if queried.is_err() || !info.dwFlags.contains(HCF_HIGHCONTRASTON) {
        return None;
    }
    // COLORREF is `0x00BBGGRR`.
    let sys = |index| {
        // SAFETY: a plain query returning an integer.
        let color = unsafe { GetSysColor(index) };
        (
            (color & 0xFF) as u8,
            ((color >> 8) & 0xFF) as u8,
            ((color >> 16) & 0xFF) as u8,
        )
    };
    Some(HighContrastColors {
        window: sys(COLOR_WINDOW),
        window_text: sys(COLOR_WINDOWTEXT),
        highlight: sys(COLOR_HIGHLIGHT),
        highlight_text: sys(COLOR_HIGHLIGHTTEXT),
        gray_text: sys(COLOR_GRAYTEXT),
    })
}

#[cfg(not(windows))]
pub fn high_contrast_colors() -> Option<HighContrastColors> {
    None
}

/// Paints a top-level window's title bar dark or light
/// (`DWMWA_USE_IMMERSIVE_DARK_MODE`): winit leaves the caption light whatever
/// the client area draws, and a light caption over a dark form is the one
/// thing that makes a window look foreign on Windows 11. Documented for
/// Windows 11 (build 22000+); Windows 10 20H1+ honours the same value in
/// practice — a compatibility target for the run-book, not a guarantee: a
/// refusal degrades to the light caption and a debug log. `hwnd` is the raw
/// handle (`raw_window_handle::Win32WindowHandle::hwnd`).
#[cfg(windows)]
pub fn set_dark_title_bar(hwnd: isize, is_dark: bool) {
    use windows::core::BOOL;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
    let value = BOOL(i32::from(is_dark));
    // SAFETY: a BOOL of the size passed, on a window handle winit owns for
    // the life of the app.
    let result = unsafe {
        DwmSetWindowAttribute(
            HWND(hwnd as *mut _),
            DWMWA_USE_IMMERSIVE_DARK_MODE,
            &value as *const BOOL as *const _,
            std::mem::size_of::<BOOL>() as u32,
        )
    };
    if let Err(error) = result {
        log::debug!("platform.dark_title_bar_unsupported error={error}");
    }
}

#[cfg(not(windows))]
pub fn set_dark_title_bar(_hwnd: isize, _is_dark: bool) {}

/// `%WINDIR%\Fonts` — where the system's own faces live (Segoe UI Variable,
/// Segoe Fluent Icons). `None` off Windows or with no `WINDIR`.
pub fn system_fonts_directory() -> Option<std::path::PathBuf> {
    let windir = std::env::var_os("WINDIR").or_else(|| std::env::var_os("SystemRoot"))?;
    Some(std::path::PathBuf::from(windir).join("Fonts"))
}
