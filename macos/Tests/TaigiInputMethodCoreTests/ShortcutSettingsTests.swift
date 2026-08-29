// The persistence and text contracts the shortcut rows are built on.

@testable import TaigiInputMethodCore
import XCTest

/// What a unit test can hold the 快速齒 pane's rows to, given a SwiftUI
/// form cannot be brought up here: the raw values the rows persist, and the
/// strings they read. Whether each row is wired to the right action is a render
/// check, not a case below.
@MainActor
final class ShortcutSettingsTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "ShortcutSettingsTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
    }

    override func tearDown() {
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    /// The settings keys the rows write. Renaming an action's raw value would
    /// silently drop every chord recorded under the old spelling.
    func testActionRawValues_stayStable() {
        XCTAssertEqual(
            ComposingAction.allCases.map(\.rawValue),
            [
                "nextCandidate", "previousCandidate", "pageForward", "pageBackward",
                "confirmHighlighted", "commitLiteral", "commitAlternateScript",
            ],
        )
        XCTAssertEqual(
            ComposingAction.nextCandidate.settingsKeyName,
            "composingShortcut.nextCandidate",
        )
        XCTAssertEqual(CandidateSlotKeySet.allCases.map(\.rawValue), ["bareKeys", "shift", "control", "option"])
    }

    /// A row whose text is missing in one language reads as an identifier — or
    /// as nothing — for the users who chose that language.
    func testEveryPaneString_resolvesInEveryDisplayLanguage() {
        for language in DisplayLanguage.selectableLanguages where language != .system {
            let store = TestFixtures.makeDisplayLanguageStore(language, userDefaults: userDefaults)
            for key in Self.paneStrings {
                let text = store.string(key)
                XCTAssertFalse(text.isEmpty, "\(key) has nothing to show in \(language)")
                XCTAssertNotEqual(text, key.rawValue, "\(key) fell back to its own identifier in \(language)")
            }
            for action in ComposingAction.allCases {
                XCTAssertFalse(
                    action.label(store).isEmpty,
                    "\(action) has no row label in \(language)",
                )
            }
        }
    }

    /// Two rows reading alike would leave the user guessing which key they are
    /// about to rebind.
    func testActionLabels_readDistinctly() {
        let store = TestFixtures.makeDisplayLanguageStore(.hanji, userDefaults: userDefaults)
        let labels = ComposingAction.allCases.map { $0.label(store) }

        XCTAssertEqual(Set(labels).count, labels.count, "two rows read the same: \(labels)")
    }

    /// What the recorder button shows. Keycap legends rather than translations:
    /// these are the names printed on the keyboard.
    func testRecordedChords_readAsKeycapLegends() throws {
        let cases: [(String, NSEvent.ModifierFlags, String)] = [
            (" ", [], "Space"),
            ("\r", [], "↩"),
            ("\r", .shift, "⇧↩"),
            ("]", [], "]"),
            ("j", [.control, .option], "⌃⌥J"),
            // A bare letter shows the character it types: uppercase on a
            // modifier-less row would read as ⇧Z (USER 2026-08-22).
            ("z", [], "z"),
            ("Z", .shift, "⇧Z"),
        ]

        for (key, modifiers, expected) in cases {
            let chord = try ComposingKeyChord.make(key: key, modifiers: modifiers).get()
            XCTAssertEqual(ShortcutKeyDisplay.text(for: chord), expected)
        }
    }

    /// Every row comes back, whatever state the domain was left in: a chord the
    /// user recorded, the empty string that means a row was cleared, and a value
    /// this build cannot parse — the three ways a row can hold something other
    /// than its default.
    func testResetComposingShortcuts_returnsEveryRowToItsDefault() throws {
        let store = SettingsStore(userDefaults: userDefaults)
        store.setComposingChord(try ComposingKeyChord.make(key: "z", modifiers: []).get(), for: .nextCandidate)
        store.setComposingChord(nil, for: .pageBackward)
        userDefaults.set("not a chord", forKey: ComposingAction.pageForward.settingsKeyName)
        userDefaults.set(CandidateSlotKeySet.option.rawValue, forKey: SettingsStore.Keys.candidateSlotModifier.name)

        store.resetComposingShortcuts()

        let bindings = store.composingKeyBindings
        for action in ComposingAction.allCases {
            XCTAssertEqual(bindings.chord(for: action), action.defaultChord, "\(action) did not come back")
        }
        XCTAssertEqual(bindings.slotKeySet, .bareKeys)
    }

    /// Removed, not written over: a stored default would be indistinguishable
    /// from a chord the user chose, and would pin this version's default onto
    /// an install a later version means to move.
    func testResetComposingShortcuts_leavesNothingStored() throws {
        let store = SettingsStore(userDefaults: userDefaults)
        store.setComposingChord(try ComposingKeyChord.make(key: "z", modifiers: []).get(), for: .nextCandidate)
        store.setComposingChord(nil, for: .pageBackward)

        store.resetComposingShortcuts()

        for action in ComposingAction.allCases {
            XCTAssertNil(
                userDefaults.object(forKey: action.settingsKeyName),
                "\(action) still has a stored value",
            )
        }
        XCTAssertNil(userDefaults.object(forKey: SettingsStore.Keys.candidateSlotModifier.name))
    }

    private static let paneStrings: [StringKey] = [
        .desktopShortcutsTab,
        .themeEditorResetAll,
        .desktopBindingSlotModifier,
        .desktopShortcutUnbound,
        .desktopShortcutRecording,
        .desktopShortcutRejectedTypingKey,
        .desktopShortcutRejectedReservedKey,
        .desktopShortcutRejectedNoKey,
        .desktopShortcutRejectedSlotChord,
    ]
}
