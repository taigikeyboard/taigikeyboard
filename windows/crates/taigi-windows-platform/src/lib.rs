//! The few Win32 calls both the DLL and the settings window need — the
//! user's locale, opening a URL, the alert sound — behind plain functions.
//! On a non-Windows host (the macOS build gate, `make check`) each answers
//! the neutral value, so the callers compile and test natively.

// 中文: DLL 與設定視窗共用的少量 Win32 呼叫;非 Windows 主機給中性值,讓呼叫端在 macOS 上可測。

/// The user's locale name, e.g. `zh-TW` / `ja-JP` / `en-US`, for the
/// `system` display language. Empty when the platform cannot say.
#[cfg(windows)]
pub fn system_locale() -> String {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buffer = [0u16; 85];
    // SAFETY: `buffer` is a valid writable UTF-16 buffer of the passed
    // length (LOCALE_NAME_MAX_LENGTH is 85).
    let length = unsafe { GetUserDefaultLocaleName(&mut buffer) };
    if length <= 1 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..(length as usize).saturating_sub(1)])
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
#[cfg(windows)]
pub fn beep() {
    use windows::Win32::System::Diagnostics::Debug::MessageBeep;
    use windows::Win32::UI::WindowsAndMessaging::MB_OK;
    // SAFETY: a fire-and-forget system call with a constant.
    let _ = unsafe { MessageBeep(MB_OK) };
}

#[cfg(not(windows))]
pub fn beep() {}

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

/// The directory the running executable lives in — the install directory
/// for the settings window (the DLL resolves its own from its `HMODULE`).
pub fn executable_directory() -> Option<std::path::PathBuf> {
    std::env::current_exe()
        .ok()?
        .parent()
        .map(std::path::Path::to_path_buf)
}
