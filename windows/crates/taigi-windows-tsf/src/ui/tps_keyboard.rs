//! The on-screen TPS key panel (desktop TPS roadmap D6): every key of the
//! main block with the glyph it types under TPS and its Shift-layer glyph,
//! up while TPS is typed and the user asked for it (`tpsKeyboardShown`).
//! A click on a cap types its glyph — the Shift glyph from the cap's top half
//! or with Shift held — by injecting the one key that names it
//! (`taigi_windows_platform::tps_keyboard_click`), which the key sink types
//! (`TextService_Impl::tps_keyboard_click`): a window procedure may not ask
//! for an edit session (W3).
//!
//! One per service activation, owned by no context: the stored key is all
//! that crosses activations and processes, and only the activation with
//! keyboard focus shows it. Whether it should be up comes from a source the
//! text service installs, so the posted sync a focus callback sends (W3) reads
//! the focus and the settings as they are when the message arrives, with no
//! hold on the service. Placed at the bottom centre of the work area of the monitor
//! the foreground window is on — where an on-screen keyboard sits, clear of
//! most of the text being typed; the hotkey and focus paths have no caret to
//! anchor to. A key the engine took — a click included — flashes its cap
//! (`TpsKeyboard::flash`).

use super::render::{DWriteMeasurer, RenderFactory, Surface};
use super::theme::{SystemTheme, Theme};
use super::window::{
    frame_in_work_area, monitor_of_window, MonitorArea, PopupWindow, WindowHandler, WindowRef,
    BASE_DPI,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};
use taigi_desktop_core::candidates::{FontSpec, TextMeasurer};
use taigi_desktop_core::composing::ContextToken;
use taigi_desktop_core::keys::{tps_keyboard_rows, TpsKeyCap, TpsKeyCapIndex, TpsKeyboardRow};
use taigi_desktop_core::settings::{AppearanceMode, CandidateFontChoice, CandidateFontSelection};
use taigi_windows_platform::tps_keyboard_click::{
    scan_code_for_glyph, TPS_KEYBOARD_CLICK_VIRTUAL_KEY,
};
use windows::Win32::Foundation::RECT;
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F, D2D_SIZE_U};
use windows::Win32::Graphics::Direct2D::{
    ID2D1HwndRenderTarget, ID2D1SolidColorBrush, D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
    D2D1_ROUNDED_RECT,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS,
    KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
use windows_numerics::Vector2;

const WINDOW_CLASS: &str = "TaigiKeyboardTpsKeyboard";
/// A key cap's side and the gap between caps, in DIPs. Mirrors
/// `TpsKeyboardPanel.capSize` / `capGap` on macOS.
const CAP_SIZE: f32 = 48.0;
const CAP_GAP: f32 = 6.0;
const CAP_CORNER_RADIUS: f32 = 6.0;
const CAP_STROKE: f32 = 1.0;
/// The inset of the cap's corner labels from its edges.
const CAP_INSET: f32 = 4.0;
const PADDING: f32 = 12.0;
/// How far the panel sits above the bottom of the work area.
const BOTTOM_MARGIN: f32 = 24.0;
const GLYPH_FONT_SIZE: f32 = 22.0;
const SHIFT_GLYPH_FONT_SIZE: f32 = 13.0;
const LABEL_FONT_SIZE: f32 = 11.0;
/// The glyphs draw in a bundled face that holds every glyph the layout
/// types, ㆻ (U+31BB, Unicode 13) included — the system face lacks it (a
/// tofu box on the dev box, 2026-10-04) and jf open 粉圓 lacks ˪ ˫. Iansui,
/// whose family name `render.rs` `bundled_family_name` resolves.
const GLYPH_FONT: CandidateFontChoice = CandidateFontChoice::Iansui;
/// How long a typed key's cap stays lit. Mirrors
/// `TpsKeyboardPanel.flashDuration` on macOS.
const FLASH_MILLISECONDS: u32 = 150;
const FLASH_TIMER_ID: usize = 1;
/// One system timer tick (~15.6 ms): a `WM_TIMER` due at the flash's end can
/// arrive that much before it by `Instant`, and still ends it.
const TIMER_TICK_SLACK: Duration = Duration::from_millis(16);

/// The appearance to draw the panel in as focus and the settings stand now,
/// or `None` when it should not be up (`SettingsDocument::is_tps_keyboard_wanted`).
pub type TpsKeyboardSource = Box<dyn Fn() -> Option<AppearanceMode>>;

/// The panel's window; its content lives in the window's handler, shared
/// here so a key can flash a cap.
pub struct TpsKeyboard {
    content: Rc<RefCell<PanelContent>>,
    window: Option<PopupWindow>,
}

/// What one frame draws with (`Surface::frame`'s pair).
struct Pen<'a> {
    target: &'a ID2D1HwndRenderTarget,
    brush: &'a ID2D1SolidColorBrush,
}

