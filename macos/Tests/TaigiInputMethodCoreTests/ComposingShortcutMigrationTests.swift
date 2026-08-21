// Carrying the first shipped composing-key settings into the action roster.

@testable import TaigiInputMethodCore
import XCTest

/// The first shape of these settings described a key's job; this one records
/// which key does a job. A user who chose something under the old shape must
/// keep it — and a user who never opened the pane must get the new defaults
/// rather than a reconstruction of the old ones.
@MainActor
final class ComposingShortcutMigrationTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "ComposingShortcutMigrationTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
    }

    override func tearDown() {
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    private func bindings() -> ComposingKeyBindings {
        SettingsStore(userDefaults: userDefaults).composingKeyBindings
    }

    private func chord(_ key: String, _ modifiers: NSEvent.ModifierFlags = []) throws -> ComposingKeyChord {
        try ComposingKeyChord.make(key: key, modifiers: modifiers).get()
    }

    /// The overwhelmingly common case: the pane shipped hours before this
    /// change, so almost nobody stored anything. They get the Zhuyin-parity
    /// defaults, untouched by any migration.
    func testWithNothingStored_theNewDefaultsStand() {
        ComposingShortcutMigration.run(userDefaults: userDefaults)

        XCTAssertEqual(bindings(), .default)
    }

    /// `commitLiteral` was the old default for Return, so a STORED one means
    /// the user opened the pane and chose it. They keep Return on the literal,
    /// and the candidate moves to ⇧Return — the same two keys, swapped.
    func testAStoredReturnLiteralChoice_swapsThePairRatherThanSilencingOne() throws {
        userDefaults.set("commitLiteral", forKey: "returnKeyBehavior")

        ComposingShortcutMigration.run(userDefaults: userDefaults)

        XCTAssertEqual(bindings().chord(for: .commitLiteral), try chord("\r"))
        XCTAssertEqual(bindings().chord(for: .confirmHighlighted), try chord("\r", .shift))
    }

    func testAStoredSpaceCommitChoice_putsTheCommitOnSpace() throws {
        userDefaults.set("confirmHighlighted", forKey: "spaceKeyBehavior")

        ComposingShortcutMigration.run(userDefaults: userDefaults)

        XCTAssertEqual(bindings().chord(for: .confirmHighlighted), try chord(" "))
        XCTAssertNil(
            bindings().chord(for: .nextCandidate),
            "Space cannot both commit and walk — the later row takes it",
        )
    }

    func testStoredPagingOff_leavesBothBracketRowsEmpty() {
        userDefaults.set("disabled", forKey: "bracketPagingBehavior")

        ComposingShortcutMigration.run(userDefaults: userDefaults)

        XCTAssertNil(bindings().chord(for: .pageForward))
        XCTAssertNil(bindings().chord(for: .pageBackward))
    }

    func testStoredTabCyclingOn_putsTabAndShiftTabOnTheWalkRows() throws {
        userDefaults.set("enabled", forKey: "tabCycleBehavior")

        ComposingShortcutMigration.run(userDefaults: userDefaults)

        XCTAssertEqual(bindings().chord(for: .nextCandidate), try chord("\t"))
        XCTAssertEqual(bindings().chord(for: .previousCandidate), try chord("\u{19}"))
    }

    /// The slot modifier is the one old setting that survives as itself, so it
    /// is read straight through rather than migrated.
    func testTheSlotModifier_carriesAcrossUntouched() {
        userDefaults.set("option", forKey: SettingsStore.Keys.candidateSlotModifier.name)

        ComposingShortcutMigration.run(userDefaults: userDefaults)

        XCTAssertEqual(bindings().slotModifier, .option)
    }

    /// The two choices that both want a commit key. Return keeps the literal
    /// and Space takes the candidate, so the user ends with the two commits on
    /// the two keys they chose — and ⇧Return, which the Return migration also
    /// writes, gives way to the later Space choice.
    func testReturnLiteralAndSpaceConfirm_together_landOnTheKeysTheUserChose() throws {
        userDefaults.set("commitLiteral", forKey: "returnKeyBehavior")
        userDefaults.set("confirmHighlighted", forKey: "spaceKeyBehavior")

        ComposingShortcutMigration.run(userDefaults: userDefaults)

        XCTAssertEqual(bindings().chord(for: .commitLiteral), try chord("\r"))
        XCTAssertEqual(bindings().chord(for: .confirmHighlighted), try chord(" "))
    }

    /// Space taking the candidate and Tab walking are independent choices, and
    /// both survive: Space commits, Tab and ⇧Tab move.
    func testSpaceConfirmAndTabCycling_together_keepBoth() throws {
        userDefaults.set("confirmHighlighted", forKey: "spaceKeyBehavior")
        userDefaults.set("enabled", forKey: "tabCycleBehavior")

        ComposingShortcutMigration.run(userDefaults: userDefaults)

        XCTAssertEqual(bindings().chord(for: .confirmHighlighted), try chord(" "))
        XCTAssertEqual(bindings().chord(for: .nextCandidate), try chord("\t"))
        XCTAssertEqual(bindings().chord(for: .previousCandidate), try chord("\u{19}"))
    }

    /// Whatever the old settings were, the two keys that end a composition into
    /// the document must both come out of the migration reachable.
    func testEveryCombinationOfOldChoices_leavesBothCommitsBound() {
        let choices: [(String, [String?])] = [
            ("returnKeyBehavior", [nil, "commitLiteral", "confirmHighlighted"]),
            ("spaceKeyBehavior", [nil, "confirmHighlighted", "nextCandidate"]),
            ("bracketPagingBehavior", [nil, "enabled", "disabled"]),
            ("tabCycleBehavior", [nil, "enabled", "disabled"]),
        ]

        for returnKey in choices[0].1 {
            for spaceKey in choices[1].1 {
                for paging in choices[2].1 {
                    for tab in choices[3].1 {
                        userDefaults.removePersistentDomain(forName: suiteName)
                        for (name, value) in zip(choices.map(\.0), [returnKey, spaceKey, paging, tab]) {
                            if let value { userDefaults.set(value, forKey: name) }
                        }

                        ComposingShortcutMigration.run(userDefaults: userDefaults)

                        let resolved = bindings()
                        for action in ComposingAction.alwaysBound {
                            XCTAssertNotNil(
                                resolved.chord(for: action),
                                """
                                \(action) unbound after migrating \
                                return=\(returnKey ?? "-") space=\(spaceKey ?? "-") \
                                paging=\(paging ?? "-") tab=\(tab ?? "-")
                                """,
                            )
                        }
                    }
                }
            }
        }
    }

    func testMigration_removesTheOldKeysAndStampsTheSchema() {
        for name in ["returnKeyBehavior", "spaceKeyBehavior", "bracketPagingBehavior", "tabCycleBehavior"] {
            userDefaults.set("commitLiteral", forKey: name)
        }

        ComposingShortcutMigration.run(userDefaults: userDefaults)

        for name in ["returnKeyBehavior", "spaceKeyBehavior", "bracketPagingBehavior", "tabCycleBehavior"] {
            XCTAssertNil(userDefaults.object(forKey: name), "\(name) should be gone")
        }
        XCTAssertEqual(
            userDefaults.integer(forKey: SettingsStore.Keys.composingShortcutSchema.name),
            ComposingShortcutMigration.currentSchema,
        )
    }

    /// The reason this runs behind a schema stamp rather than every launch: an
    /// old key written back by a downgraded build, or by a `defaults write`,
    /// must not keep overwriting what the user has recorded since.
    func testMigration_doesNotRunASecondTimeOverLaterEdits() throws {
        userDefaults.set("commitLiteral", forKey: "returnKeyBehavior")
        ComposingShortcutMigration.run(userDefaults: userDefaults)

        let store = SettingsStore(userDefaults: userDefaults)
        store.setComposingChord(try chord("\r", .option), for: .commitLiteral)
        userDefaults.set("commitLiteral", forKey: "returnKeyBehavior")
        ComposingShortcutMigration.run(userDefaults: userDefaults)

        XCTAssertEqual(
            bindings().chord(for: .commitLiteral),
            try chord("\r", .option),
            "the user's later recording stands",
        )
    }
}
