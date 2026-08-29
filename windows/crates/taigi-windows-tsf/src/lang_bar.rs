//! The tray button and its menu — the macOS input-source menu, exactly:
//! 設定 / separator / 檢查更新 (`TaigiInputController.swift:264-329`;
//! roadmap W6). The button sits in the standard input-mode slot
//! (`GUID_LBI_INPUTMODE`, rakukan `language_bar.rs:21-23`).

// 中文: 語言列(系統匣)按鈕與選單:設定 / 分隔線 / 檢查更新,與 macOS 輸入來源選單一致。

use crate::guids::CLSID_TEXT_SERVICE;
use crate::module::instance;
use crate::registration::SERVICE_DESCRIPTION;
use crate::wide::{fill_fixed, to_wide};
use taigi_windows_core::keys::ShortcutAction;
use taigi_windows_core::settings::SettingsDocument;
use taigi_windows_core::strings::{StringKey, StringResolver};
use windows::core::{Result, PCWSTR};
use windows::Win32::Graphics::Gdi::HBITMAP;
use windows::Win32::UI::TextServices::{
    ITfMenu, GUID_LBI_INPUTMODE, TF_LANGBARITEMINFO, TF_LBI_STYLE_BTN_MENU,
    TF_LBI_STYLE_SHOWNINTRAY, TF_LBMENUF_SEPARATOR,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CopyIcon, LoadIconW, LoadImageW, HICON, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTSIZE,
};

/// Menu command ids `OnMenuSelect` receives back.
pub const MENU_OPEN_SETTINGS: u32 = 1;
pub const MENU_CHECK_FOR_UPDATES: u32 = 2;
/// The one cookie `ITfSource::AdviseSink` hands out for the lang-bar sink.
pub const LANG_BAR_SINK_COOKIE: u32 = 0x5461_6967;
/// The DLL icon resource the installer build adds (PR10); index 1.
const ICON_RESOURCE_ID: u16 = 1;
/// What the tray shows when it draws text instead of the icon.
pub const TRAY_TEXT: &str = "台";

pub fn item_info() -> TF_LANGBARITEMINFO {
    let mut info = TF_LANGBARITEMINFO {
        clsidService: CLSID_TEXT_SERVICE,
        guidItem: GUID_LBI_INPUTMODE,
        dwStyle: TF_LBI_STYLE_BTN_MENU | TF_LBI_STYLE_SHOWNINTRAY,
        ulSort: 0,
        szDescription: [0; 32],
    };
    fill_fixed(&mut info.szDescription, SERVICE_DESCRIPTION);
    info
}

/// The rows, in order, as (id, label) — `None` is a separator. Pure, so the
/// menu is testable without an `ITfMenu`. The 設定 row prints the chord the
/// user last recorded on it, tab-separated the way Win32 menus place
/// accelerators — `ITfMenu::AddMenuItem` documents only a text buffer, so
/// whether the language bar renders the tab as an accelerator column or
/// literally is a Windows DOGFOOD item (run-book); 檢查更新 carries none by
/// design.
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

pub fn populate(menu: &ITfMenu, rows: &[Option<(u32, String)>]) -> Result<()> {
    for row in rows {
        // SAFETY: `menu` is the live ITfMenu TSF handed InitMenu; the text
        // slice outlives the call; no bitmaps, no submenu out-pointer.
        unsafe {
            match row {
                Some((id, label)) => {
                    let text = to_wide(label);
                    menu.AddMenuItem(
                        *id,
                        0,
                        HBITMAP::default(),
                        HBITMAP::default(),
                        &text,
                        std::ptr::null_mut(),
                    )?;
                }
                None => {
                    menu.AddMenuItem(
                        0,
                        TF_LBMENUF_SEPARATOR,
                        HBITMAP::default(),
                        HBITMAP::default(),
                        &[],
                        std::ptr::null_mut(),
                    )?;
                }
            }
        }
    }
    Ok(())
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
        assert_eq!(
            item_info().dwStyle,
            TF_LBI_STYLE_BTN_MENU | TF_LBI_STYLE_SHOWNINTRAY
        );
    }
}
