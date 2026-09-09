//! Which key picks which candidate, and how that key is written in the
//! window. Port of `CandidateIndexLabel.swift`.

use super::horizontal::HorizontalPageLayout;
use crate::keys::CandidateSlotKeySet;

pub struct CandidateIndexLabel;

impl CandidateIndexLabel {
    /// The key for a zero-based position, or `""` past the ninth — a
    /// position no key names. The cell still reserves the slot.
    pub fn text_for_slot(slot: usize, key_set: CandidateSlotKeySet) -> String {
        if slot < HorizontalPageLayout::PAGE_SIZE {
            key_set.label_for_slot(slot)
        } else {
            String::new()
        }
    }

    /// How far the keys move along the row when the first cell takes none: 1
    /// while `lead_cell_is_unkeyed` and the row `index_for_slot` numbers
    /// begins with cell 0 — the §34 literal — and 0 otherwise. Hoisted out of
    /// the per-slot lookup because it is invariant across a whole repaint.
    /// Port of macOS `CandidateIndexLabel.keySlotShift(leadCellIsUnkeyed:indexForSlot:)`.
    pub fn key_slot_shift(
        lead_cell_is_unkeyed: bool,
        index_for_slot: impl Fn(usize) -> Option<usize>,
    ) -> usize {
        usize::from(lead_cell_is_unkeyed && index_for_slot(0) == Some(0))
    }

    /// The absolute index the `slot`-th KEY addresses, given the layout's own
    /// `index_for_slot` mapping: the unkeyed §34 literal is skipped and the
    /// keys shift by one wherever the row they number begins with it. Port of
    /// macOS `CandidateIndexLabel.candidateIndex(forKeySlot:leadCellIsUnkeyed:indexForSlot:)`,
    /// which carries the full rationale.
    pub fn candidate_index_for_key_slot(
        slot: usize,
        lead_cell_is_unkeyed: bool,
        index_for_slot: impl Fn(usize) -> Option<usize>,
    ) -> Option<usize> {
        if slot >= HorizontalPageLayout::PAGE_SIZE {
            return None;
        }
        let shift = Self::key_slot_shift(lead_cell_is_unkeyed, &index_for_slot);
        index_for_slot(slot + shift)
    }

    /// Every form a slot can be drawn in — each slot under each set — so the
    /// column is measured against all of them and does not change width
    /// when the user switches tone scheme.
    pub fn widest_label_forms() -> Vec<String> {
        (0..HorizontalPageLayout::PAGE_SIZE)
            .flat_map(|slot| {
                CandidateSlotKeySet::ALL
                    .iter()
                    .map(move |set| Self::text_for_slot(slot, *set))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_follow_the_set_and_a_tenth_slot_is_blank() {
        // trace: CandidateIndexLabelTests.swift — bare row under Standard,
        // digits under Telex, nothing past the ninth.
        let bare: Vec<String> = (0..9)
            .map(|slot| CandidateIndexLabel::text_for_slot(slot, CandidateSlotKeySet::BareKeys))
            .collect();
        assert_eq!(bare, ["q", "w", "d", "f", "z", "x", "v", "y", ";"]);
        let digits: Vec<String> = (0..9)
            .map(|slot| CandidateIndexLabel::text_for_slot(slot, CandidateSlotKeySet::Digits))
            .collect();
        assert_eq!(digits, ["1", "2", "3", "4", "5", "6", "7", "8", "9"]);
        assert_eq!(
            CandidateIndexLabel::text_for_slot(9, CandidateSlotKeySet::Digits),
            ""
        );
        assert_eq!(CandidateIndexLabel::widest_label_forms().len(), 9 * 2);
    }

    /// The row a nine-cell page numbers, as a layout would answer it.
    fn page(first_index: usize) -> impl Fn(usize) -> Option<usize> {
        move |slot| (slot < 9).then_some(first_index + slot)
    }

    #[test]
    fn the_unkeyed_literal_moves_the_first_key_onto_the_second_cell() {
        // trace: page starts at cell 0 = the literal → key 0 picks cell 1,
        // key 7 picks cell 8, and the ninth key falls off the row.
        let keyed: Vec<Option<usize>> = (0..9)
            .map(|slot| CandidateIndexLabel::candidate_index_for_key_slot(slot, true, page(0)))
            .collect();
        assert_eq!(
            keyed,
            [
                Some(1),
                Some(2),
                Some(3),
                Some(4),
                Some(5),
                Some(6),
                Some(7),
                Some(8),
                None
            ]
        );
    }

    #[test]
    fn a_row_the_literal_does_not_lead_keeps_every_key() {
        // trace: a later page / a scrolled column starts at cell 9, so the
        // literal is not on it and nothing shifts — even with the flag set.
        assert_eq!(
            CandidateIndexLabel::candidate_index_for_key_slot(0, true, page(9)),
            Some(9)
        );
        assert_eq!(
            CandidateIndexLabel::candidate_index_for_key_slot(8, true, page(9)),
            Some(17)
        );
        // Flag off: cell 0 keeps its key, which is the setting-off list.
        assert_eq!(
            CandidateIndexLabel::candidate_index_for_key_slot(0, false, page(0)),
            Some(0)
        );
        // Past the key row, either way.
        assert_eq!(
            CandidateIndexLabel::candidate_index_for_key_slot(9, false, page(0)),
            None
        );
    }

    #[test]
    fn a_short_row_leading_with_the_literal_keys_what_it_holds() {
        // trace: an expanded grid row of three cells (0, 1, 2) starting at the
        // literal — keys 0 and 1 pick cells 1 and 2, key 2 picks nothing.
        let row = |slot: usize| (slot < 3).then_some(slot);
        assert_eq!(
            CandidateIndexLabel::candidate_index_for_key_slot(0, true, row),
            Some(1)
        );
        assert_eq!(
            CandidateIndexLabel::candidate_index_for_key_slot(1, true, row),
            Some(2)
        );
        assert_eq!(
            CandidateIndexLabel::candidate_index_for_key_slot(2, true, row),
            None
        );
    }

    #[test]
    fn a_literal_only_list_keys_nothing() {
        // trace: the literal is the whole list — every key falls off the row.
        let only_literal = |slot: usize| (slot == 0).then_some(0);
        assert!(
            (0..9).all(|slot| CandidateIndexLabel::candidate_index_for_key_slot(
                slot,
                true,
                only_literal
            )
            .is_none())
        );
    }
}
