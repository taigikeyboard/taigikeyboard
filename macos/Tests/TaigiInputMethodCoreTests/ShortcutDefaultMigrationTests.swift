// The one-time moves of a shortcut default between builds.

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

    // MARK: - The second migration (⌃⌘R → ⌃⌘C, 2026-08-25)

    private static let oldRomanizationDefault =
        KeyboardShortcuts.Shortcut(.r, modifiers: [.control, .command])

    func testAnInstallOnTheOldRomanizationDefault_movesToControlCommandC() {
        stored[.toggleRomanization] = Self.oldRomanizationDefault

        migrate()

        XCTAssertEqual(
            stored[.toggleRomanization],
            KeyboardShortcuts.Shortcut(.c, modifiers: [.control, .command]),
        )
    }

    func testARecordedRomanizationChord_isKept() {
        let recorded = KeyboardShortcuts.Shortcut(.f13, modifiers: [.control, .option])
        stored[.toggleRomanization] = recorded

        migrate()

        XCTAssertEqual(stored[.toggleRomanization], recorded)
    }

    /// The reason each migration carries its OWN flag. An install that skipped
    /// the build carrying the first one arrives with neither done; a shared
    /// flag would let whichever ran first mark the other as already handled,
    /// and that install would keep an old default forever.
    func testBothMigrations_runOnAnInstallThatMissedTheFirstBuild() {
        stored[.toggleTranslateSwapped] = Self.oldDefault
        stored[.toggleRomanization] = Self.oldRomanizationDefault

        migrate()

        XCTAssertEqual(stored[.toggleTranslateSwapped], Self.newDefault)
        XCTAssertEqual(
            stored[.toggleRomanization],
            KeyboardShortcuts.Shortcut(.c, modifiers: [.control, .command]),
        )
    }

    /// And one already done does not hold the other back.
    func testTheSecondMigration_runsWithTheFirstAlreadyFlagged() {
        userDefaults.set(true, forKey: "didMoveTranslateSwappedDefaultToBacktick")
        stored[.toggleRomanization] = Self.oldRomanizationDefault

        migrate()

        XCTAssertEqual(
            stored[.toggleRomanization],
            KeyboardShortcuts.Shortcut(.c, modifiers: [.control, .command]),
        )
    }

    func testTheOldRomanizationDefaultRecordedAfterMigration_isNotClobbered() {
        migrate()
        stored[.toggleRomanization] = Self.oldRomanizationDefault

        migrate()

        XCTAssertEqual(stored[.toggleRomanization], Self.oldRomanizationDefault)
    }
}
