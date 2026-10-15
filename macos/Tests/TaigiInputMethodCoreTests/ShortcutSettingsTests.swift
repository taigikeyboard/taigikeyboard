// The persistence and text contracts the shortcut rows are built on.

@testable import TaigiInputMethodCore
import XCTest

/// What a unit test can hold the Shortcuts pane's rows to, given a SwiftUI
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

    /// A row whose text is missing in one language reads as an identifier — or
    /// as nothing — for the users who chose that language.
    func testEveryPaneString_resolvesInEveryDisplayLanguage() throws {
        let rows = try TestFixtures.composingShortcuts().rows
        for language in DisplayLanguage.selectableLanguages where language != .system {
            let store = TestFixtures.makeDisplayLanguageStore(language, userDefaults: userDefaults)
            for key in Self.paneStrings {
                let text = store.string(key)
                XCTAssertFalse(text.isEmpty, "\(key) has nothing to show in \(language)")
                XCTAssertNotEqual(text, key.rawValue, "\(key) fell back to its own identifier in \(language)")
            }
            for row in rows {
                XCTAssertFalse(
                    store.string(row.label).isEmpty,
                    "\(row.name) has no row label in \(language)",
                )
            }
        }
    }

    /// Two rows reading alike would leave the user guessing which key they are
    /// about to rebind.
    func testActionLabels_readDistinctly() throws {
        let store = TestFixtures.makeDisplayLanguageStore(.hanji, userDefaults: userDefaults)
        let labels = try TestFixtures.composingShortcuts().rows.map { store.string($0.label) }

        XCTAssertEqual(Set(labels).count, labels.count, "two rows read the same: \(labels)")
    }

    /// Every row comes back, whatever state the domain was left in: a chord the
    /// user recorded, the empty string that means a row was cleared, and a value
    /// this build cannot parse — the three ways a row can hold something other
    /// than its default.
    func testResetComposingShortcuts_returnsEveryRowToItsDefault() throws {
        let store = SettingsStore(userDefaults: userDefaults)
        let shortcuts = try TestFixtures.composingShortcuts(in: store)
        try store.setComposingChord(
            TestFixtures.chordNoDefaultHolds(), for: TestFixtures.row("nextCandidate", in: shortcuts).first,
        )
        try store.setComposingChord(nil, for: TestFixtures.row("pageBackward", in: shortcuts).first)
        userDefaults.set("not a chord", forKey: "composingShortcut.pageForward")

        store.resetComposingShortcuts(shortcuts)

        let rows = try TestFixtures.composingShortcuts(in: store).rows
        XCTAssertEqual(rows.count, 7)
        for row in rows {
            for field in row.fields {
                XCTAssertEqual(field.chord, field.shippedChord, "\(field.settingsKey) did not come back")
            }
        }
    }

    /// The tone scheme is the General pane's, not this pane's: the shortcut
    /// reset leaves it where the user put it.
    func testResetComposingShortcuts_leavesTheToneSchemeAlone() throws {
        let store = SettingsStore(userDefaults: userDefaults)
        userDefaults.set(ToneInputScheme.telex.rawValue, forKey: SettingsStore.Keys.toneInputScheme.name)

        try store.resetComposingShortcuts(TestFixtures.composingShortcuts(in: store))

        XCTAssertEqual(store.toneInputScheme, .telex)
    }

    /// Removed, not written over: a stored default would be indistinguishable
    /// from a chord the user chose, and would pin this version's default onto
    /// an install a later version means to move.
    func testResetComposingShortcuts_leavesNothingStored() throws {
        let store = SettingsStore(userDefaults: userDefaults)
        let shortcuts = try TestFixtures.composingShortcuts(in: store)
        try store.setComposingChord(
            TestFixtures.chordNoDefaultHolds(), for: TestFixtures.row("nextCandidate", in: shortcuts).first,
        )
        try store.setComposingChord(nil, for: TestFixtures.row("pageBackward", in: shortcuts).first)

        store.resetComposingShortcuts(shortcuts)

        XCTAssertEqual(shortcuts.rows.count, 7)
        for row in shortcuts.rows {
            for field in row.fields {
                XCTAssertNil(userDefaults.object(forKey: field.settingsKey), "\(field.settingsKey) still has a stored value")
            }
        }
    }

    private static let paneStrings: [StringKey] = [
        .desktopShortcutsTab,
        .themeEditorResetAll,
        .desktopShortcutUnbound,
        .desktopShortcutRecording,
        .desktopShortcutRejectedTaken,
        .desktopShortcutRejectedNoKey,
    ]
}
