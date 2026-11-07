//! The floating Telex key table a global chord keeps up while the user types
//! (#684), port of `TelexGuidePanel.swift`: the same non-activating popup as
//! the mode flash (`mode_flash.rs`), with NO timer — a table is read at the
//! user's pace, not the flash's — at the bottom centre of the work area of the
//! foreground window's monitor, as the TPS key panel is. Display-only: it does
//! nothing with a click. Rows are key | meaning, built in core
//! (`telex_guide_rows`) so the table is tested where this window cannot be.
//!
//! One per service activation, as `tps_keyboard.rs` is and for the same
//! reasons: the stored key (`telexGuideShown`) is all that crosses activations
//! and processes, only the activation with keyboard focus shows it, and every
//! move is a posted sync that reads a source the text service installs (W3).

use super::render::{RenderFactory, Surface};
use super::theme::{SystemTheme, Theme};
use super::tps_keyboard::{bottom_frame, foreground_monitor};
use super::window::{PopupWindow, WindowHandler, WindowRef, BASE_DPI};
use std::cell::RefCell;
use std::rc::Rc;
use taigi_desktop_core::candidates::{FontSpec, TextMeasurer};
use taigi_desktop_core::composing::ContextToken;
use taigi_desktop_core::keys::TelexGuideRow;
use taigi_desktop_core::settings::{AppearanceMode, CandidateFontChoice, CandidateFontSelection};
use windows::Win32::Foundation::RECT;
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
/// Small: the card stays up beside the text being typed, about the height of
/// the TPS key panel that takes its place under TPS.
/// Mirrors `TelexGuidePanel.titleFontSize` / `rowFontSize`.
const TITLE_FONT_SIZE: f32 = 14.0;
const ROW_FONT_SIZE: f32 = 13.0;
/// The insets and gaps of `TelexGuidePanel.makePanel` / `makeGrid`.
const PADDING_X: f32 = 16.0;
const PADDING_TOP: f32 = 12.0;
const PADDING_BOTTOM: f32 = 12.0;
const SECTION_GAP: f32 = 8.0;
const ROW_GAP: f32 = 3.0;
const COLUMN_GAP: f32 = 16.0;

/// What the card says: the title and the rows for the romanization in use,
/// resolved by the caller, who has the display language; this module only
/// draws. No dismiss hint — the card carries the keys and nothing else
/// (USER 2026-09-09).
#[derive(PartialEq)]
pub struct TelexGuideContent {
    pub title: String,
    pub rows: Vec<TelexGuideRow>,
}

/// What to draw and the appearance to draw it in as focus and the settings
/// stand now, or `None` when the guide should not be up
/// (`SettingsDocument::is_telex_guide_wanted`).
pub type TelexGuideSource = Box<dyn Fn() -> Option<(TelexGuideContent, AppearanceMode)>>;

/// The guide's window; its content lives in the window's handler.
pub struct TelexGuide {
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
    rows: Vec<[Cell; 2]>,
    /// The widest cell of each column, which is where the next column starts.
    column_widths: [f32; 2],
    /// Both columns plus the gap — what the table is centred by.
    table_width: f32,
    title_height: f32,
    row_height: f32,
    size: (f32, f32),
}

struct GuideContent {
    factory: Rc<RenderFactory>,
    source: TelexGuideSource,
    /// What `layout` was measured from: a sync with the same content keeps it.
    measured: Option<TelexGuideContent>,
    layout: Option<GuideLayout>,
    appearance: AppearanceMode,
    theme: Theme,
    dpi: f32,
    surface: Option<Surface>,
}

impl TelexGuide {
    /// Made with its (hidden) window, at activation: a focus callback may
    /// neither create nor show a window (W3), and the guide can come up on
    /// focus alone (`TpsKeyboard::new`).
    pub fn new(factory: Rc<RenderFactory>, source: TelexGuideSource) -> Self {
        let appearance = AppearanceMode::Auto;
        let handler: Rc<RefCell<dyn WindowHandler>> = Rc::new(RefCell::new(GuideContent {
            factory,
            source,
            measured: None,
            layout: None,
            appearance,
            theme: Theme::resolve(appearance, &SystemTheme::read()),
            dpi: BASE_DPI,
            surface: None,
        }));
        let window = PopupWindow::create(WINDOW_CLASS, handler)
            .inspect_err(|error| log::error!("ui.telex_guide_window_failed error={error}"))
            .ok();
        Self { window }
    }

