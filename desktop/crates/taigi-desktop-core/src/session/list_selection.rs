//! The highlight over a list: paged by [`LookupSelection`] when the shell's
//! framework pages it, or held by an [`ExpandableListModel`] when the shell
//! draws its own window and can measure it ([`ListLayout`]). The session reads
//! one type either way; Linux supplies no layout, so its lists stay paged.

use std::fmt::Debug;

use super::{LookupSelection, PAGE_SIZE};
use crate::candidates::{CandidateIndexLabel, ExpandableDisplayMode, ExpandableListModel};
use crate::composing::CandidateCellContent;
use crate::keys::CandidateNavigation;
use crate::settings::SettingsDocument;

/// A shell that measures its own candidate window. Runs from `present_table`,
/// outside the coordinator lock, once per list.
pub trait ListLayout: Debug + Send + Sync {
    /// The model for `cells`, or `None` when this list pages through
    /// [`LookupSelection`].
    fn layout(
        &self,
        cells: &[CandidateCellContent],
        settings: &SettingsDocument,
    ) -> Option<ExpandableListModel>;
}

#[derive(Clone, Debug, PartialEq)]
enum Highlight {
    Paged(LookupSelection),
    Expandable(ExpandableListModel),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ListSelection {
    highlight: Highlight,
    /// The list's first cell is the §34 literal, which takes no key.
    lead_cell_is_unkeyed: bool,
    /// A new list, or cells changed in place: a shell with a [`ListLayout`] has
    /// not seen these.
    needs_layout: bool,
}

impl ListSelection {
    pub fn new(count: usize) -> Self {
        Self {
            highlight: Highlight::Paged(LookupSelection::new(count, PAGE_SIZE)),
            lead_cell_is_unkeyed: false,
            needs_layout: true,
        }
    }

    /// The model, when this list has one.
    pub fn expandable(&self) -> Option<&ExpandableListModel> {
        match &self.highlight {
            Highlight::Paged(_) => None,
            Highlight::Expandable(model) => Some(model),
        }
    }

    pub fn expandable_mut(&mut self) -> Option<&mut ExpandableListModel> {
        match &mut self.highlight {
            Highlight::Paged(_) => None,
            Highlight::Expandable(model) => Some(model),
        }
    }

    pub fn lead_cell_is_unkeyed(&self) -> bool {
        self.lead_cell_is_unkeyed
    }

    pub fn set_unkeyed_lead(&mut self, lead_cell_is_unkeyed: bool) {
        self.lead_cell_is_unkeyed = lead_cell_is_unkeyed;
        if let Highlight::Paged(selection) = &mut self.highlight {
            selection.set_unkeyed_lead(lead_cell_is_unkeyed);
        }
    }

    /// The highlighted absolute index, `None` for an empty list.
    pub fn selected_index(&self) -> Option<usize> {
        match &self.highlight {
            Highlight::Paged(selection) => selection.selected_index(),
            Highlight::Expandable(model) => (model.count() > 0).then(|| model.selected_index()),
        }
    }

    /// How many cells at the start of the keyed row take no key.
    pub fn key_slot_shift(&self) -> usize {
        match &self.highlight {
            Highlight::Paged(selection) => selection.key_slot_shift(),
            Highlight::Expandable(model) => {
                CandidateIndexLabel::key_slot_shift(self.lead_cell_is_unkeyed, |position| {
                    model.candidate_index_for_slot(position)
                })
            }
        }
    }

    /// The absolute index the `slot`-th key addresses.
    pub fn candidate_index_for_key_slot(&self, slot: usize) -> Option<usize> {
        match &self.highlight {
            Highlight::Paged(selection) => selection.candidate_index_for_key_slot(slot),
            Highlight::Expandable(model) => CandidateIndexLabel::candidate_index_for_key_slot(
                slot,
                self.lead_cell_is_unkeyed,
                |position| model.candidate_index_for_slot(position),
            ),
        }
    }

    /// The absolute index a click on `position` means: the cell of the current
    /// page, or under a model, which draws the whole list, that very cell.
    pub fn candidate_index_at(&self, position: usize) -> Option<usize> {
        match &self.highlight {
            Highlight::Paged(selection) => selection.candidate_index_on_page(position),
            Highlight::Expandable(model) => (position < model.count()).then_some(position),
        }
    }

    pub fn select(&mut self, index: usize) {
        match &mut self.highlight {
            Highlight::Paged(selection) => selection.select(index),
            Highlight::Expandable(model) => {
                model.select(index);
            }
        }
    }

    /// `vertical` = the panel draws a paged list as a column; a model knows
    /// its own axis.
    pub fn navigate(&mut self, direction: CandidateNavigation, vertical: bool) {
        match &mut self.highlight {
            Highlight::Paged(selection) => {
                selection.navigate(direction, vertical);
            }
            Highlight::Expandable(model) => {
                model.navigate(direction);
            }
        }
    }

