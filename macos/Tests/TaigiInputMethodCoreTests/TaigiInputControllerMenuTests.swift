import AppKit
import KeyboardShortcuts
@testable import TaigiInputMethodCore
import XCTest

/// The input-source menu: the only surface of this input method a user can
/// reach without typing into it.
@MainActor
final class TaigiInputControllerMenuTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard
    private var controller: TaigiInputController!

    /// `KeyboardShortcuts` stores in `UserDefaults.standard` and offers no
    /// suite injection, so a case that records a chord is writing into the
    /// settings of whoever is running the tests. Saved here and put back in
    /// `tearDown`.
    private var savedOpenSettingsShortcut: KeyboardShortcuts.Shortcut?

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "TaigiInputControllerMenuTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        controller = try TestFixtures.makeInputController()
        controller.settings = SettingsStore(userDefaults: userDefaults)
        // Pinned, so the menu's titles are the language this case asked for rather than the
        // language of whatever machine is running it.
        controller.displayLanguageOverride = TestFixtures.makeDisplayLanguageStore(.hanji, userDefaults: userDefaults)
        savedOpenSettingsShortcut = KeyboardShortcuts.getShortcut(for: .openSettings)
    }

    override func tearDown() {
        KeyboardShortcuts.setShortcut(savedOpenSettingsShortcut, for: .openSettings)
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    private func menu() throws -> NSMenu {
        try XCTUnwrap(controller.menu(), "the controller must offer an input-source menu")
    }

    /// Found by the command it sends, not by its title: the titles follow the display language, and
    /// a lookup by text would only pass in the language the case happened to be authored in.
    private func item(action: Selector, in menu: NSMenu) throws -> NSMenuItem {
        try XCTUnwrap(
            menu.items.first { $0.action == action },
            "no menu item sending \(action) — the menu was \(menu.items.map(\.title))",
        )
    }

    func testMenu_offersSettingsAndBothRomanizations() throws {
        let titles = try menu().items.filter { !$0.isSeparatorItem }.map(\.title)

        // The literal oracle for this surface: the copy is the authored Hanji, and the two
        // romanizations read the same here as they do in the settings form.
        XCTAssertEqual(titles, ["設定…", "台羅", "白話字"])
    }

    /// The menu is rebuilt on every draw, which is what lets it follow a language change with no
    /// refresh wiring of its own.
    func testMenu_redrawnAfterALanguageChange_readsInTheNewLanguage() throws {
        XCTAssertEqual(try item(action: Self.showPreferences, in: menu()).title, "設定…")

        try XCTUnwrap(controller.displayLanguageOverride).setLanguage(.english)

        XCTAssertEqual(try item(action: Self.showPreferences, in: menu()).title, "Settings…")
    }

    private static let showPreferences = Selector(("showPreferences:"))
    private static let selectTL = Selector(("selectInputModeTL:"))
    private static let selectPOJ = Selector(("selectInputModePOJ:"))

    /// The chord lives in the shortcut registry now, not in this file: a user
    /// who never opens the recorder still sees `Ctrl+Shift+,` because that is
    /// 開啟設定's initial value.
    func testMenu_showsTheInitialSettingsChord() throws {
        KeyboardShortcuts.reset(.openSettings)

        let settingsItem = try item(action: Self.showPreferences, in: menu())

        XCTAssertEqual(settingsItem.keyEquivalent, ",")
        XCTAssertEqual(settingsItem.keyEquivalentModifierMask, [.control, .shift])
    }

    /// The menu is rebuilt on every draw, which is what lets it show a chord
    /// the user recorded after the process started.
    func testMenu_showsARecordedSettingsChord() throws {
        KeyboardShortcuts.setShortcut(.init(.k, modifiers: [.control, .option]), for: .openSettings)

        let settingsItem = try item(action: Self.showPreferences, in: menu())

        XCTAssertEqual(settingsItem.keyEquivalent, "k")
        XCTAssertEqual(settingsItem.keyEquivalentModifierMask, [.control, .option])
    }

    /// A cleared shortcut claims no key equivalent at all — the menu item
    /// stays, the chord does not, and no host key is taken for a command the
    /// user unbound.
    func testMenu_claimsNoChordWhenTheShortcutIsCleared() throws {
        KeyboardShortcuts.setShortcut(nil, for: .openSettings)

        let settingsItem = try item(action: Self.showPreferences, in: menu())

        XCTAssertEqual(settingsItem.keyEquivalent, "")
    }

    /// ⌘, belongs to the application being typed into. A key equivalent claimed
    /// from here is taken away from the host for as long as this input source
    /// is selected, so claiming that one would cost the user their app's own
    /// settings shortcut.
    func testMenu_doesNotClaimCommandComma() throws {
        KeyboardShortcuts.reset(.openSettings)

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

        XCTAssertEqual(try item(action: Self.selectPOJ, in: menu).state, .on)
        XCTAssertEqual(try item(action: Self.selectTL, in: menu).state, .off)
    }

    /// The system calls `menu()` every time the menu is drawn so the input
    /// method can reflect its current state (`IMKInputController.h:307-310`).
    /// A menu built once and cached would show the mode that was in use when
    /// the process started.
    func testMenu_reflectsAModeChangedSinceTheLastTimeItWasDrawn() throws {
        XCTAssertEqual(try item(action: Self.selectTL, in: menu()).state, .on)

        controller.settings.inputMode = .poj

        XCTAssertEqual(try item(action: Self.selectTL, in: menu()).state, .off)
        XCTAssertEqual(try item(action: Self.selectPOJ, in: menu()).state, .on)
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
    private func select(_ action: Selector) throws {
        let sent = try XCTUnwrap(item(action: action, in: menu()).action)
        controller.doCommand(by: sent, command: [:])
    }

    func testSelectingARomanization_storesIt() throws {
        try select(Self.selectPOJ)

        XCTAssertEqual(controller.settings.inputMode, .poj)

        try select(Self.selectTL)

        XCTAssertEqual(controller.settings.inputMode, .tl)
    }
}
