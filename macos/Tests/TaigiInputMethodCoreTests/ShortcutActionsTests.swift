// The shortcut registry itself: the action list, its stored names, and the
// one-chord-one-action rule.

import KeyboardShortcuts
@testable import TaigiInputMethodCore
import XCTest

/// The shortcut registry: which actions exist, what a fresh install has bound,
/// and the rule that keeps one chord meaning one thing.
final class ShortcutActionsTests: XCTestCase {
    func testEveryAction_hasItsOwnStorageName() {
        let names = ShortcutAction.allCases.map(\.name.rawValue)

        XCTAssertEqual(Set(names).count, names.count, "two actions share a storage name: \(names)")
    }

    func testEveryAction_hasItsOwnLabel() {
        let labels = ShortcutAction.allCases.map(\.label)

        XCTAssertEqual(Set(labels).count, labels.count, "two recorder rows read the same: \(labels)")
    }

    /// A user who never opens the recorder keeps the chord PR5 shipped
    /// hardcoded. `initial:` is what carries it, and it is the reason the menu
    /// item can stop declaring a key equivalent of its own.
    func testOpenSettings_startsBoundToControlShiftComma() {
        XCTAssertEqual(
            KeyboardShortcuts.Name.openSettings.defaultShortcut,
            KeyboardShortcuts.Shortcut(.comma, modifiers: [.control, .shift]),
        )
    }

    /// Only 開啟設定 arrives bound: a default chord is a key equivalent taken
    /// from every host application for as long as this input source is
    /// selected, so the rest stay unset until the user asks for them.
    func testOtherActions_startUnbound() {
        for action in ShortcutAction.allCases where action != .openSettings {
            XCTAssertNil(
                action.name.defaultShortcut,
                "\(action.label) claims a chord nobody asked for",
            )
        }
    }

    // MARK: - Conflict resolution

    private let chord = KeyboardShortcuts.Shortcut(.k, modifiers: [.control, .option])

    func testRecordingAChordAnotherActionHolds_reportsThatAction() {
        let held: [ShortcutAction: KeyboardShortcuts.Shortcut] = [
            .toggleRomanization: chord,
            .toggleBothScripts: chord,
        ]

        let losers = ShortcutConflicts.conflictingActions(with: .toggleRomanization) { held[$0] }

        XCTAssertEqual(losers, [.toggleBothScripts])
    }

    func testRecordingAUniqueChord_reportsNoConflict() {
        let held: [ShortcutAction: KeyboardShortcuts.Shortcut] = [
            .toggleRomanization: chord,
            .toggleBothScripts: .init(.j, modifiers: [.control, .option]),
        ]

        let losers = ShortcutConflicts.conflictingActions(with: .toggleRomanization) { held[$0] }

        XCTAssertTrue(losers.isEmpty)
    }

    /// Clearing a recorder must not make every other cleared action a
    /// conflict — `nil` is not a chord two actions can share.
    func testClearingAChord_reportsNoConflict() {
        let losers = ShortcutConflicts.conflictingActions(with: .toggleRomanization) { _ in nil }

        XCTAssertTrue(losers.isEmpty)
    }

    /// End to end through the library's own storage, which is what the
    /// recorder rows call: the policy above decides, and `resolve` is what
    /// actually empties the other row.
    @MainActor
    func testResolvingAConflict_clearsTheOtherActionsChord() {
        let saved = ShortcutAction.allCases.map { ($0, KeyboardShortcuts.getShortcut(for: $0.name)) }
        defer {
            for (action, shortcut) in saved {
                KeyboardShortcuts.setShortcut(shortcut, for: action.name)
            }
        }
        KeyboardShortcuts.setShortcut(chord, for: ShortcutAction.toggleBothScripts.name)
        KeyboardShortcuts.setShortcut(chord, for: ShortcutAction.toggleRomanization.name)

        ShortcutConflicts.resolve(after: .toggleRomanization)

        XCTAssertEqual(KeyboardShortcuts.getShortcut(for: .toggleRomanization), chord)
        XCTAssertNil(
            KeyboardShortcuts.getShortcut(for: .toggleBothScripts),
            "both actions still hold the same chord — one press would run both",
        )
    }
}
