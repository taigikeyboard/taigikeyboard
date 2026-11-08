//! A non-activating popup window (`WS_POPUP`, `WS_EX_TOPMOST | TOOLWINDOW |
//! NOACTIVATE`; khiin `candidate_window.rs:62-64`), created inside a
//! per-monitor-v2 thread DPI scope (an in-proc DLL must not change the host
//! process's DPI context, roadmap W4) and placed inside one, rounded by DWM
//! with its show / hide fade forced off (`DWMWA_TRANSITIONS_FORCEDISABLED`),
//! shown with `SW_SHOWNA` so the host's caret keeps blinking. Messages are
//! forwarded to a [`WindowHandler`] whose box the [`PopupWindow`] OWNS — the
//! HWND only borrows a pointer to it, so creation failure, `WM_NCDESTROY` and
//! `destroy` cannot free it twice. RULE: a window procedure never calls
//! `RequestEditSession` (W3).

use crate::module::instance;
use crate::wide::to_wide_nul;
use std::cell::RefCell;
use std::ffi::c_void;
use std::rc::Rc;
use taigi_desktop_core::composing::ContextToken;
use windows::core::{Result, BOOL, PCWSTR};
use windows::Win32::Foundation::{
    GetLastError, ERROR_CLASS_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM,
};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_ROUND,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, EndPaint, GetMonitorInfoW, InvalidateRect, MonitorFromPoint, MonitorFromWindow,
    HBRUSH, HMONITOR, MONITORINFO, MONITOR_DEFAULTTONEAREST, PAINTSTRUCT,
};
use windows::Win32::UI::HiDpi::{
    GetDpiForMonitor, GetDpiForWindow, SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT,
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, MDT_EFFECTIVE_DPI,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetWindowLongPtrW, IsWindowVisible, KillTimer,
    LoadCursorW, PostMessageW, RegisterClassExW, SetTimer, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, GWLP_USERDATA, HWND_TOPMOST, IDC_ARROW,
    MA_NOACTIVATE, SWP_NOACTIVATE, SW_HIDE, SW_SHOWNA, WM_APP, WM_CAPTURECHANGED, WM_DPICHANGED,
    WM_DWMCOLORIZATIONCOLORCHANGED, WM_ERASEBKGND, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEACTIVATE,
    WM_MOUSEWHEEL, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_SETTINGCHANGE, WM_THEMECHANGED,
    WM_TIMER, WNDCLASSEXW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

/// 96 DPI = one DIP per pixel.
pub const BASE_DPI: f32 = 96.0;

/// Posted by a focus / context callback that may not touch the window
/// synchronously (W3): "hide, if `wparam` (a context token, 0 = whoever)
/// still owns you". Handled on the message loop like any other message.
const WM_HIDE_REQUEST: u32 = WM_APP + 1;
/// Posted by a focus callback that may not show a window synchronously
/// (W3): "bring yourself in line with the settings as they are now".
const WM_SYNC_REQUEST: u32 = WM_APP + 2;

/// What a popup asks its content to do. Coordinates are DIPs.
pub trait WindowHandler {
    /// Draw everything (inside `BeginPaint` / `EndPaint`).
    fn paint(&mut self, window: &WindowRef);
    /// A left-button press at `point` (client DIPs). Only the TPS key panel
    /// answers one; every other popup acts on the release alone.
    fn press(&mut self, _window: &WindowRef, _point: (f32, f32)) {}
    /// A left-button release at `point` (client DIPs).
    fn click(&mut self, window: &WindowRef, point: (f32, f32));
    /// The mouse this window captured went to another (`WM_CAPTURECHANGED`).
    fn press_cancelled(&mut self, _window: &WindowRef) {}
    /// Wheel `delta` in notches (positive = away from the user).
    fn wheel(&mut self, window: &WindowRef, delta: f32);
    /// The window's DPI changed (it moved to another monitor, or the
    /// monitor's scale changed). `suggested` is the OS's scaled frame
    /// (screen pixels), the placement to use when the content cannot
    /// re-anchor itself.
    fn dpi_changed(&mut self, window: &WindowRef, dpi: f32, suggested: RECT);
    /// The timer `id` fired.
    fn timer(&mut self, window: &WindowRef, id: usize);
    /// A hide posted by a focus / context callback; `owner` names the
    /// context whose list should come down (`None` = whichever is up).
    fn hide_requested(&mut self, window: &WindowRef, owner: Option<ContextToken>);
    /// The system theme or accent changed (`WM_SETTINGCHANGE`,
    /// `WM_THEMECHANGED`, `WM_DWMCOLORIZATIONCOLORCHANGED`).
    fn system_theme_changed(&mut self);
    /// A sync posted by a focus callback: show or hide as the content's
    /// own source now says. The TPS key panel and the Telex guide post one.
    fn sync_requested(&mut self, _window: &WindowRef) {}
}

type Handler = Rc<RefCell<dyn WindowHandler>>;

/// A handle to a popup: what a handler callback and the owner both talk to
/// the HWND through. Copyable; carries no ownership.
#[derive(Clone, Copy)]
pub struct WindowRef {
    hwnd: HWND,
}

impl WindowRef {
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// The window's DPI, or 96 before it has one.
    pub fn dpi(&self) -> f32 {
        // SAFETY: a query on our own window.
        let dpi = unsafe { GetDpiForWindow(self.hwnd) };
        if dpi == 0 {
            BASE_DPI
        } else {
            dpi as f32
        }
    }

    /// Places the window at `frame` (screen PIXELS) without activating it,
    /// re-asserts topmost, and shows it if hidden. Under the per-monitor-v2
    /// scope: `SetWindowPos` reads its coordinates in the CALLING thread's DPI
    /// context, and the key path runs on the host's thread — a DPI-unaware
    /// host scaled the physical frame a second time (125% put the list 1.25×
    /// off the caret, enlarged). Window-procedure callers are already in the
    /// window's context; the nested scope changes nothing for them.
    pub fn show_at(&self, frame: RECT) {
        // SAFETY: our own window; topmost, no activation.
        with_per_monitor_dpi(|| unsafe {
            SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                frame.left,
                frame.top,
                (frame.right - frame.left).max(1),
                (frame.bottom - frame.top).max(1),
                SWP_NOACTIVATE,
            )
            .ok();
            let _ = ShowWindow(self.hwnd, SW_SHOWNA);
            let _ = InvalidateRect(Some(self.hwnd), None, false);
        });
    }

    pub fn hide(&self) {
        // SAFETY: our own window.
        let _ = unsafe { ShowWindow(self.hwnd, SW_HIDE) };
    }

    pub fn invalidate(&self) {
        // SAFETY: our own window.
        let _ = unsafe { InvalidateRect(Some(self.hwnd), None, false) };
    }

    pub fn set_timer(&self, id: usize, milliseconds: u32) {
        // SAFETY: a WM_TIMER on our own window.
        unsafe { SetTimer(Some(self.hwnd), id, milliseconds, None) };
    }

    /// Takes the mouse until the button comes up (`WM_LBUTTONUP` lets it go).
    pub fn capture_mouse(&self) {
        // SAFETY: our own window, on its own thread.
        unsafe { SetCapture(self.hwnd) };
    }

    pub fn is_visible(&self) -> bool {
        // SAFETY: a plain query of our own window.
        unsafe { IsWindowVisible(self.hwnd) }.as_bool()
    }

    pub fn kill_timer(&self, id: usize) {
        // SAFETY: our own window.
        unsafe { KillTimer(Some(self.hwnd), id).ok() };
    }

    /// Queues a hide for the message loop — the one thing a focus /
    /// context callback may do to the window (W3). `owner` = the context
    /// whose list should come down; `None` = whichever is up.
    pub fn post_hide_request(&self, owner: Option<ContextToken>) {
        let token = owner.map_or(0, |token| token.0);
        // SAFETY: a posted message to our own window; nothing is borrowed.
        if let Err(error) =
            unsafe { PostMessageW(Some(self.hwnd), WM_HIDE_REQUEST, WPARAM(token), LPARAM(0)) }
        {
            log::warn!("ui.post_hide_failed error={error}");
        }
    }

    /// Queues a sync for the message loop — what a focus callback calls to
    /// bring a window up (W3), never the show itself.
    pub fn post_sync_request(&self) {
        // SAFETY: a posted message to our own window; nothing is borrowed.
        if let Err(error) =
            unsafe { PostMessageW(Some(self.hwnd), WM_SYNC_REQUEST, WPARAM(0), LPARAM(0)) }
        {
            log::warn!("ui.post_sync_failed error={error}");
        }
    }
}

