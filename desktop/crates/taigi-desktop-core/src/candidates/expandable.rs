//! The expandable window's two modes — a packed row (or a column) that
//! unfolds into a grid — and its navigation; macOS keeps a Swift twin in
//! `ExpandableCandidatePanel.swift`. The mode change's choreography —
//! which drawing moves where and fades how (`ModeTransition`) — is computed
//! here; the renderer plays it.

use super::grid::ExpandedGridLayout;
use super::horizontal::HorizontalPageLayout;
use super::metrics::CandidateMetrics;
use super::positioning::{Point, Rect};
use super::vertical::widest_columns;
use super::MAX_DISPLAY_CANDIDATES;
use crate::keys::CandidateNavigation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpandableDisplayMode {
    Collapsed,
    Expanded,
}

/// Which way the collapsed list runs. Both unfold into the same grid; the
/// axis decides the collapsed shape and which arrow unfolds it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpandableAxis {
    /// One packed row, the chevron column at its right (Expandable).
    Row,
    /// The first page stacked as a column, the chevron column at its right
    /// (Vertical Expandable).
    Column,
}

impl ExpandableAxis {
    /// The arrow across the list, the one that unfolds it: `↓` under a row,
    /// `→` beside a column. The renderer points the chevron this way.
    pub fn unfold_direction(self) -> CandidateNavigation {
        match self {
            Self::Row => CandidateNavigation::Down,
            Self::Column => CandidateNavigation::Right,
        }
    }
}

/// How the collapsed cells are laid out, with what each shape measures from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CollapsedLayout<'a> {
    /// Packed from the summed cell widths `new` takes.
    Row,
    /// Aligned like the vertical list — every row's annotation at one x —
    /// so it measures the two columns apart (`VerticalLayoutInput`'s inputs).
    Column {
        metrics: &'a CandidateMetrics,
        primary_widths: &'a [f32],
        annotation_widths: &'a [Option<f32>],
    },
}

/// One collapsed cell and its frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollapsedCell {
    pub candidate_index: usize,
    pub rect: Rect,
}

/// What `new` frames the collapsed mode into, per shape.
struct CollapsedFrames {
    cells: Vec<CollapsedCell>,
    content_width: f32,
    content_height: f32,
    primary_column_width: Option<f32>,
}

/// What a navigation or click asked the renderer to animate.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpandableModeChange {
    Expand,
    Collapse,
}

/// What the window needs to lay a list out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExpandableGeometryInput<'a> {
    pub collapsed: CollapsedLayout<'a>,
    /// `CandidateMetrics::base_width` — the packing slot.
    pub base_width: f32,
    pub maximum_window_width: f32,
    /// The chevron's reserved width.
    pub chrome_width: f32,
    /// The expanded grid's scroller width, reserved whether or not the grid
    /// ends up scrolling.
    pub scroller_width: f32,
    pub item_height: f32,
}

/// The expanded window's size:
/// up to 5 rows plus a half-row peek when it scrolls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExpandedGeometry {
    pub window_width: f32,
    pub window_height: f32,
    /// The scrollable content's height, including the peek.
    pub container_height: f32,
    pub needs_scrolling: bool,
}

/// One drawing's journey through a mode change: a frame in CONTENT
/// coordinates (the renderer applies the viewport's scroll offset to every
/// track alike, as macOS lays both modes in one scrolled document) and an
/// opacity, each interpolated by the eased progress.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Track {
    pub from: Rect,
    pub to: Rect,
    pub from_alpha: f32,
    pub to_alpha: f32,
}

impl Track {
    fn still(rect: Rect) -> Self {
        Self::new(rect, rect, 1.0, 1.0)
    }

    fn new(from: Rect, to: Rect, from_alpha: f32, to_alpha: f32) -> Self {
        Self {
            from,
            to,
            from_alpha,
            to_alpha,
        }
    }

    /// The frame and opacity at `eased` (0 = start, 1 = end).
    pub fn at(&self, eased: f32) -> (Rect, f32) {
        let lerp = |from: f32, to: f32| from + (to - from) * eased;
        (
            Rect::new(
                lerp(self.from.x, self.to.x),
                lerp(self.from.y, self.to.y),
                lerp(self.from.width, self.to.width),
                lerp(self.from.height, self.to.height),
            ),
            lerp(self.from_alpha, self.to_alpha),
        )
    }

    fn reversed(self) -> Self {
        Self::new(self.to, self.from, self.to_alpha, self.from_alpha)
    }
}

/// Which drawing of a candidate a track moves. Beside a column the two
/// modes align their annotations on different candidate columns, so a cell
/// cross-fades between its two drawings; under a row a survivor is one
/// drawing in both modes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellLayer {
    /// Drawn as the collapsed list draws it (`collapsed_primary_column_width`).
    Collapsed,
    /// Drawn as the grid draws it (`expanded_primary_column_width`).
    Expanded,
    /// One drawing in both modes, decorated (key label, highlight) as the
    /// current mode decorates it.
    Shared,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellTrack {
    pub candidate_index: usize,
    pub layer: CellLayer,
    pub track: Track,
}

/// The last mode change's choreography — macOS
/// `ExpandableCandidatePanel`'s `rowUnfoldAnimations` /
/// `columnUnfoldAnimations` and their fold twins — oriented the way it
/// plays: every drawing the renderer draws while the window eases between
/// the two sizes over `ANIMATION_DURATION_SECONDS` (`ease_in_out`).
#[derive(Clone, Debug, PartialEq)]
pub struct ModeTransition {
    pub change: ExpandableModeChange,
    /// In drawing order; a cell with no track is not drawn.
    pub cells: Vec<CellTrack>,
    /// The chevron column; its alpha is the separator's and glyph's.
    pub chevron: Track,
    /// The keyed band, fading in where it lands or out where it was.
    pub band: Option<Rect>,
}

impl ModeTransition {
    /// The same choreography played the other way.
    fn reversed(self) -> Self {
        Self {
            change: match self.change {
                ExpandableModeChange::Expand => ExpandableModeChange::Collapse,
                ExpandableModeChange::Collapse => ExpandableModeChange::Expand,
            },
            cells: self
                .cells
                .into_iter()
                .map(|cell| CellTrack {
                    track: cell.track.reversed(),
                    ..cell
                })
                .collect(),
            chevron: self.chevron.reversed(),
            band: self.band,
        }
    }

    /// The keyed band's opacity at `eased`.
    pub fn band_alpha(&self, eased: f32) -> f32 {
        match self.change {
            ExpandableModeChange::Expand => eased,
            ExpandableModeChange::Collapse => 1.0 - eased,
        }
    }
}

