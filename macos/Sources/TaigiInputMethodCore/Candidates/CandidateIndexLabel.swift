// Which key picks which candidate, and how that key is written in the window.

import Foundation

/// The key drawn beside a candidate — the one that picks it.
///
/// One place rather than a literal per layout, because the labels are a
/// contract with the key handler: a slot's label is the key
/// `CandidateSlotKeySet.slot(for:)` maps to that slot, and every layout
/// answers those slots from the positions it draws
/// (`CandidatePresenter.candidateIndex(forSlot:)`).
///
/// Always the live set: it is the only thing that picks (USER 2026-08-28,
/// which retired both the bare-digit-after-a-tone rule and the `↓` latch
/// that used to flip the hint to `1`…`9`), and it follows the tone scheme
/// (`ToneInputScheme.slotKeySet`) — letters under Standard, digits under
/// Telex.
///
/// `0` is deliberately absent, unlike upstream MacishType's `"1234567890"`
/// (`MacishCandidateWindow/CandidateWindow.swift:49`): `⌃0` is not bound and a
/// bare `0` is document text, so drawing a `0` would name a key that does
/// nothing.
enum CandidateIndexLabel {
    /// The key for a zero-based position, or `""` past the ninth — a position
    /// no key names. The cell still reserves the slot, so a tenth candidate
    /// lines up with the nine above it.
    static func text(forSlot slot: Int, keySet: CandidateSlotKeySet) -> String {
        guard (0 ..< HorizontalPageLayout.pageSize).contains(slot) else { return "" }
        return keySet.label(forSlot: slot)
    }

    /// How far the keys move along the row when the first cell takes none:
    /// 1 while `leadCellIsUnkeyed` and the row `indexForSlot` numbers begins
    /// with cell 0 — the §34 literal — and 0 otherwise.
    ///
    /// Conditional on the ROW, not on the list: the layouts number a page
    /// (horizontal), the rows from the scroll anchor (vertical) or the row the
    /// selection is on (expandable), and cell 0 leads only some of those.
    /// Asking the layout where its own slot 0 lands is what keeps this one
    /// rule right for all three. Hoisted out of the per-slot lookup because it
    /// is invariant across a whole label repaint.
    static func keySlotShift(
        leadCellIsUnkeyed: Bool,
        indexForSlot: (Int) -> Int?,
    ) -> Int {
        leadCellIsUnkeyed && indexForSlot(0) == 0 ? 1 : 0
    }

    /// The absolute index the `slot`-th KEY addresses: cell 0 is excluded from
    /// the key row when it is the unkeyed §34 literal (USER 2026-09-09 — it is
    /// what the user is already typing, not an offer to pick), and the keys
    /// shift by one wherever the row they number begins with it.
    ///
    /// The ninth key falls off the shifted row — a layout answers nil past
    /// what the row holds — so a row that leads with the literal keys eight
    /// candidates and `;` / `9` picks nothing there (USER 2026-09-09: the page
    /// is not enlarged to keep nine).
    static func candidateIndex(
        forKeySlot slot: Int,
        leadCellIsUnkeyed: Bool,
        indexForSlot: (Int) -> Int?,
    ) -> Int? {
        guard (0 ..< HorizontalPageLayout.pageSize).contains(slot) else { return nil }
        let shift = keySlotShift(leadCellIsUnkeyed: leadCellIsUnkeyed, indexForSlot: indexForSlot)
        return indexForSlot(slot + shift)
    }

    /// Every form a slot can be drawn in — each slot under each set — so the
    /// column is measured against all of them and does not change width when
    /// the user switches tone scheme (`CandidateMetrics.indexWidth`).
    /// Measured rather than counted, since a letter sets wider than a digit.
    static let widestLabelForms: [String] = (0 ..< HorizontalPageLayout.pageSize).flatMap { slot in
        CandidateSlotKeySet.allCases.map { text(forSlot: slot, keySet: $0) }
    }
}
