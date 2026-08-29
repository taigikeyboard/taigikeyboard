//! The candidate window's content: the three layouts over the core models,
//! drawn with Direct2D. Owns no composition state — the presenter hands it
//! cells and asks it for absolute indices (the window is authoritative for
//! the selection, `CandidatePresenter.swift:59-63`).
//!
//! Rendering rules follow the Mac item view (`CandidateItemView.swift`):
//! index slot · candidate · annotation, inline on one baseline or stacked
//! and centred; the selected cell is painted with the accent and white
//! text; vertical rows are separated by a hairline and align their
//! annotation column; the horizontal window's paging edge is a separator
//! and two chevrons; the collapsed expandable row ends in a chevron.
//! NAMED SIMPLIFICATION: the unfold is a window-size ease over the same
//! 183 ms rather than per-cell slides.

// 中文: 候選窗內容 — 三種版面套 core 模型,Direct2D 繪製;選取以視窗為準;展開動畫簡化為視窗尺寸過渡。

use super::render::{DWriteMeasurer, RenderFactory, Surface};
use super::theme::{SystemTheme, Theme};
use super::window::{monitor_at, MonitorArea, WindowRef, BASE_DPI};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::time::Instant;
use taigi_windows_core::candidates::{
    panel_frame, CandidateCellArrangement, CandidateIndexLabel, CandidateMetrics,
    ExpandableDisplayMode, ExpandableGeometryInput, ExpandableListModel, ExpandableModeChange,
    HorizontalListModel, HorizontalPageLayout, Point, Rect, ScrollerStyle, Size, TextMeasurer,
    VerticalLayoutInput, VerticalListModel, MAX_DISPLAY_CANDIDATES,
};
use taigi_windows_core::composing::CandidateCellContent;
use taigi_windows_core::keys::{CandidateNavigation, CandidateSlotKeySet};
use taigi_windows_core::settings::{
    keys, AppearanceMode, CandidateFontChoice, CandidateLayout, CandidateTextSizeChoice,
    CandidateWindowSizeChoice, SettingsDocument,
};
use windows::Win32::Foundation::{POINT, RECT};
use windows::Win32::Graphics::Direct2D::Common::{D2D1_COLOR_F, D2D_RECT_F, D2D_SIZE_U};
use windows::Win32::Graphics::Direct2D::{
    ID2D1HwndRenderTarget, ID2D1SolidColorBrush, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE,
    D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT, D2D1_ROUNDED_RECT,
};
use windows::Win32::Graphics::DirectWrite::IDWriteTextLayout;
use windows_numerics::Vector2;

/// How much of its screen a stretched window leaves unspent
/// (`CandidateBasePanel.screenEdgeMargin`).
const SCREEN_EDGE_MARGIN: f32 = 12.0;
/// The width budget before any caret has named a screen.
const FALLBACK_MAXIMUM_WINDOW_WIDTH: f32 = 640.0;
/// Our drawn scroller (overlay style): its width and the gap to the text.
const SCROLLER_WIDTH: f32 = 12.0;
/// The Sequoia chrome radius (`CandidateBasePanel.sequoiaCornerRadius`).
const CORNER_RADIUS: f32 = 6.0;
/// Page-arrow / chevron geometry at 16 pt (`CandidatePageArrowView`,
/// `CandidateChevronView`), scaled through `scaled_symbol_metric`.
const ARROW_SPACING: f32 = 4.0;
const ARROW_IMAGE_WIDTH: f32 = 16.0;
const ARROW_PADDING: f32 = 7.0;
const CHEVRON_SPACING: f32 = 5.0;
const CHEVRON_PADDING: f32 = 6.0;
/// The unfold's timer.
const UNFOLD_TIMER_ID: usize = 1;
const UNFOLD_FRAME_MILLISECONDS: u32 = 16;
/// Wheel scrolling: rows per notch.
const ROWS_PER_WHEEL_NOTCH: f32 = 3.0;

/// The layout the window is showing, with its selection state.
enum LayoutModel {
    Horizontal(HorizontalListModel),
    Vertical(VerticalListModel),
    Expandable(ExpandableListModel),
}

/// A window-size transition (the unfold / fold).
struct SizeTransition {
    started: Instant,
    from: Size,
    to: Size,
}

/// What the presenter reads back after a change.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WindowFrame {
    /// Screen pixels.
    pub frame: RECT,
}

/// Which text of a cell a cached layout is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum CellPart {
    Index,
    Primary,
    Annotation,
}

/// A cached DirectWrite layout is exact for its text (the cell index, the
/// list generation being the cache's lifetime) and its box.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct LayoutKey {
    index: usize,
    part: CellPart,
    width_centi: u32,
    height_centi: u32,
}

impl LayoutKey {
    fn of(index: usize, part: CellPart, width: f32, height: f32) -> Self {
        Self {
            index,
            part,
            width_centi: (width * 100.0).round() as u32,
            height_centi: (height * 100.0).round() as u32,
        }
    }
}

