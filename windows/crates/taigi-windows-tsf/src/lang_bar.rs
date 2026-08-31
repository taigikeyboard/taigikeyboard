//! The tray button and its menu — the macOS input-source menu, exactly:
//! 設定 / separator / 檢查更新 (`TaigiInputController.swift:264-329`;
//! roadmap W6). The button sits in the standard input-mode slot
//! (`GUID_LBI_INPUTMODE`, rakukan `language_bar.rs:21-23`).
//!
//! The menu is DRAWN HERE, from `OnClick`, rather than declared through
//! TSF's `TF_LBI_STYLE_BTN_MENU` / `InitMenu`: the Windows 8+ taskbar input
//! indicator that hosts `GUID_LBI_INPUTMODE` routes a click to
//! `ITfLangBarItemButton::OnClick` and never drives the TSF menu, so a
//! menu-style button there answers a click with nothing at all (observed
//! 2026-08-31 on Windows 11). Both mainstream TIPs do it this way — mozc
//! registers its tray item as a non-menu button and builds a Win32 popup in
//! `OnClick` (`tip_lang_bar.cc:196-240`, `tip_lang_bar_menu.cc:243-330`),
//! and khiin-rs does the same (`lang_bar_indicator.rs:53-58,194-215`).
//! `InitMenu` is the legacy desktop language bar's path, off by default on
//! Windows 11.

// 中文: 語言列(系統匣)按鈕與選單:設定 / 分隔線 / 檢查更新,與 macOS 輸入來源選單一致。
// Windows 8 以後工作列的輸入指示器只會呼叫 OnClick,不會走 TSF 的 InitMenu,所以選單由這裡自己畫。

use crate::guids::CLSID_TEXT_SERVICE;
use crate::module::instance;
use crate::registration::{PRODUCT_NAME_STRING_ID, SERVICE_DESCRIPTION};
use crate::ui::window;
use crate::wide::{fill_fixed, to_wide_nul};
use taigi_windows_core::keys::ShortcutAction;
use taigi_windows_core::settings::SettingsDocument;
use taigi_windows_core::strings::{StringKey, StringResolver};
use windows::core::{Result, PCWSTR, PWSTR};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetActiveWindow, GetFocus};
use windows::Win32::UI::TextServices::{
    GUID_LBI_INPUTMODE, TF_LANGBARITEMINFO, TF_LBI_STYLE_BTN_BUTTON, TF_LBI_STYLE_SHOWNINTRAY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CopyIcon, CreatePopupMenu, DestroyMenu, LoadIconW, LoadImageW, LoadStringW,
    TrackPopupMenuEx, HICON, HMENU, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTSIZE, MF_SEPARATOR,
    MF_STRING, TPM_NONOTIFY, TPM_RETURNCMD,
};

/// Menu command ids `show_popup` answers with. `TPM_RETURNCMD` spells a
/// dismissed menu as 0, so an id of 0 would read as "the user chose
/// nothing" — the one rule a new row has to keep.
pub const MENU_OPEN_SETTINGS: u32 = 1;
pub const MENU_CHECK_FOR_UPDATES: u32 = 2;
const _: () = assert!(MENU_OPEN_SETTINGS != 0 && MENU_CHECK_FOR_UPDATES != 0);
/// The one cookie `ITfSource::AdviseSink` hands out for the lang-bar sink.
pub const LANG_BAR_SINK_COOKIE: u32 = 0x5461_6967;
/// The DLL icon resource the installer build adds (PR10); index 1.
const ICON_RESOURCE_ID: u16 = 1;
/// What the tray shows when it draws text instead of the icon.
pub const TRAY_TEXT: &str = "台";

/// The product name in the user's UI language — the DLL's own STRINGTABLE
/// (`build-support/resource.rs`), resolved by `LoadString` with Windows'
/// language fallback. The same string Windows Settings shows through the
/// indirect profile description, so the tray, its tooltip and Settings agree.
/// A DLL built without string resources answers the untranslated name.
pub fn product_name() -> String {
    let id: u32 = PRODUCT_NAME_STRING_ID
        .parse()
        .expect("TAIGI_PRODUCT_NAME_STRING_ID is a build-script integer");
    let mut buffer = [0u16; 128];
    // SAFETY: `buffer` is a writable UTF-16 buffer of the passed length; the
    // instance is this DLL's own (set in DllMain).
    let length = unsafe {
        LoadStringW(
            Some(instance()),
            id,
            PWSTR(buffer.as_mut_ptr()),
            buffer.len() as i32,
        )
    };
    if length <= 0 {
        return SERVICE_DESCRIPTION.to_owned();
    }
    String::from_utf16_lossy(&buffer[..length as usize])
}