struct PanelContent {
    factory: Rc<RenderFactory>,
    source: TpsKeyboardSource,
    /// The core's rows, fixed for the life of the process.
    rows: Vec<TpsKeyboardRow>,
    appearance: AppearanceMode,
    theme: Theme,
    dpi: f32,
    surface: Option<Surface>,
    /// The cap flashing now, and when its flash ends.
    flash: Option<(TpsKeyCapIndex, Instant)>,
    /// The cap the left button went down on, lit until the button comes up.
    pressed: Option<TpsKeyCapIndex>,
}

impl TpsKeyboard {
    /// Made with its (hidden) window, at activation: every later move is a
    /// posted sync, and a focus callback may neither create nor show a window
    /// (W3) — one hidden popup per activation is the price of a panel that
    /// can come up on focus alone.
    pub fn new(factory: Rc<RenderFactory>, source: TpsKeyboardSource) -> Self {
        let appearance = AppearanceMode::Auto;
        let panel = Rc::new(RefCell::new(PanelContent {
            factory,
            source,
            rows: tps_keyboard_rows(),
            appearance,
            theme: Theme::resolve(appearance, &SystemTheme::read()),
            dpi: BASE_DPI,
            surface: None,
            flash: None,
            pressed: None,
        }));
        let handler: Rc<RefCell<dyn WindowHandler>> = panel.clone();
        let window = PopupWindow::create(WINDOW_CLASS, handler)
            .inspect_err(|error| log::error!("ui.tps_keyboard_window_failed error={error}"))
            .ok();
        Self {
            content: panel,
            window,
        }
    }

    /// Lights `cap` for `FLASH_MILLISECONDS` — the engine just took its
    /// glyph (`IntentSurface::tps_keyboard_cap_typed`). One cap at a time: a
    /// new key moves the light. Nothing while the panel is down. Called from
    /// the key path after the edit session: it repaints and starts a timer,
    /// and shows nothing (W3).
    pub fn flash(&self, cap: TpsKeyCapIndex) {
        let Some(window) = self.window.as_deref() else {
            return;
        };
        if !window.is_visible() {
            return;
        }
        // The window procedure holds this borrow while it handles a message;
        // the key path normally runs between messages (an injected click
        // arrives as a later one). Should a host re-enter it there, the
        // flash is skipped — it only lights a cap.
        let Ok(mut content) = self.content.try_borrow_mut() else {
            log::debug!("ui.tps_keyboard_flash_skipped reason=busy");
            return;
        };
        let ends = Instant::now() + Duration::from_millis(u64::from(FLASH_MILLISECONDS));
        content.flash = Some((cap, ends));
        window.invalidate();
        window.set_timer(FLASH_TIMER_ID, FLASH_MILLISECONDS);
    }

    /// Queues a sync for the message loop: the panel shows or hides as its
    /// source says when the message runs, so whatever focus or setting moved
    /// in between, the last sync settles it. Every path asks this way — a
    /// focus callback must (W3), and a synchronous show could re-enter a
    /// focus callback that then finds the panel busy and is lost.
    pub fn request_sync(&self) {
        if let Some(window) = self.window.as_deref() {
            window.post_sync_request();
        }
    }

