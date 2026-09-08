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
}
