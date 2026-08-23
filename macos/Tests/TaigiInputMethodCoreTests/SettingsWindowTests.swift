// The settings window's chrome and the sidebar pane roster.

import AppKit
import SwiftUI
@testable import TaigiInputMethodCore
import XCTest

/// The settings window's chrome — the System Settings-style split view — and
/// the pane roster its sidebar is built from.
@MainActor
final class SettingsWindowTests: XCTestCase {
    /// Where AppKit persists the frame for `setFrameAutosaveName` — in the
    /// STANDARD defaults, not a test suite, so every window-building test
    /// stashes whatever the developer's machine had saved and puts it back.
    private static let frameAutosaveDefaultsKey = "NSWindow Frame TaigiSettingsWindow"

    private var suiteName = ""
    private var userDefaults = UserDefaults.standard
    private var stashedFrameValue: String?

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "SettingsWindowTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        stashedFrameValue = UserDefaults.standard.string(forKey: Self.frameAutosaveDefaultsKey)
        UserDefaults.standard.removeObject(forKey: Self.frameAutosaveDefaultsKey)
    }

    override func tearDown() {
        if let stashedFrameValue {
            UserDefaults.standard.set(stashedFrameValue, forKey: Self.frameAutosaveDefaultsKey)
        } else {
            UserDefaults.standard.removeObject(forKey: Self.frameAutosaveDefaultsKey)
        }
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    private func makeStore(_ language: DisplayLanguage = .hanji) -> DisplayLanguageStore {
        TestFixtures.makeDisplayLanguageStore(language, userDefaults: userDefaults)
    }

    /// A bare resizable window with no floor of its own, for driving
    /// `growToMinimum` through both of its branches — `makeWindow` puts
    /// `contentMinSize` on before sizing, which would clamp the shrink this
    /// setup needs and leave the grow branch untested.
    private func makeUnflooredWindow(contentSize: NSSize) -> NSWindow {
        let window = NSWindow(
            contentRect: NSRect(origin: .zero, size: contentSize),
            styleMask: [.titled, .resizable],
            backing: .buffered,
            defer: false,
        )
        window.isReleasedWhenClosed = false
        return window
    }

    // MARK: - Window chrome

    func testWindow_hostsTheSplitViewRoot() {
        let window = SettingsWindowController.makeWindow(language: makeStore())

        XCTAssertTrue(window.styleMask.contains(.resizable))
        XCTAssertNotNil(window.contentViewController as? NSHostingController<SettingsRootView>)
    }

    /// The titlebar shows the selected pane's name only if the SwiftUI
    /// `navigationTitle` is bridged out of the hosting controller — nothing
    /// else writes `window.title` any more.
    func testWindow_bridgesTheSwiftUITitleIntoTheTitlebar() throws {
        let window = SettingsWindowController.makeWindow(language: makeStore())

        let hosting = try XCTUnwrap(window.contentViewController as? NSHostingController<SettingsRootView>)
        XCTAssertEqual(hosting.sceneBridgingOptions, .all)
    }

    /// The window must survive being closed: it is cached so reopening returns
    /// the user to where they left it, and releasing it would make the second
    /// open a message to a freed object.
    func testWindow_isNotReleasedWhenClosed() {
        XCTAssertFalse(SettingsWindowController.makeWindow(language: makeStore()).isReleasedWhenClosed)
    }

    /// With no autosaved frame (setUp clears it), a fresh window opens at no
    /// less than the explicit initial size — not whatever `.zero` settles
    /// into — with the one window-wide floor applied. `>=` rather than `==`:
    /// bridging the split view's toolbar in re-lays-out the window, and the
    /// exact resulting height belongs to SwiftUI, not to `makeWindow`.
    func testFreshWindow_opensAtTheInitialSizeWithTheFloorApplied() {
        let window = SettingsWindowController.makeWindow(language: makeStore())

        XCTAssertEqual(window.contentMinSize, SettingsWindowController.minimumContentSize)
        XCTAssertGreaterThanOrEqual(
            window.contentLayoutRect.size.width,
            SettingsWindowController.initialContentSize.width - 1,
        )
        XCTAssertGreaterThanOrEqual(
            window.contentLayoutRect.size.height,
            SettingsWindowController.initialContentSize.height - 1,
        )
    }

    /// The real restore path: `contentMinSize` does not grow a frame autosaved
    /// by a build with a smaller floor — the tabbed window this layout
    /// replaced had one — so `makeWindow` has to grow it by hand after
    /// `setFrameAutosaveName` restores it.
    func testWindow_growsANarrowAutosavedFrameToTheFloor() throws {
        let screen = try XCTUnwrap(NSScreen.main).frame
        UserDefaults.standard.set(
            "100 100 380 300 0 0 \(Int(screen.width)) \(Int(screen.height))",
            forKey: Self.frameAutosaveDefaultsKey,
        )

        let window = SettingsWindowController.makeWindow(language: makeStore())

        let floor = SettingsWindowController.minimumContentSize
        XCTAssertGreaterThanOrEqual(window.contentLayoutRect.size.width, floor.width)
        XCTAssertGreaterThanOrEqual(window.contentLayoutRect.size.height, floor.height)
    }

    /// Both branches of the grow helper, on a window with no floor of its own
    /// so the shrink actually lands (see `makeUnflooredWindow`).
    func testGrowToMinimum_growsASmallWindowAndLeavesALargerOneAlone() {
        let small = makeUnflooredWindow(contentSize: NSSize(width: 200, height: 200))
        SettingsWindowController.growToMinimum(small)
        let floor = SettingsWindowController.minimumContentSize
        XCTAssertGreaterThanOrEqual(small.contentLayoutRect.size.width, floor.width)
        XCTAssertGreaterThanOrEqual(small.contentLayoutRect.size.height, floor.height)

        let large = makeUnflooredWindow(contentSize: NSSize(width: 900, height: 700))
        SettingsWindowController.growToMinimum(large)
        XCTAssertEqual(large.contentLayoutRect.size.width, 900, accuracy: 1)
        XCTAssertEqual(large.contentLayoutRect.size.height, 700, accuracy: 1)
    }

    // MARK: - Pane roster

    /// Raw values are the persistence contract: `@AppStorage` writes them, so
    /// renaming a case silently resets every user to 一般. The order is also
    /// the sidebar order — the flat list renders `allCases` directly — so
    /// this doubles as the roster. 詞頻紀錄 / 詞關聯紀錄 / 備份復原 are absent by
    /// decision, not by omission — see `RetiredSettingsCleanup`, which sweeps a
    /// selection left pointing at one of them.
    func testPaneRawValues_stayStable() {
        XCTAssertEqual(
            SettingsPane.allCases.map(\.rawValue),
            ["general", "appearance", "shortcuts", "customDictionary", "dictionarySources"],
        )
    }

    /// The other half of the persistence contract: the defaults key the raw
    /// values are written under, and the pane an unknown or never-written
    /// value must land on — `@AppStorage` resolves an unknown raw value to
    /// its default, which is why `init(rawValue:)` returning `nil` is safe.
    func testSelectedPaneKey_namesTheDefaultsKeyAndFallsBackToGeneral() {
        XCTAssertEqual(SettingsStore.Keys.selectedSettingsPane.name, "selectedSettingsPane")
        XCTAssertEqual(SettingsStore.Keys.selectedSettingsPane.defaultValue, .general)
        XCTAssertNil(SettingsPane(rawValue: "bogus"))
    }

    func testPaneLabels_followTheDisplayLanguage() {
        let hanji = makeStore(.hanji)
        XCTAssertEqual(
            SettingsPane.allCases.map { hanji.string($0.labelKey) },
            ["一般", "外觀", "快捷鍵", "自訂詞庫", "選辭典"],
        )

        let english = makeStore(.english)
        XCTAssertEqual(
            SettingsPane.allCases.map { english.string($0.labelKey) },
            [
                "General", "Appearance", "Shortcuts", "Custom Dictionary", "Choose Dictionaries",
            ],
        )
    }

    /// Sidebar rows are icon-first; a symbol name that stops resolving would
    /// render a blank slot next to the label.
    func testPaneSymbols_allResolve() {
        for pane in SettingsPane.allCases {
            XCTAssertNotNil(
                NSImage(systemSymbolName: pane.symbolName, accessibilityDescription: nil),
                "pane \(pane.rawValue) has no resolvable symbol \(pane.symbolName)",
            )
        }
    }
}