/// The popup and its handler. Owns both: dropping (or `destroy`) takes the
/// HWND down first, then the handler box the HWND was pointing at.
pub struct PopupWindow {
    handle: WindowRef,
    /// Boxed so its address is stable for the HWND's lifetime; the HWND
    /// stores `&*handler` at `WM_NCCREATE` and forgets it at `WM_NCDESTROY`.
    _handler: Box<Handler>,
}

impl std::ops::Deref for PopupWindow {
    type Target = WindowRef;

    fn deref(&self) -> &WindowRef {
        &self.handle
    }
}

impl PopupWindow {
    /// Registers `class_name` (once per process) and creates a hidden popup
    /// whose messages go to `handler`.
    pub fn create(class_name: &str, handler: Handler) -> Result<Self> {
        let class = to_wide_nul(class_name);
        // SAFETY: class registration with a static procedure and the stock
        // arrow cursor. The class outlives the DLL on purpose
        // (`DllCanUnloadNow` = S_FALSE), so a second activation finds it
        // registered — the one failure that is not a failure.
        unsafe {
            let cursor = LoadCursorW(None, IDC_ARROW).unwrap_or_default();
            let class_info = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(window_procedure),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: instance(),
                hIcon: Default::default(),
                hCursor: cursor,
                hbrBackground: HBRUSH::default(),
                lpszMenuName: PCWSTR::null(),
                lpszClassName: PCWSTR(class.as_ptr()),
                hIconSm: Default::default(),
            };
            if RegisterClassExW(&class_info) == 0 {
                let error = GetLastError();
                if error != ERROR_CLASS_ALREADY_EXISTS {
                    log::warn!("ui.register_class_failed class={class_name} error={error:?}");
                }
            }
        }
        let handler: Box<Handler> = Box::new(handler);
        // A BORROWED pointer: the box stays owned by the `PopupWindow`
        // returned below (or dropped on failure), never by the HWND.
        let param = &*handler as *const Handler;
        // SAFETY: the class is registered above and `param` outlives the
        // HWND (see `_handler`); the window takes the scope's per-monitor-v2
        // awareness, fixed at creation.
        let created = with_per_monitor_dpi(|| unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                PCWSTR(class.as_ptr()),
                PCWSTR::null(),
                WS_POPUP,
                0,
                0,
                1,
                1,
                None,
                None,
                Some(instance()),
                Some(param as *const c_void),
            )
        });
        // On failure the HWND is gone (WM_NCDESTROY cleared its pointer, or
        // WM_NCCREATE never ran) and `handler` is dropped here, once.
        let hwnd = created?;
        Ok(Self {
            handle: WindowRef { hwnd },
            _handler: handler,
        })
    }

    /// Destroys the window; the handler box follows when `self` drops,
    /// after the HWND can no longer dispatch to it.
    pub fn destroy(self) {
        // SAFETY: our own window; WM_NCDESTROY clears the borrowed pointer.
        unsafe { DestroyWindow(self.handle.hwnd).ok() };
    }
}

