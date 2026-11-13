//! The highlighted candidate and its page, for a list the SHELL pages
//! (Linux roadmap L4): the panel shows `page_size` cells per page from index 0,
//! and the engine only tells it which absolute index is highlighted. So
//! this is the part of `HorizontalListModel` that decides rather than
//! measures — with no widths, a "page" is a fixed run of cells.
//!
//! Horizontal: `Down` / `PageDown` step a page, `Right` / `Next` a cell,
//! exactly as `HorizontalPageLayout::target` reads the six directions.
//! Vertical: the arrows turn, as in `VerticalListModel::navigate` — `Up` /
//! `Down` step a cell, `Left` / `Right` a page. A step past either end is
//! refused (the highlight stays), as on the Mac.

use crate::candidates::CandidateIndexLabel;
use crate::keys::CandidateNavigation;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LookupSelection {
    count: usize,
    page_size: usize,
    selected: usize,
    /// Start index of each page when pages are not equal runs of `page_size`.
    /// Empty means the uniform pages.
    page_starts: Vec<usize>,
    /// The list begins with the §34 literal, which takes no key — as on
    /// macOS and Windows (`CandidateIndexLabel`).
    lead_cell_is_unkeyed: bool,
}

impl LookupSelection {
    pub fn new(count: usize, page_size: usize) -> Self {
        Self {
            count,
            page_size: page_size.max(1),
            selected: 0,
            page_starts: Vec::new(),
            lead_cell_is_unkeyed: false,
        }
    }

    /// Pages begin at these absolute indexes. A list that does not start at 0,
    /// or that is not strictly increasing, is ignored and pages stay uniform.
    pub fn set_page_starts(&mut self, starts: Vec<usize>) {
        let ok = starts.first() == Some(&0)
            && starts.windows(2).all(|pair| pair[0] < pair[1])
            && starts
                .last()
                .is_some_and(|start| *start < self.count.max(1));
        self.page_starts = if ok { starts } else { Vec::new() };
    }

    pub fn clear_page_starts(&mut self) {
        self.page_starts.clear();
    }

    /// The current page, `start..end` over the absolute list.
    pub fn page_bounds(&self) -> (usize, usize) {
        self.bounds_at(self.selected)
    }

    fn bounds_at(&self, index: usize) -> (usize, usize) {
        if self.page_starts.is_empty() {
            let start = index / self.page_size * self.page_size;
            return (start, (start + self.page_size).min(self.count));
        }
        let mut start = 0;
        for &next in self.page_starts.iter().skip(1) {
            if index < next {
                return (start, next.min(self.count));
            }
            start = next;
        }
        (start, self.count)
    }

    /// Pages of `page_size` from now on, the highlight kept — the slot key
    /// set the list is presented under (`CandidateSlotKeySet::page_size`).
    pub fn set_page_size(&mut self, page_size: usize) {
        self.page_size = page_size.max(1);
    }

    /// Whether the list's first cell is the §34 literal, which takes no key —
    /// set whenever the list is built or presented again
    /// (`CandidateSource::leads_with_literal_roman`).
    pub fn set_unkeyed_lead(&mut self, lead_cell_is_unkeyed: bool) {
        self.lead_cell_is_unkeyed = lead_cell_is_unkeyed;
    }

    /// The highlighted absolute index, `None` for an empty list.
    pub fn selected_index(&self) -> Option<usize> {
        (self.count > 0).then_some(self.selected)
    }

    pub fn current_page(&self) -> usize {
        if self.page_starts.is_empty() {
            return self.selected / self.page_size;
        }
        self.page_starts
            .iter()
            .rposition(|start| *start <= self.selected)
            .unwrap_or(0)
    }

