//! The vertical window's width model, selection, scrolling and slot
//! numbering (macOS keeps a Swift twin in `VerticalCandidatePanel.swift`:
//! `rebuildRows`, `widenForRevealedRows`, scrolling). The scroll viewport is
//! modelled in points so the renderer only draws and reports scrolls back.
//!
//! Width follows what the viewport has revealed, not the whole list: the
//! engine sorts low-frequency prefix extensions to the tail, and a long
//! one at row 30 must not widen the nine rows the user actually sees. As
//! the viewport reaches wider rows the window grows; within one list it
//! never shrinks back, so scrolling up does not make the window jitter.

use super::metrics::CandidateMetrics;
use super::positioning::{Point, Rect};
use super::MAX_DISPLAY_CANDIDATES;
use crate::keys::CandidateNavigation;

/// How the renderer's scroller takes its share of the window — from upstream
/// (MacishType `MacishVerticalPanel.swift:283-300`):
/// a legacy scroller gets its own column outside the rows so rounded corners
/// are not clipped; an overlay scroller floats over a widened trailing inset
/// so text stays clear. PR6 picks per Windows' "always show scrollbars"
/// setting; either way the window is `content_width + allowance` wide.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ScrollerStyle {
    Legacy { width: f32 },
    Overlay { width: f32 },
}

impl ScrollerStyle {
    /// The air between an overlay scroller and the text under it
    /// (macOS: `VerticalCandidatePanel.overlayScrollerGap`).
    pub const OVERLAY_GAP: f32 = 2.0;
}

/// The scroller's share, resolved from one read of the style so the
/// allowance and the geometry that spends it cannot disagree.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ScrollerLayout {
    allowance: f32,
    rows_span_allowance: bool,
}

impl ScrollerLayout {
    fn resolve(has_overflow: bool, style: ScrollerStyle, horizontal_padding: f32) -> Self {
        if !has_overflow {
            return Self {
                allowance: 0.0,
                rows_span_allowance: false,
            };
        }
        match style {
            ScrollerStyle::Legacy { width } => Self {
                allowance: width,
                rows_span_allowance: false,
            },
            ScrollerStyle::Overlay { width } => Self {
                allowance: (width + ScrollerStyle::OVERLAY_GAP - horizontal_padding).max(0.0),
                rows_span_allowance: true,
            },
        }
    }
}

/// What the vertical window needs to lay a list out.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VerticalLayoutInput<'a> {
    pub metrics: &'a CandidateMetrics,
    /// `CandidateMetrics::measure_primary_width` per candidate, display order.
    pub primary_widths: &'a [f32],
    /// `CandidateMetrics::measure_annotation` per candidate, same order and
    /// length. The two columns are taken apart rather than as a summed cell
    /// width: the window is sized to the widest column plus the widest
    /// annotation, which need not be one row (`resolve_width`).
    pub annotation_widths: &'a [Option<f32>],
    pub maximum_window_width: f32,
    pub scroller: ScrollerStyle,
}

/// The vertical window's resolved geometry (`rebuildRows` `:205-232`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VerticalGeometry {
    pub window_width: f32,
    pub window_height: f32,
    /// Every row's width — the window's under an overlay scroller, the
    /// content's beside a legacy one.
    pub item_width: f32,
    /// The trailing inset each row keeps clear of the scroller.
    pub item_trailing: f32,
    /// One width for the candidate column all the way down, so every row's
    /// annotation starts at the same x.
    pub primary_column_width: f32,
    /// Every row laid out, plus the peek — the container's natural height.
    pub natural_content_height: f32,
}

/// The widest candidate column and the widest annotation over `rows` —
/// what an aligned column is sized from (they need not be one row's). Also
/// the collapsed vertical expandable column's rule (`expandable.rs`).
pub(super) fn widest_columns(
    primary_widths: &[f32],
    annotation_widths: &[Option<f32>],
) -> (f32, Option<f32>) {
    let widest_primary = primary_widths.iter().copied().fold(0.0, f32::max);
    let widest_annotation = annotation_widths.iter().flatten().copied().reduce(f32::max);
    (widest_primary, widest_annotation)
}

