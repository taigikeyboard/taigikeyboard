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

    /// Every form a slot can be drawn in — each slot under each set — so the
    /// column is measured against all of them and does not change width when
    /// the user switches tone scheme (`CandidateMetrics.indexWidth`).
    /// Measured rather than counted, since a letter sets wider than a digit.
    static let widestLabelForms: [String] = (0 ..< HorizontalPageLayout.pageSize).flatMap { slot in
        CandidateSlotKeySet.allCases.map { text(forSlot: slot, keySet: $0) }
    }
}
