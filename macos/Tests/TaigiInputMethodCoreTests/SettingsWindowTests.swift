// The settings window's chrome and the per-tab sizing rules.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// The settings window's chrome: the preferences-style tabs, and the sizing
/// rules that let one window hold both a fixed-width form and a list page.
@MainActor
final class SettingsWindowTests: XCTestCase {
    private func loadedTabController() -> SettingsTabViewController {
        let controller = SettingsTabViewController()
        // Tabs are built in `viewDidLoad`; touching `view` is what runs it.
        _ = controller.view
        return controller
    }

    func testTabController_showsGeneralAndDictionaryAsToolbarTabs() {
        let controller = loadedTabController()

        XCTAssertEqual(controller.tabStyle, .toolbar)
        XCTAssertEqual(controller.tabViewItems.map(\.label), ["一般", "詞庫"])
    }

    /// Toolbar tabs are icon-first: a label with no image renders as a bare
    /// word in the toolbar, and reaching for an emoji instead would bake a
    /// glyph into a string meant to be localizable text.
    func testTabController_givesEveryTabAnImage() {
        for item in loadedTabController().tabViewItems {
            XCTAssertNotNil(item.image, "tab \(item.label) has no toolbar image")
        }
    }

    func testWindow_isResizableWithPreferenceToolbarStyle() {
        let window = SettingsWindowController.makeWindow()

        XCTAssertTrue(window.styleMask.contains(.resizable))
        XCTAssertEqual(window.toolbarStyle, .preference)
        XCTAssertTrue(window.contentViewController is SettingsTabViewController)
    }

    /// The window must survive being closed: it is cached so reopening returns
    /// the user to where they left it, and releasing it would make the second
    /// open a message to a freed object.
    func testWindow_isNotReleasedWhenClosed() {
        XCTAssertFalse(SettingsWindowController.makeWindow().isReleasedWhenClosed)
    }

    private func tabController(of window: NSWindow) throws -> SettingsTabViewController {
        try XCTUnwrap(window.contentViewController as? SettingsTabViewController)
    }

    /// The tab the window OPENS on gets its floor too. Nothing selects it —
    /// it is current from the moment the tabs are built, which happens before
    /// the controller has a window to put a floor on.
    func testFreshWindow_alreadyCarriesTheGeneralTabsFloor() {
        let window = SettingsWindowController.makeWindow()

        let expected = SettingsTabViewController.ContentTab.general.minimumContentSize
        XCTAssertEqual(window.contentMinSize, expected)
        XCTAssertGreaterThanOrEqual(window.contentLayoutRect.size.width, expected.width)
        XCTAssertGreaterThanOrEqual(window.contentLayoutRect.size.height, expected.height)
    }

    /// Selecting a tab raises the floor to that tab's minimum, and grows a
    /// window that sits below it — in both dimensions. The 詞庫 tab's floor is
    /// the larger one.
    func testSelectingDictionaryTab_growsTheWindowToItsMinimum() throws {
        let window = SettingsWindowController.makeWindow()
        let controller = try tabController(of: window)
        window.setContentSize(NSSize(width: 200, height: 200))

        controller.selectedTabViewItemIndex = SettingsTabViewController.ContentTab.dictionary.rawValue

        let expected = SettingsTabViewController.ContentTab.dictionary.minimumContentSize
        XCTAssertEqual(window.contentMinSize, expected)
        XCTAssertGreaterThanOrEqual(window.contentLayoutRect.size.width, expected.width)
        XCTAssertGreaterThanOrEqual(window.contentLayoutRect.size.height, expected.height)
    }

    /// Switching back must not shrink a window the user has sized: the floor
    /// drops to the 一般 tab's, the frame stays where the user left it.
    func testSelectingGeneralTab_dropsTheFloorAndLeavesALargerWindowAlone() throws {
        let window = SettingsWindowController.makeWindow()
        let controller = try tabController(of: window)
        controller.selectedTabViewItemIndex = SettingsTabViewController.ContentTab.dictionary.rawValue
        window.setContentSize(NSSize(width: 900, height: 700))

        controller.selectedTabViewItemIndex = SettingsTabViewController.ContentTab.general.rawValue

        XCTAssertEqual(window.contentMinSize, SettingsTabViewController.ContentTab.general.minimumContentSize)
        XCTAssertEqual(window.contentLayoutRect.size.width, 900, accuracy: 1)
        XCTAssertEqual(window.contentLayoutRect.size.height, 700, accuracy: 1)
    }
}