/// The monitor holding `point` (screen pixels): its work area and DPI.
pub struct MonitorArea {
    pub work_area: RECT,
    pub dpi: f32,
}

/// Runs `body` under the per-monitor-v2 thread DPI context and puts the
/// host's context back afterwards. In-proc, every call runs on the host's
/// thread, and a DPI-unaware host's context reads screen coordinates, work
/// areas and monitor DPI in its virtualised 96-DPI space and creates
/// bitmap-stretched windows — so monitor queries, the popup's creation and
/// placement (`show_at`) and the tray menu (`lang_bar::show_popup`) all go
/// through here.
pub fn with_per_monitor_dpi<T>(body: impl FnOnce() -> T) -> T {
    // SAFETY: the context is thread-local; the scope restores it.
    let _scope = ThreadDpiScope(unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2)
    });
    body()
}

/// The thread DPI context [`with_per_monitor_dpi`] replaced, restored on
/// drop — on an unwind too: a panic in the body is caught at the COM
/// boundary, and the host's thread must not stay per-monitor-v2 after it.
/// The raw handle keeps it `!Send`, so it drops on the thread it changed.
struct ThreadDpiScope(DPI_AWARENESS_CONTEXT);

impl Drop for ThreadDpiScope {
    fn drop(&mut self) {
        // NULL = the switch failed and nothing changed. Not `is_invalid()`:
        // it also rejects -1, which is `DPI_AWARENESS_CONTEXT_UNAWARE`.
        if self.0 .0.is_null() {
            return;
        }
        // SAFETY: restores the context this thread had before the scope.
        unsafe { SetThreadDpiAwarenessContext(self.0) };
    }
}

