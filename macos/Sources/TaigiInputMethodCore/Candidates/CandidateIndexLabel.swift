// Which key picks which candidate, and how that key is written in the window.

import Foundation

/// Which key picks a candidate right now.
///
/// A digit is a TL/POJ tone marker first: while the raw buffer ends in a
/// letter, `2` tones the syllable being typed and only the chord selects. Once
/// no tone can follow — after `tai5`, or the hyphen that starts a new syllable
/// — the bare digit selects instead (`ComposingKeyIntent.canTypeToneDigit`).
///
/// The window draws whichever key is live, so the hint can never name a key
/// that would do something else. McBopomofo writes the same distinction the
/// same way, folding the modifier into the label text for the states that need
/// it (`references/McBopomofo/Source/InputMethodController.swift:869-877`:
/// `{ "⇧ " + $0 }`) rather than styling the digit — which keeps the intensity
/// of the text free to mean "selected", as it does here.
enum CandidateSlotKeyStyle: Equatable, Sendable {
    /// A bare `1`…`9` picks — nothing else can follow a tone digit.
    case bare
    /// Only the chord picks, so the digit is drawn under its modifier.
    case chorded(CandidateSlotModifier)
}

/// The key drawn beside a candidate — the one that picks it.
///
/// One place rather than a literal per layout, because the digits are a
/// contract with the key handler: `1`…`9` map to slots 0…8 on both selection
/// tiers (`ComposingKeyIntent.directSelectionSlot`, and the bare-digit tier),
/// and every layout answers those slots from the positions it draws
/// (`CandidatePresenter.candidateIndex(forSlot:)`).
///
/// `0` is deliberately absent, unlike upstream MacishType's `"1234567890"`
/// (`MacishCandidateWindow/CandidateWindow.swift:49`): `⌃0` is not bound and a
/// bare `0` is document text, so drawing a `0` would name a key that does
/// nothing.
enum CandidateIndexLabel {
    /// The key for a zero-based position, or `""` past the ninth — a position
    /// no digit names. The cell still reserves the slot, so a tenth candidate
    /// lines up with the nine above it.
    static func text(forSlot slot: Int, style: CandidateSlotKeyStyle) -> String {
        guard (0 ..< HorizontalPageLayout.pageSize).contains(slot) else { return "" }
        let digit = String(slot + 1)
        switch style {
        case .bare: return digit
        case let .chorded(modifier): return modifier.symbol + digit
        }
    }

    /// The forms the slot has to be wide enough for — a bare digit and every
    /// modifier's, against the widest digit. Measured rather than counted,
    /// since `⌥` sets wider than `⌃`; the slot takes the widest so the column
    /// does not change width under the user as the live key changes, or when
    /// they rebind the modifier (`CandidateMetrics.indexWidth`).
    static let widestLabelForms: [String] =
        ["9"] + CandidateSlotModifier.allCases.map { $0.symbol + "9" }
}