pub struct CandidateWindow {
    factory: Rc<RenderFactory>,
    surface: Option<Surface>,
    /// What the surface was last sized / scaled to, so a paint only asks
    /// Direct2D to resize when something changed.
    surface_pixel_size: D2D_SIZE_U,
    surface_dpi: f32,
    /// The system's appearance answers, read once and dropped when Windows
    /// reports a change (`system_theme_changed`).
    system_theme: Option<SystemTheme>,
    appearance_mode: AppearanceMode,
    theme: Theme,
    metrics: Option<CandidateMetrics>,
    layout: Option<LayoutModel>,
    cells: Vec<CandidateCellContent>,
    /// Per-cell measurements taken once per list (a paint never measures).
    primary_widths: Vec<f32>,
    annotation_widths: Vec<f32>,
    /// The two line heights a stacked cell is composed of, for this list's
    /// fonts.
    stacked_line_heights: (f32, f32),
    /// DirectWrite layouts by cell and box, for this list; cleared with it.
    layouts: RefCell<HashMap<LayoutKey, IDWriteTextLayout>>,
    slot_key_set: CandidateSlotKeySet,
    /// The caret the window was anchored to (screen pixels) and its
    /// monitor — pages that change the size re-place against the same caret.
    caret: RECT,
    monitor: Option<MonitorArea>,
    dpi: f32,
    /// The size the window is drawn at (DIPs), and the transition toward it.
    size: Size,
    transition: Option<SizeTransition>,
}