pub fn monitor_at(point: POINT) -> Option<MonitorArea> {
    // SAFETY: a monitor query by point.
    with_per_monitor_dpi(|| {
        monitor_area(unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) })
    })
}

/// The monitor holding most of `hwnd` — chosen by the window itself, so no
/// rectangle is read under the host's DPI virtualization.
pub fn monitor_of_window(hwnd: HWND) -> Option<MonitorArea> {
    // SAFETY: a monitor query by window.
    with_per_monitor_dpi(|| {
        monitor_area(unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) })
    })
}

/// `monitor`'s work area and DPI; call under [`with_per_monitor_dpi`].
///
/// `GetDpiForMonitor` follows the calling THREAD's context: measured
/// 2026-10-05 on Windows 11 at 120 DPI, an unaware process under the scope
/// gets 120 and a per-monitor-v2 process on an unaware thread gets 96. Its
/// Microsoft Learn reference (read 2026-10-05) still describes the answer
/// by process awareness and advises against calling it from a
/// per-monitor-aware thread, pointing to `GetDpiForWindow`; that cannot
/// stand in here — placement asks about the target monitor before the
/// window is on it.
fn monitor_area(monitor: HMONITOR) -> Option<MonitorArea> {
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // SAFETY: monitor queries with valid out-structs.
    unsafe {
        if !GetMonitorInfoW(monitor, &mut info).as_bool() {
            return None;
        }
        let (mut dpi_x, mut dpi_y) = (0u32, 0u32);
        let dpi = match GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) {
            Ok(()) if dpi_x != 0 => dpi_x as f32,
            _ => BASE_DPI,
        };
        Some(MonitorArea {
            work_area: info.rcWork,
            dpi,
        })
    }
}

/// A popup `size` DIPs big, centred across `monitor`'s work area: `bottom_margin`
/// DIPs above its bottom edge, or centred top to bottom when `None` — the
/// hotkey cards' placements (Telex guide, TPS key panel), which have no caret
/// to anchor to.
pub fn frame_in_work_area(
    monitor: &MonitorArea,
    size: (f32, f32),
    bottom_margin: Option<f32>,
) -> RECT {
    let scale = monitor.dpi / BASE_DPI;
    let work = monitor.work_area;
    let px_width = (size.0 * scale).round() as i32;
    let px_height = (size.1 * scale).round() as i32;
    let left = (work.left + work.right) / 2 - px_width / 2;
    let top = match bottom_margin {
        Some(margin) => work.bottom - (margin * scale).round() as i32 - px_height,
        None => (work.top + work.bottom) / 2 - px_height / 2,
    };
    RECT {
        left,
        top,
        right: left + px_width,
        bottom: top + px_height,
    }
}

