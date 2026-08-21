// The one-time move of the 漢羅對調 default from ⌃⌘H to the bare backtick.

import KeyboardShortcuts
@testable import TaigiInputMethodCore
import XCTest

/// The migration runs against injected closures so no case touches the real
/// shortcut domain of whoever is running the tests — the library stores in
/// `UserDefaults.standard` with no suite injection.
@MainActor
final class ShortcutDefaultMigrationTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard
    private var stored: [ShortcutAction: KeyboardShortcuts.Shortcut] = [:]

    private static let oldDefault = KeyboardShortcuts.Shortcut(.h, modifiers: [.control, .command])
    private static let newDefault = KeyboardShortcuts.Shortcut(.backtick)

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "ShortcutDefaultMigrationTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        stored = [:]
    }

    override func tearDown() {
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    private func migrate() {
        ShortcutDefaultMigration.run(
            userDefaults: userDefaults,
            shortcutFor: { self.stored[$0] },
            setShortcut: { self.stored[$1] = $0 },
        )
    }

    func testAnInstallOnTheOldDefault_movesToTheBacktick() {
        stored[.toggleTranslateSwapped] = Self.oldDefault

        migrate()

        XCTAssertEqual(stored[.toggleTranslateSwapped], Self.newDefault)
    }

    func testARecordedChord_isKept() {
        let recorded = KeyboardShortcuts.Shortcut(.k, modifiers: [.control, .option])
        stored[.toggleTranslateSwapped] = recorded

        migrate()

        XCTAssertEqual(stored[.toggleTranslateSwapped], recorded)
    }

    /// A fresh install has nothing stored — `initial:` already seeds the new
    /// default there, and the migration must not write anything of its own.
    func testAFreshInstall_isLeftToTheInitial() {
        migrate()

        XCTAssertNil(stored[.toggleTranslateSwapped])
    }

    /// The flag is written even when nothing migrated, so no later launch
    /// revisits the question.
    func testTheFlag_isWrittenOnTheFirstRunEitherWay() {
        migrate()

        XCTAssertTrue(userDefaults.bool(forKey: "didMoveTranslateSwappedDefaultToBacktick"))
    }

    /// The reason the flag exists: a user who deliberately records the OLD
    /// default after migrating must keep it on the next launch.
    func testTheOldDefaultRecordedAfterMigration_isNotClobbered() {
        migrate()
        stored[.toggleTranslateSwapped] = Self.oldDefault

        migrate()

        XCTAssertEqual(stored[.toggleTranslateSwapped], Self.oldDefault)
    }
}