    /// The absolute index the `slot`-th key on the current page addresses,
    /// `None` past the end of a short last page (the key is consumed all the
    /// same — `SelectCandidateSlot`). On a page that begins with the unkeyed
    /// literal the keys start at its second cell, and the last key has no
    /// cell.
    pub fn candidate_index_for_key_slot(&self, slot: usize) -> Option<usize> {
        CandidateIndexLabel::candidate_index_for_key_slot(
            slot,
            self.lead_cell_is_unkeyed,
            |position| self.candidate_index_on_page(position),
        )
    }

    /// How many cells at the start of the current page take no key: 1 on the
    /// page that begins with the unkeyed literal, else 0.
    pub fn key_slot_shift(&self) -> usize {
        CandidateIndexLabel::key_slot_shift(self.lead_cell_is_unkeyed, |position| {
            self.candidate_index_on_page(position)
        })
    }

    /// The absolute index of the `position`-th cell on the current page —
    /// what the panel reports for a click.
    pub fn candidate_index_on_page(&self, position: usize) -> Option<usize> {
        let (start, end) = self.page_bounds();
        if position >= end - start || position >= self.page_size {
            return None;
        }
        let index = start + position;
        (index < self.count).then_some(index)
    }

    pub fn select(&mut self, index: usize) {
        if index < self.count {
            self.selected = index;
        }
    }

    /// Answers whether the highlight moved. `vertical` = the panel draws the
    /// list as a column.
    pub fn navigate(&mut self, direction: CandidateNavigation, vertical: bool) -> bool {
        let direction = if vertical {
            Self::column_direction(direction)
        } else {
            direction
        };
        let Some(target) = self.target(direction) else {
            return false;
        };
        self.selected = target;
        true
    }

    /// A column has no cell beside a cell, so the horizontal arrows page
    /// and the vertical ones step a cell.
    fn column_direction(direction: CandidateNavigation) -> CandidateNavigation {
        match direction {
            CandidateNavigation::Up => CandidateNavigation::PreviousCandidate,
            CandidateNavigation::Down => CandidateNavigation::NextCandidate,
            CandidateNavigation::Left => CandidateNavigation::PageUp,
            CandidateNavigation::Right => CandidateNavigation::PageDown,
            other => other,
        }
    }

