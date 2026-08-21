// The composing actions a user can put on a key of their own choosing.

import AppKit

/// One thing the input method does while a composition is running, as the
/// settings window names it.
///
/// Only the actions whose key is the user's to choose are here. Two other tiers
/// exist and are deliberately absent:
///
/// - The fixed navigation contract — the arrows, Page Up/Page Down, Escape and
///   Backspace. A user who mis-bound one would lose the way to move through
///   candidates, or the way out of a composition, with the composition still on
///   screen and no key left to fix it.
/// - The candidate-slot chord, which is a modifier plus `1`…`9` rather than one
///   key, and so is chosen as a modifier (`CandidateSlotModifier`) instead.
///
/// Defaults follow the system Zhuyin input method's candidate window wherever
/// the romanization allows it, so a user arriving from that keyboard finds
/// their muscle memory intact.
enum ComposingAction: String, CaseIterable, Sendable {
    /// Moves the highlight one candidate along. Space by default, the way
    /// Zhuyin's space bar walks the candidate window — the arrows do this too,
    /// and always will.
    case nextCandidate
    /// Moves the highlight one candidate back. Unbound by default: `←` already
    /// does it, and Zhuyin has no second key for it.
    case previousCandidate
    /// Shows the next page of candidates. `]`, as in the system candidate
    /// window.
    case pageForward
    /// Shows the previous page. `[`.
    case pageBackward
    /// Commits the highlighted candidate, as the settings say to write it.
    /// Return, as in Zhuyin.
    case confirmHighlighted
    /// Commits the romanization exactly as typed, ignoring the highlight.
    /// ⇧Return — the escape hatch that keeps what was typed reachable, which
    /// is why it is one of the two actions a binding may never leave unbound.
    case commitLiteral
    /// Commits the highlighted candidate as Hanji, whatever the 漢羅 setting
    /// says. Unbound by default: it eats a chord, and Zhuyin has no equivalent
    /// to inherit a default from.
    case commitHanji
    /// Commits the highlighted candidate as romanization, same terms.
    case commitRomanization

    /// The chord a fresh install has on this action.
    var defaultChord: ComposingKeyChord? {
        switch self {
        case .nextCandidate: Self.chord(" ")
        case .pageForward: Self.chord("]")
        case .pageBackward: Self.chord("[")
        case .confirmHighlighted: Self.chord("\r")
        case .commitLiteral: Self.chord("\r", modifiers: .shift)
        case .previousCandidate, .commitHanji, .commitRomanization: nil
        }
    }

    /// Whether this action needs candidates on screen to mean anything.
    ///
    /// A chord whose action does not apply is not consumed: it falls through to
    /// whatever the key would otherwise be, so `]` still types a bracket when
    /// there is no page to turn, and Return still ends a composition with no
    /// bar up.
    var requiresCandidates: Bool {
        switch self {
        case .commitLiteral: false
        case .nextCandidate, .previousCandidate, .pageForward, .pageBackward,
             .confirmHighlighted, .commitHanji, .commitRomanization: true
        }
    }

    /// What this action does, once its chord has been matched and its state
    /// checked.
    var intent: ComposingKeyIntent {
        switch self {
        case .nextCandidate: .navigate(.nextCandidate)
        case .previousCandidate: .navigate(.previousCandidate)
        case .pageForward: .navigate(.pageDown)
        case .pageBackward: .navigate(.pageUp)
        case .confirmHighlighted: .commitHighlightedCandidate(.settings)
        case .commitHanji: .commitHighlightedCandidate(.hanji)
        case .commitRomanization: .commitHighlightedCandidate(.romanization)
        case .commitLiteral: .commit
        }
    }

    /// Actions that must always be reachable, whatever else the user rebinds.
    ///
    /// Between them these two are the only way to end a composition into the
    /// document: one takes the candidate, the other takes what was typed. A
    /// roster that let both go unbound would leave a user with a composition
    /// they can only cancel.
    static let alwaysBound: Set<ComposingAction> = [.confirmHighlighted, .commitLiteral]

    /// The settings key this action's chord is stored under.
    var settingsKeyName: String { "composingShortcut.\(rawValue)" }

    /// The recorder row's label, under the active display language.
    @MainActor
    func label(_ language: DisplayLanguageStore) -> String {
        switch self {
        case .nextCandidate: language.string(.macosActionNextCandidate)
        case .previousCandidate: language.string(.macosActionPreviousCandidate)
        case .pageForward: language.string(.macosActionPageForward)
        case .pageBackward: language.string(.macosActionPageBackward)
        case .confirmHighlighted: language.string(.macosActionConfirmHighlighted)
        case .commitLiteral: language.string(.macosActionCommitLiteral)
        case .commitHanji: language.string(.macosActionCommitHanji)
        case .commitRomanization: language.string(.macosActionCommitRomanization)
        }
    }

    /// A default chord, built through the same gate a recorded one goes
    /// through. Trapping rather than falling back to nil: a default the gate
    /// refuses is a mistake in this file, and shipping it as "unbound" would
    /// hide it.
    private static func chord(_ key: String, modifiers: NSEvent.ModifierFlags = []) -> ComposingKeyChord {
        guard case let .success(chord) = ComposingKeyChord.make(key: key, modifiers: modifiers) else {
            preconditionFailure("default chord for \(key) is not bindable")
        }
        return chord
    }
}
