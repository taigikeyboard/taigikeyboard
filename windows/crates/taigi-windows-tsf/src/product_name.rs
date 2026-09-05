//! This DLL's own name: the untranslated fallback, the STRINGTABLE id the
//! build script wrote the translations under, and the localized name Windows
//! resolves against the user's UI language. Every surface that shows the
//! product's name reads it from here — the tray button and its tooltip
//! (`lang_bar`, `text_service`), the CLSID key and the TSF profile
//! (`registration`) — so none of them owns it.

// DLL 家己的名:未翻譯的退路、字串資源 id、以及照使用者介面語言解出來的在地化名稱。

use crate::module::instance;
use windows::core::PWSTR;
use windows::Win32::UI::WindowsAndMessaging::LoadStringW;

/// The untranslated name, and what every reader that cannot resolve a resource
/// falls back to: the CLSID key's default value, the TSF profile's plain
/// `Description`, and the tray on a DLL built without string resources (no
/// resource compiler on PATH). The Mac's unlocalized `CFBundleName`.
pub const UNTRANSLATED: &str = "TaigiKeyboard";

/// The STRINGTABLE id the build script stores the localized product name
/// under, in every language `product_name_strings.rs` carries
/// (`build-support/resource.rs`, exported as this env var so the two cannot
/// drift). `windows/scripts/lib/identity.sh` reads the same constant out of
/// that build script and hands it to the installer.
pub fn string_id() -> u32 {
    env!("TAIGI_PRODUCT_NAME_STRING_ID")
        .parse()
        .expect("TAIGI_PRODUCT_NAME_STRING_ID is a build-script integer")
}

/// The product name in the user's UI language — the DLL's own STRINGTABLE
/// (`build-support/resource.rs`), resolved by `LoadString` with Windows'
/// language fallback. The same resource Windows Settings resolves through the
/// profile's display name, so the tray, its tooltip and Settings agree.
pub fn localized() -> String {
    let mut buffer = [0u16; 128];
    // SAFETY: `buffer` is a writable UTF-16 buffer of the passed length; the
    // instance is this DLL's own (set in DllMain).
    let length = unsafe {
        LoadStringW(
            Some(instance()),
            string_id(),
            PWSTR(buffer.as_mut_ptr()),
            buffer.len() as i32,
        )
    };
    if length <= 0 {
        return UNTRANSLATED.to_owned();
    }
    String::from_utf16_lossy(&buffer[..length as usize])
}
