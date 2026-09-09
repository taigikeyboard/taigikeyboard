// What a key means while the symbol picker is up, and which level the picker is on. Pure, no IMK.

import AppKit

/// Which list the symbol picker is showing: the three categories, or the
/// symbols of one of them. `nil` on the controller means the picker is closed.
///
/// Only the level lives here. The selection is the window's, as it is for the
/// candidate bar (`CandidatePresenter`): a second copy could only disagree.
enum SymbolPickerLevel: Equatable, Sendable {
    case categories
    case items(SymbolCategoryID)
}

/// The picker's reading of one key event, decided before any window is asked
/// anything. Its own table rather than a branch of `ComposingKeyIntent`: that
/// classifier is the contract of a COMPOSITION, and the picker runs with none
/// — its keys pick from a list the engine never fetched.
enum SymbolPickerIntent: Equatable, Sendable {
    /// Take the picker down and swallow the key. Escape, from either level
    /// (USER 2026-09-09: 「Fork B 依照你的建議處理」 — one key out, never two).
    case close
    /// Move the selection the way the window's layout reads `direction`.
    case navigate(CandidateNavigation)
    /// Pick the cell the `slot`-th selection key addresses on the visible
    /// page — the same keys that pick a candidate (`CandidateSlotKeySet`).
    case pickSlot(Int)
    /// Pick the highlighted cell.
    case confirm
    /// Take the picker down and let the key go on to do its job: the picker
    /// is a list to pick from, not a mode, so a letter typed over it starts
    /// the composition it would have started anyway.
    case closeAndPassThrough

    /// Classifies `key` for a picker that is on screen.
    ///
    /// Reads the same rules the candidate bar does, in the same order — the
    /// fixed navigation keys, then the slot keys, then whatever the user put
    /// on the paging and confirm rows (`ComposingKeyBindings`) — so a user who
    /// moved paging to ⌥Return pages the picker with it too. Only the
    /// bar-specific outcomes differ: there is no other script to commit, so
    /// the 漢羅 key confirms like Return, and the literal-commit key has no
    /// literal to write, so it falls through.
    static func intent(for key: KeyEventSnapshot, bindings: ComposingKeyBindings) -> SymbolPickerIntent {
        let modifiers = key.modifiers.intersection(.deviceIndependentFlagsMask)
        let isPlainKey = modifiers.isDisjoint(with: ComposingKeyIntent.hostChords)

        if ComposingKeyIntent.isPlainEscape(key) {
            return .close
        }
        if isPlainKey, !modifiers.contains(.shift), let navigation = key.navigationKey {
            return .navigate(CandidateNavigation(navigation))
        }
        if let slot = bindings.slotKeySet.slot(for: key) {
            return .pickSlot(slot)
        }
        if let action = bindings.action(for: key) {
            switch action {
            case .nextCandidate: return .navigate(.nextCandidate)
            case .previousCandidate: return .navigate(.previousCandidate)
            case .pageForward: return .navigate(.pageDown)
            case .pageBackward: return .navigate(.pageUp)
            case .confirmHighlighted, .commitAlternateScript: return .confirm
            case .commitLiteral: return .closeAndPassThrough
            }
        }
        return .closeAndPassThrough
    }
}