#[derive(Clone, Debug, PartialEq)]
pub struct VerticalListModel {
    count: usize,
    item_height: f32,
    metrics: CandidateMetrics,
    /// The measured widths the geometry is re-derived from as rows are
    /// revealed (`VerticalLayoutInput::primary_widths` / `annotation_widths`).
    primary_widths: Vec<f32>,
    annotation_widths: Vec<Option<f32>>,
    maximum_window_width: f32,
    scroller: ScrollerStyle,
    /// How many rows from the top the viewport has shown so far — the
    /// rows the width is measured over. Only ever grows within a list.
    revealed_rows: usize,
    geometry: VerticalGeometry,
    selected_index: usize,
    /// The row the slot keys start numbering from — the row MOSTLY on
    /// screen at the viewport's top.
    anchor_row: usize,
    /// Scroll offset in points; `0` = row 0 at the top.
    scroll_y: f32,
    /// The container's current height: grows past the natural height when
    /// a page jump needs the anchor row to reach the top, and shrinks back
    /// as the user scrolls away.
    container_height: f32,
}

impl VerticalListModel {
    pub const VISIBLE_ROWS: usize = 9;
    pub const SEPARATOR_HEIGHT: f32 = 1.0;

    /// Lays the list out (capped at [`MAX_DISPLAY_CANDIDATES`]) with the
    /// viewport at the top, sized to the rows that opening viewport shows.
    pub fn new(input: VerticalLayoutInput<'_>) -> Self {
        let metrics = input.metrics;
        assert_eq!(
            input.primary_widths.len(),
            input.annotation_widths.len(),
            "one annotation measurement per candidate"
        );
        let count = input.primary_widths.len().min(MAX_DISPLAY_CANDIDATES);
        let item_height = metrics.item_height();
        let has_overflow = count > Self::VISIBLE_ROWS;
        let visible = count.min(Self::VISIBLE_ROWS);
        let bottom_peek = if has_overflow { item_height / 2.0 } else { 0.0 };
        let window_height = Self::rows_height(visible, item_height) + bottom_peek;
        let natural_content_height = Self::rows_height(count, item_height) + bottom_peek;
        let mut model = Self {
            count,
            item_height,
            metrics: metrics.clone(),
            primary_widths: input.primary_widths[..count].to_vec(),
            annotation_widths: input.annotation_widths[..count].to_vec(),
            maximum_window_width: input.maximum_window_width,
            scroller: input.scroller,
            revealed_rows: 0,
            geometry: VerticalGeometry {
                window_width: 0.0,
                window_height,
                item_width: 0.0,
                item_trailing: 0.0,
                primary_column_width: 0.0,
                natural_content_height,
            },
            selected_index: 0,
            anchor_row: 0,
            scroll_y: 0.0,
            container_height: natural_content_height,
        };
        model.revealed_rows = model.viewport_row_count();
        model.resolve_width();
        model
    }

    /// How many rows from the top the viewport currently intersects — the
    /// peeking half row included, since its text is painted.
    fn viewport_row_count(&self) -> usize {
        let bottom = self.scroll_y + self.viewport_height();
        ((bottom / self.row_height()).ceil() as usize).min(self.count)
    }

    /// The rows the viewport now shows join the revealed set; the width
    /// fields are re-derived when that set grew.
    fn reveal_viewport_rows(&mut self) {
        let revealed = self.viewport_row_count();
        if revealed > self.revealed_rows {
            self.revealed_rows = revealed;
            self.resolve_width();
        }
    }

