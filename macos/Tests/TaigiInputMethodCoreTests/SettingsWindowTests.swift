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
    /// The window's chrome — its title and its light/dark override — is read
    /// from the STANDARD defaults by the split controller, which builds its
    /// own `SettingsStore` rather than taking the suite a test hands the
    /// language store. So a test that drives the chrome writes there, and
    /// this puts back whatever the developer's machine had.
    private var stashedSelectedPane: String?
    /// Every window a test built. An autosave name binds to one live window at
    /// a time, so a window still alive from an earlier test makes the next
    /// one's frame restore silently do nothing — `tearDown` hands the name
    /// back rather than trusting each window to have been released by then.
    private var windowsUnderTest: [NSWindow] = []

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "SettingsWindowTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        stashedFrameValue = UserDefaults.standard.string(forKey: Self.frameAutosaveDefaultsKey)
        UserDefaults.standard.removeObject(forKey: Self.frameAutosaveDefaultsKey)
        stashedSelectedPane = UserDefaults.standard.string(
            forKey: SettingsStore.Keys.selectedSettingsPane.name,
        )
    }

    override func tearDown() {
        if let stashedFrameValue {
            UserDefaults.standard.set(stashedFrameValue, forKey: Self.frameAutosaveDefaultsKey)
        } else {
            UserDefaults.standard.removeObject(forKey: Self.frameAutosaveDefaultsKey)
        }
        if let stashedSelectedPane {
            UserDefaults.standard.set(stashedSelectedPane, forKey: SettingsStore.Keys.selectedSettingsPane.name)
        } else {
            UserDefaults.standard.removeObject(forKey: SettingsStore.Keys.selectedSettingsPane.name)
        }
        for window in windowsUnderTest {
            window.setFrameAutosaveName("")
        }
        windowsUnderTest.removeAll()
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    /// What `contentMinSize` / `setContentSize` speak, which is NOT
    /// `contentLayoutRect`: under `.fullSizeContentView` the layout rect
    /// excludes the titlebar, so a test reading it would measure a window a
    /// titlebar shorter than the one the limits describe.
    private func contentSize(of window: NSWindow) -> NSSize {
        window.contentRect(forFrameRect: window.frame).size
    }

    private func makeStore(_ language: DisplayLanguage = .hanji) -> DisplayLanguageStore {
        TestFixtures.makeDisplayLanguageStore(language, userDefaults: userDefaults)
    }

    /// Builds the settings window and registers it for the autosave-name
    /// release in `tearDown`.
    private func makeWindow(language: DisplayLanguageStore) -> NSWindow {
        let window = SettingsWindowController.makeWindow(language: language)
        windowsUnderTest.append(window)
        return window
    }

    // MARK: - Window chrome

    func testWindow_hostsTheSplitViewRoot() {
        let window = makeWindow(language: makeStore())

        XCTAssertTrue(window.styleMask.contains(.resizable))
        XCTAssertNotNil(window.contentViewController as? SettingsSplitViewController)
    }

    /// The sidebar is AppKit's, and pinned: SwiftUI's `NavigationSplitView`
    /// collapsed on device whatever it was told, so the column states what it
    /// is through the one public API that can — `NSSplitViewItem`.
    func testSidebar_cannotCollapseOrResize() throws {
        let window = makeWindow(language: makeStore())

        let split = try XCTUnwrap(window.contentViewController as? SettingsSplitViewController)
        let sidebar = try XCTUnwrap(split.splitViewItems.first)
        XCTAssertFalse(sidebar.canCollapse)
        XCTAssertEqual(sidebar.minimumThickness, SettingsPaneLayout.sidebarWidth)
        XCTAssertEqual(sidebar.maximumThickness, SettingsPaneLayout.sidebarWidth)
        XCTAssertEqual(split.splitViewItems.count, 2, "sidebar and detail, nothing else")
        // A divider position persisted for an immovable divider would be a
        // second, competing authority on the sidebar's width.
        XCTAssertNil(split.splitView.autosaveName)
    }

    /// The System Settings shape: one width, both directions, so the window
    /// resizes vertically only — including under the zoom button, which reads
    /// the same two limits.
    ///
    /// Asserted AFTER a layout pass, which is the only state that matters and
    /// the one an earlier version of this test missed: `NSHostingController`
    /// ships with `sizingOptions` containing `.minSize` and `.maxSize`, so at
    /// first layout it overwrites both limits with what SwiftUI measured —
    /// a window pinned only in `makeWindow` measured `min (283, 20)` and
    /// `max (∞, ∞)` a moment later, and could be dragged to any width.
    func testWindow_pinsItsWidthAndLeavesTheHeightFree() {
        let window = makeWindow(language: makeStore())
        window.contentView?.layoutSubtreeIfNeeded()
        window.layoutIfNeeded()

        XCTAssertEqual(window.contentMinSize.width, SettingsPaneLayout.contentWidth)
        XCTAssertEqual(window.contentMaxSize.width, SettingsPaneLayout.contentWidth)
        XCTAssertEqual(window.contentMinSize.height, SettingsPaneLayout.minimumContentHeight)
        XCTAssertGreaterThan(window.contentMaxSize.height, window.contentMinSize.height)
    }

    /// The titlebar names the selected pane, in the display language, written
    /// by the split controller — the SwiftUI `navigationTitle` bridge is gone
    /// with the `NavigationSplitView` it came from.
    func testWindow_titlesTheTitlebarWithTheSelectedPane() {
        let language = makeStore()
        SettingsStore().selectedSettingsPane = .shortcuts

        let window = makeWindow(language: language)
        window.contentViewController?.viewWillAppear()

        XCTAssertEqual(window.title, language.string(SettingsPane.shortcuts.labelKey))
    }

    /// The window follows the app's own 外觀 setting, not just the system's —
    /// the same choice the candidate window reads.
    func testWindow_followsTheAppearanceSetting() {
        let window = makeWindow(language: makeStore())

        window.contentViewController?.viewWillAppear()

        // Whatever the machine running this has stored, the window's override
        // has to match what the setting resolves to — including nil for 自動.
        XCTAssertEqual(window.appearance, SettingsStore().appearanceMode.forcedAppearance)
    }

    /// The window must survive being closed: it is cached so reopening returns
    /// the user to where they left it, and releasing it would make the second
    /// open a message to a freed object.
    func testWindow_isNotReleasedWhenClosed() {
        XCTAssertFalse(makeWindow(language: makeStore()).isReleasedWhenClosed)
    }

    /// With no autosaved frame (setUp clears it), a fresh window opens at the
    /// explicit initial size — not whatever `.zero` settles into. The width is
    /// exact because it is pinned; the height is `>=` because laying the
    /// content out can grow it.
    func testFreshWindow_opensAtTheInitialSize() {
        let window = makeWindow(language: makeStore())

        XCTAssertEqual(
            contentSize(of: window).width,
            SettingsPaneLayout.contentWidth,
            accuracy: 1,
        )
        XCTAssertGreaterThanOrEqual(
            contentSize(of: window).height,
            SettingsPaneLayout.initialContentHeight - 1,
        )
    }

    /// The real restore path: neither limit resizes a frame
    /// `setFrameAutosaveName` has just restored, so `makeWindow` has to apply
    /// them by hand afterwards. A frame this narrow is what a build with a
    /// smaller floor autosaved.
    ///
    /// One window per test, not a loop over staged frames: an autosave name
    /// binds to one live window at a time, so a second window built under the
    /// same name inside one test restores nothing and would test the fallback.
    func testWindow_growsANarrowAutosavedFrameToTheFloor() throws {
        try stageAutosavedFrame(width: 380, height: 300)

        let window = makeWindow(language: makeStore())

        let size = contentSize(of: window)
        XCTAssertEqual(size.width, SettingsPaneLayout.contentWidth, accuracy: 1)
        XCTAssertEqual(size.height, SettingsPaneLayout.minimumContentHeight, accuracy: 1)
    }

    /// The other direction, which the build before the width pin autosaved: a
    /// frame wider than the window may now be comes back in, and the height
    /// the user had dragged to survives it — `>` the floor rather than an
    /// exact number, which would pin the titlebar's height into the test.
    func testWindow_narrowsAWideAutosavedFrameAndKeepsItsHeight() throws {
        try stageAutosavedFrame(width: 1100, height: 700)

        let window = makeWindow(language: makeStore())

        let size = contentSize(of: window)
        XCTAssertEqual(size.width, SettingsPaneLayout.contentWidth, accuracy: 1)
        XCTAssertGreaterThan(size.height, SettingsPaneLayout.minimumContentHeight)
    }

    /// Stages an autosaved frame in the STANDARD defaults, where AppKit reads
    /// it back from; `tearDown` puts the developer's own value back.
    private func stageAutosavedFrame(width: Int, height: Int) throws {
        let screen = try XCTUnwrap(NSScreen.main).frame
        UserDefaults.standard.set(
            "100 100 \(width) \(height) 0 0 \(Int(screen.width)) \(Int(screen.height))",
            forKey: Self.frameAutosaveDefaultsKey,
        )
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
            ["一般", "外觀", "快速齒", "自訂詞庫", "辭典管理"],
        )

        let english = makeStore(.english)
        XCTAssertEqual(
            SettingsPane.allCases.map { english.string($0.labelKey) },
            [
                "General", "Appearance", "Shortcuts", "Custom Dictionary", "Manage Dictionaries",
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