/// The ease-in-out cubic bezier (0.42, 0, 0.58, 1) — CSS `ease-in-out` and
/// macOS `CAMediaTimingFunction(.easeInEaseOut)` — at linear `progress`,
/// solved by Newton iteration as macOS `ExpandableCandidatePanel.easeInOut`.
pub fn ease_in_out(progress: f32) -> f32 {
    let x = progress.clamp(0.0, 1.0);
    let (x1, x2) = (0.42, 0.58);
    let mut t = x;
    for _ in 0..8 {
        let mt = 1.0 - t;
        let error = 3.0 * mt * mt * t * x1 + 3.0 * mt * t * t * x2 + t * t * t - x;
        if error.abs() < 1e-7 {
            break;
        }
        let derivative = 3.0 * mt * mt * x1 + 6.0 * mt * t * (x2 - x1) + 3.0 * t * t * (1.0 - x2);
        if derivative.abs() < 1e-7 {
            break;
        }
        t -= error / derivative;
    }
    3.0 * t * t - 2.0 * t * t * t
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExpandableListModel {
    axis: ExpandableAxis,
    count: usize,
    item_height: f32,
    chrome_width: f32,
    scroller_width: f32,
    /// The collapsed cells in reading order, framed once per list: the
    /// first page of the horizontal packer end to end (row), or the first
    /// page stacked at one aligned width (column).
    collapsed_cells: Vec<CollapsedCell>,
    /// The collapsed cells' extent, left of the chevron column.
    collapsed_content_width: f32,
    collapsed_content_height: f32,
    /// The column's candidate column width, which every row's annotation
    /// starts after; `None` for a row.
    collapsed_primary_column_width: Option<f32>,
    /// Row: the whole list, row-major. Column: the selected page only —
    /// `ROWS_PER_COLUMN`-tall columns, `column_count` of them a page,
    /// Microsoft Bopomofo's shape — transposed into visual rows
    /// (`ExpandedGridLayout::column_major_page`), so renderers draw both from
    /// the same rows.
    grid: ExpandedGridLayout,
    column_width: f32,
    /// The unfolded column's candidate column width, shared by every column
    /// so their annotations start at one x; `None` for a row.
    expanded_primary_column_width: Option<f32>,
    mode: ExpandableDisplayMode,
    selected_index: usize,
    /// The expanded viewport's scroll offset in points.
    scroll_y: f32,
    /// The last mode change's choreography, set by `expand` / `collapse`.
    transition: Option<ModeTransition>,
}

impl ExpandableListModel {
    pub const MAX_VISIBLE_ROWS: usize = 5;
    pub const SEPARATOR_HEIGHT: f32 = 1.0;
    /// Two thirds of a page: `PAGE_SIZE - PAGE_SIZE / 3`.
    pub const EXPANDED_COLUMN_COUNT: usize =
        HorizontalPageLayout::PAGE_SIZE - HorizontalPageLayout::PAGE_SIZE / 3;
    /// An unfolded column's height in candidates: one page of the list.
    pub const ROWS_PER_COLUMN: usize = HorizontalPageLayout::PAGE_SIZE;
    /// How many columns an unfolded column shows at once at most — a third of
    /// a page, Microsoft Bopomofo's three.
    pub const COLUMNS_PER_PAGE: usize = HorizontalPageLayout::PAGE_SIZE / 3;
    /// The unfold's duration, for the renderer: quicker than upstream's
    /// measured 0.183 s, so the grid never reads as a wait (USER 2026-11-08).
    /// Twin of macOS `ExpandableCandidatePanel.animationDuration`.
    pub const ANIMATION_DURATION_SECONDS: f32 = 0.13;

    /// Lays out `measured_widths` (one per candidate; capped at
    /// [`MAX_DISPLAY_CANDIDATES`] here) under `geometry`.
    pub fn new(measured_widths: &[f32], geometry: ExpandableGeometryInput<'_>) -> Self {
        let measured_widths = &measured_widths[..measured_widths.len().min(MAX_DISPLAY_CANDIDATES)];
        let count = measured_widths.len();
        let (axis, collapsed, column_width, grid, expanded_primary_column_width) = match geometry
            .collapsed
        {
            CollapsedLayout::Row => {
                let column_width = Self::resolved_column_width(measured_widths, &geometry);
                (
                    ExpandableAxis::Row,
                    Self::collapsed_row(measured_widths, &geometry),
                    column_width,
                    ExpandedGridLayout::compute(
                        measured_widths,
                        column_width,
                        Self::EXPANDED_COLUMN_COUNT,
                    ),
                    None,
                )
            }
            CollapsedLayout::Column {
                metrics,
                primary_widths,
                annotation_widths,
            } => {
                let collapsed = Self::collapsed_column(
                    count,
                    metrics,
                    primary_widths,
                    annotation_widths,
                    &geometry,
                );
                let (column_width, columns_per_page, primary_column_width) = Self::expanded_columns(
                    count,
                    metrics,
                    primary_widths,
                    annotation_widths,
                    collapsed.content_width,
                    &geometry,
                );
                (
                    ExpandableAxis::Column,
                    collapsed,
                    column_width,
                    ExpandedGridLayout::column_major_page(
                        count,
                        Self::ROWS_PER_COLUMN,
                        0,
                        columns_per_page,
                    ),
                    Some(primary_column_width),
                )
            }
        };
        Self {
            axis,
            count,
            item_height: geometry.item_height,
            chrome_width: geometry.chrome_width,
            scroller_width: geometry.scroller_width,
            collapsed_cells: collapsed.cells,
            collapsed_content_width: collapsed.content_width,
            collapsed_content_height: collapsed.content_height,
            collapsed_primary_column_width: collapsed.primary_column_width,
            grid,
            column_width,
            expanded_primary_column_width,
            mode: ExpandableDisplayMode::Collapsed,
            selected_index: 0,
            scroll_y: 0.0,
            transition: None,
        }
    }

    /// The collapsed row: the first page of the horizontal packer, its cells
    /// end to end.
    fn collapsed_row(widths: &[f32], geometry: &ExpandableGeometryInput<'_>) -> CollapsedFrames {
        let page = HorizontalPageLayout::pack_with_chrome(
            widths,
            geometry.base_width,
            geometry.maximum_window_width,
            geometry.chrome_width,
        )
        .pages
        .into_iter()
        .next()
        .unwrap_or_default();
        let mut x = 0.0;
        let cells = page
            .iter()
            .map(|slot| {
                let rect = Rect::new(x, 0.0, slot.width, geometry.item_height);
                x += slot.width;
                CollapsedCell {
                    candidate_index: slot.candidate_index,
                    rect,
                }
            })
            .collect();
        CollapsedFrames {
            cells,
            content_width: x,
            content_height: geometry.item_height,
            primary_column_width: None,
        }
    }

    /// The collapsed column: the first page stacked at the grid's row pitch,
    /// sized as the vertical list sizes itself (`VerticalListModel`
    /// `resolve_width`) — the widest candidate column plus the widest
    /// annotation, floored at one slot and capped by the screen, less the
    /// chevron column when there is one.
    fn collapsed_column(
        count: usize,
        metrics: &CandidateMetrics,
        primary_widths: &[f32],
        annotation_widths: &[Option<f32>],
        geometry: &ExpandableGeometryInput<'_>,
    ) -> CollapsedFrames {
        let rows = count
            .min(Self::ROWS_PER_COLUMN)
            .min(primary_widths.len())
            .min(annotation_widths.len());
        let chrome = if count > rows {
            geometry.chrome_width
        } else {
            0.0
        };
        let (width, primary_column_width) = Self::aligned_column(
            metrics,
            &primary_widths[..rows],
            &annotation_widths[..rows],
            chrome,
            geometry,
        );
        let pitch = geometry.item_height + Self::SEPARATOR_HEIGHT;
        let cells = (0..rows)
            .map(|candidate_index| CollapsedCell {
                candidate_index,
                rect: Rect::new(
                    0.0,
                    candidate_index as f32 * pitch,
                    width,
                    geometry.item_height,
                ),
            })
            .collect();
        CollapsedFrames {
            cells,
            content_width: width,
            content_height: Self::rows_height(rows, geometry.item_height),
            primary_column_width: Some(primary_column_width),
        }
    }

    /// One column sized as the vertical list sizes itself: the widest
    /// candidate column plus the widest annotation, floored at one slot and
    /// capped by the screen less `chrome`. Answers the cell width and the
    /// candidate column width every annotation starts after.
    fn aligned_column(
        metrics: &CandidateMetrics,
        primary_widths: &[f32],
        annotation_widths: &[Option<f32>],
        chrome: f32,
        geometry: &ExpandableGeometryInput<'_>,
    ) -> (f32, f32) {
        let (widest_primary, widest_annotation) = widest_columns(primary_widths, annotation_widths);
        let width = metrics
            .cell_width(widest_primary, widest_annotation)
            .max(geometry.base_width)
            .min(
                geometry
                    .base_width
                    .max(geometry.maximum_window_width - chrome),
            );
        let primary_column_width = widest_primary
            .min(metrics.maximum_primary_column_width(width, metrics.horizontal_padding()));
        (width, primary_column_width)
    }

    /// The unfolded column's shared width — measured over the whole list so
    /// it holds still while paging, never narrower than the collapsed column
    /// so column 0 does not shrink as it unfolds — and how many columns a
    /// page shows: up to `COLUMNS_PER_PAGE`, as many as the screen fits
    /// beside the chevron, never more than the list fills.
    fn expanded_columns(
        count: usize,
        metrics: &CandidateMetrics,
        primary_widths: &[f32],
        annotation_widths: &[Option<f32>],
        collapsed_width: f32,
        geometry: &ExpandableGeometryInput<'_>,
    ) -> (f32, usize, f32) {
        let rows = count.min(primary_widths.len()).min(annotation_widths.len());
        let (width, primary_column_width) = Self::aligned_column(
            metrics,
            &primary_widths[..rows],
            &annotation_widths[..rows],
            geometry.chrome_width,
            geometry,
        );
        let width = width.max(collapsed_width);
        let columns = count.div_ceil(Self::ROWS_PER_COLUMN);
        let fit = |chrome: f32| {
            let fit = ((geometry.maximum_window_width - chrome) / width)
                .floor()
                .max(1.0) as usize;
            Self::COLUMNS_PER_PAGE.min(fit).min(columns).max(1)
        };
        // A list that fits one page needs no chevron column to make room for.
        let unpaged = fit(0.0);
        let columns_per_page = if unpaged >= columns {
            unpaged
        } else {
            fit(geometry.chrome_width)
        };
        (width, columns_per_page, primary_column_width)
    }

    /// `rows` items with a separator between each two.
    fn rows_height(rows: usize, item_height: f32) -> f32 {
        rows as f32 * item_height + rows.saturating_sub(1) as f32 * Self::SEPARATOR_HEIGHT
    }

    /// The nine-slot row the collapsed mode packs to, widened until the
    /// longest candidate fits a full row of columns, and capped by what the
    /// screen leaves once the scroller has its share. Widening the columns
    /// rather than letting a cell span more of them keeps the grid a grid
    /// (columns widen, spans stay whole).
    fn resolved_column_width(widths: &[f32], geometry: &ExpandableGeometryInput<'_>) -> f32 {
        let baseline = HorizontalPageLayout::row_budget(geometry.base_width);
        let budget = geometry.maximum_window_width - geometry.scroller_width;
        let widest = widths.iter().copied().fold(0.0, f32::max);
        let narrowest = geometry.base_width * Self::EXPANDED_COLUMN_COUNT as f32;
        let width = baseline.max(widest).min(narrowest.max(budget));
        width / Self::EXPANDED_COLUMN_COUNT as f32
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn axis(&self) -> ExpandableAxis {
        self.axis
    }

    pub fn mode(&self) -> ExpandableDisplayMode {
        self.mode
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn scroll_y(&self) -> f32 {
        self.scroll_y
    }

    pub fn collapsed_cells(&self) -> &[CollapsedCell] {
        &self.collapsed_cells
    }

    /// The column's candidate column width (`None` for a row): the renderer
    /// starts every row's annotation after it.
    pub fn collapsed_primary_column_width(&self) -> Option<f32> {
        self.collapsed_primary_column_width
    }

    pub fn grid(&self) -> &ExpandedGridLayout {
        &self.grid
    }

    pub fn column_width(&self) -> f32 {
        self.column_width
    }

    /// Whether the list holds more than the collapsed cells show — what the
    /// chevron, the paging-edge corner and the expand paths all key on.
    pub fn has_overflow(&self) -> bool {
        self.count > self.collapsed_cells.len()
    }

    /// The grid's columns: six under a row, a page of columns beside a
    /// column — the same on every page, so paging never resizes the window.
    pub fn grid_width(&self) -> f32 {
        self.column_width * self.grid.column_count as f32
    }

    /// The candidate column width every unfolded column's annotations start
    /// after (`None` for a row, whose grid cells keep their scripts inline).
    pub fn expanded_primary_column_width(&self) -> Option<f32> {
        self.expanded_primary_column_width
    }

    /// Whether an unfolded column has a page after the selection's.
    pub fn has_next_page(&self) -> bool {
        self.page_span()
            .is_some_and(|span| (self.selected_index / span + 1) * span < self.count)
    }

    /// The candidates one page of an unfolded column holds; `None` for a row.
    fn page_span(&self) -> Option<usize> {
        (self.axis == ExpandableAxis::Column)
            .then(|| self.grid.column_count * Self::ROWS_PER_COLUMN)
    }

    /// Whether an unfolded column pages at all — what reserves its chevron
    /// column, on every page alike.
    pub fn is_paged(&self) -> bool {
        self.page_span().is_some_and(|span| self.count > span)
    }

    /// The unfolded column's chevron frame, right of the grid at the window's
    /// full height, on every page of a list that pages (`None` otherwise).
    /// The renderer draws `›` in it while `has_next_page`.
    pub fn expanded_chevron_rect(&self) -> Option<Rect> {
        self.is_paged().then(|| {
            Rect::new(
                self.grid_width(),
                0.0,
                self.chrome_width,
                // A column keeps the collapsed height as it unfolds.
                self.collapsed_window_height(),
            )
        })
    }

    pub fn item_height(&self) -> f32 {
        self.item_height
    }

    pub fn row_height(&self) -> f32 {
        self.item_height + Self::SEPARATOR_HEIGHT
    }

    /// The collapsed cells' extent left of the chevron: a row's cells laid
    /// end to end, or the column's one shared width.
    pub fn collapsed_content_width(&self) -> f32 {
        self.collapsed_content_width
    }

    /// The collapsed window: the cells plus the chevron column when there is
    /// more to show.
    pub fn collapsed_window_width(&self) -> f32 {
        self.collapsed_content_width()
            + if self.has_overflow() {
                self.chrome_width
            } else {
                0.0
            }
    }

    /// One item tall for a row; for a column, its rows with a separator
    /// between each two — the grid's row pitch, so the unfold keeps it.
    pub fn collapsed_window_height(&self) -> f32 {
        self.collapsed_content_height
    }

    /// The chevron's collapsed frame: the chrome column right of the cells,
    /// the collapsed window's full height.
    pub fn chevron_rect(&self) -> Rect {
        Rect::new(
            self.collapsed_content_width(),
            0.0,
            self.chrome_width,
            self.collapsed_window_height(),
        )
    }

    /// How many grid rows the expanded viewport shows: `MAX_VISIBLE_ROWS`
    /// under a row; beside a column, the column's own rows — the window
    /// keeps its height as it unfolds to the right (USER 2026-10-31).
    pub fn visible_rows(&self) -> usize {
        match self.axis {
            ExpandableAxis::Row => Self::MAX_VISIBLE_ROWS,
            ExpandableAxis::Column => self.collapsed_cells.len(),
        }
    }

    /// The half row a scrolling viewport shows under its last full one, as a
    /// hint there is more. Under a row only: a column keeps its exact height.
    fn peek(&self) -> f32 {
        match self.axis {
            ExpandableAxis::Row => self.item_height / 2.0,
            ExpandableAxis::Column => 0.0,
        }
    }

    /// Under a row the window grows with the grid up to `visible_rows` plus a
    /// half-row peek; beside a column it stays exactly the collapsed height,
    /// padding below a shorter page, and pages instead of scrolling — its
    /// chevron column added on every page of a list that pages.
    pub fn expanded_geometry(&self) -> ExpandedGeometry {
        let content_height = Self::rows_height(self.grid.rows.len(), self.item_height);
        let peek = self.peek();
        let viewport_height = Self::rows_height(self.visible_rows(), self.item_height) + peek;
        let needs_scrolling = content_height > viewport_height;
        ExpandedGeometry {
            window_width: self.grid_width()
                + if needs_scrolling {
                    self.scroller_width
                } else {
                    0.0
                }
                + if self.is_paged() {
                    self.chrome_width
                } else {
                    0.0
                },
            window_height: if needs_scrolling || self.axis == ExpandableAxis::Column {
                viewport_height
            } else {
                content_height
            },
            container_height: if needs_scrolling {
                content_height + peek
            } else {
                content_height
            },
            needs_scrolling,
        }
    }

    /// The collapsed cell frames: end to end along a row, stacked down a
    /// column.
    pub fn collapsed_cell_rect(&self, slot: usize) -> Option<Rect> {
        self.collapsed_cells.get(slot).map(|cell| cell.rect)
    }

    /// A grid cell's frame in CONTENT coordinates (before the scroll offset).
    pub fn grid_cell_rect(&self, candidate_index: usize) -> Option<Rect> {
        let (row, cell) = self.grid.position_of(candidate_index)?;
        Some(Rect::new(
            cell.column_start as f32 * self.column_width,
            row as f32 * self.row_height(),
            cell.column_span as f32 * self.column_width,
            self.item_height,
        ))
    }

    /// The frame (content coordinates) of the cells the slot keys number —
    /// the selected grid row at full width, or beside a column the selected
    /// column down its filled cells. The highlight paints it behind them.
    pub fn keyed_band_rect(&self) -> Option<Rect> {
        let (row, cell) = self.grid.position_of(self.selected_index)?;
        Some(match self.axis {
            ExpandableAxis::Row => Rect::new(
                0.0,
                row as f32 * self.row_height(),
                self.grid_width(),
                self.item_height,
            ),
            ExpandableAxis::Column => {
                let filled = self
                    .grid
                    .rows
                    .iter()
                    .take_while(|row| row.iter().any(|c| c.column_start == cell.column_start))
                    .count();
                Rect::new(
                    cell.column_start as f32 * self.column_width,
                    0.0,
                    self.column_width,
                    Self::rows_height(filled, self.item_height),
                )
            }
        })
    }

    /// The choreography of the mode change the last `navigate` / `select` /
    /// `expand` / `collapse` reported — `None` after a call that changed no
    /// mode — for the renderer to play once.
    pub fn mode_transition(&self) -> Option<&ModeTransition> {
        self.transition.as_ref()
    }

    /// Drops the transition: a list laid out again comes back in the mode it
    /// was in, which is no change to play.
    pub(crate) fn forget_transition(&mut self) {
        self.transition = None;
    }

    /// The unfold between the collapsed cells and the grid as it stands —
    /// the page on screen beside a column — which a fold plays backwards.
    /// Under a row, the collapsed cells on grid row 0 glide to it (one
    /// drawing); the rest — the overflow — slide out right fading, while
    /// their grid twins slide in from the left; every other grid cell stands
    /// still. Beside a column, each cell cross-fades from its collapsed
    /// frame into its grid frame. The chevron is pushed to the grid's edge
    /// fading, or to a paged column's chevron column, its `›` kept while a
    /// next page follows.
    fn unfold_transition(&self, band: Option<Rect>) -> ModeTransition {
        let collapsed_at = |index: usize| {
            self.collapsed_cells
                .iter()
                .find(|cell| cell.candidate_index == index)
                .map(|cell| cell.rect)
        };
        let grid_cells = || {
            self.grid
                .rows
                .iter()
                .flatten()
                .map(|cell| cell.candidate_index)
        };
        let mut cells = Vec::new();
        match self.axis {
            ExpandableAxis::Row => {
                let row0: Vec<usize> = self
                    .grid
                    .rows
                    .first()
                    .map(|row| row.iter().map(|cell| cell.candidate_index).collect())
                    .unwrap_or_default();
                let exit_x = self.collapsed_window_width();
                for cell in &self.collapsed_cells {
                    let index = cell.candidate_index;
                    cells.push(match self.grid_cell_rect(index) {
                        Some(to) if row0.contains(&index) => CellTrack {
                            candidate_index: index,
                            layer: CellLayer::Shared,
                            track: Track::new(cell.rect, to, 1.0, 1.0),
                        },
                        _ => CellTrack {
                            candidate_index: index,
                            layer: CellLayer::Collapsed,
                            track: Track::new(
                                cell.rect,
                                Rect {
                                    x: exit_x,
                                    ..cell.rect
                                },
                                1.0,
                                0.0,
                            ),
                        },
                    });
                }
                for index in grid_cells().filter(|index| !row0.contains(index)) {
                    let Some(to) = self.grid_cell_rect(index) else {
                        continue;
                    };
                    // The collapsed cells are candidates 0.., so a lower index is
                    // an overflow cell's twin.
                    let track = if index < self.collapsed_cells.len() {
                        Track::new(
                            Rect {
                                x: -(to.x + to.width),
                                ..to
                            },
                            to,
                            0.0,
                            1.0,
                        )
                    } else {
                        Track::still(to)
                    };
                    cells.push(CellTrack {
                        candidate_index: index,
                        layer: CellLayer::Expanded,
                        track,
                    });
                }
            }
            ExpandableAxis::Column => {
                for cell in &self.collapsed_cells {
                    let to = self
                        .grid_cell_rect(cell.candidate_index)
                        .unwrap_or(cell.rect);
                    cells.push(CellTrack {
                        candidate_index: cell.candidate_index,
                        layer: CellLayer::Collapsed,
                        track: Track::new(cell.rect, to, 1.0, 0.0),
                    });
                }
                for index in grid_cells() {
                    let Some(to) = self.grid_cell_rect(index) else {
                        continue;
                    };
                    cells.push(CellTrack {
                        candidate_index: index,
                        layer: CellLayer::Expanded,
                        track: Track::new(collapsed_at(index).unwrap_or(to), to, 0.0, 1.0),
                    });
                }
            }
        }
        let from = self.chevron_rect();
        let chevron_to = self.expanded_chevron_rect().unwrap_or(Rect {
            x: (self.grid_width() - from.width).max(from.x),
            ..from
        });
        let shown = |is_shown: bool| if is_shown { 1.0 } else { 0.0 };
        ModeTransition {
            change: ExpandableModeChange::Expand,
            cells,
            chevron: Track::new(
                from,
                chevron_to,
                shown(self.has_overflow()),
                shown(self.has_next_page()),
            ),
            band,
        }
    }

    /// The candidate under `point` in WINDOW coordinates, for the current
    /// mode (the expanded viewport's scroll offset applied).
    pub fn hit_test(&self, point: Point) -> Option<usize> {
        if point.x < 0.0 || point.y < 0.0 {
            return None;
        }
        match self.mode {
            ExpandableDisplayMode::Collapsed => self
                .collapsed_cells
                .iter()
                .find(|cell| cell.rect.contains(point))
                .map(|cell| cell.candidate_index),
            ExpandableDisplayMode::Expanded => {
                let content_y = point.y + self.scroll_y;
                let row = (content_y / self.row_height()).floor() as usize;
                if content_y - row as f32 * self.row_height() >= self.item_height {
                    return None;
                }
                let column = (point.x / self.column_width).floor() as usize;
                self.grid
                    .rows
                    .get(row)?
                    .iter()
                    .find(|cell| {
                        column >= cell.column_start && column < cell.column_start + cell.column_span
                    })
                    .map(|cell| cell.candidate_index)
            }
        }
    }

    /// The slot keys address the collapsed row, or the selected grid row —
    /// one row at a time, so a chord always names exactly one candidate.
    /// Beside a column they address the selected column, never past its
    /// ninth cell into the next one.
    pub fn candidate_index_for_slot(&self, slot: usize) -> Option<usize> {
        match self.mode {
            ExpandableDisplayMode::Collapsed => self
                .collapsed_cells
                .get(slot)
                .map(|cell| cell.candidate_index),
            // The grid holds one page, so its rows end at the column's foot.
            ExpandableDisplayMode::Expanded if self.axis == ExpandableAxis::Column => {
                let (_, keyed) = self.grid.position_of(self.selected_index)?;
                self.grid
                    .rows
                    .get(slot)?
                    .iter()
                    .find(|cell| cell.column_start == keyed.column_start)
                    .map(|cell| cell.candidate_index)
            }
            ExpandableDisplayMode::Expanded => {
                let (row_index, _) = self.grid.position_of(self.selected_index)?;
                self.grid.rows[row_index]
                    .get(slot)
                    .map(|cell| cell.candidate_index)
            }
        }
    }

    /// The row the selection is on, in the expanded grid.
    pub fn selected_row(&self) -> Option<usize> {
        self.grid
            .position_of(self.selected_index)
            .map(|(row, _)| row)
    }

    /// The grid row at the viewport's top, with the half-row bias
    /// (rounds to the nearest row).
    pub fn scroll_top_row(&self) -> usize {
        ((self.scroll_y.max(0.0) + self.row_height() / 2.0) / self.row_height()).floor() as usize
    }

    pub fn navigate(&mut self, direction: CandidateNavigation) -> Option<ExpandableModeChange> {
        // A transition describes only the change this call reports.
        self.transition = None;
        if self.count == 0 {
            return None;
        }
        match self.mode {
            ExpandableDisplayMode::Collapsed => self.navigate_collapsed(direction),
            ExpandableDisplayMode::Expanded => self.navigate_expanded(direction),
        }
    }

    fn navigate_collapsed(
        &mut self,
        direction: CandidateNavigation,
    ) -> Option<ExpandableModeChange> {
        use CandidateNavigation::*;
        // The arrows along the list walk it; the one across it unfolds.
        let (forward, backward) = match self.axis {
            ExpandableAxis::Row => (Right, Left),
            ExpandableAxis::Column => (Down, Up),
        };
        let across = self.axis.unfold_direction();
        match direction {
            NextCandidate => self.select_next_collapsed(),
            PreviousCandidate => self.select(self.selected_index.saturating_sub(1)),
            PageDown => self.expand_if_overflowing(),
            PageUp => None,
            _ if direction == forward => self.select_next_collapsed(),
            _ if direction == backward => self.select(self.selected_index.saturating_sub(1)),
            // `→` beside a column steps into the next column as it unfolds
            // it, the way walking off the foot lands past it.
            _ if direction == across && self.axis == ExpandableAxis::Column => {
                if !self.has_overflow() {
                    return None;
                }
                self.selected_index =
                    (self.selected_index + Self::ROWS_PER_COLUMN).min(self.count - 1);
                self.expand()
            }
            _ if direction == across => self.expand_if_overflowing(),
            _ => None,
        }
    }

    fn select_next_collapsed(&mut self) -> Option<ExpandableModeChange> {
        let target = self.selected_index + 1;
        if target >= self.count {
            return None;
        }
        // Walking off the collapsed list's end both expands AND lands on the
        // next candidate — an expand that kept the old selection would eat
        // one keypress.
        self.select(target)
    }

    fn expand_if_overflowing(&mut self) -> Option<ExpandableModeChange> {
        if self.has_overflow() {
            self.expand()
        } else {
            None
        }
    }

    /// An unfolded column's keys, by index arithmetic: `↑` / `↓` walk the
    /// list down each column and on into the next, `←` / `→` step a column
    /// keeping the row, PageUp / PageDown a page keeping the cell's place.
    /// `←` in the first column, `↑` on the first candidate and PageUp on
    /// the first page fold the window.
    fn navigate_expanded_columns(
        &mut self,
        direction: CandidateNavigation,
        page_span: usize,
    ) -> Option<ExpandableModeChange> {
        use CandidateNavigation::*;
        let index = self.selected_index;
        let last = self.count - 1;
        let rows = Self::ROWS_PER_COLUMN;
        match direction {
            Down | NextCandidate if index < last => self.select(index + 1),
            Up | PreviousCandidate if index > 0 => self.select(index - 1),
            Up => self.collapse(),
            Right if (index / rows + 1) * rows <= last => self.select((index + rows).min(last)),
            Left if index >= rows => self.select(index - rows),
            Left => self.collapse(),
            PageDown if (index / page_span + 1) * page_span <= last => {
                self.select((index + page_span).min(last))
            }
            PageUp if index >= page_span => self.select(index - page_span),
            PageUp => self.collapse(),
            _ => None,
        }
    }

    fn navigate_expanded(
        &mut self,
        direction: CandidateNavigation,
    ) -> Option<ExpandableModeChange> {
        use CandidateNavigation::*;
        if let Some(span) = self.page_span() {
            return self.navigate_expanded_columns(direction, span);
        }
        match direction {
            Right | NextCandidate => {
                if self.selected_index + 1 < self.count {
                    self.select(self.selected_index + 1)
                } else {
                    None
                }
            }
            // `←` walks the row and folds only at the first cell.
            Left => {
                if self.selected_index > 0 {
                    self.select(self.selected_index - 1)
                } else {
                    self.collapse()
                }
            }
            // Unlike `←`, this never folds the window: its label says
            // "previous candidate", and a collapse would be something else.
            PreviousCandidate => {
                if self.selected_index > 0 {
                    self.select(self.selected_index - 1)
                } else {
                    None
                }
            }
            Down => match self.grid.vertical_target(self.selected_index, 1) {
                Some(target) => self.select(target),
                None => None,
            },
            Up => match self.grid.vertical_target(self.selected_index, -1) {
                Some(target) => self.select(target),
                None if self.selected_row() == Some(0) => self.collapse(),
                None => None,
            },
            PageDown => self.page_viewport(true),
            PageUp => self.page_viewport(false),
        }
    }

    /// Pages the expanded viewport a whole `visible_rows`, keeping the
    /// highlight at its visual row — read off the ACTUAL scroll offset, so a
    /// wheel scroll before the key counts. Paging up from the very top folds
    /// the window back into the row it came from.
    fn page_viewport(&mut self, forward: bool) -> Option<ExpandableModeChange> {
        let (highlight_row, cell) = self.grid.position_of(self.selected_index)?;
        let top_row = self.scroll_top_row();
        if !forward && top_row == 0 && highlight_row == 0 {
            return self.collapse();
        }
        let offset = highlight_row.saturating_sub(top_row);
        let last_row = self.grid.rows.len().checked_sub(1)?;
        let step = self.visible_rows() as i32 * if forward { 1 } else { -1 };
        let new_row = (highlight_row as i32 + step).clamp(0, last_row as i32) as usize;
        let target = self
            .grid
            .overlapping_candidate(
                new_row,
                cell.column_start,
                cell.column_start + cell.column_span,
                forward,
            )
            .or_else(|| self.grid.rows[new_row].last().map(|c| c.candidate_index))?;
        self.scroll_row_to_top(new_row.saturating_sub(offset));
        self.selected_index = target;
        self.ensure_selected_row_visible();
        None
    }

    /// Selects `index` (a click, a walk). A candidate the collapsed row does
    /// not show unfolds the window.
    pub fn select(&mut self, index: usize) -> Option<ExpandableModeChange> {
        self.transition = None;
        if index >= self.count {
            return None;
        }
        self.selected_index = index;
        if self.mode == ExpandableDisplayMode::Collapsed && index >= self.collapsed_cells.len() {
            return self.expand();
        }
        if self.mode == ExpandableDisplayMode::Expanded {
            self.follow_selection();
        }
        None
    }

    pub fn expand(&mut self) -> Option<ExpandableModeChange> {
        self.transition = None;
        if self.mode == ExpandableDisplayMode::Expanded {
            return None;
        }
        self.mode = ExpandableDisplayMode::Expanded;
        self.scroll_y = 0.0;
        self.follow_selection();
        // After the selection settled: it decides the page a column unfolds onto.
        self.transition = Some(self.unfold_transition(self.keyed_band_rect()));
        Some(ExpandableModeChange::Expand)
    }

    pub fn collapse(&mut self) -> Option<ExpandableModeChange> {
        self.transition = None;
        if self.mode == ExpandableDisplayMode::Collapsed {
            return None;
        }
        // The band fades out where it was, before the selection moves.
        let band = self.keyed_band_rect();
        self.mode = ExpandableDisplayMode::Collapsed;
        self.scroll_y = 0.0;
        // A selection beyond the collapsed cells clamps to the last of them;
        // beside a column it keeps its row first, as `←` and PageUp fold from
        // any column.
        if self.axis == ExpandableAxis::Column {
            self.selected_index %= Self::ROWS_PER_COLUMN;
        }
        if self.selected_index >= self.collapsed_cells.len() {
            self.selected_index = self.collapsed_cells.len().saturating_sub(1);
        }
        self.follow_selection();
        // The unfold backwards: every fold starts on the first page (`←` in
        // the first column, `↑` on 0, PageUp on page 0), the grid that
        // page, and the viewport scrolled home, as macOS folds.
        self.transition = Some(self.unfold_transition(band).reversed());
        Some(ExpandableModeChange::Collapse)
    }

    /// The renderer scrolled the expanded viewport to `offset`.
    pub fn on_viewport_scrolled(&mut self, offset: f32) {
        if self.mode == ExpandableDisplayMode::Expanded {
            self.scroll_y = offset.max(0.0).min(self.max_scroll_y());
        }
    }

    fn max_scroll_y(&self) -> f32 {
        let geometry = self.expanded_geometry();
        (geometry.container_height - geometry.window_height).max(0.0)
    }

    fn scroll_row_to_top(&mut self, row: usize) {
        self.scroll_y = (row as f32 * self.row_height()).min(self.max_scroll_y());
    }

    /// Keeps the grid on the selection: beside a column, rebuilds the grid
    /// as the selection's page — the page is read off the selection, so every
    /// path that moves the selection must end here; under an unfolded row,
    /// scrolls its row into view.
    fn follow_selection(&mut self) {
        if let Some(span) = self.page_span() {
            let columns = self.grid.column_count;
            self.grid = ExpandedGridLayout::column_major_page(
                self.count,
                Self::ROWS_PER_COLUMN,
                self.selected_index / span * columns,
                columns,
            );
        } else if self.mode == ExpandableDisplayMode::Expanded {
            self.ensure_selected_row_visible();
        }
    }

    fn ensure_selected_row_visible(&mut self) {
        let Some(row) = self.selected_row() else {
            return;
        };
        let row_top = row as f32 * self.row_height();
        let row_bottom = row_top + self.item_height;
        let viewport = self.expanded_geometry().window_height;
        if row_top < self.scroll_y {
            self.scroll_y = row_top;
        } else if row_bottom > self.scroll_y + viewport {
            let target = (row_bottom + self.peek() - viewport).min(self.max_scroll_y());
            self.scroll_y = target.max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidates::metrics::test_support;
    use crate::candidates::{CandidateCellArrangement, CandidateIndexLabel};
    use crate::settings::CandidateSizeChoice;
    use CandidateNavigation::*;

    fn geometry() -> ExpandableGeometryInput<'static> {
        ExpandableGeometryInput {
            collapsed: CollapsedLayout::Row,
            base_width: 10.0,
            maximum_window_width: 640.0,
            chrome_width: 20.0,
            scroller_width: 15.0,
            item_height: 30.0,
        }
    }

    fn model(widths: &[f32]) -> ExpandableListModel {
        ExpandableListModel::new(widths, geometry())
    }

    fn inline_metrics() -> CandidateMetrics {
        test_support::metrics(CandidateSizeChoice::Large, CandidateCellArrangement::Inline)
    }

    /// A column whose cells measure `primary` / `annotation` apart — the
    /// summed widths for the grid come from the same metrics.
    fn column_model_with(
        metrics: &CandidateMetrics,
        primary: &[f32],
        annotation: &[Option<f32>],
    ) -> ExpandableListModel {
        let widths: Vec<f32> = primary
            .iter()
            .zip(annotation)
            .map(|(&p, &a)| metrics.cell_width(p, a))
            .collect();
        ExpandableListModel::new(
            &widths,
            ExpandableGeometryInput {
                collapsed: CollapsedLayout::Column {
                    metrics,
                    primary_widths: primary,
                    annotation_widths: annotation,
                },
                base_width: metrics.base_width(),
                ..geometry()
            },
        )
    }

    /// `count` annotation-less candidates whose candidate text is `primary` wide.
    fn column_model(metrics: &CandidateMetrics, primary: f32, count: usize) -> ExpandableListModel {
        column_model_with(metrics, &vec![primary; count], &vec![None; count])
    }

    #[test]
    fn collapsed_row_is_the_first_packed_page_and_overflow_gates_the_chevron() {
        // trace: 20 × 5 → the first page holds 9 (row budget 90 - 20 chrome).
        let m = model(&[5.0; 20]);
        assert_eq!(m.collapsed_cells().len(), 9);
        assert!(m.has_overflow());
        assert_eq!(m.candidate_index_for_slot(8), Some(8));
        assert_eq!(m.candidate_index_for_slot(9), None);
        assert_eq!(m.collapsed_window_width(), 9.0 * 10.0 + 20.0);
        let short = model(&[5.0; 3]);
        assert!(!short.has_overflow());
        assert_eq!(short.collapsed_content_width(), 30.0);
        assert_eq!(short.collapsed_window_height(), 30.0);
        assert_eq!(
            short.collapsed_window_width(),
            30.0,
            "no chevron without overflow"
        );
        assert_eq!(
            model(&[5.0; 300]).count(),
            MAX_DISPLAY_CANDIDATES,
            "capped here"
        );
    }

    #[test]
    fn column_width_widens_for_a_long_candidate_and_caps_at_the_screen() {
        assert_eq!(
            model(&[5.0; 20]).column_width(),
            90.0 / 6.0,
            "baseline = row budget"
        );
        let long = model(&[5.0, 300.0]);
        assert_eq!(long.column_width(), 300.0 / 6.0);
        assert_eq!(long.grid_width(), 300.0);
        let huge = model(&[5.0, 5_000.0]);
        assert_eq!(
            huge.grid_width(),
            640.0 - 15.0,
            "capped at the screen minus the scroller"
        );
    }

    #[test]
    fn expand_triggers_and_collapse_triggers_match_macos() {
        let mut m = model(&[5.0; 20]);
        assert_eq!(m.navigate(Up), None, "nothing above a collapsed row");
        assert_eq!(m.navigate(Down), Some(ExpandableModeChange::Expand));
        assert_eq!(m.mode(), ExpandableDisplayMode::Expanded);
        assert_eq!(
            m.navigate(Up),
            Some(ExpandableModeChange::Collapse),
            "up on row 0 folds"
        );
        assert_eq!(m.mode(), ExpandableDisplayMode::Collapsed);

        // Walking off the row's end expands AND advances.
        m.select(8);
        assert_eq!(m.navigate(Right), Some(ExpandableModeChange::Expand));
        assert_eq!(m.selected_index(), 9);
        assert_eq!(m.navigate(PreviousCandidate), None);
        assert_eq!(m.selected_index(), 8);
        for _ in 0..8 {
            m.navigate(PreviousCandidate);
        }
        assert_eq!(m.selected_index(), 0);
        assert_eq!(m.navigate(PreviousCandidate), None, "never collapses");
        assert_eq!(
            m.navigate(Left),
            Some(ExpandableModeChange::Collapse),
            "← at 0 folds"
        );

        // A click beyond the row unfolds; collapsing clamps the selection.
        assert_eq!(m.select(15), Some(ExpandableModeChange::Expand));
        assert_eq!(m.collapse(), Some(ExpandableModeChange::Collapse));
        assert_eq!(m.selected_index(), 8);
        let mut no_overflow = model(&[5.0; 3]);
        assert_eq!(no_overflow.navigate(Down), None);
    }

    #[test]
    fn expanded_grid_navigation_and_slots_address_the_selected_row() {
        // 20 cells of 15 → column width 15 (90/6) → each spans 1 → rows of 6.
        let mut m = model(&[15.0; 20]);
        m.expand();
        assert_eq!(m.grid().rows.len(), 4);
        assert_eq!(m.navigate(Down), None);
        assert_eq!(m.selected_index(), 6);
        assert_eq!(m.candidate_index_for_slot(0), Some(6));
        assert_eq!(m.candidate_index_for_slot(5), Some(11));
        assert_eq!(m.candidate_index_for_slot(6), None);
        m.navigate(Right);
        assert_eq!(m.selected_index(), 7);
        m.navigate(Up);
        assert_eq!(m.selected_index(), 1);
        let geometry = m.expanded_geometry();
        assert_eq!(geometry.window_width, 90.0);
        assert_eq!(geometry.window_height, 4.0 * 30.0 + 3.0);
        assert_eq!(geometry.container_height, geometry.window_height);
        assert!(!geometry.needs_scrolling);
    }

    #[test]
    fn page_viewport_keeps_the_visual_row_and_folds_at_the_top() {
        // 60 cells of 15 → 10 rows of 6 → scrolls (5.5 rows visible).
        let mut m = model(&[15.0; 60]);
        m.expand();
        let geometry = m.expanded_geometry();
        assert!(geometry.needs_scrolling);
        assert_eq!(geometry.window_height, 5.5 * 30.0 + 4.0);
        assert_eq!(geometry.container_height, 10.0 * 30.0 + 9.0 + 15.0);
        assert_eq!(geometry.window_width, 90.0 + 15.0);
        m.navigate(Down);
        assert_eq!(m.selected_row(), Some(1));
        assert_eq!(m.navigate(PageDown), None);
        assert_eq!(m.selected_row(), Some(6));
        assert_eq!(m.scroll_top_row(), 5, "highlight keeps visual row 1");
        assert_eq!(m.navigate(PageUp), None);
        assert_eq!(m.selected_row(), Some(1));
        assert_eq!(m.scroll_top_row(), 0);
        m.navigate(Up);
        assert_eq!(m.navigate(PageUp), Some(ExpandableModeChange::Collapse));
    }

    #[test]
    fn an_external_scroll_feeds_paging_and_hit_testing() {
        // trace: pageViewport:367 reads the live offset with the half-row bias.
        let mut m = model(&[15.0; 60]);
        m.expand();
        let row = m.row_height();
        m.on_viewport_scrolled(row * 2.0 + row * 0.6);
        assert_eq!(m.scroll_top_row(), 3, "mostly on row 3");
        assert_eq!(m.selected_row(), Some(0));
        assert_eq!(
            m.navigate(PageUp),
            None,
            "the highlight is not on the top row of the VIEWPORT"
        );
        assert_eq!(m.selected_row(), Some(0));
        assert_eq!(m.scroll_top_row(), 0, "paged up to the top");
        m.on_viewport_scrolled(row * 3.0);
        assert_eq!(
            m.hit_test(Point { x: 1.0, y: 1.0 }),
            Some(18),
            "row 3, column 0"
        );
        assert_eq!(m.hit_test(Point { x: 16.0, y: 1.0 }), Some(19));
        assert_eq!(m.hit_test(Point { x: 1.0, y: 30.5 }), None, "the separator");
        m.on_viewport_scrolled(1.0e9);
        assert_eq!(m.scroll_y(), m.max_scroll_y(), "clamped");
        m.collapse();
        m.on_viewport_scrolled(50.0);
        assert_eq!(m.scroll_y(), 0.0, "a collapsed row does not scroll");
        assert_eq!(m.hit_test(Point { x: 12.0, y: 5.0 }), Some(0));
        assert_eq!(m.hit_test(Point { x: 16.0, y: 5.0 }), Some(1));
        assert_eq!(m.hit_test(Point { x: 5.0, y: 31.0 }), None);
    }

    #[test]
    fn row_unfold_glides_survivors_and_swaps_the_overflow() {
        // trace: 20 × 5 → collapsed cells 10 wide (slot floor) at x 0..90,
        // chevron 90..110; grid columns 15, six a row: rows 0-5 / 6-11 / …
        let mut m = model(&[5.0; 20]);
        assert_eq!(m.mode_transition(), None, "no mode change yet");
        assert_eq!(m.chevron_rect(), Rect::new(90.0, 0.0, 20.0, 30.0));
        assert_eq!(m.grid_cell_rect(20), None);
        m.expand();
        let plan = m.mode_transition().unwrap().clone();
        assert_eq!(plan.change, ExpandableModeChange::Expand);
        // 9 collapsed drawings + the 14 grid cells off row 0.
        assert_eq!(plan.cells.len(), 23);
        assert_eq!(
            plan.cells[1],
            CellTrack {
                candidate_index: 1,
                layer: CellLayer::Shared,
                track: Track::new(
                    Rect::new(10.0, 0.0, 10.0, 30.0),
                    Rect::new(15.0, 0.0, 15.0, 30.0),
                    1.0,
                    1.0
                ),
            },
            "a survivor glides to its column"
        );
        // trace: 6 is past grid row 0 → slides out to the window's edge (110).
        assert_eq!(
            plan.cells[6],
            CellTrack {
                candidate_index: 6,
                layer: CellLayer::Collapsed,
                track: Track::new(
                    Rect::new(60.0, 0.0, 10.0, 30.0),
                    Rect::new(110.0, 0.0, 10.0, 30.0),
                    1.0,
                    0.0
                ),
            }
        );
        // trace: its grid twin at (0, 31) enters from x = -(0 + 15).
        assert_eq!(
            plan.cells[9],
            CellTrack {
                candidate_index: 6,
                layer: CellLayer::Expanded,
                track: Track::new(
                    Rect::new(-15.0, 31.0, 15.0, 30.0),
                    Rect::new(0.0, 31.0, 15.0, 30.0),
                    0.0,
                    1.0
                ),
            }
        );
        // trace: 9 is only on the grid (row 1, column 3) → stands still.
        assert_eq!(
            plan.cells[12].track,
            Track::still(Rect::new(45.0, 31.0, 15.0, 30.0))
        );
        // trace: grid 90 wide → chevron to max(90 − 20, 90) = 90, fading.
        assert_eq!(
            plan.chevron,
            Track::new(
                Rect::new(90.0, 0.0, 20.0, 30.0),
                Rect::new(90.0, 0.0, 20.0, 30.0),
                1.0,
                0.0
            )
        );
        assert_eq!(plan.band, Some(Rect::new(0.0, 0.0, 90.0, 30.0)));

        // The fold plays the same choreography backwards.
        m.collapse();
        let fold = m.mode_transition().unwrap();
        assert_eq!(fold.change, ExpandableModeChange::Collapse);
        assert_eq!(*fold, plan.clone().reversed());

        m.select(7);
        assert_eq!(m.keyed_band_rect(), Some(Rect::new(0.0, 31.0, 90.0, 30.0)));
    }

    #[test]
    fn ease_in_out_is_the_symmetric_css_curve() {
        assert_eq!(ease_in_out(0.0), 0.0);
        assert_eq!(ease_in_out(1.0), 1.0);
        assert!((ease_in_out(0.5) - 0.5).abs() < 1e-5);
        // trace: cubic-bezier(0.42, 0, 0.58, 1) is point-symmetric about 0.5.
        assert!((ease_in_out(0.25) + ease_in_out(0.75) - 1.0).abs() < 1e-5);
        assert!(ease_in_out(0.25) < 0.25, "slow start");
        let samples: Vec<f32> = (0..=20).map(|i| ease_in_out(i as f32 / 20.0)).collect();
        assert!(samples.windows(2).all(|pair| pair[0] <= pair[1]));
        assert_eq!(ease_in_out(-1.0), 0.0);
        assert_eq!(ease_in_out(2.0), 1.0);
    }

    #[test]
    fn collapsed_column_stacks_the_first_page_at_one_aligned_width() {
        let metrics = inline_metrics();
        // trace: 20 × primary 5, no annotation → width = cell_width(5, None)
        // floored at one slot; rows at the grid's pitch 30 + 1.
        let width = metrics.cell_width(5.0, None).max(metrics.base_width());
        let m = column_model(&metrics, 5.0, 20);
        assert_eq!(m.collapsed_cells().len(), 9);
        assert!(m.has_overflow());
        assert_eq!(m.candidate_index_for_slot(8), Some(8));
        assert_eq!(m.candidate_index_for_slot(9), None);
        assert_eq!(m.collapsed_content_width(), width);
        assert_eq!(m.collapsed_window_width(), width + 20.0);
        assert_eq!(m.collapsed_window_height(), 9.0 * 30.0 + 8.0);
        assert_eq!(
            m.collapsed_cell_rect(2),
            Some(Rect::new(0.0, 62.0, width, 30.0))
        );
        assert_eq!(m.collapsed_cell_rect(9), None);
        assert_eq!(m.collapsed_primary_column_width(), Some(5.0));
        assert_eq!(model(&[5.0; 3]).collapsed_primary_column_width(), None);
    }

    #[test]
    fn collapsed_column_sizes_like_the_vertical_list() {
        let metrics = inline_metrics();
        // The widest primary (row 1) and the widest annotation (row 0) are
        // different rows: the column fits both, wider than either row alone.
        let primary = [10.0, 40.0, 10.0];
        let annotation = [Some(50.0), None, Some(20.0)];
        let m = column_model_with(&metrics, &primary, &annotation);
        let aligned = metrics
            .cell_width(40.0, Some(50.0))
            .max(metrics.base_width());
        assert!(!m.has_overflow());
        assert_eq!(m.collapsed_window_width(), aligned, "no chevron column");
        assert!(aligned > metrics.cell_width(40.0, None));
        assert!(aligned > metrics.cell_width(10.0, Some(50.0)));
        assert_eq!(m.collapsed_primary_column_width(), Some(40.0));
        assert_eq!(m.collapsed_window_height(), 3.0 * 30.0 + 2.0);
        assert_eq!(
            column_model(&metrics, 5_000.0, 1).collapsed_content_width(),
            640.0,
            "no chevron to leave room for"
        );
        assert_eq!(
            column_model(&metrics, 5_000.0, 10).collapsed_content_width(),
            640.0 - 20.0,
            "capped where the chevron still fits"
        );
    }

    #[test]
    fn collapsed_column_walks_down_and_unfolds_to_the_right() {
        let metrics = inline_metrics();
        // trace: 20 candidates → columns 0-8 / 9-17 / 18-19, one page of 3.
        let mut m = column_model(&metrics, 5.0, 20);
        assert_eq!(m.axis().unfold_direction(), Right);
        assert_eq!(m.navigate(Left), None, "nothing left of a column");
        assert_eq!(m.navigate(Down), None);
        assert_eq!(m.selected_index(), 1);
        assert_eq!(m.navigate(Up), None);
        assert_eq!(m.selected_index(), 0);

        // `→` unfolds AND steps into the next column, same row.
        assert_eq!(m.navigate(Right), Some(ExpandableModeChange::Expand));
        assert_eq!(m.selected_index(), 9);
        // `←` walks back a column; only the first column folds.
        assert_eq!(m.navigate(Left), None);
        assert_eq!(m.selected_index(), 0);
        assert_eq!(m.navigate(Left), Some(ExpandableModeChange::Collapse));
        assert_eq!(m.selected_index(), 0);

        // Walking off the column's foot expands AND advances.
        m.select(8);
        assert_eq!(m.navigate(Down), Some(ExpandableModeChange::Expand));
        assert_eq!(m.selected_index(), 9);
        assert_eq!(m.collapse(), Some(ExpandableModeChange::Collapse));
        assert_eq!(m.selected_index(), 0, "folds onto its own row: 9 % 9");
        assert_eq!(m.navigate(PageDown), Some(ExpandableModeChange::Expand));
        assert_eq!(m.selected_index(), 0, "PageDown unfolds in place");

        // trace: 13 = column 1 row 4 → ← to 4, ← folds keeping row 4.
        m.select(13);
        assert_eq!(m.navigate(Left), None);
        assert_eq!(m.selected_index(), 4);
        assert_eq!(m.navigate(Left), Some(ExpandableModeChange::Collapse));
        assert_eq!(m.selected_index(), 4);

        // trace: 4 → 13; 13 → min(22, 19) = 19 on the short last column;
        // no column after 18-19.
        assert_eq!(m.navigate(Right), Some(ExpandableModeChange::Expand));
        assert_eq!(m.selected_index(), 13);
        assert_eq!(m.navigate(Right), None);
        assert_eq!(m.selected_index(), 19, "clamped to the short column");
        assert_eq!(m.navigate(Right), None);
        assert_eq!(m.selected_index(), 19);
        assert_eq!(m.navigate(Down), None, "the list's last candidate");
        assert_eq!(m.navigate(NextCandidate), None);

        // `↑` walks up and on into the previous column; on 0 it folds.
        m.select(9);
        assert_eq!(m.navigate(Up), None);
        assert_eq!(m.selected_index(), 8);
        m.select(0);
        assert_eq!(m.navigate(PreviousCandidate), None);
        assert_eq!(m.navigate(Up), Some(ExpandableModeChange::Collapse));

        let mut row = model(&[5.0; 40]);
        row.expand();
        row.select(13);
        assert_eq!(row.navigate(Left), None, "a row's ← still walks");
        assert_eq!(row.selected_index(), 12);

        let mut no_overflow = column_model(&metrics, 5.0, 3);
        assert_eq!(no_overflow.navigate(Right), None);
        assert_eq!(no_overflow.selected_index(), 0);
        assert_eq!(no_overflow.navigate(PageDown), None);
        assert_eq!(model(&[5.0; 3]).axis().unfold_direction(), Down);
    }

    #[test]
    fn expanded_column_pages_columns_at_the_collapsed_height() {
        let metrics = inline_metrics();
        // trace: 9 rows × 30 + 8 hairlines = 278.
        let short = column_model(&metrics, 5.0, 20);
        let w = short.column_width();
        assert_eq!(
            w,
            short.collapsed_content_width(),
            "column 0 keeps its width"
        );
        assert_eq!(short.visible_rows(), 9);
        let geometry = short.expanded_geometry();
        assert!(!geometry.needs_scrolling);
        assert_eq!(geometry.window_height, 278.0);
        assert_eq!(geometry.container_height, 278.0);
        assert_eq!(geometry.window_width, 3.0 * w, "one page: no chevron");
        assert_eq!(short.expanded_chevron_rect(), None);
        assert!(!short.has_next_page());
        assert_eq!(
            short.grid().rows[0]
                .iter()
                .map(|c| c.candidate_index)
                .collect::<Vec<_>>(),
            [0, 9, 18]
        );
        assert_eq!(short.grid().rows.len(), 9);
        assert_eq!(short.grid().rows[2].len(), 2, "18-19 end at row 1");

        // trace: 12 → 2 columns; the page is no wider than the list.
        let two = column_model(&metrics, 5.0, 12);
        assert_eq!(two.expanded_geometry().window_width, 2.0 * w);

        // trace: 60 → 7 columns, 3 a page → pages of 27 at 0 / 27 / 54.
        let mut long = column_model(&metrics, 5.0, 60);
        assert_eq!(long.expanded_geometry().window_width, 3.0 * w + 20.0);
        assert_eq!(
            long.expanded_chevron_rect(),
            Some(Rect::new(3.0 * w, 0.0, 20.0, 278.0))
        );
        long.expand();
        assert!(long.has_next_page());
        assert_eq!(long.navigate(PageDown), None);
        assert_eq!(long.selected_index(), 27);
        assert_eq!(
            long.grid().rows[0]
                .iter()
                .map(|c| c.candidate_index)
                .collect::<Vec<_>>(),
            [27, 36, 45]
        );
        assert_eq!(long.navigate(PageDown), None);
        assert_eq!(long.selected_index(), 54);
        assert!(!long.has_next_page());
        assert!(
            long.expanded_chevron_rect().is_some(),
            "kept on the last page"
        );
        assert_eq!(long.expanded_geometry().window_width, 3.0 * w + 20.0);
        assert_eq!(long.grid().rows.len(), 6, "54-59: one short column");
        assert_eq!(long.navigate(PageDown), None);
        assert_eq!(long.selected_index(), 54);
        // trace: 26 = page 0 column 2 → `→` 35 opens page 1.
        long.select(26);
        assert_eq!(long.grid().rows[0][0].candidate_index, 0);
        assert_eq!(long.navigate(Right), None);
        assert_eq!(long.selected_index(), 35);
        assert_eq!(long.grid().rows[0][0].candidate_index, 27);
        assert_eq!(long.navigate(PageUp), None);
        assert_eq!(long.selected_index(), 8);
        assert_eq!(long.navigate(PageUp), Some(ExpandableModeChange::Collapse));
        assert_eq!(long.selected_index(), 8);

        // trace: 200 → 188 + 27 = 215 clamps to 199.
        let mut most = column_model(&metrics, 5.0, 200);
        most.expand();
        most.select(188);
        assert_eq!(most.navigate(PageDown), None);
        assert_eq!(most.selected_index(), 199);
        // The wheel has nothing to scroll.
        most.on_viewport_scrolled(500.0);
        assert_eq!(most.scroll_y(), 0.0);

        let row = model(&[5.0; 200]);
        assert_eq!(row.visible_rows(), ExpandableListModel::MAX_VISIBLE_ROWS);
    }

    #[test]
    fn paged_column_transition_keeps_the_chevron_and_folds_the_band_where_it_was() {
        let metrics = inline_metrics();
        // trace: 60 → 7 columns, 3 a page; the chevron column at 3w.
        let mut m = column_model(&metrics, 5.0, 60);
        let w = m.column_width();
        m.expand();
        let chevron_column = Rect::new(3.0 * w, 0.0, 20.0, 278.0);
        assert_eq!(
            m.mode_transition().unwrap().chevron,
            Track::new(m.chevron_rect(), chevron_column, 1.0, 1.0),
            "pushed into its column, `›` kept for page 1"
        );
        // trace: 26 = page 0 column 2; PageUp folds onto row 26 % 9 = 8.
        m.select(26);
        assert_eq!(m.navigate(PageUp), Some(ExpandableModeChange::Collapse));
        assert_eq!(m.selected_index(), 8);
        let fold = m.mode_transition().unwrap();
        assert_eq!(fold.band, Some(Rect::new(2.0 * w, 0.0, w, 278.0)));
        assert_eq!(fold.band_alpha(0.0), 1.0);
        assert_eq!(fold.band_alpha(1.0), 0.0);
        assert_eq!(
            fold.chevron,
            Track::new(chevron_column, m.chevron_rect(), 1.0, 1.0)
        );
        assert_eq!(m.navigate(Up), None);
        assert_eq!(m.mode_transition(), None, "a walk changes no mode");
    }

    #[test]
    fn a_narrow_screen_pages_one_column_at_a_time() {
        let metrics = inline_metrics();
        let w = column_model(&metrics, 5.0, 20).column_width();
        let count = 20;
        let primary = vec![5.0; count];
        let annotation = vec![None; count];
        let widths: Vec<f32> = primary
            .iter()
            .map(|&p| metrics.cell_width(p, None))
            .collect();
        let narrow = |maximum_window_width: f32| {
            ExpandableListModel::new(
                &widths,
                ExpandableGeometryInput {
                    collapsed: CollapsedLayout::Column {
                        metrics: &metrics,
                        primary_widths: &primary,
                        annotation_widths: &annotation,
                    },
                    base_width: metrics.base_width(),
                    maximum_window_width,
                    ..geometry()
                },
            )
        };
        // trace: 3 columns fit 3w + 10 once no chevron is needed — one page.
        let snug = narrow(3.0 * w + 10.0);
        assert!(!snug.is_paged());
        assert_eq!(snug.expanded_geometry().window_width, 3.0 * w);
        // trace: (1.5w + 20 − 20) / w floors to 1 column a page.
        let mut m = narrow(1.5 * w + 20.0);
        assert_eq!(m.expanded_geometry().window_width, w + 20.0);
        // Walking off the foot lands on page 1: column 0 is no longer shown.
        m.select(8);
        assert_eq!(m.navigate(Down), Some(ExpandableModeChange::Expand));
        assert_eq!(m.selected_index(), 9);
        assert_eq!(m.grid_cell_rect(0), None);
        // Nothing shared to glide: the column fades out in place as page 1
        // fades in.
        let plan = m.mode_transition().unwrap();
        let (collapsed, expanded): (Vec<&CellTrack>, Vec<&CellTrack>) = plan
            .cells
            .iter()
            .partition(|cell| cell.layer == CellLayer::Collapsed);
        assert_eq!(collapsed.len(), 9);
        assert!(collapsed
            .iter()
            .all(|cell| cell.track.from == cell.track.to && cell.track.to_alpha == 0.0));
        assert_eq!(
            expanded
                .iter()
                .map(|cell| cell.candidate_index)
                .collect::<Vec<_>>(),
            (9..18).collect::<Vec<_>>()
        );
        assert!(expanded
            .iter()
            .all(|cell| cell.track.from == cell.track.to && cell.track.from_alpha == 0.0));
        assert_eq!(m.navigate(Left), None);
        assert_eq!(m.selected_index(), 0);
        assert_eq!(m.grid().rows[0][0].candidate_index, 0);
    }

    #[test]
    fn expanded_column_slots_address_the_keyed_column() {
        let metrics = inline_metrics();
        let mut m = column_model(&metrics, 5.0, 20);
        let w = m.column_width();
        m.expand();
        m.select(13);
        assert_eq!(
            (0..9)
                .map(|slot| m.candidate_index_for_slot(slot))
                .collect::<Vec<_>>(),
            (9..18).map(Some).collect::<Vec<_>>()
        );
        assert_eq!(m.candidate_index_for_slot(9), None, "never into column 2");
        assert_eq!(m.keyed_band_rect(), Some(Rect::new(w, 0.0, w, 278.0)));
        // trace: 19 sits in the short column 18-19: two rows, 30 + 1 + 30.
        m.select(19);
        assert_eq!(m.candidate_index_for_slot(0), Some(18));
        assert_eq!(m.candidate_index_for_slot(1), Some(19));
        assert_eq!(m.candidate_index_for_slot(2), None);
        assert_eq!(m.keyed_band_rect(), Some(Rect::new(2.0 * w, 0.0, w, 61.0)));
        // A leading literal shifts the keys by one: key 9 asks slot 9, which
        // must not reach candidate 9 in the next column.
        m.select(4);
        let key = |slot| {
            CandidateIndexLabel::candidate_index_for_key_slot(slot, true, |position| {
                m.candidate_index_for_slot(position)
            })
        };
        assert_eq!(key(0), Some(1));
        assert_eq!(key(7), Some(8));
        assert_eq!(key(8), None);
    }

    #[test]
    fn collapsed_column_hit_test_and_cross_fade() {
        let metrics = inline_metrics();
        let mut m = column_model(&metrics, 5.0, 20);
        let width = m.collapsed_content_width();
        assert_eq!(m.hit_test(Point { x: 5.0, y: 65.0 }), Some(2));
        assert_eq!(m.hit_test(Point { x: 5.0, y: 61.5 }), None, "a separator");
        assert_eq!(m.hit_test(Point { x: 5.0, y: 280.0 }), None, "below row 9");
        assert_eq!(
            m.hit_test(Point {
                x: width + 1.0,
                y: 5.0
            }),
            None,
            "the chevron"
        );
        assert_eq!(m.chevron_rect(), Rect::new(width, 0.0, 20.0, 278.0));

        m.expand();
        let plan = m.mode_transition().unwrap();
        let row1 = Rect::new(0.0, 31.0, width, 30.0);
        // Each cell cross-fades between its two drawings; column 0 holds still.
        assert_eq!(
            plan.cells[1],
            CellTrack {
                candidate_index: 1,
                layer: CellLayer::Collapsed,
                track: Track::new(row1, row1, 1.0, 0.0),
            }
        );
        let expanded: Vec<_> = plan.cells[9..].iter().collect();
        // trace: page rows [0, 9, 18] / [1, 10, 19] / [2, 11] … in display order.
        assert_eq!(
            expanded
                .iter()
                .map(|cell| cell.candidate_index)
                .collect::<Vec<_>>(),
            [0, 9, 18, 1, 10, 19, 2, 11, 3, 12, 4, 13, 5, 14, 6, 15, 7, 16, 8, 17]
        );
        assert!(expanded
            .iter()
            .all(|cell| cell.layer == CellLayer::Expanded));
        assert_eq!(expanded[3].track, Track::new(row1, row1, 0.0, 1.0));
        // trace: 9 has no collapsed frame → fades in at column 1 row 0.
        let at_9 = Rect::new(width, 0.0, width, 30.0);
        assert_eq!(expanded[1].track, Track::new(at_9, at_9, 0.0, 1.0));
        // trace: one page, no chevron column → pushed to max(3w − 20, w), fading.
        assert_eq!(
            plan.chevron.to,
            Rect::new(3.0 * width - 20.0, 0.0, 20.0, 278.0)
        );
        assert_eq!(plan.chevron.to_alpha, 0.0);

        // trace: column 1 row 1 → 10; column 2 row 2 → 20, past the list.
        assert_eq!(
            m.hit_test(Point {
                x: width + 1.0,
                y: 36.0
            }),
            Some(10)
        );
        assert_eq!(
            m.hit_test(Point {
                x: 2.0 * width + 1.0,
                y: 67.0
            }),
            None
        );
    }
}