    pub fn destroy(&mut self) {
        if let Some(window) = self.window.take() {
            window.destroy();
        }
    }
}

impl PanelContent {
    /// Up at the bottom of the foreground window's monitor when the source
    /// answers content; down otherwise. With no foreground window to place
    /// it by, nothing changes until the next sync.
    fn sync(&mut self, window: &WindowRef) {
        let Some(appearance) = (self.source)() else {
            self.end_flash(window);
            self.pressed = None;
            window.hide();
            return;
        };
        let Some(monitor) = foreground_monitor() else {
            return;
        };
        self.appearance = appearance;
        self.theme = Theme::resolve(appearance, &SystemTheme::read());
        // The surface is sized in pixels: kept unless the scale moved.
        if monitor.dpi != self.dpi {
            self.dpi = monitor.dpi;
            self.surface = None;
        }
        window.show_at(bottom_frame(&monitor, panel_size(&self.rows)));
    }

    fn end_flash(&mut self, window: &WindowRef) {
        window.kill_timer(FLASH_TIMER_ID);
        if self.flash.take().is_some() {
            window.invalidate();
        }
    }

    fn draw_text(
        &self,
        pen: &Pen<'_>,
        text: &str,
        font: FontSpec,
        frame: (f32, f32, f32, f32),
        centered: bool,
        colour: D2D1_COLOR_F,
    ) {
        let (x, y, width, height) = frame;
        let layout = if centered {
            self.factory.layout_centered(text, font, width, height)
        } else {
            self.factory.layout(text, font, width, height)
        };
        let Ok(layout) = layout else {
            return;
        };
        // SAFETY: drawing on our own target between Begin/EndDraw.
        unsafe {
            pen.brush.SetColor(&colour);
            pen.target.DrawTextLayout(
                Vector2 { X: x, Y: y },
                &layout,
                pen.brush,
                D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
            );
        }
    }

    /// One cap at `(x, y)`: its outline, the key's label top left, the Shift
    /// glyph top right, the glyph large in the middle. A cap flashing or
    /// pressed is lit as the candidate window's selection is: the highlight
    /// behind, every text in the highlighted-text colour.
    fn draw_cap(&self, pen: &Pen<'_>, cap: &TpsKeyCap, x: f32, y: f32, is_lit: bool) {
        let theme = self.theme;
        let colour = |resting: D2D1_COLOR_F| {
            if is_lit {
                theme.highlighted_text
            } else {
                resting
            }
        };
        let rounded = D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F {
                left: x + CAP_STROKE / 2.0,
                top: y + CAP_STROKE / 2.0,
                right: x + CAP_SIZE - CAP_STROKE / 2.0,
                bottom: y + CAP_SIZE - CAP_STROKE / 2.0,
            },
            radiusX: CAP_CORNER_RADIUS,
            radiusY: CAP_CORNER_RADIUS,
        };
        // SAFETY: drawing on our own target between Begin/EndDraw.
        unsafe {
            if is_lit {
                pen.brush.SetColor(&theme.highlight);
                pen.target.FillRoundedRectangle(&rounded, pen.brush);
            }
            pen.brush.SetColor(&theme.border);
            pen.target
                .DrawRoundedRectangle(&rounded, pen.brush, CAP_STROKE, None);
        }
        let corner_height = SHIFT_GLYPH_FONT_SIZE * 1.4;
        self.draw_text(
            pen,
            &cap.label.to_string(),
            font(CandidateFontChoice::System, LABEL_FONT_SIZE),
            (x + CAP_INSET, y + CAP_INSET, CAP_SIZE / 2.0, corner_height),
            false,
            colour(theme.tertiary_text),
        );
        if let Some(shift_glyph) = cap.shift_glyph {
            let shift_font = font(GLYPH_FONT, SHIFT_GLYPH_FONT_SIZE);
            let width = DWriteMeasurer {
                factory: &self.factory,
            }
            .width(shift_glyph, shift_font);
            self.draw_text(
                pen,
                shift_glyph,
                shift_font,
                (
                    x + CAP_SIZE - CAP_INSET - width,
                    y + CAP_INSET,
                    width.max(1.0),
                    corner_height,
                ),
                false,
                colour(theme.secondary_text),
            );
        }
        let glyph_top = y + CAP_INSET + corner_height / 2.0;
        self.draw_text(
            pen,
            cap.glyph,
            font(GLYPH_FONT, GLYPH_FONT_SIZE),
            (x, glyph_top, CAP_SIZE, y + CAP_SIZE - glyph_top),
            true,
            colour(theme.text),
        );
    }
}