fn handler_of(hwnd: HWND) -> Option<Handler> {
    // SAFETY: the pointer was stored by WM_NCCREATE and borrows a box the
    // owning `PopupWindow` keeps alive until after `DestroyWindow`; it is
    // cleared at WM_NCDESTROY, so a null read means "not ours (any more)".
    unsafe {
        let raw = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const Handler;
        if raw.is_null() {
            None
        } else {
            Some((*raw).clone())
        }
    }
}

/// `BeginPaint` / `EndPaint` as a pair that survives a panic in between.
struct PaintScope {
    hwnd: HWND,
    paint: PAINTSTRUCT,
}

impl PaintScope {
    fn begin(hwnd: HWND) -> Self {
        let mut paint = PAINTSTRUCT::default();
        // SAFETY: our own window; ended by `Drop`.
        unsafe { BeginPaint(hwnd, &mut paint) };
        Self { hwnd, paint }
    }
}

impl Drop for PaintScope {
    fn drop(&mut self) {
        // SAFETY: the paint `begin` started.
        let _ = unsafe { EndPaint(self.hwnd, &self.paint) };
    }
}

unsafe extern "system" fn window_procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // Every message is a boundary: a panic in a handler must not reach the
    // host's message loop. The payload is logged and dropped here.
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        handle_message(hwnd, message, wparam, lparam)
    }));
    match outcome {
        Ok(Some(result)) => result,
        // SAFETY: the default procedure for a message we did not handle.
        Ok(None) => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        Err(_) => {
            log::error!("ui.panic message={message:#x}");
            if message == WM_NCCREATE {
                // FALSE: the window is not created.
                LRESULT(0)
            } else {
                // SAFETY: as above — the default still answers sensibly.
                unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
            }
        }
    }
}

/// A mouse message's client point, in DIPs.
fn client_point(window: &WindowRef, lparam: LPARAM) -> (f32, f32) {
    let scale = window.dpi() / BASE_DPI;
    let x = (lparam.0 & 0xFFFF) as i16 as f32 / scale;
    let y = ((lparam.0 >> 16) & 0xFFFF) as i16 as f32 / scale;
    (x, y)
}

