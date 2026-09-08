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
/// - The candidate-slot keys, which are nine names for nine slots rather than
///   one key, and follow from the tone scheme (`ToneInputScheme.slotKeySet`)
///   instead.
///
/// Defaults follow the system Zhuyin input method's candidate window wherever
/// the romanization allows it, so a user arriving from that keyboard finds
/// their muscle memory intact. Where Zhuyin has nothing to inherit, they follow
/// another Taiwanese input method or stay in the Return family — each case says
/// which below. No action ships unbound (USER 2026-08-21).
enum ComposingAction: String, CaseIterable, Sendable {
    /// Moves the highlight one candidate along. ⇥ by default — the ⇥
    /// McBopomofo walks its candidates with, and the forward half of the pair
    /// `previousCandidate` below already held. The arrows do this too, and
    /// always will.
    ///
    /// It was Space until 2026-08-25, the way Zhuyin's space bar walks the
    /// candidate window. Space went to `commitAlternateScript`, which has no
    /// second key that could do its job; walking always had the arrows.
    case nextCandidate
    /// Moves the highlight one candidate back. ⇧⇥, the reverse of the ⇥
    /// McBopomofo walks its candidates with
    /// (`references/McBopomofo/Source/KeyHandler.mm:817-870`) — `←` does it too,
    /// and always will.
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
    /// Commits the highlighted candidate in the OTHER script — romanization
    /// while the settings lead with hanji, and hanji while they lead with
    /// romanization — without moving the settings.
    ///
    /// Space, which this takes from `nextCandidate`. Space was exactly
    /// redundant there: every layout already reads an arrow as one step along
    /// the list (`HorizontalPageLayout.target(for:from:)` maps `.right` and
    /// `.nextCandidate` to the same move; `VerticalCandidatePanel.navigate`
    /// maps `.down` and `.nextCandidate` to the same move), so the most
    /// reachable key on the keyboard was spending itself on a duplicate.
    ///
    /// What it buys is the 漢羅 sentence. `我ê名` in 漢字 mode used to be
    /// Return, then `` ` `` to flip the whole input method to romanization,
    /// Return, `` ` `` to flip back, then Return — three actions for the one
    /// romanized word. It is now Return / Space / Return.
    ///
    /// Mode-relative and self-inverting, which is what makes it one key rather
    /// than the pair of pinned 直接送出漢字 / 直接送出羅馬字 actions removed in
    /// #609: there is nothing to remember about which key writes which script.
    /// rime-phah-taibun binds the same gesture the same way, on `\`
    /// (`references/rime-phah-taibun/lua/phah_taibun_commit.lua:264`).
    case commitAlternateScript

    /// The chord a fresh install has on this action. Every action has one: a
    /// row the user has never touched prints a key rather than a blank
    /// (USER 2026-08-21).
    ///
    /// Read from a table built once rather than rebuilt per read: the bindings
    /// are resolved on every keystroke and every menu draw, and each entry runs
    /// the bindability gate to build.
    var defaultChord: ComposingKeyChord { Self.defaultChords[self] ?? freshChord }

    private static let defaultChords: [ComposingAction: ComposingKeyChord] =
        Dictionary(uniqueKeysWithValues: allCases.map { ($0, $0.freshChord) })

    /// The table's own source, kept a `switch` so a new case cannot compile
    /// without one.
    private var freshChord: ComposingKeyChord {
        switch self {
        // ⇥ and ⇧⇥, a pair: the reverse half was already bound here, so the
        // forward half costs the user nothing new to learn. Space went to
        // `commitAlternateScript`, and the arrows walk the list in every layout
        // whatever this row says.
        case .nextCandidate: Self.chord("\t")
        case .previousCandidate: Self.chord("\t", modifiers: .shift)
        case .pageForward: Self.chord("]")
        case .pageBackward: Self.chord("[")
        case .confirmHighlighted: Self.chord("\r")
        case .commitLiteral: Self.chord("\r", modifiers: .shift)
        case .commitAlternateScript: Self.chord(" ")
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
             .confirmHighlighted, .commitAlternateScript: true
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
        case .confirmHighlighted: .commitHighlightedCandidate
        case .commitLiteral: .commit
        case .commitAlternateScript: .commitAlternateScript
        }
    }

    /// The roster split into the groups the settings pane and the input-source
    /// menu both draw: the keys that move through the candidates, and the keys
    /// that end the composition.
    ///
    /// Written out rather than derived from `allCases` order so that adding a
    /// case has to say which group it belongs to — `ComposingActionTests` pins
    /// that every case appears exactly once.
    static let groups: [[ComposingAction]] = [
        [.nextCandidate, .previousCandidate, .pageForward, .pageBackward],
        [.confirmHighlighted, .commitLiteral, .commitAlternateScript],
    ]

    /// Actions that must always be reachable, whatever else the user rebinds.
    ///
    /// Between them these two are the only way to end a composition into the
    /// document: one takes the candidate, the other takes what was typed. A
    /// roster that let both go unbound would leave a user with a composition
    /// they can only cancel.
    static let alwaysBound: Set<ComposingAction> = [.confirmHighlighted, .commitLiteral]

    /// The settings key this action's chord is stored under.
    var settingsKeyName: String { Self.settingsKeyName(rawValue: rawValue) }

    /// The same key for a raw value the roster no longer has a case for — what
    /// `RetiredSettingsCleanup` sweeps. Composed here rather than written out
    /// there, so the namespace has one owner and a tombstone cannot be left
    /// behind by a rename.
    static func settingsKeyName(rawValue: String) -> String { "composingShortcut.\(rawValue)" }

    /// The recorder row's label, under the active display language.
    @MainActor
    func label(_ language: DisplayLanguageStore) -> String {
        switch self {
        case .nextCandidate: language.string(.desktopActionNextCandidate)
        case .previousCandidate: language.string(.desktopActionPreviousCandidate)
        case .pageForward: language.string(.desktopActionPageForward)
        case .pageBackward: language.string(.desktopActionPageBackward)
        case .confirmHighlighted: language.string(.desktopActionConfirmHighlighted)
        case .commitLiteral: language.string(.desktopActionCommitLiteral)
        case .commitAlternateScript: language.string(.desktopActionCommitAlternateScript)
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