fn font(choice: CandidateFontChoice, size: f32) -> FontSpec {
    FontSpec {
        selection: CandidateFontSelection::BuiltIn(choice),
        size,
    }
}

/// The panel's size in DIPs: the widest row (its indent and caps) and four
/// rows of caps, padded.
fn panel_size(rows: &[TpsKeyboardRow]) -> (f32, f32) {
    let pitch = CAP_SIZE + CAP_GAP;
    let width = rows
        .iter()
        .map(|row| (row.indent + row.caps.len() as f32) * pitch - CAP_GAP)
        .fold(0.0f32, f32::max);
    let count = rows.len() as f32;
    let height = count * pitch - CAP_GAP;
    (width + 2.0 * PADDING, height.max(0.0) + 2.0 * PADDING)
}

/// The top left of a cap, in DIPs — where `paint` draws it and `cap_at`
/// finds it.
fn cap_origin(row_index: usize, row: &TpsKeyboardRow, cap_index: usize) -> (f32, f32) {
    let pitch = CAP_SIZE + CAP_GAP;
    (
        PADDING + (row.indent + cap_index as f32) * pitch,
        PADDING + row_index as f32 * pitch,
    )
}

/// The cap under `point` (DIPs, the panel's top left at 0, 0) — where it
/// sits and what it is — and whether the point is in its top half, the
/// Shift glyph's, as the cap draws it.
fn cap_at(
    rows: &[TpsKeyboardRow],
    point: (f32, f32),
) -> Option<(TpsKeyCapIndex, &TpsKeyCap, bool)> {
    rows.iter().enumerate().find_map(|(row_index, row)| {
        row.caps.iter().enumerate().find_map(|(cap_index, cap)| {
            let (x, y) = cap_origin(row_index, row, cap_index);
            let is_inside =
                (x..x + CAP_SIZE).contains(&point.0) && (y..y + CAP_SIZE).contains(&point.1);
            let index = TpsKeyCapIndex {
                row: row_index,
                cap: cap_index,
            };
            is_inside.then_some((index, cap, point.1 < y + CAP_SIZE / 2.0))
        })
    })
}

/// Injects the key that types `glyph` (`tps_keyboard_click`), down then up.
fn send_click(glyph: &str) {
    let Some(scan_code) = scan_code_for_glyph(glyph) else {
        return;
    };
    let key = |flags: KEYBD_EVENT_FLAGS| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(TPS_KEYBOARD_CLICK_VIRTUAL_KEY),
                wScan: scan_code,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inputs = [key(KEYBD_EVENT_FLAGS(0)), key(KEYEVENTF_KEYUP)];
    // SAFETY: two keyboard inputs alive for the call, with the size of one.
    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        // Fewer inputs went in than were sent (`SendInput` does not say why):
        // none means the click typed nothing; one means the key went down
        // without its release.
        log::warn!("ui.tps_keyboard_click_short_send sent={sent}");
    }
}