    fn target(&self, direction: CandidateNavigation) -> Option<usize> {
        if self.count == 0 {
            return None;
        }
        match direction {
            CandidateNavigation::Right | CandidateNavigation::NextCandidate => {
                (self.selected + 1 < self.count).then_some(self.selected + 1)
            }
            CandidateNavigation::Left | CandidateNavigation::PreviousCandidate => {
                self.selected.checked_sub(1)
            }
            CandidateNavigation::Down | CandidateNavigation::PageDown => {
                let (start, end) = self.page_bounds();
                (end < self.count).then(|| {
                    let offset = self.selected - start;
                    let (_, next_end) = self.bounds_at(end);
                    end + offset.min(next_end.saturating_sub(end).saturating_sub(1))
                })
            }
            CandidateNavigation::Up | CandidateNavigation::PageUp => {
                let (start, _) = self.page_bounds();
                (start > 0).then(|| {
                    let offset = self.selected - start;
                    let (prev, prev_end) = self.bounds_at(start - 1);
                    prev + offset.min(prev_end.saturating_sub(prev).saturating_sub(1))
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_step_and_pages_step_with_the_ends_refused() {
        // trace: 20 cells, 9 per page → pages [0..9), [9..18), [18..20).
        let mut selection = LookupSelection::new(20, 9);
        assert_eq!(selection.selected_index(), Some(0));
        assert!(!selection.navigate(CandidateNavigation::Left, false));
        assert!(selection.navigate(CandidateNavigation::Right, false));
        assert_eq!(selection.selected_index(), Some(1));
        assert!(selection.navigate(CandidateNavigation::PageDown, false));
        assert_eq!(selection.selected_index(), Some(10));
        assert_eq!(selection.current_page(), 1);
        assert!(selection.navigate(CandidateNavigation::Down, false));
        // Past the last page's short end: clamped to the last cell.
        assert_eq!(selection.selected_index(), Some(19));
        assert!(!selection.navigate(CandidateNavigation::PageDown, false));
        assert!(selection.navigate(CandidateNavigation::Up, false));
        assert_eq!(selection.selected_index(), Some(10));
        assert!(selection.navigate(CandidateNavigation::PreviousCandidate, false));
        assert_eq!(selection.selected_index(), Some(9));
    }

    #[test]
    fn a_column_steps_cells_on_up_down_and_pages_on_left_right() {
        // trace: 20 cells, 9 per page, vertical → Up/Down = cell, Left/Right = page.
        let mut selection = LookupSelection::new(20, 9);
        assert!(!selection.navigate(CandidateNavigation::Up, true));
        assert!(selection.navigate(CandidateNavigation::Down, true));
        assert_eq!(selection.selected_index(), Some(1));
        assert!(selection.navigate(CandidateNavigation::Right, true));
        assert_eq!(selection.selected_index(), Some(10));
        assert_eq!(selection.current_page(), 1);
        assert!(selection.navigate(CandidateNavigation::Up, true));
        assert_eq!(selection.selected_index(), Some(9));
        assert!(selection.navigate(CandidateNavigation::Left, true));
        assert_eq!(selection.selected_index(), Some(0));
        assert!(!selection.navigate(CandidateNavigation::Left, true));
        // The panel's own buttons read the same either way.
        assert!(selection.navigate(CandidateNavigation::PageDown, true));
        assert_eq!(selection.selected_index(), Some(9));
    }

    #[test]
    fn slot_keys_address_the_current_page_and_refuse_a_short_pages_gap() {
        let mut selection = LookupSelection::new(11, 9);
        assert_eq!(selection.candidate_index_for_key_slot(3), Some(3));
        selection.select(10);
        assert_eq!(selection.current_page(), 1);
        assert_eq!(selection.candidate_index_for_key_slot(0), Some(9));
        assert_eq!(selection.candidate_index_for_key_slot(1), Some(10));
        assert_eq!(selection.candidate_index_for_key_slot(2), None);
        assert_eq!(selection.candidate_index_for_key_slot(9), None);
    }

    #[test]
    fn an_unkeyed_lead_moves_the_keys_on_its_page_only() {
        // trace: 20 cells, 9 per page, cell 0 = the §34 literal. Page 0:
        // keys 0..7 → cells 1..8, key 8 → none; page 1 starts at cell 9, so
        // nothing shifts there. A click still reaches the literal.
        let mut selection = LookupSelection::new(20, 9);
        selection.set_unkeyed_lead(true);
        assert_eq!(selection.key_slot_shift(), 1);
        assert_eq!(selection.candidate_index_for_key_slot(0), Some(1));
        assert_eq!(selection.candidate_index_for_key_slot(7), Some(8));
        assert_eq!(selection.candidate_index_for_key_slot(8), None);
        assert_eq!(selection.candidate_index_on_page(0), Some(0));
        selection.select(9);
        assert_eq!(selection.key_slot_shift(), 0);
        assert_eq!(selection.candidate_index_for_key_slot(0), Some(9));
        assert_eq!(selection.candidate_index_for_key_slot(8), Some(17));
        // A literal-only list: no key picks anything.
        let mut only = LookupSelection::new(1, 9);
        only.set_unkeyed_lead(true);
        assert_eq!(only.candidate_index_for_key_slot(0), None);
        assert_eq!(only.candidate_index_on_page(0), Some(0));
    }

    #[test]
    fn an_empty_list_has_no_selection_and_moves_nowhere() {
        let mut selection = LookupSelection::new(0, 9);
        assert_eq!(selection.selected_index(), None);
        assert!(!selection.navigate(CandidateNavigation::Right, false));
        assert_eq!(selection.candidate_index_for_key_slot(0), None);
    }
}
