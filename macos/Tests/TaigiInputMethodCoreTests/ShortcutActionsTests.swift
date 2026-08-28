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
            ["拍開設定選單", "切換輸入模式", "漢字/羅馬字代先"],
        )
        XCTAssertEqual(
            labels(.japanese),
            [
                "設定メニューを開く",
                "入力モードを切り替える",
                "漢字／ローマ字を先に",
            ],
        )
        XCTAssertEqual(labels(.english).first, "Open Settings Menu")
        // The trap this replaced: composing a row from the setting's own label produced a doubled
        // verb — "括弧で併記を切り替える", "Toggle Annotate in Brackets".
        XCTAssertFalse(labels(.japanese).contains { $0.contains("併記を切り替えるを") })
    }

    /// The two mid-sentence switches arrive bound as well: no row in the pane
    /// is blank (USER 2026-08-21). ⌃⌘ is vChewing's toggle family, and C
    /// rather than the R this had until 2026-08-25 (USER): a switch reached
    /// for all day is muscle memory by the second day, so what is left to
    /// optimise is travel — ⌃, ⌘ and C are all bottom row, while R is two rows
    /// up with the pinky still anchored. The 漢羅 swap sits on the bare
    /// backtick, the classic Taiwanese-IME function key — no TL or POJ
    /// syllable is spelled with it, and the hotkey is armed only while a Taigi
    /// session holds the engine (USER 2026-08-21).
    func testTheSwitches_startOnTheirConventionKeys() {
        XCTAssertEqual(
            ShortcutAction.toggleRomanization.defaultShortcut,
            KeyboardShortcuts.Shortcut(.c, modifiers: [.control, .command]),
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
        let recorded = KeyboardShortcuts.Shortcut(.c, modifiers: [.control, .command])
        // toggleRomanization's default, recorded by hand on another row. Named
        // rather than switched with a `default`, which would quietly absorb
        // every action added later.
        let holders: Set<ShortcutAction> = [.openLastSettingsPane, .toggleRomanization]
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

    /// The whole global roster, on one modifier family.
    ///
    /// ⌃⇧1–⌃⇧5 opened the five panes until 2026-08-25 (USER: five chords for
    /// panes the menu bar already lists by name).
    /// One ⌃⌘S doorway replaced them — a command reached for rarely, so a
    /// mnemonic pays — and the switch moved to ⌃⌘C, where a command reached
    /// for all day wants the hand to stay on the bottom row.
    func testTheGlobalRoster_isOneDoorwayAndTwoSwitches() {
        XCTAssertEqual(
            ShortcutAction.allCases.map(\.defaultShortcut),
            [
                KeyboardShortcuts.Shortcut(.s, modifiers: [.control, .command]),
                KeyboardShortcuts.Shortcut(.c, modifiers: [.control, .command]),
                KeyboardShortcuts.Shortcut(.backtick),
            ],
        )
    }

    /// No global chord opens a NAMED pane any more; the menu bar does that
    /// (`TaigiInputControllerMenuTests`). The one doorway reopens wherever the
    /// user left off, which is what makes it a single key rather than five.
    func testExactlyOneAction_opensSettings() {
        XCTAssertEqual(
            ShortcutAction.allCases.filter(\.opensSettings), [.openLastSettingsPane],
        )
    }

    /// A slot key is how a user picks the third candidate on screen, and the
    /// classifier reads that tier before it reads any binding — under every
    /// set the picker offers, and the fixed `⇧1`…`⇧9` besides. The tier
    /// refuses any chord carrying a modifier it was not bound to, so the check
    /// is worth pinning rather than reasoning about.
    func testNoDefault_isACandidateSlotChord() throws {
        for action in ShortcutAction.allCases {
            let shortcut = try XCTUnwrap(action.defaultShortcut)
            let chord = try ComposingKeyChord.make(
                key: XCTUnwrap(shortcut.nsMenuItemKeyEquivalent, action.name.rawValue),
                modifiers: shortcut.modifiers,
            ).get()

            for slotKeySet in CandidateSlotKeySet.allCases {
                XCTAssertFalse(
                    chord.isCandidateSlotChord(under: slotKeySet),
                    "\(action.name.rawValue) collides with the \(slotKeySet) slot tier",
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

    /// The live roster and the tombstone roster must not overlap: a retired
    /// name is cleared on every launch, so an action still using one would
    /// lose the user's chord each time the app started.
    func testNoLiveAction_reusesARetiredName() {
        let retired = Set([
            "toggleBothScripts", "toggleLiteralRomanCandidate", "openSettings",
            "openGeneralPane", "openAppearancePane", "openShortcutPane",
            "openCustomDictionaryPane", "openDictionarySourcesPane",
        ])

        for action in ShortcutAction.allCases {
            XCTAssertFalse(
                retired.contains(action.name.rawValue),
                "\(action.name.rawValue) is swept by RetiredSettingsCleanup every launch",
            )
        }
    }

    /// The doorway reopens where the user LEFT OFF, which is the whole reason
    /// one chord replaced five: a chord that landed on a fixed pane would be
    /// the ⌃⇧1 that was just retired, wearing a different key.
    @MainActor
    func testTheSettingsDoorway_leavesTheStoredPaneAlone() throws {
        let store = try makeScratchSettingsStore()
        store.selectedSettingsPane = .customDictionary
        var shown = 0

        ShortcutHotkeys.openSettings(on: nil, in: store, show: { shown += 1 })

        XCTAssertEqual(store.selectedSettingsPane, .customDictionary)
        XCTAssertEqual(shown, 1, "the window still has to come up")
    }
}
