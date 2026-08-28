// The upgrade path for the 2026-08-21 settings trim: retired state is removed.

import KeyboardShortcuts
@testable import TaigiInputMethodCore
import XCTest

/// An install upgraded across the trim must behave like a fresh one: the
/// retired toggles fall back to their `false` defaults, a selection on the
/// unlisted pane falls back to 一般, and the retired hotkeys lose their chords.
@MainActor
final class RetiredSettingsCleanupTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "RetiredSettingsCleanupTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
    }

    override func tearDown() {
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    func testStoredTrueValuesOfRetiredToggles_areRemoved() {
        userDefaults.set(true, forKey: SettingsStore.Keys.isOutputBothScripts.name)
        userDefaults.set(true, forKey: SettingsStore.Keys.isLiteralRomanCandidateEnabled.name)

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        XCTAssertNil(userDefaults.object(forKey: SettingsStore.Keys.isOutputBothScripts.name))
        XCTAssertNil(
            userDefaults.object(forKey: SettingsStore.Keys.isLiteralRomanCandidateEnabled.name),
        )
        let settings = SettingsStore(userDefaults: userDefaults)
        XCTAssertFalse(settings.isOutputBothScripts)
        XCTAssertFalse(settings.isLiteralRomanCandidateEnabled)
    }

    /// The first shape of the composing-key settings. Nothing reads them any
    /// more, so clearing them changes no behaviour — it keeps the domain from
    /// carrying values a later setting reusing one of these names would
    /// inherit.
    func testTheFirstShapeOfTheComposingKeySettings_isRemoved() {
        let names = [
            "returnKeyBehavior", "spaceKeyBehavior",
            "bracketPagingBehavior", "tabCycleBehavior",
        ]
        for name in names {
            userDefaults.set("commitLiteral", forKey: name)
        }

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        for name in names {
            XCTAssertNil(userDefaults.object(forKey: name), "\(name) should be gone")
        }
    }

    /// The two retired script commits. A chord recorded on either row is
    /// stored under `ComposingAction.settingsKeyName`, and no row can reach it
    /// once the action is gone — so it is cleared rather than left in the
    /// domain for a later action reusing the name to inherit.
    func testChordsRecordedOnTheRetiredScriptCommits_areRemoved() throws {
        let names = ["composingShortcut.commitHanji", "composingShortcut.commitRomanization"]
        let recorded = try ComposingKeyChord.make(key: "z", modifiers: []).get().rawValue
        for name in names {
            userDefaults.set(recorded, forKey: name)
        }

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        for name in names {
            XCTAssertNil(userDefaults.object(forKey: name), "\(name) should be gone")
        }
    }

    /// The 外觀 pane's two retired rows: the accent-colour swatch and the
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

    /// The one setting of that shape that survived into the new one, so it must
    /// NOT be swept up with its neighbours.
    func testTheCandidateSlotKeySet_isKept() {
        userDefaults.set(
            CandidateSlotKeySet.option.rawValue,
            forKey: SettingsStore.Keys.candidateSlotModifier.name,
        )

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        XCTAssertEqual(
            SettingsStore(userDefaults: userDefaults).composingKeyBindings.slotKeySet,
            .option,
        )
    }

    /// The learning toggles went with the 詞頻紀錄 / 詞關聯紀錄 panes. This one
    /// is a behaviour fix, not hygiene: the engine still reads both keys, so a
    /// stored `false` would keep learning off with no UI left to turn it on.
    func testStoredFalseOnTheRetiredRecordingToggles_returnsThemToOn() {
        userDefaults.set(false, forKey: SettingsStore.Keys.isFrequencyRecordingEnabled.name)
        userDefaults.set(false, forKey: SettingsStore.Keys.isAssociationRecordingEnabled.name)

        RetiredSettingsCleanup.run(userDefaults: userDefaults)

        let settings = SettingsStore(userDefaults: userDefaults).current
        XCTAssertTrue(settings.isFrequencyRecordingEnabled)
        XCTAssertTrue(settings.isAssociationRecordingEnabled)
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
            KeyboardShortcuts.Name("toggleBothScripts"),
            KeyboardShortcuts.Name("toggleLiteralRomanCandidate"),
            KeyboardShortcuts.Name("openSettings"),
            KeyboardShortcuts.Name("openGeneralPane"),
            KeyboardShortcuts.Name("openAppearancePane"),
            KeyboardShortcuts.Name("openShortcutPane"),
            KeyboardShortcuts.Name("openCustomDictionaryPane"),
            KeyboardShortcuts.Name("openDictionarySourcesPane"),
        ]
        let keys: [KeyboardShortcuts.Key] = [.k, .j, .l, .f13, .f14, .f15, .f16, .f17]
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