pub fn item_info() -> TF_LANGBARITEMINFO {
    let mut info = TF_LANGBARITEMINFO {
        clsidService: CLSID_TEXT_SERVICE,
        guidItem: GUID_LBI_INPUTMODE,
        // A plain button, NOT `TF_LBI_STYLE_BTN_MENU`: the taskbar input
        // indicator gives a menu-style button's click to nobody (see the
        // module header). The rows come from `OnClick` → `show_popup`.
        dwStyle: TF_LBI_STYLE_BTN_BUTTON | TF_LBI_STYLE_SHOWNINTRAY,
        ulSort: 0,
        szDescription: [0; 32],
    };
    fill_fixed(&mut info.szDescription, &product_name());
    info
}

/// The rows, in order, as (id, label) — `None` is a separator. Pure, so the
/// menu is testable without a live menu. The 設定 row prints the chord the
/// user last recorded on it, tab-separated: a Win32 menu draws what follows
/// a tab in its accelerator column, which is what `show_popup` builds.
/// 檢查更新 carries no chord by design.
pub fn menu_rows(
    strings: &StringResolver,
    settings: &SettingsDocument,
) -> Vec<Option<(u32, String)>> {
    let settings_label = match ShortcutAction::OpenLastSettingsPane.chord_in(settings) {
        Some(chord) => format!(
            "{}\t{}",
            strings.resolve(StringKey::DesktopMenuSettings),
            chord.display()
        ),
        None => strings.resolve(StringKey::DesktopMenuSettings).to_owned(),
    };
    vec![
        Some((MENU_OPEN_SETTINGS, settings_label)),
        None,
        Some((
            MENU_CHECK_FOR_UPDATES,
            strings.resolve(StringKey::DesktopUpdateCheckNow).to_owned(),
        )),
    ]
}

/// The window a popup is owned by: the focused window of the calling
/// thread, which inside a lang-bar callback is the host's own text window
/// (mozc passes `GetFocus()` the same way), and the thread's active window
/// when nothing holds focus. `TrackPopupMenuEx` refuses a null owner, so a
/// thread with neither gets no menu rather than a failed call.
fn popup_owner() -> Option<HWND> {
    // SAFETY: plain queries about the calling thread's own windows.
    let window = unsafe {
        let focused = GetFocus();
        if focused.is_invalid() {
            GetActiveWindow()
        } else {
            focused
        }
    };
    (!window.is_invalid()).then_some(window)
}

/// `point` with x pulled back inside the monitor's work area, so a menu
/// raised from the right-hand end of the taskbar is not drawn off-screen
/// (mozc `tip_lang_bar_menu.cc:311-322`).
fn clamped_to_work_area(point: POINT) -> POINT {
    let Some(area) = window::monitor_at(point) else {
        return point;
    };
    POINT {
        x: point.x.clamp(area.work_area.left, area.work_area.right),
        y: point.y,
    }
}