fn handle_message(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT> {
    let window = WindowRef { hwnd };
    match message {
        WM_NCCREATE => {
            // SAFETY: lparam is the CREATESTRUCTW CreateWindowExW built; its
            // lpCreateParams is the borrowed handler pointer.
            unsafe {
                let create = lparam.0 as *const CREATESTRUCTW;
                if !create.is_null() {
                    // `as _`, not `as isize`: on i686 the `windows` crate maps
                    // `SetWindowLongPtrW` onto `SetWindowLongW`, whose value
                    // is an `i32` — pointer-sized on that target too.
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*create).lpCreateParams as _);
                }
                let preference = DWMWCP_ROUND;
                DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_WINDOW_CORNER_PREFERENCE,
                    &preference as *const _ as *const c_void,
                    std::mem::size_of_val(&preference) as u32,
                )
                .ok();
                // DWM fades every top-level window in and out (~200 ms) on
                // show / hide; a candidate popup must appear and vanish on
                // the keystroke, so the transition is forced off.
                let no_transitions = BOOL(1);
                DwmSetWindowAttribute(
                    hwnd,
                    DWMWA_TRANSITIONS_FORCEDISABLED,
                    &no_transitions as *const _ as *const c_void,
                    std::mem::size_of_val(&no_transitions) as u32,
                )
                .ok();
            }
            None
        }
        WM_NCDESTROY => {
            // The pointer is only forgotten: the box belongs to the
            // `PopupWindow`, which outlives the HWND.
            // SAFETY: clearing our own window word.
            unsafe { SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) };
            None
        }
        WM_MOUSEACTIVATE => Some(LRESULT(MA_NOACTIVATE as isize)),
        WM_ERASEBKGND => Some(LRESULT(1)),
        WM_PAINT => {
            let handler = handler_of(hwnd)?;
            let painted = {
                let _scope = PaintScope::begin(hwnd);
                match handler.try_borrow_mut() {
                    Ok(mut handler) => {
                        handler.paint(&window);
                        true
                    }
                    Err(_) => false,
                }
            };
            if !painted {
                // The content was busy (a session in flight re-entered us):
                // the region was validated by EndPaint, so ask again.
                window.invalidate();
            }
            Some(LRESULT(0))
        }
        WM_LBUTTONDOWN => {
            // A content busy here (a re-entered session) misses the press,
            // and so its release types nothing — a click lost, never a
            // wrong one.
            let handler = handler_of(hwnd)?;
            let point = client_point(&window, lparam);
            if let Ok(mut handler) = handler.try_borrow_mut() {
                handler.press(&window, point);
            }
            Some(LRESULT(0))
        }
        WM_LBUTTONUP => {
            let handler = handler_of(hwnd)?;
            let point = client_point(&window, lparam);
            if let Ok(mut handler) = handler.try_borrow_mut() {
                handler.click(&window, point);
            }
            // After the click, with no borrow held: the release sends
            // WM_CAPTURECHANGED, which finds the press already ended.
            // SAFETY: a plain query and release on this thread.
            if unsafe { GetCapture() } == hwnd {
                let _ = unsafe { ReleaseCapture() };
            }
            Some(LRESULT(0))
        }
        WM_CAPTURECHANGED => {
            let handler = handler_of(hwnd)?;
            if let Ok(mut handler) = handler.try_borrow_mut() {
                handler.press_cancelled(&window);
            }
            Some(LRESULT(0))
        }
        WM_MOUSEWHEEL => {
            let handler = handler_of(hwnd)?;
            let delta = ((wparam.0 >> 16) & 0xFFFF) as i16 as f32 / 120.0;
            if let Ok(mut handler) = handler.try_borrow_mut() {
                handler.wheel(&window, delta);
            }
            Some(LRESULT(0))
        }
        WM_DPICHANGED => {
            let handler = handler_of(hwnd)?;
            let dpi = (wparam.0 & 0xFFFF) as f32;
            // SAFETY: lParam is the suggested RECT for the new DPI.
            let suggested = unsafe {
                let rect = lparam.0 as *const RECT;
                if rect.is_null() {
                    RECT::default()
                } else {
                    *rect
                }
            };
            if let Ok(mut handler) = handler.try_borrow_mut() {
                handler.dpi_changed(&window, dpi, suggested);
            }
            Some(LRESULT(0))
        }
        WM_TIMER => {
            let handler = handler_of(hwnd)?;
            if let Ok(mut handler) = handler.try_borrow_mut() {
                handler.timer(&window, wparam.0);
            }
            Some(LRESULT(0))
        }
        WM_HIDE_REQUEST => {
            let handler = handler_of(hwnd)?;
            let owner = (wparam.0 != 0).then_some(ContextToken(wparam.0));
            match handler.try_borrow_mut() {
                Ok(mut handler) => handler.hide_requested(&window, owner),
                // Busy: re-queued behind whatever is running.
                Err(_) => window.post_hide_request(owner),
            }
            Some(LRESULT(0))
        }
        WM_SYNC_REQUEST => {
            let handler = handler_of(hwnd)?;
            match handler.try_borrow_mut() {
                Ok(mut handler) => handler.sync_requested(&window),
                // Busy: re-queued behind whatever is running.
                Err(_) => window.post_sync_request(),
            }
            Some(LRESULT(0))
        }
        WM_SETTINGCHANGE | WM_THEMECHANGED | WM_DWMCOLORIZATIONCOLORCHANGED => {
            let handler = handler_of(hwnd)?;
            if let Ok(mut handler) = handler.try_borrow_mut() {
                handler.system_theme_changed();
            }
            window.invalidate();
            None
        }
        _ => None,
    }
}