    /// Queues a sync for the message loop: the guide shows or hides as its
    /// source says when the message runs (`TpsKeyboard::request_sync`).
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
    /// Key | meaning. The key is semibold so it stays the thing the eye
    /// lands on (`makeGrid`).
    fn measure(factory: &RenderFactory, content: &TelexGuideContent) -> Self {
        let measurer = super::render::DWriteMeasurer { factory };
        let title = Cell::measure(
            factory,
            &content.title,
            TITLE_FONT_SIZE,
            DWRITE_FONT_WEIGHT_SEMI_BOLD,
        );
        let rows: Vec<[Cell; 2]> = content
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
                ]
            })
            .collect();
        let mut column_widths = [0.0f32; 2];
        for row in &rows {
            for (column, cell) in row.iter().enumerate() {
                column_widths[column] = column_widths[column].max(cell.width);
            }
        }
        let title_height = measurer.line_height(font(TITLE_FONT_SIZE));
        let row_height = measurer.line_height(font(ROW_FONT_SIZE));
        let table_width = column_widths.iter().sum::<f32>() + COLUMN_GAP;
        let content_width = title.width.max(table_width);
        let row_count = rows.len() as f32;
        let table_height = row_count * row_height + (row_count - 1.0).max(0.0) * ROW_GAP;
        let height = PADDING_TOP + title_height + SECTION_GAP + table_height + PADDING_BOTTOM;
        Self {
            title,
            rows,
            column_widths,
            table_width,
            title_height,
            row_height,
            size: (content_width + 2.0 * PADDING_X, height),
        }
    }
}

impl GuideContent {
    /// Up at the bottom of the foreground window's monitor when the source
    /// answers content; down otherwise. With no foreground window to place
    /// it by, nothing changes until the next sync.
    fn sync(&mut self, window: &WindowRef) {
        let Some((content, appearance)) = (self.source)() else {
            window.hide();
            return;
        };
        let Some(monitor) = foreground_monitor() else {
            return;
        };
        self.appearance = appearance;
        self.theme = Theme::resolve(appearance, &SystemTheme::read());
        // The surface is sized in pixels: kept unless the scale or the card
        // moved. The content follows the mode and the display language.
        if monitor.dpi != self.dpi {
            self.dpi = monitor.dpi;
            self.surface = None;
        }
        if self.measured.as_ref() != Some(&content) {
            self.layout = Some(GuideLayout::measure(&self.factory, &content));
            self.measured = Some(content);
            self.surface = None;
        }
        if let Some(layout) = &self.layout {
            window.show_at(bottom_frame(&monitor, layout.size));
            window.invalidate();
        }
    }

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
                // Title and table each centred, so the margins read symmetric
                // on every edge (USER 2026-09-20, `TelexGuidePanel.makePanel`).
                let mut y = PADDING_TOP;
                self.draw_cell(
                    &pen,
                    &layout.title,
                    layout.title_height,
                    ((layout.size.0 - layout.title.width) / 2.0, y),
                    theme.text,
                );
                y += layout.title_height + SECTION_GAP;
                let colours = [theme.text, theme.text];
                let table_x = (layout.size.0 - layout.table_width) / 2.0;
                for row in &layout.rows {
                    let mut x = table_x;
                    for (column, cell) in row.iter().enumerate() {
                        self.draw_cell(&pen, cell, layout.row_height, (x, y), colours[column]);
                        x += layout.column_widths[column] + COLUMN_GAP;
                    }
                    y += layout.row_height + ROW_GAP;
                }
            })
            .is_err();
        if lost {
            self.surface = None;
        }
    }

    fn click(&mut self, _window: &WindowRef, _point: (f32, f32)) {}

    fn wheel(&mut self, _window: &WindowRef, _delta: f32) {}

    /// The card moved to a monitor with another scale: placed again there.
    fn dpi_changed(&mut self, window: &WindowRef, dpi: f32, _suggested: RECT) {
        self.dpi = dpi;
        self.surface = None;
        self.sync(window);
    }

    fn timer(&mut self, _window: &WindowRef, _id: usize) {}

    /// Follows its source, not the hides a focus callback posts.
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
