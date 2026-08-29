//! How a horizontal candidate window pages: pure width arithmetic, plus the
//! selection state machine over it. Port of `HorizontalPageLayout.swift`
//! (packing + navigation, MacishType-derived) and the selection half of
//! `HorizontalCandidatePanel.swift`.

// 中文: 橫式候選窗的分頁(寬度算術)與選取狀態機。

use super::positioning::{Point, Rect};
use super::MAX_DISPLAY_CANDIDATES;
use crate::keys::CandidateNavigation;

/// One candidate's place on a page: its absolute index and the width its
/// cell renders at. Indices run in display order across pages, so the page
/// structure is a partition of `0..candidate_count`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PageSlot {
    pub candidate_index: usize,
    pub width: f32,
}

/// The page structure of a horizontal candidate window.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HorizontalPageLayout {
    pub pages: Vec<Vec<PageSlot>>,
    /// The width a page is packed to before it breaks, already clamped to
    /// what the window may render. A short page still reserves the full
    /// budget so the page-turn arrow's edge does not wander.
    pub page_budget: f32,
}

impl HorizontalPageLayout {
    /// How many candidates one page holds — what the nine slot keys address.
    pub const PAGE_SIZE: usize = 9;
    /// A page never packs fewer potential columns than this.
    pub const MINIMUM_PAGE_COLUMNS: usize = 4;

    /// The nominal width of a full row of slots.
    pub fn row_budget(slot_width: f32) -> f32 {
        slot_width * Self::PAGE_SIZE.max(Self::MINIMUM_PAGE_COLUMNS) as f32
    }

    /// Packs for a window `window_budget` wide whose page-turn chrome costs
    /// `chrome_width` and only appears once the list needs more than one
    /// page: packed against the whole budget first, and only if that spills
    /// into a second page, again against what the chrome leaves. A narrower
    /// budget never packs into fewer pages, so the second pass cannot flip
    /// the answer back.
    pub fn pack_with_chrome(
        widths: &[f32],
        slot_width: f32,
        window_budget: f32,
        chrome_width: f32,
    ) -> Self {
        let chromeless = Self::pack(widths, slot_width, window_budget);
        if chromeless.pages.len() <= 1 {
            return chromeless;
        }
        Self::pack(widths, slot_width, window_budget - chrome_width)
    }

    /// Packs `widths` into pages. A cell renders at its measured width,
    /// floored at `slot_width` and capped at `max_cell_width`. A page breaks
    /// before its total passes the budget — unless the page is still empty,
    /// so a single candidate wider than the budget gets a page of its own AT
    /// ITS MEASURED WIDTH — and no page holds more than `PAGE_SIZE`. At most
    /// [`MAX_DISPLAY_CANDIDATES`] are packed.
    ///
    /// NAMED NUMERIC DIVERGENCE: `f32` (Direct2D's own unit) where the Mac
    /// computes in 64-bit `CGFloat`; a cell whose running total lands within
    /// float rounding of the budget may break a page differently. The
    /// epsilon test below pins the strict `>` at the boundary.
    pub fn pack(widths: &[f32], slot_width: f32, max_cell_width: f32) -> Self {
        let widths = &widths[..widths.len().min(MAX_DISPLAY_CANDIDATES)];
        let cell_limit = slot_width.max(max_cell_width);
        let page_budget = Self::row_budget(slot_width).min(cell_limit);
        let mut pages: Vec<Vec<PageSlot>> = Vec::new();
        let mut page: Vec<PageSlot> = Vec::new();
        let mut used_width = 0.0;
        for (index, raw_width) in widths.iter().enumerate() {
            let width = slot_width.max(raw_width.min(cell_limit));
            if page.len() == Self::PAGE_SIZE
                || (used_width + width > page_budget && !page.is_empty())
            {
                pages.push(std::mem::take(&mut page));
                used_width = 0.0;
            }
            page.push(PageSlot {
                candidate_index: index,
                width,
            });
            used_width += width;
        }
        if !page.is_empty() {
            pages.push(page);
        }
        Self { pages, page_budget }
    }

    pub fn candidate_count(&self) -> usize {
        self.pages.iter().map(Vec::len).sum()
    }

    /// The page holding `candidate_index`, by arithmetic over the partition.
    pub fn page_index_containing(&self, candidate_index: usize) -> Option<usize> {
        let mut start = 0;
        for (page_index, page) in self.pages.iter().enumerate() {
            if candidate_index < start + page.len() {
                return Some(page_index);
            }
            start += page.len();
        }
        None
    }

    /// The absolute index the slot key picks on `page_index`, or `None` for
    /// a slot the (usually last) page does not fill.
    pub fn candidate_index_for_slot(&self, slot: usize, page_index: usize) -> Option<usize> {
        self.pages
            .get(page_index)?
            .get(slot)
            .map(|slot| slot.candidate_index)
    }

