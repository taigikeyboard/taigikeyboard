//! The floating Telex key table a global chord toggles up and any key takes
//! down, port of `TelexGuidePanel.swift`: the same non-activating popup as
//! the mode flash (`mode_flash.rs`), with NO timer — a table is read at the
//! user's pace, not the flash's — centred in the work area of the monitor
//! holding the last caret this service saw. Rows are key | meaning |
//! example, built in core (`telex_guide_rows`) so the table is tested where
//! this window cannot be.
//!
//! Owned by the context that raised it, the way the candidate window is
//! (`presenter.rs`): a focus callback's posted hide names the context whose
//! guide should come down, so a context on its way out cannot take down a
//! guide the incoming one just put up. The key path and the toggle hide
//! whichever is up — both run on the context the user is in.

use super::render::{RenderFactory, Surface};
use super::theme::{SystemTheme, Theme};
use super::window::{monitor_at, MonitorArea, PopupWindow, WindowHandler, WindowRef, BASE_DPI};
use std::cell::RefCell;
use std::rc::Rc;
use taigi_windows_core::candidates::{FontSpec, TextMeasurer};
use taigi_windows_core::composing::ContextToken;
use taigi_windows_core::keys::TelexGuideRow;
use taigi_windows_core::settings::{AppearanceMode, CandidateFontChoice, CandidateFontSelection};
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_SIZE_U};
use windows::Win32::Graphics::Direct2D::{
    ID2D1HwndRenderTarget, ID2D1SolidColorBrush, D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
};
use windows::Win32::Graphics::DirectWrite::{
    IDWriteTextLayout, DWRITE_FONT_WEIGHT, DWRITE_FONT_WEIGHT_NORMAL, DWRITE_FONT_WEIGHT_SEMI_BOLD,
    DWRITE_TEXT_METRICS, DWRITE_TEXT_RANGE,
};
use windows_numerics::Vector2;

const WINDOW_CLASS: &str = "TaigiKeyboardTelexGuide";
/// WinUI body / subtitle / caption sizes, so the card reads as a Windows 11
/// flyout rather than the Mac panel it was ported from (`theme.rs`).
const TITLE_FONT_SIZE: f32 = 16.0;
const ROW_FONT_SIZE: f32 = 14.0;
const HINT_FONT_SIZE: f32 = 12.0;
/// The insets and gaps of `TelexGuidePanel.makePanel` / `makeGrid`.
const PADDING_X: f32 = 22.0;
const PADDING_TOP: f32 = 16.0;
const PADDING_BOTTOM: f32 = 14.0;
const SECTION_GAP: f32 = 10.0;
const ROW_GAP: f32 = 4.0;
const COLUMN_GAP: f32 = 18.0;

/// What the card says: the title, the rows for the romanization in use,
/// and the dismiss hint — resolved by the caller, who has the display
/// language; this module only draws.
pub struct TelexGuideContent {
    pub title: String,
    pub rows: Vec<TelexGuideRow>,
    pub hint: String,
}

pub struct TelexGuide {
    content: Rc<RefCell<GuideContent>>,
    window: Option<PopupWindow>,
}

/// One measured cell: its text, the face it draws in, and its DIP width.
struct Cell {
    text: String,
    font_size: f32,
    weight: DWRITE_FONT_WEIGHT,
    width: f32,
}

/// What one frame draws with (`Surface::frame`'s pair).
struct Pen<'a> {
    target: &'a ID2D1HwndRenderTarget,
    brush: &'a ID2D1SolidColorBrush,
}

/// The card laid out in DIPs: every cell measured once at `show`, so
/// `paint` only places layouts.
struct GuideLayout {
    title: Cell,
    rows: Vec<[Cell; 3]>,
    hint: Cell,
    /// The widest cell of each column, which is where the next column starts.
    column_widths: [f32; 3],
    title_height: f32,
    row_height: f32,
    hint_height: f32,
    size: (f32, f32),
}

struct GuideContent {
    factory: Rc<RenderFactory>,
    layout: Option<GuideLayout>,
    theme: Theme,
    dpi: f32,
    /// The context the guide is showing for; `None` = nothing showing.
    /// The one flag `is_showing` reads, cleared by every path that hides.
    owner: Option<ContextToken>,
    surface: Option<Surface>,
}

