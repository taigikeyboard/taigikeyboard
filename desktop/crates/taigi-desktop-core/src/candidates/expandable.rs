//! The expandable window's two modes — a packed row (or a column) that
//! unfolds into a grid — and its navigation; macOS keeps a Swift twin in
//! `ExpandableCandidatePanel.swift`. The unfold's timing and drawing are
//! the renderer's, its
//! geometry (which cell moves where) is computed here.

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

/// One candidate's frame in both modes, for the unfold animation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellMove {
    pub candidate_index: usize,
    pub collapsed: Rect,
    pub expanded: Rect,
}

/// The unfold's geometry: cells the collapsed row shows move to their grid
/// frames; cells only the grid shows appear in place.
#[derive(Clone, Debug, PartialEq)]
pub struct UnfoldPlan {
    pub moves: Vec<CellMove>,
    /// Grid cells with no collapsed counterpart (their frames are
    /// `grid_cell_rect`), in display order.
    pub appearing: Vec<usize>,
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
    grid: ExpandedGridLayout,
    column_width: f32,
    mode: ExpandableDisplayMode,
    selected_index: usize,
    /// The expanded viewport's scroll offset in points.
    scroll_y: f32,
}

impl ExpandableListModel {
    pub const MAX_VISIBLE_ROWS: usize = 5;
    pub const SEPARATOR_HEIGHT: f32 = 1.0;
    /// Two thirds of a page: `PAGE_SIZE - PAGE_SIZE / 3`.
    pub const EXPANDED_COLUMN_COUNT: usize =
        HorizontalPageLayout::PAGE_SIZE - HorizontalPageLayout::PAGE_SIZE / 3;
    /// The unfold's duration, for the renderer.
    pub const ANIMATION_DURATION_SECONDS: f32 = 0.183;