    /// The width of one page's cells laid end to end.
    pub fn page_width(&self, page_index: usize) -> f32 {
        self.pages
            .get(page_index)
            .map_or(0.0, |page| page.iter().map(|slot| slot.width).sum())
    }

    /// Where `direction` moves the selection, or `None` at an end — the
    /// no-wrap clamp rule. Left/right walk by adjacency across pages; the
    /// paging directions land on the candidate of the target page whose
    /// horizontal extent overlaps the current one's.
    pub fn target(&self, direction: CandidateNavigation, from: usize) -> Option<usize> {
        match direction {
            CandidateNavigation::Right | CandidateNavigation::NextCandidate => {
                (from + 1 < self.candidate_count()).then_some(from + 1)
            }
            CandidateNavigation::Left | CandidateNavigation::PreviousCandidate => {
                from.checked_sub(1)
            }
            CandidateNavigation::Down | CandidateNavigation::PageDown => self.paged_target(from, 1),
            CandidateNavigation::Up | CandidateNavigation::PageUp => self.paged_target(from, -1),
        }
    }

    fn paged_target(&self, from: usize, step: i32) -> Option<usize> {
        let current_page = self.page_index_containing(from)?;
        let target_page = usize::try_from(current_page as i32 + step).ok()?;
        let page = self.pages.get(target_page)?;
        let mut x_start = 0.0;
        let mut x_end = 0.0;
        for slot in &self.pages[current_page] {
            x_end += slot.width;
            if slot.candidate_index == from {
                break;
            }
            x_start = x_end;
        }
        self.overlapping_candidate(target_page, x_start, x_end, step > 0)
            .or_else(|| page.last().map(|slot| slot.candidate_index))
    }

    /// The first candidate on `page_index` whose extent overlaps
    /// `x_start..x_end`. Paging backward prefers the right-hand neighbour
    /// when the overlap starts left of the origin extent.
    fn overlapping_candidate(
        &self,
        page_index: usize,
        x_start: f32,
        x_end: f32,
        forward: bool,
    ) -> Option<usize> {
        let page = &self.pages[page_index];
        let mut x = 0.0;
        for (position, slot) in page.iter().enumerate() {
            let slot_end = x + slot.width;
            if x < x_end && slot_end > x_start {
                if !forward && x < x_start && position + 1 < page.len() && slot_end < x_end {
                    return Some(page[position + 1].candidate_index);
                }
                return Some(slot.candidate_index);
            }
            x = slot_end;
        }
        None
    }
}

/// The horizontal window's selection over a packed layout — the part of
/// `HorizontalCandidatePanel` that decides rather than draws.
#[derive(Clone, Debug, PartialEq)]
pub struct HorizontalListModel {
    pub layout: HorizontalPageLayout,
    /// The page-turn arrow column's width (`pageArrowView.intrinsicContentSize`).
    arrow_width: f32,
    item_height: f32,
    selected_index: usize,
}