    /// Width: the widest revealed candidate column plus the widest revealed
    /// annotation — which can come from different rows, since every row's
    /// annotation starts at the column's edge — floored at one slot and
    /// capped at what the screen leaves once the scroller has its share
    /// (`resolveWidth`). Sizing to the widest single row left kuānn beside 汗
    /// truncated once 舅仔 / kǔ-á had widened the column.
    fn resolve_width(&mut self) {
        let metrics = &self.metrics;
        let scroller = ScrollerLayout::resolve(
            self.has_overflow(),
            self.scroller,
            metrics.horizontal_padding(),
        );
        let (widest_primary, widest_annotation) = widest_columns(
            &self.primary_widths[..self.revealed_rows],
            &self.annotation_widths[..self.revealed_rows],
        );
        let widest = metrics.cell_width(widest_primary, widest_annotation);
        let content_width = widest.max(metrics.base_width()).min(
            metrics
                .base_width()
                .max(self.maximum_window_width - scroller.allowance),
        );
        let window_width = content_width + scroller.allowance;
        let (item_width, item_trailing) = if scroller.rows_span_allowance {
            (
                window_width,
                metrics.horizontal_padding() + scroller.allowance,
            )
        } else {
            (content_width, metrics.horizontal_padding())
        };
        let primary_column_width =
            widest_primary.min(metrics.maximum_primary_column_width(item_width, item_trailing));
        self.geometry.window_width = window_width;
        self.geometry.item_width = item_width;
        self.geometry.item_trailing = item_trailing;
        self.geometry.primary_column_width = primary_column_width;
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn geometry(&self) -> &VerticalGeometry {
        &self.geometry
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn anchor_row(&self) -> usize {
        self.anchor_row
    }

    pub fn scroll_y(&self) -> f32 {
        self.scroll_y
    }

    /// The scrollable content's current height (≥ natural).
    pub fn container_height(&self) -> f32 {
        self.container_height
    }

    pub fn has_overflow(&self) -> bool {
        self.count > Self::VISIBLE_ROWS
    }

    pub fn row_height(&self) -> f32 {
        self.item_height + Self::SEPARATOR_HEIGHT
    }

    /// The half-row "scroll me" peek shown when the list overflows.
    pub fn bottom_peek(&self) -> f32 {
        if self.has_overflow() {
            self.item_height / 2.0
        } else {
            0.0
        }
    }

    /// Rows on screen: `min(count, 9)` full rows plus the peek.
    pub fn viewport_height(&self) -> f32 {
        self.geometry.window_height
    }

    fn rows_height(rows: usize, item_height: f32) -> f32 {
        if rows == 0 {
            0.0
        } else {
            rows as f32 * item_height + (rows - 1) as f32 * Self::SEPARATOR_HEIGHT
        }
    }

    pub fn y_for_row(&self, row: usize) -> f32 {
        row as f32 * self.row_height()
    }

    /// Row `row`'s frame in CONTENT coordinates (before the scroll offset).
    pub fn row_rect(&self, row: usize) -> Rect {
        Rect::new(
            0.0,
            self.y_for_row(row),
            self.geometry.item_width,
            self.item_height,
        )
    }

    /// The row under `point`, given in WINDOW coordinates (the scroll offset
    /// applied); separators and the peek count as no row.
    pub fn hit_test(&self, point: Point) -> Option<usize> {
        if point.x < 0.0 || point.x >= self.geometry.item_width || point.y < 0.0 {
            return None;
        }
        let content_y = point.y + self.scroll_y;
        let row = (content_y / self.row_height()).floor() as usize;
        (row < self.count && content_y - self.y_for_row(row) < self.item_height).then_some(row)
    }

    /// The slot keys address the nine rows starting at the viewport's anchor.
    pub fn candidate_index_for_slot(&self, slot: usize) -> Option<usize> {
        if slot >= Self::VISIBLE_ROWS {
            return None;
        }
        let index = self.anchor_row + slot;
        (index < self.count).then_some(index)
    }

    /// The slot `row` shows, if it is one of the nine the anchor reaches.
    #[cfg(test)]
    pub(crate) fn slot_for_row(&self, row: usize) -> Option<usize> {
        let slot = row.checked_sub(self.anchor_row)?;
        (slot < Self::VISIBLE_ROWS).then_some(slot)
    }

    /// Up/down walk; a column has no candidate to the left or right, so the
    /// horizontal keys page — backward and forward respectively.
    pub fn navigate(&mut self, direction: CandidateNavigation) {
        if self.count == 0 {
            return;
        }
        match direction {
            CandidateNavigation::Up | CandidateNavigation::PreviousCandidate => {
                self.select(self.selected_index.saturating_sub(1));
            }
            CandidateNavigation::Down | CandidateNavigation::NextCandidate => {
                self.select((self.selected_index + 1).min(self.count - 1));
            }
            CandidateNavigation::Right | CandidateNavigation::PageDown => self.jump_page(1),
            CandidateNavigation::Left | CandidateNavigation::PageUp => self.jump_page(-1),
        }
    }

    /// Moves the selection a whole viewport, keeping it at the same visual
    /// row — the highlight stays put while the list moves underneath. Clamps
    /// into the ends.
    fn jump_page(&mut self, pages: i32) {
        let visual_offset = self.selected_index.saturating_sub(self.anchor_row);
        let target_anchor = self.anchor_row as i32 + pages * Self::VISIBLE_ROWS as i32;
        if target_anchor >= 0 && (target_anchor as usize) < self.count {
            let target_anchor = target_anchor as usize;
            let target = (target_anchor + visual_offset).min(self.count - 1);
            self.scroll_row_to_top(target_anchor);
            self.select(target);
        } else if pages < 0 && self.anchor_row > 0 {
            // Less than a whole page above: anchor to the top but keep the
            // visual row — jumping the highlight to 0 would move it on screen.
            self.scroll_row_to_top(0);
            self.select(visual_offset.min(self.count - 1));
        } else {
            self.select(if pages > 0 { self.count - 1 } else { 0 });
        }
    }

    /// A click on `index` selects it (never commits).
    pub fn select(&mut self, index: usize) {
        if index >= self.count {
            return;
        }
        self.selected_index = index;
        self.ensure_selection_visible();
    }

    /// The renderer scrolled the viewport (wheel, scrollbar) to `offset`:
    /// re-derives the anchor row and lets a container a page jump grew
    /// shrink back toward its natural height
    /// (macOS: `VerticalCandidatePanel.scrollViewDidScroll`).
    pub fn on_viewport_scrolled(&mut self, offset: f32) {
        self.scroll_y = offset.max(0.0).min(self.max_scroll_y());
        let needed_height = self.scroll_y + self.viewport_height();
        let target_height = self.geometry.natural_content_height.max(needed_height);
        if target_height < self.container_height {
            self.container_height = target_height;
        }
        self.update_anchor();
    }

    fn max_scroll_y(&self) -> f32 {
        (self.container_height - self.viewport_height()).max(0.0)
    }

    fn scroll_row_to_top(&mut self, row: usize) {
        let target_y = self.y_for_row(row);
        // A jump near the end may need more container than the rows fill,
        // so the anchor row can still reach the top.
        let needed_height = target_y + self.viewport_height();
        if needed_height > self.container_height {
            self.container_height = needed_height;
        }
        self.scroll_y = target_y;
        self.update_anchor();
    }

    fn ensure_selection_visible(&mut self) {
        let row_top = self.y_for_row(self.selected_index);
        let row_bottom = row_top + self.item_height;
        let viewport = self.viewport_height();
        if row_top < self.scroll_y {
            self.scroll_y = row_top;
        } else if row_bottom > self.scroll_y + viewport {
            let max_scroll = self.max_scroll_y();
            // Half a row deeper than strictly needed, so the next row peeks.
            let target = (row_bottom + self.item_height / 2.0 - viewport).min(max_scroll);
            self.scroll_y = target.max(0.0);
        }
        self.update_anchor();
    }

    /// Half-row bias: the row MOSTLY on screen is the one the slot names.
    /// Every scroll lands here, so this is also where the rows the viewport
    /// now shows are revealed to the width model.
    fn update_anchor(&mut self) {
        let row_height = self.row_height();
        self.anchor_row = ((self.scroll_y.max(0.0) + row_height / 2.0) / row_height)
            .floor()
            .max(0.0) as usize;
        self.reveal_viewport_rows();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidates::metrics::test_support::EmMeasurer;
    use crate::candidates::CandidateCellArrangement;
    use crate::settings::{CandidateFontSelection, CandidateSizeChoice};
    use CandidateNavigation::*;

    fn metrics() -> CandidateMetrics {
        CandidateMetrics::resolve(
            CandidateSizeChoice::Large,
            CandidateFontSelection::default(),
            CandidateCellArrangement::Inline,
            &EmMeasurer,
        )
    }

    /// The chrome an annotation-less inline cell adds around its candidate.
    fn chrome(metrics: &CandidateMetrics) -> f32 {
        metrics.base_width() - metrics.primary_column_floor()
    }

    /// A model of annotation-less rows whose cells measure `cell_widths`.
    fn model_with(
        metrics: &CandidateMetrics,
        cell_widths: &[f32],
        scroller: ScrollerStyle,
    ) -> VerticalListModel {
        let primary: Vec<f32> = cell_widths.iter().map(|w| w - chrome(metrics)).collect();
        let annotations = vec![None; cell_widths.len()];
        VerticalListModel::new(VerticalLayoutInput {
            metrics,
            primary_widths: &primary,
            annotation_widths: &annotations,
            maximum_window_width: 640.0,
            scroller,
        })
    }

    fn model(count: usize) -> VerticalListModel {
        let metrics = metrics();
        let widths = vec![metrics.base_width(); count];
        model_with(&metrics, &widths, ScrollerStyle::Legacy { width: 15.0 })
    }

    #[test]
    fn sizes_follow_the_macos_formula() {
        let metrics = metrics();
        let item = metrics.item_height();
        let short = model(5);
        assert_eq!(short.viewport_height(), 5.0 * item + 4.0);
        assert_eq!(short.bottom_peek(), 0.0);
        let long = model(20);
        assert_eq!(long.viewport_height(), 9.0 * item + 8.0 + item / 2.0);
        assert_eq!(
            long.geometry().natural_content_height,
            20.0 * item + 19.0 + item / 2.0
        );
        assert_eq!(
            long.container_height(),
            long.geometry().natural_content_height
        );
        assert_eq!(long.row_height(), item + 1.0);
    }

    #[test]
    fn width_grows_for_a_long_candidate_and_stays_inside_the_budget() {
        // trace: budget 640. A base-width row: no overflow, no scroller allowance;
        // 5,000-wide rows clamp to 640 = rows + the 15 legacy column, or (overlay)
        // rows run 640 under a trailing pad + (15 + 2 − pad).
        let metrics = metrics();
        let base = metrics.base_width();
        let narrow = model_with(&metrics, &[base], ScrollerStyle::Legacy { width: 15.0 });
        let wide = model_with(
            &metrics,
            &[base, 300.0],
            ScrollerStyle::Legacy { width: 15.0 },
        );
        assert!(wide.geometry().window_width > narrow.geometry().window_width);
        assert!(
            wide.geometry().window_width >= 300.0,
            "wide enough to render the whole candidate"
        );
        assert_eq!(
            narrow.geometry().window_width,
            base,
            "no overflow: no scroller allowance"
        );
        assert_eq!(
            narrow.geometry().item_trailing,
            metrics.horizontal_padding()
        );
        let huge = model_with(
            &metrics,
            &[5_000.0; 200],
            ScrollerStyle::Legacy { width: 15.0 },
        );
        assert!(
            huge.geometry().window_width <= 640.0,
            "{}",
            huge.geometry().window_width
        );
        assert_eq!(
            huge.geometry().window_width,
            640.0,
            "budget-bound: content + legacy column"
        );
        assert_eq!(
            huge.geometry().item_width,
            640.0 - 15.0,
            "rows stop short of a legacy column"
        );
        assert_eq!(huge.count(), 200);
        // Overlay: the rows run under the scroller, the trailing inset widens.
        let overlay = model_with(
            &metrics,
            &[5_000.0; 20],
            ScrollerStyle::Overlay { width: 15.0 },
        );
        let allowance = (15.0 + 2.0 - metrics.horizontal_padding()).max(0.0);
        assert_eq!(overlay.geometry().window_width, 640.0);
        assert_eq!(overlay.geometry().item_width, 640.0);
        assert_eq!(
            overlay.geometry().item_trailing,
            metrics.horizontal_padding() + allowance
        );
        // The primary column is the widest primary, clamped to what the cell holds.
        assert_eq!(
            overlay.geometry().primary_column_width,
            metrics.maximum_primary_column_width(640.0, overlay.geometry().item_trailing)
        );
        assert_eq!(
            wide.geometry().primary_column_width,
            300.0 - chrome(&metrics)
        );
    }

    #[test]
    fn width_covers_the_widest_column_plus_the_widest_annotation_across_rows() {
        // trace (EmMeasurer, candidate font = 1 em, annotation font smaller):
        // rows 0..=39 are one glyph wide; row 0 carries the long annotation,
        // row 40 (the tail) is two glyphs beside a short one. Revealing the
        // tail widens the column to two glyphs for every row, so the window
        // must hold two glyphs PLUS the long annotation — no single row does.
        let metrics = metrics();
        let one_glyph = metrics.primary_column_floor();
        let long_annotation = Some(one_glyph * 2.5);
        let short_annotation = Some(one_glyph);
        let mut primary = vec![one_glyph; 41];
        primary[40] = one_glyph * 2.0;
        let mut annotations = vec![short_annotation; 41];
        annotations[0] = long_annotation;
        let mut model = VerticalListModel::new(VerticalLayoutInput {
            metrics: &metrics,
            primary_widths: &primary,
            annotation_widths: &annotations,
            maximum_window_width: 640.0,
            scroller: ScrollerStyle::Legacy { width: 15.0 },
        });
        assert_eq!(
            model.geometry().item_width,
            metrics.cell_width(one_glyph, long_annotation),
            "opening rows: one glyph beside the long annotation"
        );
        model.on_viewport_scrolled(model.max_scroll_y());
        model.on_viewport_scrolled(0.0);
        assert_eq!(model.geometry().primary_column_width, one_glyph * 2.0);
        assert_eq!(
            model.geometry().item_width,
            metrics.cell_width(one_glyph * 2.0, long_annotation),
            "the tail's column beside row 0's annotation"
        );
    }

    #[test]
    fn width_follows_the_rows_the_viewport_has_revealed() {
        // trace: 30 base-width rows, one 300-wide cell at row 20. Opening
        // viewport = rows 0..=9 (nine + the peek) -> base width. Scrolling
        // to row 20 reveals it -> widens; scrolling back never shrinks.
        let metrics = metrics();
        let base = metrics.base_width();
        let mut widths = vec![base; 30];
        widths[20] = 300.0;
        let mut model = model_with(&metrics, &widths, ScrollerStyle::Legacy { width: 15.0 });
        let narrow = model.geometry().window_width;
        assert_eq!(
            narrow,
            base + 15.0,
            "first page: base content + legacy column"
        );
        assert!(model.geometry().primary_column_width < 300.0 - chrome(&metrics));
        model.select(9);
        assert_eq!(
            model.geometry().window_width,
            narrow,
            "the peek row was already revealed"
        );
        model.on_viewport_scrolled(model.row_height() * 10.5);
        assert_eq!(
            model.geometry().window_width,
            narrow,
            "rows 10..=20 not yet fully reached"
        );
        model.on_viewport_scrolled(model.row_height() * 11.0);
        assert_eq!(
            model.geometry().window_width,
            315.0,
            "row 20 intersects the viewport"
        );
        assert_eq!(model.geometry().item_width, 300.0);
        assert_eq!(
            model.geometry().primary_column_width,
            300.0 - chrome(&metrics)
        );
        model.on_viewport_scrolled(0.0);
        assert_eq!(
            model.geometry().window_width,
            315.0,
            "never shrinks within a list"
        );
        let mut paged = model_with(&metrics, &widths, ScrollerStyle::Legacy { width: 15.0 });
        paged.navigate(PageDown);
        paged.navigate(PageDown);
        assert_eq!(
            paged.geometry().window_width,
            315.0,
            "a page jump reveals its page"
        );
    }

    #[test]
    fn walking_scrolls_the_viewport_and_moves_the_anchor() {
        let mut model = model(20);
        for _ in 0..9 {
            model.navigate(Down);
        }
        assert_eq!(model.selected_index(), 9);
        assert!(model.scroll_y() > 0.0, "row 9 is below the first viewport");
        assert!(model.anchor_row() >= 1, "{}", model.anchor_row());
        assert_eq!(model.candidate_index_for_slot(0), Some(model.anchor_row()));
        assert_eq!(model.candidate_index_for_slot(9), None);
        assert_eq!(model.slot_for_row(model.anchor_row() + 2), Some(2));
        assert_eq!(model.slot_for_row(0), None);
        for _ in 0..30 {
            model.navigate(Up);
        }
        assert_eq!(
            (model.selected_index(), model.scroll_y(), model.anchor_row()),
            (0, 0.0, 0)
        );
    }

    #[test]
    fn horizontal_keys_page_and_keep_the_visual_row() {
        let mut model = model(30);
        model.navigate(Down);
        model.navigate(Down);
        assert_eq!(model.selected_index(), 2);
        model.navigate(Right);
        assert_eq!(model.anchor_row(), 9);
        assert_eq!(model.selected_index(), 11, "same visual row, one page down");
        model.navigate(PageDown);
        assert_eq!(model.anchor_row(), 18);
        assert_eq!(model.selected_index(), 20);
        model.navigate(PageDown);
        assert_eq!(
            model.selected_index(),
            29,
            "past the last page lands on the end"
        );
        assert_eq!(
            model.anchor_row(),
            27,
            "and the anchor reached the top: the container grew"
        );
        assert!(model.container_height() > model.geometry().natural_content_height);
        model.navigate(Left);
        model.navigate(Left);
        model.navigate(Left);
        model.navigate(PageUp);
        assert_eq!(model.selected_index(), 0);
        assert_eq!(model.anchor_row(), 0);
    }

    #[test]
    fn partial_page_above_anchors_to_the_top_but_keeps_the_offset() {
        let mut model = model(30);
        model.select(12); // scrolls so row 12 is visible; anchor somewhere in 1..=4
        let offset = model.selected_index() - model.anchor_row();
        assert!(
            model.anchor_row() > 0 && model.anchor_row() < 9,
            "{}",
            model.anchor_row()
        );
        model.navigate(PageUp);
        assert_eq!(model.anchor_row(), 0);
        assert_eq!(model.selected_index(), offset);
        let mut empty = model_with(&metrics(), &[], ScrollerStyle::Legacy { width: 15.0 });
        empty.navigate(Down);
        assert_eq!(empty.selected_index(), 0);
        assert_eq!(empty.geometry().window_height, 0.0);
    }

    #[test]
    fn an_external_scroll_renumbers_the_slots_and_shrinks_a_grown_container() {
        // trace: updateRowNumbering (half-row bias) + scrollViewDidScroll.
        let mut model = model(30);
        let row = model.row_height();
        model.on_viewport_scrolled(row * 3.0 + row * 0.4);
        assert_eq!(model.anchor_row(), 3, "less than half a row into row 3");
        model.on_viewport_scrolled(row * 3.0 + row * 0.6);
        assert_eq!(model.anchor_row(), 4, "mostly on row 4");
        assert_eq!(model.candidate_index_for_slot(0), Some(4));
        assert_eq!(
            model.hit_test(Point { x: 5.0, y: 1.0 }),
            Some(3),
            "the top sliver is still row 3"
        );
        assert_eq!(
            model.hit_test(Point {
                x: 5.0,
                y: row * 0.5
            }),
            Some(4),
            "half a row down is the anchor row"
        );
        assert_eq!(model.hit_test(Point { x: -1.0, y: 1.0 }), None);
        // Paging to the end grows the container; scrolling back shrinks it.
        model.navigate(PageDown);
        model.navigate(PageDown);
        model.navigate(PageDown);
        assert!(model.container_height() > model.geometry().natural_content_height);
        model.on_viewport_scrolled(0.0);
        assert_eq!(
            model.container_height(),
            model.geometry().natural_content_height
        );
        assert_eq!(model.anchor_row(), 0);
        model.on_viewport_scrolled(-50.0);
        assert_eq!(model.scroll_y(), 0.0, "clamped");
        model.on_viewport_scrolled(1.0e9);
        assert_eq!(
            model.scroll_y(),
            model.max_scroll_y(),
            "clamped to the content"
        );
    }
}
