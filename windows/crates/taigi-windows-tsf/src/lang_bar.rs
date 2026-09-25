//! The tray button and its menu — the rows every desktop shares
//! (`taigi_desktop_core::keys::MENU`, the macOS `TaigiInputController.menu()`):
//! the two global shortcuts a click can stand in for / separator /
//! TaigiKeyboard Settings / separator / Check for Updates, About the Keyboard (roadmap W6). The button sits in the standard input-mode slot
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

use crate::guids::CLSID_TEXT_SERVICE;
use crate::module::instance;
use crate::product_name;
use crate::ui::window;
use crate::wide::{fill_fixed, to_wide_nul};
use taigi_desktop_core::keys::{menu_rows, LanguageMode, MenuCommand, MENU};
use taigi_desktop_core::settings::SettingsDocument;
use taigi_desktop_core::strings::StringResolver;
use windows::core::{Result, PCWSTR};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetActiveWindow, GetFocus};
use windows::Win32::UI::TextServices::{
    GUID_LBI_INPUTMODE, TF_LANGBARITEMINFO, TF_LBI_STYLE_BTN_BUTTON, TF_LBI_STYLE_SHOWNINTRAY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CopyIcon, CreatePopupMenu, DestroyMenu, LoadIconW, LoadImageW, TrackPopupMenuEx,
    HICON, HMENU, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTSIZE, MF_SEPARATOR, MF_STRING,
    TPM_NONOTIFY, TPM_RETURNCMD,
};

/// A popup row's command id is its position in the shared list, from 1:
/// `TPM_RETURNCMD` spells a dismissed menu as 0, so no row may be 0. Never
/// stored — `show_popup` answers with it and [`menu_command`] reads it
/// straight back, so every row of `keys::MENU` has one and none can print one
/// command and fire another.
fn popup_id(index: usize) -> u32 {
    u32::try_from(index + 1).unwrap_or(u32::MAX)
}

/// The command a popup id stands for; `None` for 0 (dismissed) or a
/// separator.
pub fn menu_command(id: u32) -> Option<MenuCommand> {
    let index = usize::try_from(id).ok()?.checked_sub(1)?;
    MENU.get(index).copied().flatten()
}

/// The one cookie `ITfSource::AdviseSink` hands out for the lang-bar sink.
pub const LANG_BAR_SINK_COOKIE: u32 = 0x5461_6967;
/// The DLL icon resource the installer build adds (PR10); index 1.
const ICON_RESOURCE_ID: u16 = 1;
/// What the tray shows when it draws text instead of the icon: the script
/// being typed, in one character, as 微軟注音 (Microsoft Bopomofo) spells its own 中/英 state.
/// Not an i18n string — it names the script, so it reads the same in every UI
/// language.
pub fn tray_text(mode: LanguageMode) -> &'static str {
    match mode {
        LanguageMode::Taigi => "台",
        LanguageMode::English => "英",
    }
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
    fill_fixed(&mut info.szDescription, &product_name::localized());
    info
}

/// The rows, in order, as (id, label) — `None` is a separator. Pure, so the
/// menu is testable without a live menu. A row with a recorded chord prints
/// it tab-separated: a Win32 menu draws what follows a tab in its
/// accelerator column, which is what `show_popup` builds.
pub fn popup_rows(
    strings: &StringResolver,
    settings: &SettingsDocument,
) -> Vec<Option<(u32, String)>> {
    menu_rows(strings, settings)
        .into_iter()
        .enumerate()
        .map(|(index, row)| {
            row.map(|row| {
                let label = match row.chord {
                    Some(chord) => format!("{}\t{chord}", row.title),
                    None => row.title,
                };
                (popup_id(index), label)
            })
        })
        .collect()
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
/// the DLL's own resource loaded without `LR_SHARED`, or — in a build whose
/// resource is missing — a `CopyIcon` of the stock application icon.
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
    use taigi_desktop_core::keys::ShortcutAction;
    use taigi_desktop_core::strings::DisplayLanguage;

    #[test]
    fn every_row_prints_its_chord_after_a_tab_and_its_id_reads_back() {
        // trace: keys::MENU rows (the shared literal is asserted in
        // taigi-desktop-core); here only what Windows adds — the tab join
        // and the id → command round trip.
        let strings = StringResolver::new(DisplayLanguage::Hanji);
        let rows = popup_rows(&strings, &SettingsDocument::default());
        assert_eq!(rows.len(), MENU.len());
        assert_eq!(
            rows[0].as_ref().map(|(_, label)| label.as_str()),
            Some("切換台羅/白話字\tCtrl+Alt+C")
        );
        for (row, command) in rows.iter().zip(MENU) {
            match (row, command) {
                (Some((id, _)), Some(command)) => {
                    assert_ne!(*id, 0);
                    assert_eq!(menu_command(*id), Some(command));
                }
                (None, None) => {}
                _ => panic!("a separator drawn where a row belongs, or the reverse"),
            }
        }
        assert_eq!(menu_command(0), None, "a dismissed popup");
        let mut cleared = SettingsDocument::default();
        ShortcutAction::ToggleRomanization.store_in(&mut cleared, None);
        let rows = popup_rows(&strings, &cleared);
        assert_eq!(rows[0].as_ref().unwrap().1, "切換台羅/白話字");
        // A plain tray button, whose click reaches `OnClick`. A
        // `TF_LBI_STYLE_BTN_MENU` here shows no menu at all in the
        // Windows 8+ taskbar input indicator (module header).
        assert_eq!(
            item_info().dwStyle,
            TF_LBI_STYLE_BTN_BUTTON | TF_LBI_STYLE_SHOWNINTRAY
        );
    }
}