impl TelexGuide {
    pub fn new(factory: Rc<RenderFactory>) -> Self {
        let system = SystemTheme::read();
        Self {
            content: Rc::new(RefCell::new(GuideContent {
                factory,
                layout: None,
                theme: Theme::resolve(AppearanceMode::Auto, &system),
                dpi: BASE_DPI,
                owner: None,
                surface: None,
            })),
            window: None,
        }
    }

    /// Whether a guide is up, whoever raised it.
    pub fn is_showing(&self) -> bool {
        self.content.borrow().owner.is_some()
    }

    /// The chord's meaning: up if down, down if up — whoever raised it, as
    /// the Mac's `toggle` (both run on the context the user is in).
    pub fn toggle(
        &mut self,
        content: TelexGuideContent,
        anchor: POINT,
        mode: AppearanceMode,
        owner: ContextToken,
    ) {
        if self.is_showing() {
            self.hide_now();
        } else {
            self.show(content, anchor, mode, owner);
        }
    }

    /// Shows the card on the monitor holding `anchor` (a physical screen
    /// point), replacing any guide still up. Rebuilt each time: the content
    /// follows the mode and the display language, and a card shown a few
    /// times a day is not worth keeping warm.
    pub fn show(
        &mut self,
        content: TelexGuideContent,
        anchor: POINT,
        mode: AppearanceMode,
        owner: ContextToken,
    ) {
        let Some(monitor) = monitor_at(anchor) else {
            return;
        };
        let size = {
            let mut guide = self.content.borrow_mut();
            // A guide is rare (a chord): reading the system theme here costs
            // nothing worth caching.
            guide.theme = Theme::resolve(mode, &SystemTheme::read());
            guide.dpi = monitor.dpi;
            let layout = GuideLayout::measure(&guide.factory, &content);
            let size = layout.size;
            guide.layout = Some(layout);
            guide.surface = None;
            guide.owner = Some(owner);
            size
        };
        if self.window.is_none() {
            let handler: Rc<RefCell<dyn WindowHandler>> = self.content.clone();
            match PopupWindow::create(WINDOW_CLASS, handler) {
                Ok(window) => self.window = Some(window),
                Err(error) => {
                    log::error!("ui.telex_guide_window_failed error={error}");
                    self.content.borrow_mut().owner = None;
                    return;
                }
            }
        }
        let Some(window) = self.window.as_deref() else {
            return;
        };
        window.show_at(centred_frame(&monitor, size));
    }

    /// Takes the guide down only if `owner` raised it — what a context's
    /// teardown and the other global actions want.
    pub fn hide(&mut self, owner: ContextToken) {
        if self.content.borrow().owner == Some(owner) {
            self.hide_now();
        }
    }

    /// Takes the guide down whoever raised it — the key path, the toggle,
    /// and the settings doorway.
    pub fn hide_now(&mut self) {
        self.content.borrow_mut().owner = None;
        if let Some(window) = self.window.as_deref() {
            window.hide();
        }
    }

    /// Queues a hide for the message loop — what a focus / context callback
    /// calls instead of `hide` (W3). `owner` = only if that context raised
    /// it; `None` = whichever is up.
    pub fn request_hide(&self, owner: Option<ContextToken>) {
        if let Some(window) = self.window.as_deref() {
            window.post_hide_request(owner);
        }
    }

    pub fn destroy(&mut self) {
        self.content.borrow_mut().owner = None;
        if let Some(window) = self.window.take() {
            window.destroy();
        }
    }
}

/// The card centred in `monitor`'s work area — the hotkey path has no
/// client, so there is no caret to anchor to (`HUDPanel.noticeFrame`).
fn centred_frame(monitor: &MonitorArea, size: (f32, f32)) -> RECT {
    let scale = monitor.dpi / BASE_DPI;
    let work = monitor.work_area;
    let px_width = (size.0 * scale).round() as i32;
    let px_height = (size.1 * scale).round() as i32;
    let left = (work.left + work.right) / 2 - px_width / 2;
    let top = (work.top + work.bottom) / 2 - px_height / 2;
    RECT {
        left,
        top,
        right: left + px_width,
        bottom: top + px_height,
    }
}

