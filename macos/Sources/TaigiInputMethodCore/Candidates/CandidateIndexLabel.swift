// Which key picks which candidate, and how that key is written in the window.

import Foundation

/// Which key picks a candidate right now.
///
/// A digit is a TL/POJ tone marker first: while the raw buffer ends in a
/// letter, `2` tones the syllable being typed and only the slot keys select.
/// Once no tone can follow — after `tai5`, or the hyphen that starts a new
/// syllable — the bare digit selects too (`ComposingKeyIntent.canTypeToneDigit`).
///
/// The second way to `bare` is the selection latch: after `↓`, a bare digit
/// picks whatever the buffer looks like, because someone who types no tones at
/// all would otherwise never leave `keyed`
/// (`ComposingKeyIntent.selectionLatch(after:wasLatched:)`).
///
/// The window draws whichever key is live, so the hint can never name a key
/// that would do something else — and where several are live it draws the
/// most direct: the bare digit names all nine slots, so it wins over the set
/// while it can pick. McBopomofo writes the same distinction the same way,
/// folding the modifier into the label text for the states that need it
/// (`references/McBopomofo/Source/InputMethodController.swift:869-877`:
/// `{ "⇧ " + $0 }`) rather than styling the digit — which keeps the intensity
/// of the text free to mean "selected", as it does here.
enum CandidateSlotKeyStyle: Equatable, Sendable {
    /// A bare `1`…`9` picks — no tone can follow the buffer as it stands, or
    /// the user has said with `↓` that they are choosing rather than typing.
    case bare
    /// Only the slot keys pick, so each slot is drawn under the key its set
    /// gives it.
    case keyed(CandidateSlotKeySet)
}

/// The key drawn beside a candidate — the one that picks it.
///
/// One place rather than a literal per layout, because the labels are a
/// contract with the key handler: a slot's label is the key
/// `CandidateSlotKeySet.slot(forKey:heldWith:)` maps to that slot, or the bare
/// digit `ComposingKeyIntent.directSelectionSlot` does, and every layout
/// answers those slots from the positions it draws
/// (`CandidatePresenter.candidateIndex(forSlot:)`).
///
/// `0` is deliberately absent, unlike upstream MacishType's `"1234567890"`
/// (`MacishCandidateWindow/CandidateWindow.swift:49`): `⌃0` is not bound and a
/// bare `0` is document text, so drawing a `0` would name a key that does
/// nothing.
enum CandidateIndexLabel {
    /// The key for a zero-based position, or `""` past the ninth — a position
    /// no key names. The cell still reserves the slot, so a tenth candidate
    /// lines up with the nine above it.
    static func text(forSlot slot: Int, style: CandidateSlotKeyStyle) -> String {
        guard (0 ..< HorizontalPageLayout.pageSize).contains(slot) else { return "" }
        let digit = String(slot + 1)
        switch style {
        case .bare: return digit
        // Where the set has no key for the slot, the fixed `⇧` digit is the
        // key that picks it (`ComposingKeyIntent.shiftedDigitSlot`).
        case let .keyed(keySet): return keySet.label(forSlot: slot) ?? "⇧" + digit
        }
    }

    /// Every form a slot can be drawn in — each slot under each style — so
    /// the column is measured against all of them and does not change width
    /// under the user as the live key changes, or when they choose another
    /// set (`CandidateMetrics.indexWidth`). Measured rather than counted,
    /// since `⌥` sets wider than `⌃` and a letter wider than either.
    static let widestLabelForms: [String] = {
        let styles = [CandidateSlotKeyStyle.bare] + CandidateSlotKeySet.allCases.map(CandidateSlotKeyStyle.keyed)
        return (0 ..< HorizontalPageLayout.pageSize).flatMap { slot in
            styles.map { text(forSlot: slot, style: $0) }
        }
    }()
}
