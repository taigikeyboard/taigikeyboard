// What the three shortcut tiers ship with, and how a pane gets back to it.

import KeyboardShortcuts
@testable import TaigiInputMethodCore
import XCTest

/// The 快速齒 pane's Restore Defaults button hands the user the shipped state
/// and runs no conflict resolution afterwards, which is only safe while the
/// three tiers' defaults hold no chord in common. That property is pinned here
/// rather than defended at run time: a resolver would quietly empty one of the
/// rows the button had just restored, where a failing case names the collision.
@MainActor
final class ShortcutDefaultsTests: XCTestCase {
    /// What the machine running these tests had recorded. The library writes to
    /// `UserDefaults.standard` and takes no suite, so a case that touches the
    /// global registry has to hand the user their own shortcuts back — resetting
    /// to the shipped defaults would leave this suite having quietly rebound
    /// them (`TaigiInputControllerMenuTests` does the same).
    private var savedShortcuts: [ShortcutAction: KeyboardShortcuts.Shortcut?] = [:]

    override func setUpWithError() throws {
        try super.setUpWithError()
        savedShortcuts = Dictionary(
            uniqueKeysWithValues: ShortcutAction.allCases.map {
                ($0, KeyboardShortcuts.getShortcut(for: $0.name))
            },
        )
    }

    override func tearDown() {
        for (action, shortcut) in savedShortcuts {
            KeyboardShortcuts.setShortcut(shortcut, for: action.name)
        }
        super.tearDown()
    }

    /// A global default, read as the composing tier would see the key.
    private func chord(of action: ShortcutAction) throws -> ComposingKeyChord {
        let shortcut = try XCTUnwrap(action.defaultShortcut, "\(action) ships unbound")
        return try XCTUnwrap(
            ShortcutConflicts.composingChord(occupiedBy: shortcut),
            "\(action)'s default has no composing-tier spelling",
        )
    }

    /// The nine chords the slot tier claims under the modifier it ships with.
    private func slotChords() throws -> [ComposingKeyChord] {
        let modifier = SettingsStore.Keys.candidateSlotModifier.defaultValue
        return try (1 ... 9).map { digit in
            try ComposingKeyChord.make(key: String(digit), modifiers: modifier.flag).get()
        }
    }

    func testShippedDefaults_holdNoChordInCommon() throws {
        let composing = ComposingAction.allCases.map(\.defaultChord)
        let global = try ShortcutAction.allCases.map { try chord(of: $0) }
        let all = composing + global + (try slotChords())

        XCTAssertEqual(
            Set(all).count,
            all.count,
            "two tiers ship the same chord: \(all.map(ComposingKeyDisplay.text(for:)))",
        )
    }

    /// The global half of the button. `KeyboardShortcuts.reset` writes each
    /// name's initial shortcut back — unlike the composing half, which removes
    /// its stored values — so this pins the outcome the two halves share.
    func testResettingGlobalActions_putsEveryRowBackOnItsDefault() {
        let overrides: [KeyboardShortcuts.Key] = [.f13, .f14, .f15]
        // An action added without an override here would otherwise be zipped
        // away, and the case would pass without ever having moved that row.
        XCTAssertEqual(overrides.count, ShortcutAction.allCases.count)
        for (action, key) in zip(ShortcutAction.allCases, overrides) {
            KeyboardShortcuts.setShortcut(.init(key, modifiers: [.control, .option]), for: action.name)
        }

        KeyboardShortcuts.reset(ShortcutAction.allCases.map(\.name))

        for action in ShortcutAction.allCases {
            XCTAssertEqual(
                KeyboardShortcuts.getShortcut(for: action.name),
                action.defaultShortcut,
                "\(action) did not come back",
            )
        }
    }
}
