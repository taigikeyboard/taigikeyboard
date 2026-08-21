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
    /// settings of whoever is running the tests. The whole roster is saved here
    /// and put back in `tearDown` — the menu prints every action's chord now, so
    /// a case can touch any of them.
    private var savedShortcuts: [ShortcutAction: KeyboardShortcuts.Shortcut?] = [:]

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "TaigiInputControllerMenuTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        controller = try TestFixtures.makeInputController()
        controller.settings = SettingsStore(userDefaults: userDefaults)
        // Pinned, so the menu's titles are the language this case asked for rather than the
        // language of whatever machine is running it.
        controller.displayLanguageOverride = TestFixtures.makeDisplayLanguageStore(.hanji, userDefaults: userDefaults)
        savedShortcuts = Dictionary(
            uniqueKeysWithValues: ShortcutAction.allCases.map {
                ($0, KeyboardShortcuts.getShortcut(for: $0.name))
            },
        )
        // Back to the shipped defaults, so a case that reads a row's chord sees
        // this build's default rather than whatever the machine running the
        // tests has recorded — the library has no suite to inject.
        KeyboardShortcuts.reset(ShortcutAction.allCases.map(\.name))
    }

    override func tearDown() {
        for (action, shortcut) in savedShortcuts {
            KeyboardShortcuts.setShortcut(shortcut, for: action.name)
        }
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

    /// Two doorways and nothing else (USER 2026-08-21): the settings window,
    /// and its shortcut pane. Actions and keys live behind those doors — the
    /// menu stopped being the shortcut roster when the agent proved unable to
    /// display a composing key without also dispatching it.
    func testMenu_hasExactlyTheTwoDoorways() throws {
        let items = try menu().items

        // The literal oracle for this surface: the copy is the authored Hanji.
        // 快捷鍵 reuses the pane's own name, the way a menu row that opens a
        // pane is named after it.
        XCTAssertEqual(items.map(\.title), ["開啟設定", "快捷鍵"])
        XCTAssertEqual(items.filter(\.isSeparatorItem).count, 0)
        XCTAssertEqual(items[0].action, Self.showPreferences)
        XCTAssertEqual(items[1].action, Self.openShortcutSettings)
    }

    /// The shortcut-pane row is a doorway, not a shortcut: it never claims a
    /// key equivalent. (The regression that retired the roster: anything the
    /// agent's key column can draw, typing can trigger — a bare Return sent
    /// every mid-composition Enter to the settings window, real device
    /// 2026-08-21.)
    func testTheShortcutPaneRow_claimsNoKeyEquivalent() throws {
        let row = try item(action: Self.openShortcutSettings, in: menu())

        XCTAssertEqual(row.keyEquivalent, "")
    }

    /// A bare function key IS a legal global chord (`KeyboardShortcuts` records
    /// F12 with no modifier), and it keeps its key equivalent.
    func testABareFunctionKeyGlobalChord_keepsItsKeyEquivalent() throws {
        KeyboardShortcuts.setShortcut(.init(.f12), for: .openSettings)

        let settingsItem = try item(action: Self.showPreferences, in: menu())

        XCTAssertFalse(settingsItem.keyEquivalent.isEmpty)
        XCTAssertEqual(settingsItem.keyEquivalentModifierMask, [])
    }

    /// Clicking one moves the window to the pane, so it opens where the key is
    /// rather than wherever it was left.
    func testOpeningTheShortcutPane_selectsIt() throws {
        controller.settings.selectedSettingsPane = .appearance

        try select(Self.openShortcutSettings)

        XCTAssertEqual(controller.settings.selectedSettingsPane, .shortcuts)
    }


    /// The menu is rebuilt on every draw, which is what lets it follow a language change with no
    /// refresh wiring of its own.
    func testMenu_redrawnAfterALanguageChange_readsInTheNewLanguage() throws {
        XCTAssertEqual(try item(action: Self.showPreferences, in: menu()).title, "開啟設定")

        try XCTUnwrap(controller.displayLanguageOverride).setLanguage(.english)

        XCTAssertEqual(try item(action: Self.showPreferences, in: menu()).title, "Open Settings")
    }

    private static let showPreferences = Selector(("showPreferences:"))
    private static let openShortcutSettings = Selector(("openShortcutSettings:"))

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

    /// A cleared shortcut claims no key equivalent at all — the menu item stays,
    /// the chord does not, and no host key is taken for a command the user
    /// unbound.
    func testMenu_claimsNoChordWhenTheShortcutIsCleared() throws {
        KeyboardShortcuts.setShortcut(nil, for: .openSettings)

        let settingsItem = try item(action: Self.showPreferences, in: menu())

        XCTAssertEqual(settingsItem.title, "開啟設定")
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
    /// dictionary is empty because each selector names its own action and
    /// reads nothing from the sender, which is the property being pinned — and
    /// because the real keys (`kIMKCommandMenuItemName`) are declared
    /// `extern const NSString*` and do not import into Swift.
    private func select(_ action: Selector) throws {
        let sent = try XCTUnwrap(item(action: action, in: menu()).action)
        controller.doCommand(by: sent, command: [:])
    }

}
