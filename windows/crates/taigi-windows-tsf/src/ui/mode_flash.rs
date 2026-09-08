//! A brief on-screen flash naming the mode just switched into (台羅 /
//! 白話字), port of `ModeFlashPanel.swift`: a small card centred a third
//! of the way up the monitor of the last caret the service saw (the
//! primary monitor before any), held 0.8 s, then gone. NAMED
//! SIMPLIFICATION: no fade (a plain hide after the hold); the corners are
//! DWM's rounding, not a custom radius.

use super::render::{RenderFactory, Surface};
use super::theme::{SystemTheme, Theme};
use super::window::{monitor_at, PopupWindow, WindowHandler, WindowRef, BASE_DPI};
use std::cell::RefCell;
use std::rc::Rc;
use taigi_windows_core::candidates::FontSpec;
use taigi_windows_core::composing::ContextToken;
use taigi_windows_core::settings::{AppearanceMode, CandidateFontChoice, CandidateFontSelection};
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U;
use windows_numerics::Vector2;

const WINDOW_CLASS: &str = "TaigiKeyboardModeFlash";
const HOLD_MILLISECONDS: u32 = 800;
const HIDE_TIMER_ID: usize = 2;
const FONT_SIZE: f32 = 24.0;
const PADDING_X: f32 = 28.0;
const PADDING_Y: f32 = 14.0;

pub struct ModeFlash {
    content: Rc<RefCell<FlashContent>>,
    window: Option<PopupWindow>,
}

struct FlashContent {
    factory: Rc<RenderFactory>,
    text: String,
    theme: Theme,
    dpi: f32,
    size: (f32, f32),
    surface: Option<Surface>,
}

impl ModeFlash {
    pub fn new(factory: Rc<RenderFactory>) -> Self {
        let system = SystemTheme::read();
        Self {
            content: Rc::new(RefCell::new(FlashContent {
                factory,
                text: String::new(),
                theme: Theme::resolve(AppearanceMode::Auto, &system),
                dpi: BASE_DPI,
                size: (0.0, 0.0),
                surface: None,
            })),
            window: None,
        }
    }

    /// Shows `text` on the monitor holding `anchor` (a physical screen
    /// point), replacing any flash still up. Shown AFTER the candidate
    /// window's last placement, so as the newer topmost window it sits
    /// above it.
    pub fn flash(&mut self, text: &str, anchor: POINT, mode: AppearanceMode) {
        let Some(monitor) = monitor_at(anchor) else {
            return;
        };
        let scale = monitor.dpi / BASE_DPI;
        let (width, height) = {
            let mut content = self.content.borrow_mut();
            content.text = text.to_owned();
            // A flash is rare (a chord): reading the system theme here
            // costs nothing worth caching.
            content.theme = Theme::resolve(mode, &SystemTheme::read());
            content.dpi = monitor.dpi;
            let measurer = super::render::DWriteMeasurer {
                factory: &content.factory,
            };
            use taigi_windows_core::candidates::TextMeasurer;
            let font = FlashContent::font();
            let width = measurer.width(text, font) + 2.0 * PADDING_X;
            let height = measurer.line_height(font) + 2.0 * PADDING_Y;
            content.size = (width, height);
            content.surface = None;
            (width, height)
        };
        if self.window.is_none() {
            let handler: Rc<RefCell<dyn WindowHandler>> = self.content.clone();
            match PopupWindow::create(WINDOW_CLASS, handler) {
                Ok(window) => self.window = Some(window),
                Err(error) => {
                    log::error!("ui.flash_window_failed error={error}");
                    return;
                }
            }
        }
        let Some(window) = self.window.as_deref() else {
            return;
        };
        let work = monitor.work_area;
        let px_width = (width * scale).round() as i32;
        let px_height = (height * scale).round() as i32;
        let left = (work.left + work.right) / 2 - px_width / 2;
        let top = work.bottom - (work.bottom - work.top) / 3 - px_height / 2;
        window.show_at(RECT {
            left,
            top,
            right: left + px_width,
            bottom: top + px_height,
        });
        window.set_timer(HIDE_TIMER_ID, HOLD_MILLISECONDS);
    }

    pub fn destroy(&mut self) {
        if let Some(window) = self.window.take() {
            window.destroy();
        }
    }
}

impl FlashContent {
    fn font() -> FontSpec {
        FontSpec {
            selection: CandidateFontSelection::BuiltIn(CandidateFontChoice::System),
            size: FONT_SIZE,
        }
    }
}

impl WindowHandler for FlashContent {
    fn paint(&mut self, window: &WindowRef) {
        let scale = self.dpi / BASE_DPI;
        let pixel_size = D2D_SIZE_U {
            width: (self.size.0 * scale).ceil().max(1.0) as u32,
            height: (self.size.1 * scale).ceil().max(1.0) as u32,
        };
        if self.surface.is_none() {
            match Surface::create(&self.factory, window.hwnd(), pixel_size, self.dpi) {
                Ok(surface) => self.surface = Some(surface),
                Err(error) => {
                    log::error!("ui.flash_surface_failed error={error}");
                    return;
                }
            }
        }
        let Some(surface) = &self.surface else { return };
        let (width, height) = self.size;
        let theme = self.theme;
        let text = self.text.clone();
        let factory = Rc::clone(&self.factory);
        let lost = surface
            .frame(|target, brush| {
                // SAFETY: drawing on our own target between Begin/EndDraw.
                unsafe {
                    target.Clear(Some(&theme.background));
                    brush.SetColor(&theme.text);
                    if let Ok(layout) = factory.layout_centered(&text, Self::font(), width, height) {
                        target.DrawTextLayout(
                            Vector2 { X: 0.0, Y: 0.0 },
                            &layout,
                            brush,
                            windows::Win32::Graphics::Direct2D::D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
                        );
                    }
                }
            })
            .is_err();
        if lost {
            self.surface = None;
        }
    }

    fn click(&mut self, _window: &WindowRef, _point: (f32, f32)) {}

    fn wheel(&mut self, _window: &WindowRef, _delta: f32) {}

    /// The card is placed once per flash; a DPI change mid-hold just
    /// re-renders at the new scale where the OS put it.
    fn dpi_changed(&mut self, window: &WindowRef, dpi: f32, suggested: RECT) {
        self.dpi = dpi;
        self.surface = None;
        if suggested.right > suggested.left {
            window.show_at(suggested);
        }
    }

    fn timer(&mut self, window: &WindowRef, id: usize) {
        if id == HIDE_TIMER_ID {
            window.kill_timer(HIDE_TIMER_ID);
            window.hide();
        }
    }

    fn hide_requested(&mut self, window: &WindowRef, _owner: Option<ContextToken>) {
        window.kill_timer(HIDE_TIMER_ID);
        window.hide();
    }

    fn system_theme_changed(&mut self) {}
}
