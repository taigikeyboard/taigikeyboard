// The upgrade path for the 2026-08-21 settings trim: retired state is removed.

import KeyboardShortcuts
@testable import TaigiInputMethodCore
import XCTest

/// An install upgraded across the trim must behave like a fresh one: the
/// retired toggles fall back to their `false` defaults, a selection on the
/// unlisted pane falls back to General, and the retired hotkeys lose their chords.
@MainActor
final class RetiredSettingsCleanupTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard

    override func setUp() async throws {
        try await super.setUp()
        suiteName = "RetiredSettingsCleanupTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
    }

    override func tearDown() async throws {
        userDefaults.removePersistentDomain(forName: suiteName)
        try await super.tearDown()
    }

    func testStoredTrueValuesOfRetiredToggles_areRemoved() {
        userDefaults.set(true, forKey: "outputBothScripts")

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        XCTAssertNil(userDefaults.object(forKey: "outputBothScripts"))
    }

    /// No Hyphens left a stored `true` behind: it becomes Syllable Separator
    /// `none`, a picker choice already stored wins, and the switch goes either way.
    func testAStoredNoHyphensSwitch_carriesOverAsNone() {
        let separatorName = SettingsStore.Keys.syllableSeparator.name
        let retiredName = RetiredSettingsCleanup.retiredHyphenlessRomanName

        userDefaults.set(true, forKey: retiredName)
        RetiredSettingsCleanup.run(userDefaults: userDefaults)
        XCTAssertEqual(userDefaults.string(forKey: separatorName), "none")
        XCTAssertNil(userDefaults.object(forKey: retiredName))

        userDefaults.removeObject(forKey: separatorName)
        userDefaults.set(false, forKey: retiredName)
        RetiredSettingsCleanup.run(userDefaults: userDefaults)
        XCTAssertNil(userDefaults.object(forKey: separatorName), "off carries nothing over")
        XCTAssertNil(userDefaults.object(forKey: retiredName))

        userDefaults.set("space", forKey: separatorName)
        userDefaults.set(true, forKey: retiredName)
        RetiredSettingsCleanup.run(userDefaults: userDefaults)
        XCTAssertEqual(userDefaults.string(forKey: separatorName), "space", "a stored choice wins")
        XCTAssertNil(userDefaults.object(forKey: retiredName))
    }

    /// A tombstone for Show Typed Text First is what left its earlier pane row dead —
    /// the sweep cleared what the row wrote, every launch (`behavioral-invariants.md`
    /// §34 desktop notes). The row is back, so the sweep must leave the key alone.
    func testTheLiteralRomanCandidateSetting_survivesTheSweep() {
        userDefaults.set(true, forKey: SettingsStore.Keys.isLiteralRomanCandidateEnabled.name)

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        XCTAssertEqual(
            userDefaults.object(forKey: SettingsStore.Keys.isLiteralRomanCandidateEnabled.name) as? Bool,
            true,
            "the sweep must not take a setting the 一般 pane still writes",
        )
    }

    /// The two retired script commits. A chord recorded on either row is
    /// stored under the roster's `composingShortcut.<name>`, and no row can reach it
    /// once the action is gone — so it is cleared rather than left in the
    /// domain for a later action reusing the name to inherit.
    func testChordsRecordedOnTheRetiredScriptCommits_areRemoved() throws {
        let names = ["composingShortcut.commitHanji", "composingShortcut.commitRomanization"]
        let recorded = try TestFixtures.chordNoDefaultHolds().rawValue
        for name in names {
            userDefaults.set(recorded, forKey: name)
        }

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        for name in names {
            XCTAssertNil(userDefaults.object(forKey: name), "\(name) should be gone")
        }
    }

    /// The Appearance pane's two retired rows: the accent-colour swatch and the
    /// candidate-window chrome picker. Nothing reads either key any more — the
    /// highlight always follows the system accent and the chrome always
    /// follows the running OS — so this keeps the defaults domain from
    /// carrying values nobody can see or change.
    func testTheRetiredAppearanceChoices_areRemoved() {
        userDefaults.set("graphite", forKey: "candidateAccentColor")
        userDefaults.set("sequoia", forKey: "candidateWindowStyle")

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        XCTAssertNil(userDefaults.object(forKey: "candidateAccentColor"))
        XCTAssertNil(userDefaults.object(forKey: "candidateWindowStyle"))
    }

    /// Candidate Selection Keys, retired 2026-09-08: the slot key set follows the tone scheme
    /// now, and none of the values this key could hold (`shift`, `control`,
    /// `option`) has a set left to migrate to.
    func testTheCandidateSlotModifier_isRemoved() {
        userDefaults.set("option", forKey: "candidateSlotModifier")

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        XCTAssertNil(userDefaults.object(forKey: "candidateSlotModifier"))
        XCTAssertEqual(
            SettingsStore(userDefaults: userDefaults).toneInputScheme.slotKeySet,
            .bareKeys,
        )
    }

    /// The setting that replaced it must NOT be swept up with its neighbours.
    func testTheToneInputScheme_survivesTheSweep() {
        userDefaults.set(ToneInputScheme.telex.rawValue, forKey: SettingsStore.Keys.toneInputScheme.name)

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        XCTAssertEqual(
            SettingsStore(userDefaults: userDefaults).toneInputScheme,
            .telex,
        )
    }

    /// The learning toggles went with the Frequency Records / Association Records panes.
    /// Neither key is read any more (recording is unconditional), so a stored
    /// `false` is simply swept.
    func testStoredFalseOnTheRetiredRecordingToggles_isRemoved() {
        userDefaults.set(false, forKey: "frequencyRecordingEnabled")
        userDefaults.set(false, forKey: "associationRecordingEnabled")

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        XCTAssertNil(userDefaults.object(forKey: "frequencyRecordingEnabled"))
        XCTAssertNil(userDefaults.object(forKey: "associationRecordingEnabled"))
    }

    func testSelectionOnAnyRetiredPane_fallsBackToTheDefault() {
        for pane in ["dictionarySearch", "frequencyData", "associationData", "backupRestore"] {
            userDefaults.set(pane, forKey: SettingsStore.Keys.selectedSettingsPane.name)

            RetiredSettingsCleanup.run(userDefaults: userDefaults)

            XCTAssertNil(
                userDefaults.object(forKey: SettingsStore.Keys.selectedSettingsPane.name),
                "\(pane) survived the sweep",
            )
        }
    }

    func testSelectionOnASurvivingPane_isKept() {
        userDefaults.set("appearance", forKey: SettingsStore.Keys.selectedSettingsPane.name)

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        XCTAssertEqual(
            userDefaults.string(forKey: SettingsStore.Keys.selectedSettingsPane.name),
            "appearance",
        )
    }

    /// The library persists chords by raw name in the standard domain, so a
    /// chord recorded by a pre-trim build would spring back to life on any
    /// future action that reused the name — cleanup must clear EVERY retired
    /// name there. Saved and restored per name, because the standard domain is
    /// the developer's real one.
    ///
    /// Whatever the user recorded, not merely the shipped default: every one
    /// of these was re-recordable for as long as it existed, and a chord the
    /// user chose is exactly the one that would come back.
    ///
    /// The five pane doorways matter most here: `openLastSettingsPane` is a
    /// LIVE action on ⌃⌘S, and one of the retired names springing back would
    /// put a second handler on whatever the user had recorded.
    func testRetiredHotkeyChords_areCleared() {
        let retiredNames = [
            KeyboardShortcuts.Name("openSettings"),
            KeyboardShortcuts.Name("openGeneralPane"),
            KeyboardShortcuts.Name("openAppearancePane"),
            KeyboardShortcuts.Name("openShortcutPane"),
            KeyboardShortcuts.Name("openCustomDictionaryPane"),
            KeyboardShortcuts.Name("openDictionarySourcesPane"),
        ]
        let keys: [KeyboardShortcuts.Key] = [.l, .f13, .f14, .f15, .f16, .f17]
        let saved = retiredNames.map { ($0, KeyboardShortcuts.getShortcut(for: $0)) }
        defer {
            for (name, shortcut) in saved {
                KeyboardShortcuts.setShortcut(shortcut, for: name)
            }
        }
        for (index, name) in retiredNames.enumerated() {
            KeyboardShortcuts.setShortcut(
                .init(keys[index], modifiers: [.control, .option]), for: name,
            )
        }

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        for name in retiredNames {
            XCTAssertNil(KeyboardShortcuts.getShortcut(for: name), "\(name.rawValue) kept its chord")
        }
    }
}