/// The monitor the foreground window is on — the application the user is
/// typing in. `None` while no window is foreground (activation moving).
pub(super) fn foreground_monitor() -> Option<MonitorArea> {
    // SAFETY: a plain window query.
    let foreground = unsafe { GetForegroundWindow() };
    if foreground.is_invalid() {
        return None;
    }
    monitor_of_window(foreground)
}

/// The panel at the bottom centre of `monitor`'s work area.
pub(super) fn bottom_frame(monitor: &MonitorArea, size: (f32, f32)) -> RECT {
    frame_in_work_area(monitor, size, Some(BOTTOM_MARGIN))
}

impl WindowHandler for PanelContent {
    fn paint(&mut self, window: &WindowRef) {
        let size = panel_size(&self.rows);
        let scale = self.dpi / BASE_DPI;
        let pixel_size = D2D_SIZE_U {
            width: (size.0 * scale).ceil().max(1.0) as u32,
            height: (size.1 * scale).ceil().max(1.0) as u32,
        };
        if self.surface.is_none() {
            match Surface::create(&self.factory, window.hwnd(), pixel_size, self.dpi) {
                Ok(surface) => self.surface = Some(surface),
                Err(error) => {
                    log::error!("ui.tps_keyboard_surface_failed error={error}");
                    return;
                }
            }
        }
        let Some(surface) = &self.surface else { return };
        let theme = self.theme;
        let lost = surface
            .frame(|target, brush| {
                // SAFETY: drawing on our own target between Begin/EndDraw.
                unsafe { target.Clear(Some(&theme.background)) };
                let pen = Pen { target, brush };
                let flashed = self.flash.map(|(cap, _)| cap);
                for (row_index, row) in self.rows.iter().enumerate() {
                    for (cap_index, cap) in row.caps.iter().enumerate() {
                        let (x, y) = cap_origin(row_index, row, cap_index);
                        let index = Some(TpsKeyCapIndex {
                            row: row_index,
                            cap: cap_index,
                        });
                        let is_lit = flashed == index || self.pressed == index;
                        self.draw_cap(&pen, cap, x, y, is_lit);
                    }
                }
            })
            .is_err();
        if lost {
            self.surface = None;
        }
    }

    /// A press on a cap lights it, and takes the mouse so the release comes
    /// back here wherever it happens; a press between caps, nothing.
    fn press(&mut self, window: &WindowRef, point: (f32, f32)) {
        let Some((index, _, _)) = cap_at(&self.rows, point) else {
            return;
        };
        self.pressed = Some(index);
        window.capture_mouse();
        window.invalidate();
    }

    /// The release types the glyph of the cap the press began on, if it
    /// comes up over that cap — as the macOS panel does; anywhere else,
    /// nothing. The half is read where the button comes up. The window
    /// takes no activation (`WM_MOUSEACTIVATE`), so the document keeps the
    /// keyboard and the injected key reaches its key sink.
    fn click(&mut self, window: &WindowRef, point: (f32, f32)) {
        let Some(pressed) = self.pressed.take() else {
            return;
        };
        window.invalidate();
        let Some((index, cap, is_top_half)) = cap_at(&self.rows, point) else {
            return;
        };
        if index != pressed {
            return;
        }
        // SAFETY: a plain query of this thread's keyboard state.
        let is_shift_held = unsafe { GetKeyState(i32::from(VK_SHIFT.0)) } < 0;
        send_click(cap.pressed_glyph(is_top_half || is_shift_held));
    }

    /// The mouse went elsewhere mid-press (another window took it): the
    /// press ends, typing nothing.
    fn press_cancelled(&mut self, window: &WindowRef) {
        if self.pressed.take().is_some() {
            window.invalidate();
        }
    }

    fn wheel(&mut self, _window: &WindowRef, _delta: f32) {}

    /// The scale moved (another monitor, a new display setting): the surface
    /// is rebuilt at the new DPI and the panel synced — placed again where it
    /// belongs when it is wanted, and never brought up by the message alone,
    /// since every activation holds this window hidden.
    fn dpi_changed(&mut self, window: &WindowRef, dpi: f32, _suggested: RECT) {
        self.dpi = dpi;
        self.surface = None;
        self.sync(window);
    }