impl CandidateWindow {
    pub fn new(factory: Rc<RenderFactory>) -> Self {
        let system = SystemTheme::read();
        Self {
            factory,
            surface: None,
            surface_pixel_size: D2D_SIZE_U::default(),
            surface_dpi: BASE_DPI,
            theme: Theme::resolve(AppearanceMode::Auto, &system),
            system_theme: Some(system),
            appearance_mode: AppearanceMode::Auto,
            metrics: None,
            layout: None,
            cells: Vec::new(),
            primary_widths: Vec::new(),
            annotation_widths: Vec::new(),
            stacked_line_heights: (0.0, 0.0),
            layouts: RefCell::new(HashMap::new()),
            slot_key_set: CandidateSlotKeySet::BareKeys,
            caret: RECT::default(),
            monitor: None,
            dpi: BASE_DPI,
            size: Size::default(),
            transition: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.cells.is_empty()
    }

    pub fn cell_count(&self) -> usize {
        self.cells.len()
    }

    /// The leading script of cell `index` — what a host drawing the list
    /// itself shows.
    pub fn cell_text(&self, index: usize) -> Option<&str> {
        self.cells.get(index).map(|cell| cell.text.as_str())
    }

    fn scale(&self) -> f32 {
        self.dpi / BASE_DPI
    }

    /// The width budget: the caret's screen less the margins, floored at
    /// one slot (`CandidateBasePanel.layout(_:forCaret:)`).
    fn maximum_window_width(&self, metrics: &CandidateMetrics) -> f32 {
        let screen = self
            .monitor
            .as_ref()
            .map(|monitor| {
                (monitor.work_area.right - monitor.work_area.left) as f32 / (monitor.dpi / BASE_DPI)
            })
            .unwrap_or(FALLBACK_MAXIMUM_WINDOW_WIDTH);
        metrics.base_width().max(screen - 2.0 * SCREEN_EDGE_MARGIN)
    }

    fn arrow_width(metrics: &CandidateMetrics) -> f32 {
        1.0 + metrics.scaled_symbol_metric(ARROW_SPACING)
            + metrics.scaled_symbol_metric(ARROW_IMAGE_WIDTH)
            + metrics.scaled_symbol_metric(ARROW_PADDING)
    }

    fn chevron_width(metrics: &CandidateMetrics) -> f32 {
        1.0 + metrics.scaled_symbol_metric(CHEVRON_SPACING)
            + metrics.scaled_symbol_metric(ARROW_IMAGE_WIDTH)
            + metrics.scaled_symbol_metric(CHEVRON_PADDING)
    }

    /// A fresh list under `settings`, anchored to `caret` (screen pixels).
    /// Selects the first candidate. Answers the frame to show at, or `None`
    /// when the caret is on no monitor.
    pub fn show(
        &mut self,
        cells: Vec<CandidateCellContent>,
        slot_key_set: CandidateSlotKeySet,
        caret: RECT,
        settings: &SettingsDocument,
    ) -> Option<WindowFrame> {
        let monitor = monitor_at(POINT {
            x: caret.left,
            y: caret.top,
        })?;
        self.dpi = monitor.dpi;
        self.monitor = Some(monitor);
        self.caret = caret;
        self.appearance_mode = settings.choice(&keys::APPEARANCE_MODE);
        let system = *self.system_theme.get_or_insert_with(SystemTheme::read);
        self.theme = Theme::resolve(self.appearance_mode, &system);
        self.slot_key_set = slot_key_set;
        self.replace_cells(cells);
        self.transition = None;
        let layout: CandidateLayout = settings.choice(&keys::CANDIDATE_LAYOUT);
        let measurer = DWriteMeasurer {
            factory: &self.factory,
        };
        let metrics = CandidateMetrics::resolve(
            settings.choice::<CandidateTextSizeChoice>(&keys::CANDIDATE_TEXT_SIZE),
            settings.choice::<CandidateWindowSizeChoice>(&keys::CANDIDATE_WINDOW_SIZE),
            settings.choice::<CandidateFontChoice>(&keys::FONT_TYPE),
            layout.cell_arrangement(),
            &measurer,
        );
        self.stacked_line_heights = (
            measurer.line_height(metrics.candidate_font()),
            measurer.line_height(metrics.annotation_font()),
        );
        self.metrics = Some(metrics.clone());
        self.measure_cells(&metrics);
        self.layout = Some(self.build_layout(layout, &metrics));
        self.size = self.natural_size();
        Some(self.frame())
    }

    /// One list for the popup AND the UI-less element: what the layout
    /// models can hold (`MAX_DISPLAY_CANDIDATES`) is what the host sees, so
    /// `GetCount` / `SetSelection` cannot name a cell no layout has.
    fn replace_cells(&mut self, mut cells: Vec<CandidateCellContent>) {
        cells.truncate(MAX_DISPLAY_CANDIDATES);
        self.cells = cells;
        self.layouts.borrow_mut().clear();
    }

    /// The per-cell text widths, measured once per list.
    fn measure_cells(&mut self, metrics: &CandidateMetrics) {
        let measurer = DWriteMeasurer {
            factory: &self.factory,
        };
        self.primary_widths = self
            .cells
            .iter()
            .map(|cell| metrics.measure_primary_width(&cell.text, &measurer))
            .collect();
        self.annotation_widths = self
            .cells
            .iter()
            .map(|cell| metrics.annotation_text_width(cell.annotation.as_deref(), &measurer))
            .collect();
    }

    /// The frame the window currently occupies, or `None` with no list up.
    pub fn current_frame(&self) -> Option<WindowFrame> {
        self.layout.as_ref().map(|_| self.frame())
    }

    /// Windows reported a theme / accent change: the next paint reads the
    /// system again.
    pub fn system_theme_changed(&mut self) {
        let system = *self.system_theme.insert(SystemTheme::read());
        self.theme = Theme::resolve(self.appearance_mode, &system);
    }

    /// Same list, new rendering (the 漢羅 flip): re-packed, selection kept
    /// on its absolute index.
    pub fn update_cells(
        &mut self,
        cells: Vec<CandidateCellContent>,
        settings: &SettingsDocument,
    ) -> Option<WindowFrame> {
        if self.cells.is_empty() || cells.is_empty() {
            return None;
        }
        let kept = self.selected_index()?;
        let expanded = matches!(&self.layout, Some(LayoutModel::Expandable(model)) if model.mode() == ExpandableDisplayMode::Expanded);
        self.replace_cells(cells);
        let layout: CandidateLayout = settings.choice(&keys::CANDIDATE_LAYOUT);
        let metrics = self.metrics.clone()?;
        self.measure_cells(&metrics);
        let mut model = self.build_layout(layout, &metrics);
        match &mut model {
            LayoutModel::Horizontal(model) => model.select(kept.min(self.cells.len() - 1)),
            LayoutModel::Vertical(model) => model.select(kept.min(self.cells.len() - 1)),
            LayoutModel::Expandable(model) => {
                model.select(kept.min(self.cells.len() - 1));
                if expanded {
                    model.expand();
                }
            }
        }
        self.layout = Some(model);
        self.size = self.natural_size();
        Some(self.frame())
    }

    fn build_layout(&self, layout: CandidateLayout, metrics: &CandidateMetrics) -> LayoutModel {
        let measurer = DWriteMeasurer {
            factory: &self.factory,
        };
        let widths: Vec<f32> = self
            .cells
            .iter()
            .map(|cell| metrics.measure_width(cell, &measurer))
            .collect();
        let maximum = self.maximum_window_width(metrics);
        match layout {
            CandidateLayout::Horizontal => {
                let arrow = Self::arrow_width(metrics);
                let pages = HorizontalPageLayout::pack_with_chrome(
                    &widths,
                    metrics.base_width(),
                    maximum,
                    arrow,
                );
                LayoutModel::Horizontal(HorizontalListModel::new(
                    pages,
                    arrow,
                    metrics.item_height(),
                ))
            }
            CandidateLayout::Vertical => {
                LayoutModel::Vertical(VerticalListModel::new(VerticalLayoutInput {
                    metrics,
                    cell_widths: &widths,
                    primary_widths: &self.primary_widths,
                    maximum_window_width: maximum,
                    scroller: ScrollerStyle::Overlay {
                        width: SCROLLER_WIDTH,
                    },
                }))
            }
            CandidateLayout::Expandable => LayoutModel::Expandable(ExpandableListModel::new(
                &widths,
                ExpandableGeometryInput {
                    base_width: metrics.base_width(),
                    maximum_window_width: maximum,
                    chrome_width: Self::chevron_width(metrics),
                    scroller_width: SCROLLER_WIDTH,
                    item_height: metrics.item_height(),
                },
            )),
        }
    }

    /// The window's size in DIPs for the layout's current state.
    fn natural_size(&self) -> Size {
        let Some(metrics) = &self.metrics else {
            return Size::default();
        };
        match &self.layout {
            None => Size::default(),
            Some(LayoutModel::Horizontal(model)) => Size {
                width: model.window_width(),
                height: metrics.item_height(),
            },
            Some(LayoutModel::Vertical(model)) => Size {
                width: model.geometry().window_width,
                height: model.geometry().window_height,
            },
            Some(LayoutModel::Expandable(model)) => match model.mode() {
                ExpandableDisplayMode::Collapsed => Size {
                    width: model.collapsed_window_width(),
                    height: metrics.item_height(),
                },
                ExpandableDisplayMode::Expanded => {
                    let geometry = model.expanded_geometry();
                    Size {
                        width: geometry.window_width,
                        height: geometry.window_height,
                    }
                }
            },
        }
    }

    /// The screen frame (pixels) for the current size against the anchor.
    fn frame(&self) -> WindowFrame {
        let scale = self.scale();
        let work = self
            .monitor
            .as_ref()
            .map(|monitor| monitor.work_area)
            .unwrap_or_default();
        let to_dip = |value: i32| value as f32 / scale;
        let visible = Rect::new(
            to_dip(work.left),
            to_dip(work.top),
            to_dip(work.right - work.left),
            to_dip(work.bottom - work.top),
        );
        let caret = Rect::new(
            to_dip(self.caret.left),
            to_dip(self.caret.top),
            to_dip(self.caret.right - self.caret.left),
            to_dip(self.caret.bottom - self.caret.top),
        );
        let placed = panel_frame(caret, self.size, visible);
        WindowFrame {
            frame: RECT {
                left: (placed.x * scale).round() as i32,
                top: (placed.y * scale).round() as i32,
                right: ((placed.x + placed.width) * scale).round() as i32,
                bottom: ((placed.y + placed.height) * scale).round() as i32,
            },
        }
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        self.primary_widths.clear();
        self.annotation_widths.clear();
        self.layouts.borrow_mut().clear();
        self.layout = None;
        self.transition = None;
    }

    pub fn selected_index(&self) -> Option<usize> {
        if self.cells.is_empty() {
            return None;
        }
        Some(match self.layout.as_ref()? {
            LayoutModel::Horizontal(model) => model.selected_index(),
            LayoutModel::Vertical(model) => model.selected_index(),
            LayoutModel::Expandable(model) => model.selected_index(),
        })
    }

    pub fn candidate_index_for_slot(&self, slot: usize) -> Option<usize> {
        match self.layout.as_ref()? {
            LayoutModel::Horizontal(model) => model.candidate_index_for_slot(slot),
            LayoutModel::Vertical(model) => model.candidate_index_for_slot(slot),
            LayoutModel::Expandable(model) => model.candidate_index_for_slot(slot),
        }
    }

    /// Moves the selection; answers the frame when the window's size changed
    /// (a page turn, an unfold) so the presenter re-places it.
    pub fn navigate(&mut self, direction: CandidateNavigation) -> Option<WindowFrame> {
        let before = self.size;
        let mut change = None;
        match self.layout.as_mut()? {
            LayoutModel::Horizontal(model) => {
                model.navigate(direction);
            }
            LayoutModel::Vertical(model) => model.navigate(direction),
            LayoutModel::Expandable(model) => change = model.navigate(direction),
        }
        self.after_change(before, change)
    }

    /// A click on `index` selects it (never commits).
    pub fn select(&mut self, index: usize) -> Option<WindowFrame> {
        let before = self.size;
        let mut change = None;
        match self.layout.as_mut()? {
            LayoutModel::Horizontal(model) => model.select(index),
            LayoutModel::Vertical(model) => model.select(index),
            LayoutModel::Expandable(model) => change = model.select(index),
        }
        self.after_change(before, change)
    }

    fn after_change(
        &mut self,
        before: Size,
        change: Option<ExpandableModeChange>,
    ) -> Option<WindowFrame> {
        let target = self.natural_size();
        if change.is_some() {
            // The unfold / fold: a size ease over the Mac's duration.
            self.transition = Some(SizeTransition {
                started: Instant::now(),
                from: before,
                to: target,
            });
            return Some(self.frame());
        }
        if target != before {
            self.size = target;
            return Some(self.frame());
        }
        None
    }

    /// Advances the transition; `Some(frame)` while it runs, `None` when done.
    fn transition_step(&mut self) -> Option<WindowFrame> {
        let transition = self.transition.as_ref()?;
        let elapsed = transition.started.elapsed().as_secs_f32();
        let duration = ExpandableListModel::ANIMATION_DURATION_SECONDS;
        let progress = (elapsed / duration).min(1.0);
        // Ease-out cubic.
        let eased = 1.0 - (1.0 - progress).powi(3);
        self.size = Size {
            width: transition.from.width + (transition.to.width - transition.from.width) * eased,
            height: transition.from.height
                + (transition.to.height - transition.from.height) * eased,
        };
        if progress >= 1.0 {
            self.size = transition.to;
            self.transition = None;
        }
        Some(self.frame())
    }

    pub fn is_transitioning(&self) -> bool {
        self.transition.is_some()
    }

    // Drawing

    fn ensure_surface(&mut self, window: &WindowRef) -> bool {
        let scale = self.scale();
        let pixel_size = D2D_SIZE_U {
            width: (self.size.width * scale).ceil().max(1.0) as u32,
            height: (self.size.height * scale).ceil().max(1.0) as u32,
        };
        if let Some(surface) = &self.surface {
            let same_size = pixel_size.width == self.surface_pixel_size.width
                && pixel_size.height == self.surface_pixel_size.height;
            // SAFETY: a resize / DPI update on our own target, only when
            // the target really moved.
            let resized = same_size || unsafe { surface.target.Resize(&pixel_size) }.is_ok();
            if resized {
                if self.surface_dpi != self.dpi {
                    // SAFETY: as above.
                    unsafe { surface.target.SetDpi(self.dpi, self.dpi) };
                    self.surface_dpi = self.dpi;
                }
                self.surface_pixel_size = pixel_size;
                return true;
            }
            self.surface = None;
        }
        match Surface::create(&self.factory, window.hwnd(), pixel_size, self.dpi) {
            Ok(surface) => {
                self.surface = Some(surface);
                self.surface_pixel_size = pixel_size;
                self.surface_dpi = self.dpi;
                true
            }
            Err(error) => {
                log::error!("ui.surface_failed error={error}");
                false
            }
        }
    }

    fn index_label(&self, candidate_index: usize) -> String {
        for slot in 0..HorizontalPageLayout::PAGE_SIZE {
            if self.candidate_index_for_slot(slot) == Some(candidate_index) {
                return CandidateIndexLabel::text_for_slot(slot, self.slot_key_set);
            }
        }
        String::new()
    }

    /// The layout for `part` of cell `index` in a `width` × `height` box,
    /// built once per list and box (a page turn or an unfold changes the
    /// box; a repaint does not).
    fn cached_layout(
        &self,
        index: usize,
        part: CellPart,
        text: &str,
        font: taigi_windows_core::candidates::FontSpec,
        width: f32,
        height: f32,
    ) -> Option<IDWriteTextLayout> {
        let key = LayoutKey::of(index, part, width, height);
        if let Some(layout) = self.layouts.borrow().get(&key) {
            return Some(layout.clone());
        }
        let built = match part {
            CellPart::Index => self.factory.layout_centered(text, font, width, height),
            CellPart::Primary | CellPart::Annotation => {
                self.factory.layout(text, font, width, height)
            }
        };
        let layout = built.ok()?;
        self.layouts.borrow_mut().insert(key, layout.clone());
        Some(layout)
    }

    fn draw_all(&self, target: &ID2D1HwndRenderTarget, brush: &ID2D1SolidColorBrush) {
        let Some(metrics) = &self.metrics else { return };
        let theme = self.theme;
        // SAFETY: drawing calls on our own target between Begin/EndDraw.
        unsafe {
            target.Clear(Some(&theme.background));
            let Some(layout) = &self.layout else { return };
            match layout {
                LayoutModel::Horizontal(model) => {
                    self.draw_horizontal(target, brush, metrics, model)
                }
                LayoutModel::Vertical(model) => self.draw_vertical(target, brush, metrics, model),
                LayoutModel::Expandable(model) => {
                    self.draw_expandable(target, brush, metrics, model)
                }
            }
        }
    }

    unsafe fn draw_horizontal(
        &self,
        target: &ID2D1HwndRenderTarget,
        brush: &ID2D1SolidColorBrush,
        metrics: &CandidateMetrics,
        model: &HorizontalListModel,
    ) {
        for (index, rect) in model.page_cell_rects() {
            self.draw_cell(
                target,
                brush,
                metrics,
                index,
                rect,
                None,
                model.selected_index() == index,
            );
        }
        if let Some(arrow) = model.arrow_rect() {
            let (can_up, can_down) = model.page_arrows();
            self.draw_page_arrows(target, brush, metrics, arrow, can_up, can_down);
        }
    }

    unsafe fn draw_vertical(
        &self,
        target: &ID2D1HwndRenderTarget,
        brush: &ID2D1SolidColorBrush,
        metrics: &CandidateMetrics,
        model: &VerticalListModel,
    ) {
        let geometry = model.geometry();
        let clip = D2D_RECT_F {
            left: 0.0,
            top: 0.0,
            right: geometry.window_width,
            bottom: geometry.window_height,
        };
        target.PushAxisAlignedClip(&clip, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
        let offset = model.scroll_y();
        for row in 0..model.count() {
            let rect = model.row_rect(row);
            let on_screen = Rect::new(rect.x, rect.y - offset, rect.width, rect.height);
            if on_screen.bottom() < 0.0 || on_screen.y > geometry.window_height {
                continue;
            }
            self.draw_cell(
                target,
                brush,
                metrics,
                row,
                on_screen,
                Some((geometry.primary_column_width, geometry.item_trailing)),
                model.selected_index() == row,
            );
            if row + 1 < model.count() {
                let y = on_screen.bottom();
                self.fill(
                    target,
                    brush,
                    Rect::new(
                        0.0,
                        y,
                        geometry.item_width,
                        VerticalListModel::SEPARATOR_HEIGHT,
                    ),
                    self.theme.separator,
                );
            }
        }
        if model.has_overflow() {
            self.draw_scroller(
                target,
                brush,
                geometry.window_width,
                geometry.window_height,
                model.container_height(),
                offset,
            );
        }
        target.PopAxisAlignedClip();
    }

    unsafe fn draw_expandable(
        &self,
        target: &ID2D1HwndRenderTarget,
        brush: &ID2D1SolidColorBrush,
        metrics: &CandidateMetrics,
        model: &ExpandableListModel,
    ) {
        match model.mode() {
            ExpandableDisplayMode::Collapsed => {
                for slot in 0..model.collapsed_row().len() {
                    let Some(rect) = model.collapsed_cell_rect(slot) else {
                        continue;
                    };
                    let index = model.collapsed_row()[slot].candidate_index;
                    self.draw_cell(
                        target,
                        brush,
                        metrics,
                        index,
                        rect,
                        None,
                        model.selected_index() == index,
                    );
                }
                if model.has_overflow() {
                    let chevron = model.unfold_plan().chevron_collapsed;
                    self.draw_chevron(target, brush, metrics, chevron);
                }
            }
            ExpandableDisplayMode::Expanded => {
                let geometry = model.expanded_geometry();
                let clip = D2D_RECT_F {
                    left: 0.0,
                    top: 0.0,
                    right: geometry.window_width,
                    bottom: geometry.window_height,
                };
                target.PushAxisAlignedClip(&clip, D2D1_ANTIALIAS_MODE_PER_PRIMITIVE);
                let offset = model.scroll_y();
                if let Some(row) = model.selected_row_rect() {
                    // The row the slot keys address, tinted so its numbers read
                    // as a group.
                    self.fill(
                        target,
                        brush,
                        Rect::new(row.x, row.y - offset, row.width, row.height),
                        self.theme.separator,
                    );
                }
                for (row_index, row) in model.grid().rows.iter().enumerate() {
                    for cell in row {
                        let Some(rect) = model.grid_cell_rect(cell.candidate_index) else {
                            continue;
                        };
                        let on_screen = Rect::new(rect.x, rect.y - offset, rect.width, rect.height);
                        if on_screen.bottom() < 0.0 || on_screen.y > geometry.window_height {
                            continue;
                        }
                        self.draw_cell(
                            target,
                            brush,
                            metrics,
                            cell.candidate_index,
                            on_screen,
                            None,
                            model.selected_index() == cell.candidate_index,
                        );
                    }
                    if row_index + 1 < model.grid().rows.len() {
                        let y = (row_index + 1) as f32 * model.row_height()
                            - ExpandableListModel::SEPARATOR_HEIGHT
                            - offset;
                        self.fill(
                            target,
                            brush,
                            Rect::new(
                                0.0,
                                y,
                                model.grid_width(),
                                ExpandableListModel::SEPARATOR_HEIGHT,
                            ),
                            self.theme.separator,
                        );
                    }
                }
                if geometry.needs_scrolling {
                    self.draw_scroller(
                        target,
                        brush,
                        geometry.window_width,
                        geometry.window_height,
                        geometry.container_height,
                        offset,
                    );
                }
                target.PopAxisAlignedClip();
            }
        }
    }

    unsafe fn fill(
        &self,
        target: &ID2D1HwndRenderTarget,
        brush: &ID2D1SolidColorBrush,
        rect: Rect,
        color: D2D1_COLOR_F,
    ) {
        brush.SetColor(&color);
        target.FillRectangle(&d2d_rect(rect), brush);
    }

    /// One cell: highlight, index label, candidate, annotation.
    #[allow(clippy::too_many_arguments)]
    unsafe fn draw_cell(
        &self,
        target: &ID2D1HwndRenderTarget,
        brush: &ID2D1SolidColorBrush,
        metrics: &CandidateMetrics,
        index: usize,
        rect: Rect,
        column: Option<(f32, f32)>,
        is_selected: bool,
    ) {
        let Some(cell) = self.cells.get(index) else {
            return;
        };
        let theme = self.theme;
        if is_selected {
            brush.SetColor(&theme.highlight);
            let rounded = D2D1_ROUNDED_RECT {
                rect: d2d_rect(rect),
                radiusX: CandidateMetrics::corner_radius(CORNER_RADIUS, rect.width, rect.height),
                radiusY: CandidateMetrics::corner_radius(CORNER_RADIUS, rect.width, rect.height),
            };
            target.FillRoundedRectangle(&rounded, brush);
        }
        let (text_color, secondary) = if is_selected {
            (theme.highlighted_text, theme.highlighted_text)
        } else {
            (theme.text, theme.secondary_text)
        };
        let padding = metrics.horizontal_padding();
        let label = self.index_label(index);
        if !label.is_empty() {
            if let Some(layout) = self.cached_layout(
                index,
                CellPart::Index,
                &label,
                metrics.index_font(),
                metrics.index_width(),
                rect.height,
            ) {
                brush.SetColor(&secondary);
                target.DrawTextLayout(
                    Vector2 {
                        X: rect.x + padding,
                        Y: rect.y,
                    },
                    &layout,
                    brush,
                    D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
                );
            }
        }
        let text_x = rect.x + padding + metrics.index_column_width();
        let trailing = column.map_or(padding, |(_, trailing)| trailing);
        let available = (rect.right() - trailing - text_x).max(metrics.candidate_font_size());
        match metrics.cell_arrangement() {
            CandidateCellArrangement::Inline => {
                let primary_width = match column {
                    Some((primary_column, _)) => primary_column
                        .max(metrics.candidate_font_size())
                        .min(available),
                    None => self.primary_widths[index].min(available),
                };
                if let Some(layout) = self.cached_layout(
                    index,
                    CellPart::Primary,
                    &cell.text,
                    metrics.candidate_font(),
                    primary_width,
                    rect.height,
                ) {
                    brush.SetColor(&text_color);
                    target.DrawTextLayout(
                        Vector2 {
                            X: text_x,
                            Y: rect.y,
                        },
                        &layout,
                        brush,
                        D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
                    );
                }
                if let Some(annotation) = &cell.annotation {
                    let annotation_x = text_x + primary_width + metrics.candidate_annotation_gap();
                    let annotation_width = (rect.right() - trailing - annotation_x).max(0.0);
                    if annotation_width > 0.0 {
                        if let Some(layout) = self.cached_layout(
                            index,
                            CellPart::Annotation,
                            annotation,
                            metrics.annotation_font(),
                            annotation_width,
                            rect.height,
                        ) {
                            brush.SetColor(&secondary);
                            target.DrawTextLayout(
                                Vector2 {
                                    X: annotation_x,
                                    Y: rect.y,
                                },
                                &layout,
                                brush,
                                D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
                            );
                        }
                    }
                }
            }
            CandidateCellArrangement::Stacked => {
                let (line1, line2) = self.stacked_line_heights;
                let block = line1 + metrics.stacked_line_gap() + line2;
                let top = rect.y + (rect.height - block) / 2.0;
                let centre = text_x + available / 2.0;
                let primary_width = self.primary_widths[index].min(available);
                if let Some(layout) = self.cached_layout(
                    index,
                    CellPart::Primary,
                    &cell.text,
                    metrics.candidate_font(),
                    primary_width,
                    line1,
                ) {
                    brush.SetColor(&text_color);
                    target.DrawTextLayout(
                        Vector2 {
                            X: centre - primary_width / 2.0,
                            Y: top,
                        },
                        &layout,
                        brush,
                        D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
                    );
                }
                if let Some(annotation) = &cell.annotation {
                    let annotation_width = self.annotation_widths[index].min(available);
                    if let Some(layout) = self.cached_layout(
                        index,
                        CellPart::Annotation,
                        annotation,
                        metrics.annotation_font(),
                        annotation_width,
                        line2,
                    ) {
                        brush.SetColor(&secondary);
                        target.DrawTextLayout(
                            Vector2 {
                                X: centre - annotation_width / 2.0,
                                Y: top + line1 + metrics.stacked_line_gap(),
                            },
                            &layout,
                            brush,
                            D2D1_DRAW_TEXT_OPTIONS_ENABLE_COLOR_FONT,
                        );
                    }
                }
            }
        }
    }

    /// A chevron drawn as two strokes, centred in `rect`; `up` flips it.
    unsafe fn stroke_chevron(
        &self,
        target: &ID2D1HwndRenderTarget,
        brush: &ID2D1SolidColorBrush,
        centre: (f32, f32),
        half: f32,
        up: bool,
        color: D2D1_COLOR_F,
    ) {
        brush.SetColor(&color);
        let (cx, cy) = centre;
        let (tip_y, base_y) = if up {
            (cy - half / 2.0, cy + half / 2.0)
        } else {
            (cy + half / 2.0, cy - half / 2.0)
        };
        target.DrawLine(
            Vector2 {
                X: cx - half,
                Y: base_y,
            },
            Vector2 { X: cx, Y: tip_y },
            brush,
            1.5,
            None,
        );
        target.DrawLine(
            Vector2 { X: cx, Y: tip_y },
            Vector2 {
                X: cx + half,
                Y: base_y,
            },
            brush,
            1.5,
            None,
        );
    }

    unsafe fn draw_page_arrows(
        &self,
        target: &ID2D1HwndRenderTarget,
        brush: &ID2D1SolidColorBrush,
        metrics: &CandidateMetrics,
        rect: Rect,
        can_up: bool,
        can_down: bool,
    ) {
        self.fill(
            target,
            brush,
            Rect::new(rect.x, rect.y, 1.0, rect.height),
            self.theme.separator,
        );
        let half = metrics.scaled_symbol_metric(4.0);
        let cx = rect.x
            + 1.0
            + metrics.scaled_symbol_metric(ARROW_SPACING)
            + metrics.scaled_symbol_metric(ARROW_IMAGE_WIDTH) / 2.0;
        let cy = rect.y + rect.height / 2.0;
        let color = |enabled: bool| {
            if enabled {
                self.theme.secondary_text
            } else {
                self.theme.tertiary_text
            }
        };
        self.stroke_chevron(
            target,
            brush,
            (cx, cy + metrics.scaled_symbol_metric(-3.0)),
            half,
            true,
            color(can_up),
        );
        self.stroke_chevron(
            target,
            brush,
            (cx, cy + metrics.scaled_symbol_metric(4.0)),
            half,
            false,
            color(can_down),
        );
    }

    unsafe fn draw_chevron(
        &self,
        target: &ID2D1HwndRenderTarget,
        brush: &ID2D1SolidColorBrush,
        metrics: &CandidateMetrics,
        rect: Rect,
    ) {
        self.fill(
            target,
            brush,
            Rect::new(rect.x, rect.y, 1.0, rect.height),
            self.theme.separator,
        );
        let half = metrics.scaled_symbol_metric(5.5);
        let cx = rect.x
            + 1.0
            + metrics.scaled_symbol_metric(CHEVRON_SPACING)
            + metrics.scaled_symbol_metric(ARROW_IMAGE_WIDTH) / 2.0;
        self.stroke_chevron(
            target,
            brush,
            (cx, rect.y + rect.height / 2.0),
            half,
            false,
            self.theme.tertiary_text,
        );
    }

    /// An overlay scroller thumb at the right edge.
    unsafe fn draw_scroller(
        &self,
        target: &ID2D1HwndRenderTarget,
        brush: &ID2D1SolidColorBrush,
        width: f32,
        viewport: f32,
        content: f32,
        offset: f32,
    ) {
        if content <= viewport {
            return;
        }
        let track = viewport - 4.0;
        let thumb = (track * viewport / content).max(16.0);
        let y = 2.0 + (track - thumb) * (offset / (content - viewport)).clamp(0.0, 1.0);
        brush.SetColor(&self.theme.tertiary_text);
        let rounded = D2D1_ROUNDED_RECT {
            rect: D2D_RECT_F {
                left: width - SCROLLER_WIDTH + 3.0,
                top: y,
                right: width - 3.0,
                bottom: y + thumb,
            },
            radiusX: 3.0,
            radiusY: 3.0,
        };
        target.FillRoundedRectangle(&rounded, brush);
    }

    // Mouse

    /// The candidate under `point` (DIPs), or a chrome hit.
    fn hit(&mut self, point: (f32, f32)) -> Option<WindowFrame> {
        let p = Point {
            x: point.0,
            y: point.1,
        };
        let before = self.size;
        let metrics = self.metrics.clone()?;
        let mut change = None;
        match self.layout.as_mut()? {
            LayoutModel::Horizontal(model) => {
                if let Some(arrow) = model.arrow_rect() {
                    if p.x >= arrow.x && p.x < arrow.right() {
                        let direction = if p.y < arrow.y + arrow.height / 2.0 {
                            CandidateNavigation::PageUp
                        } else {
                            CandidateNavigation::PageDown
                        };
                        model.navigate(direction);
                        return self.after_change(before, None);
                    }
                }
                if let Some(index) = model.hit_test(p) {
                    model.select(index);
                }
            }
            LayoutModel::Vertical(model) => {
                if let Some(row) = model.hit_test(p) {
                    model.select(row);
                }
            }
            LayoutModel::Expandable(model) => {
                if model.mode() == ExpandableDisplayMode::Collapsed && model.has_overflow() {
                    let chevron = model.unfold_plan().chevron_collapsed;
                    if p.x >= chevron.x && p.x < chevron.right() && p.y < metrics.item_height() {
                        change = model.expand();
                        return self.after_change(before, change);
                    }
                }
                if let Some(index) = model.hit_test(p) {
                    change = model.select(index);
                }
            }
        }
        self.after_change(before, change)
    }

    fn scroll(&mut self, notches: f32) {
        let Some(metrics) = &self.metrics else { return };
        let step = -notches * metrics.item_height() * ROWS_PER_WHEEL_NOTCH;
        match self.layout.as_mut() {
            Some(LayoutModel::Vertical(model)) => {
                model.on_viewport_scrolled(model.scroll_y() + step)
            }
            Some(LayoutModel::Expandable(model)) => {
                model.on_viewport_scrolled(model.scroll_y() + step)
            }
            _ => {}
        }
    }
}

fn d2d_rect(rect: Rect) -> D2D_RECT_F {
    D2D_RECT_F {
        left: rect.x,
        top: rect.y,
        right: rect.right(),
        bottom: rect.bottom(),
    }
}

/// The message handling the presenter delegates here (it is the popup's
/// `WindowHandler`).
impl CandidateWindow {
    pub fn paint(&mut self, window: &WindowRef) {
        if !self.ensure_surface(window) {
            return;
        }
        let Some(surface) = &self.surface else { return };
        let lost = surface
            .frame(|target, brush| self.draw_all(target, brush))
            .is_err();
        if lost {
            // Device loss: the surface is rebuilt on the next paint.
            self.surface = None;
            window.invalidate();
        }
    }

    pub fn click(&mut self, window: &WindowRef, point: (f32, f32)) {
        if let Some(frame) = self.hit(point) {
            window.show_at(frame.frame);
            if self.is_transitioning() {
                window.set_timer(UNFOLD_TIMER_ID, UNFOLD_FRAME_MILLISECONDS);
            }
        }
        window.invalidate();
    }

    pub fn wheel(&mut self, window: &WindowRef, delta: f32) {
        self.scroll(delta);
        window.invalidate();
    }

    /// The window crossed to a monitor of another scale: re-anchored to
    /// the same caret against that monitor's work area. Only when the caret
    /// is on no monitor any more is the OS's suggested frame used.
    pub fn dpi_changed(&mut self, window: &WindowRef, dpi: f32, suggested: RECT) {
        self.dpi = dpi;
        self.surface = None;
        self.monitor = monitor_at(POINT {
            x: self.caret.left,
            y: self.caret.top,
        });
        if self.monitor.is_some() {
            window.show_at(self.frame().frame);
        } else if suggested.right > suggested.left {
            window.show_at(suggested);
        }
    }

    pub fn timer(&mut self, window: &WindowRef, id: usize) {
        if id != UNFOLD_TIMER_ID {
            return;
        }
        match self.transition_step() {
            Some(frame) => {
                window.show_at(frame.frame);
                if !self.is_transitioning() {
                    window.kill_timer(UNFOLD_TIMER_ID);
                }
            }
            None => window.kill_timer(UNFOLD_TIMER_ID),
        }
    }
}

/// What the presenter needs to start a transition's timer after a
/// navigation it drove itself.
pub const UNFOLD_TIMER: (usize, u32) = (UNFOLD_TIMER_ID, UNFOLD_FRAME_MILLISECONDS);
