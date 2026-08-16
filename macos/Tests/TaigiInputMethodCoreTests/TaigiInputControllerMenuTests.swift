import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// The input-source menu: the only surface of this input method a user can
/// reach without typing into it.
@MainActor
final class TaigiInputControllerMenuTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard
    private var controller: TaigiInputController!

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "TaigiInputControllerMenuTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        controller = try TestFixtures.makeInputController()
        controller.settings = SettingsStore(userDefaults: userDefaults)
    }

    override func tearDown() {
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    private func menu() throws -> NSMenu {
        try XCTUnwrap(controller.menu(), "the controller must offer an input-source menu")
    }

    private func item(titled title: String, in menu: NSMenu) throws -> NSMenuItem {
        try XCTUnwrap(
            menu.items.first { $0.title == title },
            "no menu item titled \(title) — the menu was \(menu.items.map(\.title))",
        )
    }

    func testMenu_offersSettingsAndBothRomanizations() throws {
        let titles = try menu().items.filter { !$0.isSeparatorItem }.map(\.title)

        XCTAssertEqual(titles, ["設定…", "台羅 (TL)", "白話字 (POJ)"])
    }

    /// The chord is a menu key equivalent rather than anything in the key
    /// handler, which is what keeps `ComposingKeyIntent` and the keydown-only
    /// `recognizedEvents` mask out of this feature entirely.
    func testMenu_bindsSettingsToControlShiftComma() throws {
        let settingsItem = try item(titled: "設定…", in: menu())

        XCTAssertEqual(settingsItem.keyEquivalent, ",")
        XCTAssertEqual(settingsItem.keyEquivalentModifierMask, [.control, .shift])
    }

    /// ⌘, belongs to the application being typed into. A key equivalent claimed
    /// from here is taken away from the host for as long as this input source
    /// is selected, so claiming that one would cost the user their app's own
    /// settings shortcut.
    func testMenu_doesNotClaimCommandComma() throws {
        for item in try menu().items {
            XCTAssertFalse(
                item.keyEquivalent == "," && item.keyEquivalentModifierMask.contains(.command),
                "\(item.title) claims ⌘, which belongs to the host application",
            )
        }
    }

    func testMenu_checksTheRomanizationInUse() throws {
        controller.settings.inputMode = .poj

        let menu = try menu()

        XCTAssertEqual(try item(titled: "白話字 (POJ)", in: menu).state, .on)
        XCTAssertEqual(try item(titled: "台羅 (TL)", in: menu).state, .off)
    }

    /// The system calls `menu()` every time the menu is drawn so the input
    /// method can reflect its current state (`IMKInputController.h:307-310`).
    /// A menu built once and cached would show the mode that was in use when
    /// the process started.
    func testMenu_reflectsAModeChangedSinceTheLastTimeItWasDrawn() throws {
        XCTAssertEqual(try item(titled: "台羅 (TL)", in: menu()).state, .on)

        controller.settings.inputMode = .poj

        XCTAssertEqual(try item(titled: "台羅 (TL)", in: menu()).state, .off)
        XCTAssertEqual(try item(titled: "白話字 (POJ)", in: menu()).state, .on)
    }

    /// Automatic validation disables items whose action no responder claims,
    /// and IMK routes these through `doCommandBySelector:` rather than the
    /// responder chain — so left on, it would grey out the whole menu.
    func testMenu_doesNotAutoenableItems() throws {
        XCTAssertFalse(try menu().autoenablesItems)
    }

    /// Goes through the routing IMK really uses: a menu command arrives as
    /// `doCommandBySelector:commandDictionary:`, whose default implementation
    /// checks whether the controller responds to the selector and then performs
    /// it with the info dictionary as the sender — not the `NSMenuItem`, and not
    /// the responder chain (`IMKInputController.h:283-296`). Driving it this way
    /// is what proves a `private @objc` method is reachable at all. The
    /// dictionary is empty because the two selectors name their own mode and
    /// read nothing from the sender, which is the property being pinned — and
    /// because the real keys (`kIMKCommandMenuItemName`) are declared
    /// `extern const NSString*` and do not import into Swift.
    private func select(_ title: String) throws {
        let action = try XCTUnwrap(item(titled: title, in: menu()).action)
        controller.doCommand(by: action, command: [:])
    }

    func testSelectingARomanization_storesIt() throws {
        try select("白話字 (POJ)")

        XCTAssertEqual(controller.settings.inputMode, .poj)

        try select("台羅 (TL)")

        XCTAssertEqual(controller.settings.inputMode, .tl)
    }
}