fn font(size: f32) -> FontSpec {
    FontSpec {
        selection: CandidateFontSelection::BuiltIn(CandidateFontChoice::System),
        size,
    }
}

/// The advance width of `text` at `weight`. The shared measurer
/// (`DWriteMeasurer`) only knows the regular weight `RenderFactory::format`
/// bakes in; the key column is semibold, and a semibold run is wider.
fn measured_width(
    factory: &RenderFactory,
    text: &str,
    size: f32,
    weight: DWRITE_FONT_WEIGHT,
) -> f32 {
    let Ok(layout) = factory.layout(text, font(size), f32::MAX, f32::MAX) else {
        return 0.0;
    };
    set_weight(&layout, text, weight);
    let mut metrics = DWRITE_TEXT_METRICS::default();
    // SAFETY: out-pointer to a local.
    if unsafe { layout.GetMetrics(&mut metrics) }.is_err() {
        return 0.0;
    }
    metrics.widthIncludingTrailingWhitespace.ceil()
}

/// Applies `weight` over the whole of `text` in `layout`. Per-layout rather
/// than a second cached format: one column of one rare card is not worth a
/// weight axis on every `FontSpec` the candidate window measures with.
fn set_weight(layout: &IDWriteTextLayout, text: &str, weight: DWRITE_FONT_WEIGHT) {
    if weight == DWRITE_FONT_WEIGHT_NORMAL {
        return;
    }
    let range = DWRITE_TEXT_RANGE {
        startPosition: 0,
        length: text.encode_utf16().count() as u32,
    };
    // SAFETY: a range inside the layout's own text.
    unsafe { layout.SetFontWeight(weight, range).ok() };
}

impl Cell {
    fn measure(factory: &RenderFactory, text: &str, size: f32, weight: DWRITE_FONT_WEIGHT) -> Self {
        Self {
            text: text.to_owned(),
            font_size: size,
            weight,
            width: measured_width(factory, text, size, weight),
        }
    }
}

impl GuideLayout {
    /// Key | meaning | example. The key is semibold so it stays the thing
    /// the eye lands on; the example is secondary (`makeGrid`).
    fn measure(factory: &RenderFactory, content: &TelexGuideContent) -> Self {
        let measurer = super::render::DWriteMeasurer { factory };
        let title = Cell::measure(
            factory,
            &content.title,
            TITLE_FONT_SIZE,
            DWRITE_FONT_WEIGHT_SEMI_BOLD,
        );
        let hint = Cell::measure(
            factory,
            &content.hint,
            HINT_FONT_SIZE,
            DWRITE_FONT_WEIGHT_NORMAL,
        );
        let rows: Vec<[Cell; 3]> = content
            .rows
            .iter()
            .map(|row| {
                [
                    Cell::measure(
                        factory,
                        row.key,
                        ROW_FONT_SIZE,
                        DWRITE_FONT_WEIGHT_SEMI_BOLD,
                    ),
                    Cell::measure(
                        factory,
                        &row.meaning,
                        ROW_FONT_SIZE,
                        DWRITE_FONT_WEIGHT_NORMAL,
                    ),
                    Cell::measure(
                        factory,
                        row.example,
                        ROW_FONT_SIZE,
                        DWRITE_FONT_WEIGHT_NORMAL,
                    ),
                ]
            })
            .collect();
        let mut column_widths = [0.0f32; 3];
        for row in &rows {
            for (column, cell) in row.iter().enumerate() {
                column_widths[column] = column_widths[column].max(cell.width);
            }
        }
        let title_height = measurer.line_height(font(TITLE_FONT_SIZE));
        let row_height = measurer.line_height(font(ROW_FONT_SIZE));
        let hint_height = measurer.line_height(font(HINT_FONT_SIZE));
        let table_width = column_widths.iter().sum::<f32>() + 2.0 * COLUMN_GAP;
        let content_width = title.width.max(table_width).max(hint.width);
        let row_count = rows.len() as f32;
        let table_height = row_count * row_height + (row_count - 1.0).max(0.0) * ROW_GAP;
        let height = PADDING_TOP
            + title_height
            + SECTION_GAP
            + table_height
            + SECTION_GAP
            + hint_height
            + PADDING_BOTTOM;
        Self {
            title,
            rows,
            hint,
            column_widths,
            title_height,
            row_height,
            hint_height,
            size: (content_width + 2.0 * PADDING_X, height),
        }
    }
}

