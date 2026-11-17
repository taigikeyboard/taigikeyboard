//! The tray button and its menu — the rows every desktop shares
//! (`taigi_desktop_core::keys::MENU`, the macOS `TaigiInputController.menu()`):
//! the global shortcuts a click can stand in for / separator /
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

use crate::com_guard::guarded;
use crate::guids::CLSID_TEXT_SERVICE;
use crate::mode_icons::MODE_ICON_RESOURCES;
use crate::module::instance;
use crate::product_name;
use crate::runtime::Runtime;
use crate::settings_launcher;
use crate::text_service::TextService_Impl;
use crate::ui::window;
use crate::wide::{fill_fixed, to_wide_nul};
use taigi_desktop_core::keys::{menu_rows, MenuCommand, MENU};
use taigi_desktop_core::mode_indicator::ModeIndicator;
use taigi_desktop_core::settings::SettingsDocument;
use taigi_desktop_core::strings::StringResolver;
use taigi_windows_platform::DESKTOP_PLATFORM;
use windows::core::{Error, IUnknown, Interface, Ref, BOOL, BSTR, GUID};
use windows::core::{Result, PCWSTR};
use windows::Win32::Foundation::{E_INVALIDARG, RECT};
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::System::Ole::{CONNECT_E_ADVISELIMIT, CONNECT_E_NOCONNECTION};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetActiveWindow, GetFocus};
use windows::Win32::UI::TextServices::{
    ITfLangBarItemButton_Impl, ITfLangBarItemSink, ITfLangBarItem_Impl, ITfMenu, ITfSource_Impl,
    TfLBIClick,
};
use windows::Win32::UI::TextServices::{
    GUID_LBI_INPUTMODE, TF_LANGBARITEMINFO, TF_LBI_STYLE_BTN_BUTTON, TF_LBI_STYLE_SHOWNINTRAY,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CopyIcon, CreatePopupMenu, DestroyMenu, GetSystemMetrics, LoadIconW, LoadImageW,
    TrackPopupMenuEx, HICON, HMENU, IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTCOLOR, MF_SEPARATOR,
    MF_STRING, SM_CXSMICON, SM_CYSMICON, TPM_NONOTIFY, TPM_RETURNCMD,
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
/// The DLL ICON resource drawn for `indicator` (`mode_icons.rs`).
fn icon_resource_id(indicator: ModeIndicator) -> Option<u16> {
    MODE_ICON_RESOURCES
        .iter()
        .find(|(_, name)| *name == indicator.icon_name())
        .map(|(id, _)| *id)
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
    menu_rows(strings, settings, DESKTOP_PLATFORM)
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
///
/// Tracked under [`window::with_per_monitor_dpi`]: `OnClick`'s point is in
/// physical pixels, which a DPI-unaware host's context read as 96-DPI ones
/// (a bitmap-stretched menu in the screen's corner). The host's windows keep
/// their own context in the modal loop — a window procedure runs in its
/// window's.
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
    let chosen = window::with_per_monitor_dpi(|| unsafe {
        TrackPopupMenuEx(
            menu.0,
            TPM_NONOTIFY.0 | TPM_RETURNCMD.0,
            point.x,
            point.y,
            owner,
            None,
        )
    });
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
/// the mode's own resource loaded without `LR_SHARED` at the small-icon
/// size (mozc `tip_lang_bar_menu.cc:98-111`), or — in a build whose
/// resources are missing — a `CopyIcon` of the stock application icon.
pub fn owned_icon(indicator: ModeIndicator) -> Result<HICON> {
    // SAFETY: plain metric reads.
    let (width, height) = unsafe { (GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON)) };
    let own = icon_resource_id(indicator).and_then(|id| {
        // SAFETY: LoadImageW with a resource id from this DLL's own
        // instance, no LR_SHARED, so the handle is the caller's to destroy.
        unsafe {
            LoadImageW(
                Some(instance()),
                PCWSTR(usize::from(id) as *const u16),
                IMAGE_ICON,
                width,
                height,
                LR_DEFAULTCOLOR,
            )
        }
        .ok()
    });
    match own {
        Some(handle) => Ok(HICON(handle.0)),
        // SAFETY: a stock system icon is shared; the copy is ours to hand over.
        None => unsafe { CopyIcon(LoadIconW(None, IDI_APPLICATION)?) },
    }
}

impl ITfLangBarItem_Impl for TextService_Impl {
    fn GetInfo(&self, pinfo: *mut TF_LANGBARITEMINFO) -> Result<()> {
        guarded("ITfLangBarItem::GetInfo", || {
            if pinfo.is_null() {
                return Err(Error::from_hresult(E_INVALIDARG));
            }
            // SAFETY: TSF passes a valid out-pointer to its own struct.
            unsafe { *pinfo = item_info() };
            Ok(())
        })
    }

    fn GetStatus(&self) -> Result<u32> {
        guarded("ITfLangBarItem::GetStatus", || Ok(0))
    }

    fn Show(&self, _fshow: BOOL) -> Result<()> {
        guarded("ITfLangBarItem::Show", || Ok(()))
    }

    fn GetTooltipString(&self) -> Result<BSTR> {
        guarded("ITfLangBarItem::GetTooltipString", || {
            Ok(BSTR::from(product_name::localized().as_str()))
        })
    }
}

impl ITfLangBarItemButton_Impl for TextService_Impl {
    /// Both mouse buttons raise the menu — this button has no mode to
    /// toggle, and the Mac opens its menu on a plain click
    /// (`TaigiInputController.menu()`). The rows are drawn here
    /// rather than through `InitMenu`: the taskbar input indicator never
    /// drives the TSF menu (this module's header).
    fn OnClick(&self, _click: TfLBIClick, pt: &POINT, _prcarea: *const RECT) -> Result<()> {
        guarded("ITfLangBarItemButton::OnClick", || {
            let runtime = Runtime::shared();
            let rows = popup_rows(&runtime.strings(), &runtime.settings.current());
            if let Some(id) = show_popup(&rows, *pt) {
                match menu_command(id) {
                    None => log::warn!("tsf.menu_unknown_id id={id}"),
                    Some(MenuCommand::OpenSettings) => {
                        // The picker comes down first, whoever raised it: this
                        // path never reaches the session, and the settings
                        // window taking focus is not guaranteed to end the
                        // context that owns the list.
                        self.hide_symbol_picker_now();
                        settings_launcher::open_settings();
                    }
                    Some(MenuCommand::CheckForUpdates) => {
                        // Also the settings window (on General): same doorway.
                        self.hide_symbol_picker_now();
                        settings_launcher::check_for_updates();
                    }
                    Some(MenuCommand::About) => {
                        // The settings window on About, the pane the sidebar
                        // does not list: same doorway.
                        self.hide_symbol_picker_now();
                        settings_launcher::open_about();
                    }
                    // A shortcut row does what its chord does, through the
                    // same doorway (`perform_global`), against the context
                    // focused NOW — resolved after the popup's modal loop,
                    // not before it — so the open list of that context is
                    // the one re-presented. No focused context is the
                    // `OnPreservedKey` case: identity 0, the switches still
                    // flip.
                    Some(MenuCommand::Shortcut(action)) => {
                        let identity = self.focused_context_identity();
                        self.perform_global(action, identity);
                    }
                }
            }
            Ok(())
        })
    }

    /// The declarative menu of a `TF_LBI_STYLE_BTN_MENU` button, which this
    /// item is not — `OnClick` draws its own. Answered as an empty menu
    /// rather than `E_NOTIMPL`: a host that probes it (the legacy desktop
    /// language bar) reads a failure as a broken item, and mozc leaves the
    /// same no-op `S_OK` for a non-menu button
    /// (`tip_lang_bar_menu.cc:619-624`).
    fn InitMenu(&self, _pmenu: Ref<ITfMenu>) -> Result<()> {
        guarded("ITfLangBarItemButton::InitMenu", || Ok(()))
    }

    /// The other half of the declarative menu; nothing selects from an
    /// empty one, and an unexpected id is not this item's error to raise.
    fn OnMenuSelect(&self, _wid: u32) -> Result<()> {
        guarded("ITfLangBarItemButton::OnMenuSelect", || Ok(()))
    }

    /// A caller-owned icon: TSF destroys what it is given. Re-read after
    /// every switch and whenever this thread gets the focus back
    /// (`notify_lang_bar` pushes the update), so it reads the settings file
    /// as it is now.
    fn GetIcon(&self) -> Result<HICON> {
        guarded("ITfLangBarItemButton::GetIcon", || {
            owned_icon(self.mode_indicator())
        })
    }

    /// The same state as text, for a host that draws text instead.
    fn GetText(&self) -> Result<BSTR> {
        guarded("ITfLangBarItemButton::GetText", || {
            Ok(BSTR::from(self.mode_indicator().symbol()))
        })
    }
}

impl ITfSource_Impl for TextService_Impl {
    /// TSF advises exactly one `ITfLangBarItemSink` on a lang-bar item; a
    /// second subscriber is refused rather than silently replacing the
    /// first, so every advise has a symmetric unadvise.
    fn AdviseSink(&self, riid: *const GUID, punk: Ref<IUnknown>) -> Result<u32> {
        guarded("ITfSource::AdviseSink", || {
            if riid.is_null() {
                return Err(Error::from_hresult(E_INVALIDARG));
            }
            // SAFETY: null-checked; TSF's own IID pointer.
            if unsafe { *riid } != ITfLangBarItemSink::IID {
                return Err(Error::from_hresult(CONNECT_E_NOCONNECTION));
            }
            // The QueryInterface happens before the borrow.
            let sink: ITfLangBarItemSink = punk.ok()?.cast()?;
            let mut state = self.state.borrow_mut();
            if state.lang_bar_sink.is_some() {
                drop(state);
                drop(sink);
                return Err(Error::from_hresult(CONNECT_E_ADVISELIMIT));
            }
            state.lang_bar_sink = Some(sink);
            Ok(LANG_BAR_SINK_COOKIE)
        })
    }

    fn UnadviseSink(&self, dwcookie: u32) -> Result<()> {
        guarded("ITfSource::UnadviseSink", || {
            if dwcookie != LANG_BAR_SINK_COOKIE {
                return Err(Error::from_hresult(CONNECT_E_NOCONNECTION));
            }
            // Taken under the borrow, released after it.
            let sink = self.state.borrow_mut().lang_bar_sink.take();
            drop(sink);
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taigi_desktop_core::keys::ShortcutAction;
    use taigi_desktop_core::strings::DisplayLanguage;

    #[test]
    fn every_indicator_has_its_own_icon_resource_after_the_app_icon() {
        let mut ids: Vec<u16> = ModeIndicator::ALL
            .iter()
            .map(|indicator| icon_resource_id(*indicator).expect("an icon resource"))
            .collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), ModeIndicator::ALL.len());
        assert!(ids.iter().all(|id| *id > 1), "id 1 is the app icon");
    }

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
        ShortcutAction::ToggleRomanization.store_in(&mut cleared, None, DESKTOP_PLATFORM);
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