    /// Lays out `measured_widths` (one per candidate; capped at
    /// [`MAX_DISPLAY_CANDIDATES`] here) under `geometry`.
    pub fn new(measured_widths: &[f32], geometry: ExpandableGeometryInput<'_>) -> Self {
        let measured_widths = &measured_widths[..measured_widths.len().min(MAX_DISPLAY_CANDIDATES)];
        let (axis, collapsed) = match geometry.collapsed {
            CollapsedLayout::Row => (
                ExpandableAxis::Row,
                Self::collapsed_row(measured_widths, &geometry),
            ),
            CollapsedLayout::Column {
                metrics,
                primary_widths,
                annotation_widths,
            } => (
                ExpandableAxis::Column,
                Self::collapsed_column(
                    measured_widths.len(),
                    metrics,
                    primary_widths,
                    annotation_widths,
                    &geometry,
                ),
            ),
        };
        let column_width = Self::resolved_column_width(measured_widths, &geometry);
        let grid =
            ExpandedGridLayout::compute(measured_widths, column_width, Self::EXPANDED_COLUMN_COUNT);
        Self {
            axis,
            count: measured_widths.len(),
            item_height: geometry.item_height,
            chrome_width: geometry.chrome_width,
            scroller_width: geometry.scroller_width,
            collapsed_cells: collapsed.cells,
            collapsed_content_width: collapsed.content_width,
            collapsed_content_height: collapsed.content_height,
            collapsed_primary_column_width: collapsed.primary_column_width,
            grid,
            column_width,
            mode: ExpandableDisplayMode::Collapsed,
            selected_index: 0,
            scroll_y: 0.0,
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
            .min(HorizontalPageLayout::PAGE_SIZE)
            .min(primary_widths.len())
            .min(annotation_widths.len());
        let (widest_primary, widest_annotation) =
            widest_columns(&primary_widths[..rows], &annotation_widths[..rows]);
        let chrome = if count > rows {
            geometry.chrome_width
        } else {
            0.0
        };
        let width = metrics
            .cell_width(widest_primary, widest_annotation)
            .max(geometry.base_width)
            .min(
                geometry
                    .base_width
                    .max(geometry.maximum_window_width - chrome),
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
            primary_column_width: Some(
                widest_primary
                    .min(metrics.maximum_primary_column_width(width, metrics.horizontal_padding())),
            ),
        }
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

    pub fn grid_width(&self) -> f32 {
        self.column_width * Self::EXPANDED_COLUMN_COUNT as f32
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

    pub fn expanded_geometry(&self) -> ExpandedGeometry {
        let content_height = Self::rows_height(self.grid.rows.len(), self.item_height);
        let max_visible_height =
            Self::rows_height(Self::MAX_VISIBLE_ROWS, self.item_height) + self.item_height / 2.0;
        let needs_scrolling = content_height > max_visible_height;
        ExpandedGeometry {
            window_width: self.grid_width()
                + if needs_scrolling {
                    self.scroller_width
                } else {
                    0.0
                },
            window_height: if needs_scrolling {
                max_visible_height
            } else {
                content_height
            },
            container_height: if needs_scrolling {
                content_height + self.item_height / 2.0
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

    /// The selected grid row's full-width frame (content coordinates) — what
    /// the row highlight paints behind the numbered cells.
    pub fn selected_row_rect(&self) -> Option<Rect> {
        let row = self.selected_row()?;
        Some(Rect::new(
            0.0,
            row as f32 * self.row_height(),
            self.grid_width(),
            self.item_height,
        ))
    }

    /// The unfold's geometry from the current collapsed row.
    pub fn unfold_plan(&self) -> UnfoldPlan {
        let moves: Vec<CellMove> = self
            .collapsed_cells
            .iter()
            .filter_map(|cell| {
                Some(CellMove {
                    candidate_index: cell.candidate_index,
                    collapsed: cell.rect,
                    expanded: self.grid_cell_rect(cell.candidate_index)?,
                })
            })
            .collect();
        let appearing = (self.collapsed_cells.len()..self.count).collect();
        UnfoldPlan { moves, appearing }
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
    pub fn candidate_index_for_slot(&self, slot: usize) -> Option<usize> {
        match self.mode {
            ExpandableDisplayMode::Collapsed => self
                .collapsed_cells
                .get(slot)
                .map(|cell| cell.candidate_index),
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

    fn navigate_expanded(
        &mut self,
        direction: CandidateNavigation,
    ) -> Option<ExpandableModeChange> {
        use CandidateNavigation::*;
        match direction {
            Right | NextCandidate => {
                if self.selected_index + 1 < self.count {
                    self.select(self.selected_index + 1)
                } else {
                    None
                }
            }
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

    /// Pages the expanded viewport a whole `MAX_VISIBLE_ROWS`, keeping the
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
        let step = Self::MAX_VISIBLE_ROWS as i32 * if forward { 1 } else { -1 };
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
        if index >= self.count {
            return None;
        }
        self.selected_index = index;
        if self.mode == ExpandableDisplayMode::Collapsed && index >= self.collapsed_cells.len() {
            return self.expand();
        }
        if self.mode == ExpandableDisplayMode::Expanded {
            self.ensure_selected_row_visible();
        }
        None
    }

    pub fn expand(&mut self) -> Option<ExpandableModeChange> {
        if self.mode == ExpandableDisplayMode::Expanded {
            return None;
        }
        self.mode = ExpandableDisplayMode::Expanded;
        self.scroll_y = 0.0;
        self.ensure_selected_row_visible();
        Some(ExpandableModeChange::Expand)
    }

    pub fn collapse(&mut self) -> Option<ExpandableModeChange> {
        if self.mode == ExpandableDisplayMode::Collapsed {
            return None;
        }
        self.mode = ExpandableDisplayMode::Collapsed;
        self.scroll_y = 0.0;
        // A selection beyond the collapsed row clamps to its last cell.
        if self.selected_index >= self.collapsed_cells.len() {
            self.selected_index = self.collapsed_cells.len().saturating_sub(1);
        }
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
            let target = (row_bottom + self.item_height / 2.0 - viewport).min(self.max_scroll_y());
            self.scroll_y = target.max(0.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidates::metrics::test_support;
    use crate::candidates::CandidateCellArrangement;
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
    fn unfold_plan_maps_row_cells_to_grid_frames() {
        // 20 × 5: collapsed cells are 10 wide (slot floor), grid columns 15.
        let m = model(&[5.0; 20]);
        let plan = m.unfold_plan();
        assert_eq!(plan.moves.len(), 9);
        assert_eq!(plan.moves[1].collapsed, Rect::new(10.0, 0.0, 10.0, 30.0));
        assert_eq!(plan.moves[1].expanded, Rect::new(15.0, 0.0, 15.0, 30.0));
        assert_eq!(
            plan.moves[6].expanded,
            Rect::new(0.0, 31.0, 15.0, 30.0),
            "wraps to row 1"
        );
        assert_eq!(plan.appearing, (9..20).collect::<Vec<_>>());
        assert_eq!(m.chevron_rect(), Rect::new(90.0, 0.0, 20.0, 30.0));
        assert_eq!(m.grid_cell_rect(20), None);
        let mut m = m;
        m.select(7);
        m.expand();
        assert_eq!(
            m.selected_row_rect(),
            Some(Rect::new(0.0, 31.0, 90.0, 30.0))
        );
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
        let mut m = column_model(&metrics, 5.0, 20);
        assert_eq!(m.axis().unfold_direction(), Right);
        assert_eq!(m.navigate(Left), None, "nothing left of a column");
        assert_eq!(m.navigate(Down), None);
        assert_eq!(m.selected_index(), 1);
        assert_eq!(m.navigate(Up), None);
        assert_eq!(m.selected_index(), 0);
        assert_eq!(m.navigate(Right), Some(ExpandableModeChange::Expand));
        assert_eq!(
            m.navigate(Up),
            Some(ExpandableModeChange::Collapse),
            "expanded keys are the row's: up on grid row 0 folds"
        );

        // Walking off the column's foot expands AND advances.
        m.select(8);
        assert_eq!(m.navigate(Down), Some(ExpandableModeChange::Expand));
        assert_eq!(m.selected_index(), 9);
        assert_eq!(m.collapse(), Some(ExpandableModeChange::Collapse));
        assert_eq!(m.selected_index(), 8, "clamped to the column's foot");
        assert_eq!(m.navigate(PageDown), Some(ExpandableModeChange::Expand));

        let mut no_overflow = column_model(&metrics, 5.0, 3);
        assert_eq!(no_overflow.navigate(Right), None);
        assert_eq!(no_overflow.navigate(PageDown), None);
        assert_eq!(model(&[5.0; 3]).axis().unfold_direction(), Down);
    }

    #[test]
    fn collapsed_column_hit_test_and_unfold_plan() {
        let metrics = inline_metrics();
        let m = column_model(&metrics, 5.0, 20);
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
        let plan = m.unfold_plan();
        assert_eq!(plan.moves.len(), 9);
        assert_eq!(plan.moves[1].collapsed, Rect::new(0.0, 31.0, width, 30.0));
        assert_eq!(Some(plan.moves[1].expanded), m.grid_cell_rect(1));
        assert_eq!(plan.appearing, (9..20).collect::<Vec<_>>());
    }
}
