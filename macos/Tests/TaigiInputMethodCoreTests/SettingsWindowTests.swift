// The settings window's chrome and the per-tab sizing rules.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// The settings window's chrome: the preferences-style tabs, and the sizing
/// rules that let one window hold both a fixed-width form and a list page.
@MainActor
final class SettingsWindowTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "SettingsWindowTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
    }

    override func tearDown() {
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    private func makeStore(_ language: DisplayLanguage = .hanji) -> DisplayLanguageStore {
        TestFixtures.makeDisplayLanguageStore(language, userDefaults: userDefaults)
    }

    private func loadedTabController(_ language: DisplayLanguage = .hanji) -> SettingsTabViewController {
        loadedTabController(store: makeStore(language))
    }

    private func loadedTabController(store: DisplayLanguageStore) -> SettingsTabViewController {
        let controller = SettingsTabViewController(language: store)
        // Tabs are built in `viewDidLoad`; touching `view` is what runs it.
        _ = controller.view
        return controller
    }

    /// The window title is read once at build time, so a language change has to come back through
    /// `refreshLocalizedChrome()` — the callback `AppDelegate` installs is what calls it.
    func testWindow_titleFollowsTheDisplayLanguage() {
        let store = makeStore(.hanji)
        let window = SettingsWindowController.makeWindow(language: store)
        XCTAssertEqual(window.title, "台語鍵盤設定")

        let english = SettingsWindowController.makeWindow(language: makeStore(.english))
        XCTAssertEqual(english.title, "TaigiKeyboard Settings")
    }

    func testTabController_showsGeneralAndDictionaryAsToolbarTabs() {
        let controller = loadedTabController()

        XCTAssertEqual(controller.tabStyle, .toolbar)
        XCTAssertEqual(controller.tabViewItems.map(\.label), ["一般", "詞庫"])
        XCTAssertEqual(loadedTabController(.english).tabViewItems.map(\.label), ["General", "Dictionary"])
    }

    /// VoiceOver reads the toolbar image's description, not the tab's label, so a tab whose label
    /// followed the language while its image did not would go on announcing the old one.
    func testTabController_keepsTheToolbarImageDescriptionInStepWithTheLabel() {
        for item in loadedTabController(.english).tabViewItems {
            XCTAssertEqual(item.image?.accessibilityDescription, item.label)
        }
    }

    /// The labels are copied into AppKit rather than bound, so a language change has to come back
    /// through the controller — this is the case that fails if that call is ever dropped.
    func testTabController_relabelsItselfWhenTheLanguageChanges() {
        let store = makeStore(.hanji)
        let controller = loadedTabController(store: store)
        XCTAssertEqual(controller.tabViewItems.map(\.label), ["一般", "詞庫"])

        store.setLanguage(.japanese)
        controller.applyLocalizedLabels()

        XCTAssertEqual(controller.tabViewItems.map(\.label), ["一般", "辞書"])
        XCTAssertEqual(controller.tabViewItems.first?.image?.accessibilityDescription, "一般")
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
        let window = SettingsWindowController.makeWindow(language: makeStore())

        XCTAssertTrue(window.styleMask.contains(.resizable))
        XCTAssertEqual(window.toolbarStyle, .preference)
        XCTAssertTrue(window.contentViewController is SettingsTabViewController)
    }

    /// The window must survive being closed: it is cached so reopening returns
    /// the user to where they left it, and releasing it would make the second
    /// open a message to a freed object.
    func testWindow_isNotReleasedWhenClosed() {
        XCTAssertFalse(SettingsWindowController.makeWindow(language: makeStore()).isReleasedWhenClosed)
    }

    private func tabController(of window: NSWindow) throws -> SettingsTabViewController {
        try XCTUnwrap(window.contentViewController as? SettingsTabViewController)
    }

    /// The tab the window OPENS on gets its floor too. Nothing selects it —
    /// it is current from the moment the tabs are built, which happens before
    /// the controller has a window to put a floor on.
    func testFreshWindow_alreadyCarriesTheGeneralTabsFloor() {
        let window = SettingsWindowController.makeWindow(language: makeStore())

        let expected = SettingsTabViewController.ContentTab.general.minimumContentSize
        XCTAssertEqual(window.contentMinSize, expected)
        XCTAssertGreaterThanOrEqual(window.contentLayoutRect.size.width, expected.width)
        XCTAssertGreaterThanOrEqual(window.contentLayoutRect.size.height, expected.height)
    }

    /// Selecting a tab raises the floor to that tab's minimum, and grows a
    /// window that sits below it — in both dimensions. The 詞庫 tab's floor is
    /// the larger one.
    func testSelectingDictionaryTab_growsTheWindowToItsMinimum() throws {
        let window = SettingsWindowController.makeWindow(language: makeStore())
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
        let window = SettingsWindowController.makeWindow(language: makeStore())
        let controller = try tabController(of: window)
        controller.selectedTabViewItemIndex = SettingsTabViewController.ContentTab.dictionary.rawValue
        window.setContentSize(NSSize(width: 900, height: 700))

        controller.selectedTabViewItemIndex = SettingsTabViewController.ContentTab.general.rawValue

        XCTAssertEqual(window.contentMinSize, SettingsTabViewController.ContentTab.general.minimumContentSize)
        XCTAssertEqual(window.contentLayoutRect.size.width, 900, accuracy: 1)
        XCTAssertEqual(window.contentLayoutRect.size.height, 700, accuracy: 1)
    }
}