impl HorizontalListModel {
    pub fn new(layout: HorizontalPageLayout, arrow_width: f32, item_height: f32) -> Self {
        Self {
            layout,
            arrow_width,
            item_height,
            selected_index: 0,
        }
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// The width the cells of the visible page fill — a short page keeps
    /// the full budget once the list pages, so the arrow's edge does not
    /// wander (`HorizontalCandidatePanel.swift:155`).
    pub fn content_width(&self) -> f32 {
        let page_width = self.layout.page_width(self.current_page());
        if self.is_paged() {
            page_width.max(self.layout.page_budget)
        } else {
            page_width
        }
    }

    /// Single page: the cells; paged: `max(page, budget)` plus the arrow
    /// column (`HorizontalCandidatePanel.swift:155-158`).
    pub fn window_width(&self) -> f32 {
        self.content_width()
            + if self.is_paged() {
                self.arrow_width
            } else {
                0.0
            }
    }

    /// Where the page-turn arrow column sits, when the list pages.
    pub fn arrow_rect(&self) -> Option<Rect> {
        self.is_paged().then(|| {
            Rect::new(
                self.content_width(),
                0.0,
                self.arrow_width,
                self.item_height,
            )
        })
    }

    /// The visible page's cell frames, end to end.
    pub fn page_cell_rects(&self) -> Vec<(usize, Rect)> {
        let mut x = 0.0;
        self.layout
            .pages
            .get(self.current_page())
            .map_or_else(Vec::new, |page| {
                page.iter()
                    .map(|slot| {
                        let rect = Rect::new(x, 0.0, slot.width, self.item_height);
                        x += slot.width;
                        (slot.candidate_index, rect)
                    })
                    .collect()
            })
    }

    /// The candidate under `point` (window coordinates) on the visible page.
    pub fn hit_test(&self, point: Point) -> Option<usize> {
        if point.y < 0.0 || point.y >= self.item_height {
            return None;
        }
        self.page_cell_rects()
            .into_iter()
            .find(|(_, rect)| point.x >= rect.x && point.x < rect.right())
            .map(|(index, _)| index)
    }

    pub fn is_empty(&self) -> bool {
        self.layout.candidate_count() == 0
    }

    /// The page the selection is on — the visible page.
    pub fn current_page(&self) -> usize {
        self.layout
            .page_index_containing(self.selected_index)
            .unwrap_or(0)
    }

    pub fn is_paged(&self) -> bool {
        self.layout.pages.len() > 1
    }

    /// Whether the page-up / page-down arrows are enabled.
    pub fn page_arrows(&self) -> (bool, bool) {
        let page = self.current_page();
        (page > 0, page + 1 < self.layout.pages.len())
    }

    /// The absolute index the slot key picks on the visible page.
    pub fn candidate_index_for_slot(&self, slot: usize) -> Option<usize> {
        self.layout
            .candidate_index_for_slot(slot, self.current_page())
    }

    /// Moves the selection; `false` when the direction is at an end.
    pub fn navigate(&mut self, direction: CandidateNavigation) -> bool {
        match self.layout.target(direction, self.selected_index) {
            Some(target) => {
                self.selected_index = target;
                true
            }
            None => false,
        }
    }

    /// A click on `index` selects it (never commits).
    pub fn select(&mut self, index: usize) {
        if index < self.layout.candidate_count() {
            self.selected_index = index;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLOT: f32 = 10.0;
    const ROOMY: f32 = 1_000.0;

    fn pack(widths: &[f32]) -> HorizontalPageLayout {
        HorizontalPageLayout::pack(widths, SLOT, ROOMY)
    }

    fn counts(layout: &HorizontalPageLayout) -> Vec<usize> {
        layout.pages.iter().map(Vec::len).collect()
    }

    fn indices(layout: &HorizontalPageLayout) -> Vec<Vec<usize>> {
        layout
            .pages
            .iter()
            .map(|page| page.iter().map(|slot| slot.candidate_index).collect())
            .collect()
    }

    #[test]
    fn narrow_items_fill_nine_per_page() {
        // trace: HorizontalPageLayoutTests.swift:47-59.
        let layout = pack(&[5.0; 20]);
        assert_eq!(counts(&layout), [9, 9, 2]);
        assert_eq!(layout.candidate_count(), 20);
        assert_eq!(layout.pages[1][0].candidate_index, 9);
    }

    #[test]
    fn wide_item_breaks_the_page_and_keeps_its_measured_width_up_to_the_screen() {
        let layout = pack(&[10.0, 200.0, 10.0]);
        assert_eq!(indices(&layout), [[0], [1], [2]]);
        assert_eq!(layout.pages[1][0].width, 200.0);
        let clamped = HorizontalPageLayout::pack(&[10.0, 200.0, 10.0], SLOT, 120.0);
        assert_eq!(clamped.pages[1][0].width, 120.0);
        let narrow = HorizontalPageLayout::pack(&[5.0; 20], SLOT, 45.0);
        assert_eq!(narrow.page_budget, 45.0);
        assert_eq!(counts(&narrow), [4, 4, 4, 4, 4]);
    }

    #[test]
    fn chrome_is_reserved_only_once_the_list_pages() {
        // trace: HorizontalPageLayoutTests.swift:93-112.
        let single = HorizontalPageLayout::pack_with_chrome(&[90.0], SLOT, 90.0, 20.0);
        assert_eq!(single.pages.len(), 1);
        assert_eq!(single.pages[0][0].width, 90.0);
        let paged = HorizontalPageLayout::pack_with_chrome(&[90.0, 90.0], SLOT, 90.0, 20.0);
        assert_eq!(indices(&paged), [[0], [1]]);
        assert_eq!(paged.pages[0][0].width, 70.0);
        assert_eq!(paged.page_budget, 70.0);
    }

    #[test]
    fn slots_address_the_visible_page() {
        let layout = pack(&[5.0; 20]);
        assert_eq!(layout.candidate_index_for_slot(1, 2), Some(19));
        assert_eq!(layout.candidate_index_for_slot(2, 2), None);
        assert_eq!(layout.candidate_index_for_slot(0, 3), None);
    }

    #[test]
    fn arrows_walk_and_clamp_and_paging_lands_under_the_highlight() {
        use CandidateNavigation::*;
        let layout = pack(&[5.0; 20]);
        assert_eq!(layout.target(Right, 0), Some(1));
        assert_eq!(layout.target(Right, 8), Some(9));
        assert_eq!(layout.target(Left, 0), None);
        assert_eq!(layout.target(Right, 19), None);
        assert_eq!(layout.target(PageUp, 0), None);
        assert_eq!(layout.target(PageDown, 19), None);
        assert_eq!(layout.target(Down, 0), layout.target(PageDown, 0));
        assert_eq!(layout.target(Up, 9), layout.target(PageUp, 9));
        assert_eq!(layout.target(NextCandidate, 0), layout.target(Right, 0));
        assert_eq!(layout.target(PreviousCandidate, 5), layout.target(Left, 5));
        assert_eq!(layout.target(NextCandidate, 8), Some(9));
        assert_eq!(layout.target(PreviousCandidate, 0), None);
        assert_eq!(layout.target(NextCandidate, 19), None);

        // trace: HorizontalPageLayoutTests.swift:146-171.
        let layout = pack(&[10.0, 80.0, 30.0, 30.0, 30.0]);
        assert_eq!(indices(&layout), [vec![0, 1], vec![2, 3, 4]]);
        assert_eq!(layout.target(PageDown, 1), Some(2));
        assert_eq!(layout.target(PageUp, 3), Some(1));
        let layout = pack(&[30.0, 30.0, 30.0, 10.0, 80.0]);
        assert_eq!(indices(&layout), [vec![0, 1, 2], vec![3, 4]]);
        assert_eq!(layout.target(PageUp, 4), Some(1), "backward tiebreak");
    }

    #[test]
    fn list_model_tracks_page_and_arrows() {
        let mut model = HorizontalListModel::new(pack(&[5.0; 20]), 20.0, 30.0);
        assert_eq!(model.current_page(), 0);
        assert_eq!(model.page_arrows(), (false, true));
        assert!(model.navigate(CandidateNavigation::PageDown));
        assert_eq!(model.selected_index(), 9);
        assert_eq!(model.current_page(), 1);
        assert_eq!(model.candidate_index_for_slot(0), Some(9));
        assert!(model.navigate(CandidateNavigation::PageDown));
        assert_eq!(model.page_arrows(), (true, false));
        assert!(!model.navigate(CandidateNavigation::PageDown), "clamps");
        model.select(3);
        assert_eq!(model.current_page(), 0);
        model.select(99);
        assert_eq!(model.selected_index(), 3, "out of range is ignored");
    }

    #[test]
    fn window_width_reserves_the_budget_and_the_arrow_once_the_list_pages() {
        // trace: HorizontalCandidatePanel.swift:155-158. 20 × 5 → three pages,
        // the last holding 2 cells of 10.
        let mut paged = HorizontalListModel::new(pack(&[5.0; 20]), 20.0, 30.0);
        assert_eq!(paged.content_width(), 90.0);
        assert_eq!(paged.window_width(), 90.0 + 20.0);
        paged.select(19);
        assert_eq!(paged.layout.page_width(2), 20.0);
        assert_eq!(paged.content_width(), 90.0, "a short page keeps the budget");
        assert_eq!(paged.arrow_rect(), Some(Rect::new(90.0, 0.0, 20.0, 30.0)));
        assert_eq!(paged.page_cell_rects().len(), 2);
        assert_eq!(paged.hit_test(Point { x: 15.0, y: 3.0 }), Some(19));
        assert_eq!(
            paged.hit_test(Point { x: 25.0, y: 3.0 }),
            None,
            "the empty budget"
        );
        assert_eq!(paged.hit_test(Point { x: 5.0, y: 31.0 }), None);
        let single = HorizontalListModel::new(pack(&[5.0; 3]), 20.0, 30.0);
        assert_eq!(
            single.window_width(),
            30.0,
            "no arrow, no budget reservation"
        );
        assert_eq!(single.arrow_rect(), None);
    }

    #[test]
    fn a_page_breaks_only_when_the_total_passes_the_budget() {
        // f32 boundary: exactly the budget stays; one ulp-scale step past it breaks.
        let exact = HorizontalPageLayout::pack(&[45.0, 45.0], SLOT, 90.0);
        assert_eq!(counts(&exact), [2]);
        let over = HorizontalPageLayout::pack(&[45.0, 45.0 + 0.001], SLOT, 90.0);
        assert_eq!(counts(&over), [1, 1]);
        assert_eq!(
            HorizontalPageLayout::pack(&[5.0; 300], SLOT, ROOMY).candidate_count(),
            MAX_DISPLAY_CANDIDATES
        );
        assert_eq!(HorizontalPageLayout::default().candidate_count(), 0);
    }
}