    /// The flash's end. A tick that arrives early — one already queued
    /// before a newer key restarted the timer — is let pass: the timer
    /// repeats, and the next tick ends it.
    fn timer(&mut self, window: &WindowRef, id: usize) {
        let is_over = self
            .flash
            .is_none_or(|(_, ends)| Instant::now() + TIMER_TICK_SLACK >= ends);
        if id == FLASH_TIMER_ID && is_over {
            self.end_flash(window);
        }
    }

    /// Required by the trait; never posted to this window — focus events
    /// post a sync, which hides it when it should not be up.
    fn hide_requested(&mut self, _window: &WindowRef, _owner: Option<ContextToken>) {}

    /// Stays up across a theme change, so it is re-coloured in place (the
    /// window invalidates itself after this).
    fn system_theme_changed(&mut self) {
        self.theme = Theme::resolve(self.appearance, &SystemTheme::read());
    }

    fn sync_requested(&mut self, window: &WindowRef) {
        self.sync(window);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use taigi_desktop_core::keys::tps_keyboard_rows;

    #[test]
    fn the_panel_is_as_wide_as_its_widest_staggered_row() {
        // trace: rows 12 @0, 10 @0.5, 10 @0.75, 10 @1.25 key widths; pitch
        // 54 → widths 642, 561, 574.5, 601.5; the number row is widest.
        // Height: 4 × 54 − 6 = 210. Plus 12 padding each side.
        let (width, height) = panel_size(&tps_keyboard_rows());
        assert_eq!(width, 642.0 + 24.0);
        assert_eq!(height, 210.0 + 24.0);
    }

    #[test]
    fn a_point_on_a_cap_names_it_and_its_half() {
        // trace: pitch 54, padding 12. Row 0 (indent 0): `1` at x 12..60,
        // y 12..60 — top half above y 36. Row 1 (indent 0.5): `Q` at
        // x 12 + 27 = 39..87, y 66..114.
        let rows = tps_keyboard_rows();
        let label = |point| cap_at(&rows, point).map(|(_, cap, top)| (cap.label, top));
        assert_eq!(label((13.0, 13.0)), Some(('1', true)));
        assert_eq!(label((59.0, 59.0)), Some(('1', false)));
        assert_eq!(label((66.0, 20.0)), Some(('2', true)));
        assert_eq!(label((40.0, 70.0)), Some(('Q', true)));
        assert_eq!(label((40.0, 100.0)), Some(('Q', false)));
    }

    #[test]
    fn a_point_in_a_gap_or_the_padding_names_no_cap() {
        // trace: x 61 is the gap after `1`; y 62 the gap under row 0; row 1
        // starts half a key in, so x 20 on it is before `Q`; x 700 is past
        // the widest row.
        let rows = tps_keyboard_rows();
        for point in [
            (5.0, 20.0),
            (61.0, 20.0),
            (20.0, 62.0),
            (20.0, 70.0),
            (700.0, 20.0),
        ] {
            assert!(cap_at(&rows, point).is_none(), "{point:?}");
        }
        assert!(cap_at(&rows, (20.0, 400.0)).is_none(), "below the last row");
    }

    #[test]
    fn the_frame_sits_centred_above_the_bottom_of_the_work_area() {
        // trace: work area 0..1920 × 0..1040 at 144 DPI (scale 1.5); a
        // 100 × 50 DIP panel is 150 × 75 px; margin 24 DIP = 36 px.
        let monitor = MonitorArea {
            work_area: RECT {
                left: 0,
                top: 0,
                right: 1920,
                bottom: 1040,
            },
            dpi: 144.0,
        };
        let frame = bottom_frame(&monitor, (100.0, 50.0));
        assert_eq!(
            (frame.left, frame.top, frame.right, frame.bottom),
            (885, 929, 1035, 1004)
        );
    }
}