    /// The cells changed in place: the next `present_table` lays them out
    /// again, keeping the highlight and the unfolded state.
    pub fn mark_stale(&mut self) {
        self.needs_layout = true;
    }

    pub(super) fn needs_layout(&self) -> bool {
        self.needs_layout
    }

    /// Moves the highlight onto `layout`'s answer, as the Windows window's
    /// `update_cells` does. `None` leaves a paged list as it is, and turns a
    /// list that had a model back into pages of `count` cells.
    pub(super) fn adopt(&mut self, layout: Option<ExpandableListModel>, count: usize) {
        self.needs_layout = false;
        let kept = self.selected_index().unwrap_or(0);
        let was_expanded = self
            .expandable()
            .is_some_and(|model| model.mode() == ExpandableDisplayMode::Expanded);
        match layout {
            Some(mut model) => {
                model.select(kept.min(model.count().saturating_sub(1)));
                if was_expanded {
                    model.expand();
                    model.forget_transition();
                }
                self.highlight = Highlight::Expandable(model);
            }
            None if self.expandable().is_some() => {
                let mut selection = LookupSelection::new(count, PAGE_SIZE);
                selection.select(kept);
                selection.set_unkeyed_lead(self.lead_cell_is_unkeyed);
                self.highlight = Highlight::Paged(selection);
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::test_support::row_model;

    fn expandable(count: usize) -> ListSelection {
        let mut selection = ListSelection::new(count);
        selection.adopt(Some(row_model(count)), count);
        selection
    }

    #[test]
    fn a_paged_list_stays_paged_when_no_layout_answers() {
        let mut selection = ListSelection::new(20);
        selection.select(10);
        selection.set_unkeyed_lead(true);
        assert!(selection.needs_layout());
        selection.adopt(None, 20);
        assert!(selection.expandable().is_none());
        assert_eq!(selection.selected_index(), Some(10));
        assert!(selection.lead_cell_is_unkeyed());
        // trace: page 1 starts at cell 9, so the lead moves no key there.
        assert_eq!(selection.key_slot_shift(), 0);
        // Asked once: the answer stands until the cells change.
        assert!(!selection.needs_layout());
    }

    #[test]
    fn a_click_means_a_page_position_or_under_a_model_an_absolute_cell() {
        let mut paged = ListSelection::new(20);
        paged.select(10);
        assert_eq!(paged.candidate_index_at(2), Some(11));
        let under_model = expandable(20);
        assert_eq!(under_model.candidate_index_at(11), Some(11));
        assert_eq!(under_model.candidate_index_at(20), None);
    }

    #[test]
    fn an_unkeyed_lead_shifts_the_keys_of_the_collapsed_row() {
        let mut selection = expandable(20);
        assert_eq!(selection.key_slot_shift(), 0);
        assert_eq!(selection.candidate_index_for_key_slot(0), Some(0));
        selection.set_unkeyed_lead(true);
        assert_eq!(selection.key_slot_shift(), 1);
        assert_eq!(selection.candidate_index_for_key_slot(0), Some(1));
    }

    #[test]
    fn navigating_goes_through_the_model() {
        // trace: Row axis, `Down` unfolds rather than stepping a page.
        let mut selection = expandable(40);
        selection.navigate(CandidateNavigation::Down, true);
        let model = selection.expandable().expect("a model");
        assert_eq!(model.mode(), ExpandableDisplayMode::Expanded);
        assert_eq!(selection.selected_index(), Some(0));
    }

    #[test]
    fn a_redraw_in_place_keeps_the_highlight_and_the_unfolded_state() {
        let mut selection = expandable(40);
        selection.navigate(CandidateNavigation::Down, true);
        selection.select(30);
        selection.set_unkeyed_lead(true);
        assert!(!selection.needs_layout());
        selection.mark_stale();
        assert!(selection.needs_layout());
        selection.adopt(Some(row_model(40)), 40);
        assert!(!selection.needs_layout());
        assert_eq!(selection.selected_index(), Some(30));
        assert!(selection.lead_cell_is_unkeyed());
        let model = selection.expandable().expect("a model");
        assert_eq!(model.mode(), ExpandableDisplayMode::Expanded);
        assert_eq!(model.mode_transition(), None, "no change to play");
    }

    #[test]
    fn a_list_that_shrank_in_place_keeps_its_last_cell() {
        let mut selection = expandable(40);
        selection.select(30);
        selection.mark_stale();
        selection.adopt(Some(row_model(10)), 10);
        assert_eq!(selection.selected_index(), Some(9));
    }

    #[test]
    fn a_model_given_up_falls_back_to_pages_at_the_same_cell() {
        let mut selection = expandable(40);
        selection.select(30);
        selection.set_unkeyed_lead(true);
        selection.mark_stale();
        selection.adopt(None, 40);
        assert!(selection.expandable().is_none());
        assert_eq!(selection.selected_index(), Some(30));
        assert!(selection.lead_cell_is_unkeyed());
    }

    #[test]
    fn an_empty_model_has_no_highlight() {
        assert_eq!(expandable(0).selected_index(), None);
    }
}