impl GuideContent {
    /// Draws one cell at `origin` in `colour`, on a line `line_height` tall.
    fn draw_cell(
        &self,
        pen: &Pen<'_>,
        cell: &Cell,
        line_height: f32,
        origin: (f32, f32),
        colour: D2D1_COLOR_F,
    ) {
        let Ok(layout) = self.factory.layout(
            &cell.text,
            font(cell.font_size),
            cell.width.max(1.0),
            line_height,
        ) else {
            return;
        };
        set_weight(&layout, &cell.text, cell.weight);
        // SAFETY: drawing on our own target between Begin/EndDraw.
        unsafe {
            pen.brush.SetColor(&colour);
            pen.target.DrawTextLayout(
                Vector2 {
                    X: origin.0,
                    Y: origin.1,
                },
                &layout,
                pen.brush,
                D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
            );
        }
    }
}

impl WindowHandler for GuideContent {
    fn paint(&mut self, window: &WindowRef) {
        let Some(layout) = self.layout.as_ref() else {
            return;
        };
        let scale = self.dpi / BASE_DPI;
        let pixel_size = D2D_SIZE_U {
            width: (layout.size.0 * scale).ceil().max(1.0) as u32,
            height: (layout.size.1 * scale).ceil().max(1.0) as u32,
        };
        if self.surface.is_none() {
            match Surface::create(&self.factory, window.hwnd(), pixel_size, self.dpi) {
                Ok(surface) => self.surface = Some(surface),
                Err(error) => {
                    log::error!("ui.telex_guide_surface_failed error={error}");
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
                let mut y = PADDING_TOP;
                self.draw_cell(
                    &pen,
                    &layout.title,
                    layout.title_height,
                    (PADDING_X, y),
                    theme.text,
                );
                y += layout.title_height + SECTION_GAP;
                let colours = [theme.text, theme.text, theme.secondary_text];
                for row in &layout.rows {
                    let mut x = PADDING_X;
                    for (column, cell) in row.iter().enumerate() {
                        self.draw_cell(&pen, cell, layout.row_height, (x, y), colours[column]);
                        x += layout.column_widths[column] + COLUMN_GAP;
                    }
                    y += layout.row_height + ROW_GAP;
                }
                y += SECTION_GAP - ROW_GAP;
                self.draw_cell(
                    &pen,
                    &layout.hint,
                    layout.hint_height,
                    (PADDING_X, y),
                    theme.tertiary_text,
                );
            })
            .is_err();
        if lost {
            self.surface = None;
        }
    }

    fn click(&mut self, _window: &WindowRef, _point: (f32, f32)) {}

    fn wheel(&mut self, _window: &WindowRef, _delta: f32) {}

    /// The card moved to a monitor with another scale: re-centred there at
    /// the new DPI, or left where the OS put it when the monitor cannot be
    /// read.
    fn dpi_changed(&mut self, window: &WindowRef, dpi: f32, suggested: RECT) {
        self.dpi = dpi;
        self.surface = None;
        let Some(layout) = self.layout.as_ref() else {
            return;
        };
        let centre = POINT {
            x: (suggested.left + suggested.right) / 2,
            y: (suggested.top + suggested.bottom) / 2,
        };
        match monitor_at(centre) {
            Some(monitor) => window.show_at(centred_frame(&monitor, layout.size)),
            None if suggested.right > suggested.left => window.show_at(suggested),
            None => {}
        }
    }

    fn timer(&mut self, _window: &WindowRef, _id: usize) {}

    /// A focus / context callback's posted hide: the guide goes if `owner`
    /// raised it, or whoever did when the callback names no one.
    fn hide_requested(&mut self, window: &WindowRef, owner: Option<ContextToken>) {
        if owner.is_some_and(|owner| self.owner != Some(owner)) {
            return;
        }
        self.owner = None;
        window.hide();
    }

    fn system_theme_changed(&mut self) {}
}