/// Draws `rows` as a Win32 popup at `point` and answers the id the user
/// chose, or `None` for a dismissed menu.
///
/// `TPM_RETURNCMD` hands the id back here instead of posting `WM_COMMAND`
/// to a window that has no handler for it, and `TPM_NONOTIFY` keeps the
/// owner from seeing the menu's own messages — a host that reacts to them
/// has been seen to change the menu's state underneath it (mozc's IE 10
/// note, `tip_lang_bar_menu.cc:308-310`). Alignment and button flags are
/// left at their defaults, which are all zero.
pub fn show_popup(rows: &[Option<(u32, String)>], point: POINT) -> Option<u32> {
    let owner = popup_owner()?;
    // SAFETY: a menu this call owns; `OwnedMenu` destroys it on every exit,
    // a panic through the COM guard included.
    let menu = OwnedMenu(unsafe { CreatePopupMenu() }.ok()?);
    for row in rows {
        // SAFETY: the menu is ours and still alive; each text buffer
        // outlives its call.
        let appended = unsafe {
            match row {
                Some((id, label)) => {
                    let text = to_wide_nul(label);
                    AppendMenuW(menu.0, MF_STRING, *id as usize, PCWSTR(text.as_ptr()))
                }
                None => AppendMenuW(menu.0, MF_SEPARATOR, 0, PCWSTR::null()),
            }
        };
        if let Err(error) = appended {
            log::warn!("lang_bar.menu_append_failed error={error}");
            return None;
        }
    }
    let point = clamped_to_work_area(point);
    // SAFETY: the menu is ours and filled; the owner is a live window of
    // the calling thread. This runs a nested modal message loop — nothing
    // of ours is borrowed across it, the rows are already owned values.
    let chosen = unsafe {
        TrackPopupMenuEx(
            menu.0,
            TPM_NONOTIFY.0 | TPM_RETURNCMD.0,
            point.x,
            point.y,
            owner,
            None,
        )
    };
    // `TPM_RETURNCMD` returns the chosen id, or 0 for a menu the user
    // dismissed (and for an error, which is the same nothing to do).
    (chosen.0 > 0).then_some(chosen.0 as u32)
}

/// A popup menu for as long as the call that built it.
struct OwnedMenu(HMENU);

impl Drop for OwnedMenu {
    fn drop(&mut self) {
        // SAFETY: destroying a menu this type owns, exactly once.
        unsafe { DestroyMenu(self.0).ok() };
    }
}

/// A CALLER-OWNED icon — `ITfLangBarItemButton::GetIcon`'s contract is
/// that TSF destroys what it is handed, so nothing shared may be returned:
/// the DLL's own resource loaded without `LR_SHARED`, or a `CopyIcon` of
/// the stock application icon until the resource ships (PR10).
pub fn owned_icon() -> Result<HICON> {
    // SAFETY: LoadImageW with a resource id from this DLL's own instance,
    // no LR_SHARED, so the handle is the caller's to destroy.
    let own = unsafe {
        LoadImageW(
            Some(instance()),
            PCWSTR(ICON_RESOURCE_ID as usize as *const u16),
            IMAGE_ICON,
            0,
            0,
            LR_DEFAULTSIZE,
        )
    };
    match own {
        Ok(handle) => Ok(HICON(handle.0)),
        // SAFETY: a stock system icon is shared; the copy is ours to hand over.
        Err(_) => unsafe { CopyIcon(LoadIconW(None, IDI_APPLICATION)?) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taigi_windows_core::strings::DisplayLanguage;

    #[test]
    fn the_menu_mirrors_the_macos_input_source_menu() {
        // trace: TaigiInputController.swift:264-329 → [[settings], [checkForUpdates]].
        let strings = StringResolver::new(DisplayLanguage::Hanji);
        let rows = menu_rows(&strings, &SettingsDocument::default());
        assert_eq!(rows.len(), 3);
        let (id, label) = rows[0].as_ref().unwrap();
        assert_eq!(*id, MENU_OPEN_SETTINGS);
        assert!(label.ends_with("\tCtrl+Shift+S"), "{label}");
        assert!(rows[1].is_none(), "a separator between the two groups");
        let (id, label) = rows[2].as_ref().unwrap();
        assert_eq!(*id, MENU_CHECK_FOR_UPDATES);
        assert!(
            !label.contains('\t'),
            "no chord on 檢查更新 by design: {label}"
        );
        let mut cleared = SettingsDocument::default();
        ShortcutAction::OpenLastSettingsPane.store_in(&mut cleared, None);
        let rows = menu_rows(&strings, &cleared);
        assert!(
            !rows[0].as_ref().unwrap().1.contains('\t'),
            "a cleared row prints no chord"
        );
        // A plain tray button, whose click reaches `OnClick`. A
        // `TF_LBI_STYLE_BTN_MENU` here shows no menu at all in the
        // Windows 8+ taskbar input indicator (module header).
        assert_eq!(
            item_info().dwStyle,
            TF_LBI_STYLE_BTN_BUTTON | TF_LBI_STYLE_SHOWNINTRAY
        );
    }
}
