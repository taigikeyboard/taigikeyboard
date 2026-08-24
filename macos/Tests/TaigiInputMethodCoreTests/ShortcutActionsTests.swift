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
            [
                "一般", "外觀", "快捷鍵", "自訂詞庫", "辭典管理",
                "切換 台羅/白話字", "切換 漢羅對調",
            ],
        )
        XCTAssertEqual(
            labels(.japanese),
            [
                "一般", "外観", "ショートカット", "カスタム辞書", "辞書の管理",
                "ローマ字体系を切り替える",
                "漢字とローマ字の入れ替えを切り替える",
            ],
        )
        XCTAssertEqual(labels(.english).first, "General")
        // The trap this replaced: composing a row from the setting's own label produced a doubled
        // verb — "括弧で併記を切り替える", "Toggle Annotate in Brackets".
        XCTAssertFalse(labels(.japanese).contains { $0.contains("併記を切り替えるを") })
    }

    /// The two mid-sentence switches arrive bound as well: no row in the pane
    /// is blank (USER 2026-08-21). ⌃⌘R follows vChewing's toggle convention;
    /// the 漢羅 swap sits on the bare backtick, the classic Taiwanese-IME
    /// function key — no TL or POJ syllable is spelled with it, and the hotkey
    /// is armed only while a Taigi session holds the engine
    /// (USER 2026-08-21).
    func testTheSwitches_startOnTheirConventionKeys() {
        XCTAssertEqual(
            ShortcutAction.toggleRomanization.defaultShortcut,
            KeyboardShortcuts.Shortcut(.r, modifiers: [.control, .command]),
        )
        XCTAssertEqual(
            ShortcutAction.toggleTranslateSwapped.defaultShortcut,
            KeyboardShortcuts.Shortcut(.backtick),
        )
    }

    /// Every action, not just the ones a case names above: one added without an
    /// `initial:` would draw a blank row.
    func testEveryAction_startsBound() {
        for action in ShortcutAction.allCases {
            XCTAssertNotNil(
                action.defaultShortcut,
                "\(action.name.rawValue) starts blank",
            )
        }
    }

    /// One chord per action, or both handlers fire on one keypress — the
    /// defaults have to satisfy the same rule `ShortcutConflicts` enforces on
    /// what the user records.
    func testTheDefaults_areAllDifferent() {
        let defaults = ShortcutAction.allCases.compactMap(\.name.defaultShortcut)

        XCTAssertEqual(Set(defaults).count, ShortcutAction.allCases.count)
    }

    /// An `initial:` added in a later version installs itself on every install
    /// that never recorded that action — including one where the user had
    /// already put that chord on a different action. Both handlers would fire
    /// on one keypress, so the row holding only a default gives way.
    func testADefault_givesWayToTheSameChordRecordedElsewhere() {
        let recorded = KeyboardShortcuts.Shortcut(.r, modifiers: [.control, .command])
        // toggleRomanization's default, recorded by hand on another row. Named
        // rather than switched with a `default`, which would quietly absorb
        // every action added later.
        let holders: Set<ShortcutAction> = [.openGeneralPane, .toggleRomanization]
        let shadowed = ShortcutConflicts.defaultsShadowedByRecordings { action in
            holders.contains(action) ? recorded : nil
        }

        XCTAssertEqual(shadowed, [.toggleRomanization])
    }

    /// Every action on its own default is the fresh-install state, and those
    /// are all different — nothing to resolve.
    func testTheShippedDefaults_shadowNothing() {
        let shadowed = ShortcutConflicts.defaultsShadowedByRecordings { $0.defaultShortcut }

        XCTAssertEqual(shadowed, [])
    }

    /// The pane doorways start on ⌃⇧ plus the pane's own position in the
    /// sidebar: one modifier family, and digits rather than initials because
    /// the window speaks five display languages and an initial is a mnemonic
    /// in exactly one of them.
    func testThePaneDoorways_startOnControlShiftTheirSidebarPosition() {
        let paneDefaults = ShortcutAction.allCases
            .filter { $0.settingsPane != nil }
            .map(\.defaultShortcut)

        XCTAssertEqual(
            paneDefaults,
            [
                KeyboardShortcuts.Shortcut(.one, modifiers: [.control, .shift]),
                KeyboardShortcuts.Shortcut(.two, modifiers: [.control, .shift]),
                KeyboardShortcuts.Shortcut(.three, modifiers: [.control, .shift]),
                KeyboardShortcuts.Shortcut(.four, modifiers: [.control, .shift]),
                KeyboardShortcuts.Shortcut(.five, modifiers: [.control, .shift]),
            ],
        )
    }

    /// Every pane has a doorway and every doorway has a pane, in the sidebar's
    /// own order: a pane added to `SettingsPane` without one would be reachable
    /// by no key at all, and the menu draws these rows in this order.
    func testEveryPane_hasExactlyOneDoorway() {
        XCTAssertEqual(
            ShortcutAction.allCases.compactMap(\.settingsPane),
            SettingsPane.allCases,
        )
    }

    /// ⌃ plus a digit is how a user picks the third candidate on screen, and
    /// the classifier reads that tier before it reads any binding. Shift is
    /// what keeps these chords out of it — the tier refuses any chord carrying
    /// a modifier it was not bound to — so the check is worth pinning rather
    /// than reasoning about.
    func testNoDefault_isACandidateSlotChord() throws {
        for action in ShortcutAction.allCases {
            let shortcut = try XCTUnwrap(action.defaultShortcut)
            let chord = try ComposingKeyChord.make(
                key: XCTUnwrap(shortcut.nsMenuItemKeyEquivalent, action.name.rawValue),
                modifiers: shortcut.modifiers,
            ).get()

            for slotModifier in CandidateSlotModifier.allCases {
                XCTAssertFalse(
                    chord.isCandidateSlotChord(under: slotModifier),
                    "\(action.name.rawValue) collides with the \(slotModifier) slot tier",
                )
            }
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
