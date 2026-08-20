// The shortcut registry itself: the action list, its stored names, and the
// one-chord-one-action rule.

import KeyboardShortcuts
@testable import TaigiInputMethodCore
import XCTest

/// The shortcut registry: which actions exist, what a fresh install has bound,
/// and the rule that keeps one chord meaning one thing.
@MainActor
final class ShortcutActionsTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "ShortcutActionsTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
    }

    override func tearDown() {
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    private func labels(_ language: DisplayLanguage = .hanji) -> [String] {
        let store = TestFixtures.makeDisplayLanguageStore(language, userDefaults: userDefaults)
        return ShortcutAction.allCases.map { $0.label(store) }
    }

    func testEveryAction_hasItsOwnStorageName() {
        let names = ShortcutAction.allCases.map(\.name.rawValue)

        XCTAssertEqual(Set(names).count, names.count, "two actions share a storage name: \(names)")
    }

    func testEveryAction_hasItsOwnLabel() {
        let rows = labels()

        XCTAssertEqual(Set(rows).count, rows.count, "two recorder rows read the same: \(rows)")
    }

    /// Every row is authored whole rather than composed from the settings label it flips: those
    /// labels are verb phrases, and a "toggle X" frame around one doubles the verb in ja and en.
    func testEveryAction_readsAsAWholePhraseInEveryLanguage() {
        XCTAssertEqual(
            labels(),
            ["開啟設定", "切換 台羅/白話字", "切換 漢羅對調"],
        )
        XCTAssertEqual(
            labels(.japanese),
            [
                "設定を開く",
                "ローマ字体系を切り替える",
                "漢字とローマ字の入れ替えを切り替える",
            ],
        )
        XCTAssertEqual(labels(.english).first, "Open Settings")
        // The trap this replaced: composing a row from the setting's own label produced a doubled
        // verb — "括弧で併記を切り替える", "Toggle Annotate in Brackets".
        XCTAssertFalse(labels(.japanese).contains { $0.contains("併記を切り替えるを") })
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
                "\(action.name.rawValue) claims a chord nobody asked for",
            )
        }
    }

    // MARK: - Conflict resolution

    private let chord = KeyboardShortcuts.Shortcut(.k, modifiers: [.control, .option])

    func testRecordingAChordAnotherActionHolds_reportsThatAction() {
        let held: [ShortcutAction: KeyboardShortcuts.Shortcut] = [
            .toggleRomanization: chord,
            .toggleTranslateSwapped: chord,
        ]

        let losers = ShortcutConflicts.conflictingActions(with: .toggleRomanization) { held[$0] }

        XCTAssertEqual(losers, [.toggleTranslateSwapped])
    }

    func testRecordingAUniqueChord_reportsNoConflict() {
        let held: [ShortcutAction: KeyboardShortcuts.Shortcut] = [
            .toggleRomanization: chord,
            .toggleTranslateSwapped: .init(.j, modifiers: [.control, .option]),
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
        KeyboardShortcuts.setShortcut(chord, for: ShortcutAction.toggleTranslateSwapped.name)
        KeyboardShortcuts.setShortcut(chord, for: ShortcutAction.toggleRomanization.name)

        ShortcutConflicts.resolve(after: .toggleRomanization)

        XCTAssertEqual(KeyboardShortcuts.getShortcut(for: .toggleRomanization), chord)
        XCTAssertNil(
            KeyboardShortcuts.getShortcut(for: .toggleTranslateSwapped),
            "both actions still hold the same chord — one press would run both",
        )
    }
}
