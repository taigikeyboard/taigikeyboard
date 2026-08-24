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

    /// How many times a row asked for the settings window.
    private var settingsShownCount = 0

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "TaigiInputControllerMenuTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
        controller = try TestFixtures.makeInputController()
        controller.settings = SettingsStore(userDefaults: userDefaults)
        // Pinned, so the menu's titles are the language this case asked for rather than the
        // language of whatever machine is running it.
        controller.displayLanguageOverride = TestFixtures.makeDisplayLanguageStore(.hanji, userDefaults: userDefaults)
        // Every doorway row ends in a window. Counted here rather than shown:
        // an input method's settings window ordered in mid-test lands in front
        // of whoever is running the tests.
        controller.settingsPresenterOverride = { [weak self] in self?.settingsShownCount += 1 }
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

    /// Doorways and one command, in that order: every settings pane in sidebar
    /// order, then — past the rule — the check that has somewhere to go rather
    /// than somewhere to be. No composing key appears here; the menu stopped
    /// being the shortcut roster when the agent proved unable to display one
    /// without also dispatching it.
    func testMenu_isTheSettingsDoorwaysThenCheckForUpdates() throws {
        let items = try menu().items

        // The literal oracle for this surface: the copy is the authored Hanji.
        // Each pane row reuses the pane's own name, the way a row that opens a
        // pane is named after it.
        XCTAssertEqual(
            items.map(\.title),
            ["一般", "外觀", "快捷鍵", "自訂詞庫", "辭典管理", "", "檢查更新"],
        )
        XCTAssertEqual(items.filter(\.isSeparatorItem).count, 1)
        XCTAssertFalse(try XCTUnwrap(items.first).isSeparatorItem)
        XCTAssertFalse(try XCTUnwrap(items.last).isSeparatorItem)
        XCTAssertEqual(
            items.filter { !$0.isSeparatorItem }.map(\.action),
            Self.doorways.map(\.selector) + [Self.checkForUpdates],
        )
    }

    /// The pane rows read exactly what the sidebar reads, in exactly its
    /// order: a user who learns 外觀 in one surface must find it in the other,
    /// and a pane added to the sidebar without a row here would silently have
    /// no key.
    func testThePaneRows_matchTheSidebar() throws {
        let language = try XCTUnwrap(controller.displayLanguageOverride)
        let paneTitles = try menu().items
            .filter { row in
                Self.doorways.contains { $0.selector == row.action && $0.action.settingsPane != nil }
            }
            .map(\.title)

        XCTAssertEqual(paneTitles, SettingsPane.allCases.map { language.string($0.labelKey) })
    }

    /// 檢查更新 is a command, not a shortcut, and a key equivalent claimed here
    /// is taken from the host application for as long as this input source is
    /// selected. (The regression that retired the roster: anything the agent's
    /// key column can draw, typing can trigger — a bare Return sent every
    /// mid-composition Enter to the settings window, real device 2026-08-21.)
    func testTheCheckForUpdatesRow_claimsNoKeyEquivalent() throws {
        let row = try item(action: Self.checkForUpdates, in: menu())

        XCTAssertEqual(row.keyEquivalent, "")
        XCTAssertEqual(row.keyEquivalentModifierMask, [])
    }

    /// The general form of the rule above: only a doorway may claim a key. A
    /// row that claims one the user cannot see and re-record in the 快捷鍵
    /// pane is a key taken from the host that no surface admits to.
    func testOnlyTheDoorwayRows_claimAKey() throws {
        let doorwaySelectors = Set(Self.doorways.map(\.selector))

        for row in try menu().items where !row.keyEquivalent.isEmpty {
            XCTAssertTrue(
                row.action.map(doorwaySelectors.contains) ?? false,
                "\(row.title) claims \(row.keyEquivalent) but is not a recordable action",
            )
        }
    }

    /// Every row starts on the chord its action ships with — ⌃⇧ plus the
    /// pane's position in the sidebar.
    func testEveryDoorwayRow_showsItsActionsDefaultChord() throws {
        for doorway in Self.doorways {
            let row = try item(action: doorway.selector, in: menu())
            let shortcut = try XCTUnwrap(doorway.action.defaultShortcut, row.title)

            XCTAssertEqual(row.keyEquivalent, shortcut.nsMenuItemKeyEquivalent, row.title)
            XCTAssertEqual(row.keyEquivalentModifierMask, shortcut.modifiers, row.title)
        }
    }

    /// And every row prints what its action currently holds, not what it ships
    /// with: the menu is a view of the registry, rebuilt per draw, rather than
    /// a second copy of the defaults.
    func testAPaneRow_showsAChordRecordedAfterLaunch() throws {
        KeyboardShortcuts.setShortcut(
            .init(.j, modifiers: [.control, .option]),
            for: .openAppearancePane,
        )

        let row = try item(action: Self.openAppearancePane, in: menu())

        XCTAssertEqual(row.keyEquivalent, "j")
        XCTAssertEqual(row.keyEquivalentModifierMask, [.control, .option])
    }

    /// A bare function key IS a legal global chord (`KeyboardShortcuts` records
    /// F12 with no modifier), and it keeps its key equivalent.
    func testABareFunctionKeyGlobalChord_keepsItsKeyEquivalent() throws {
        KeyboardShortcuts.setShortcut(.init(.f12), for: .openAppearancePane)

        let row = try item(action: Self.openAppearancePane, in: menu())

        XCTAssertFalse(row.keyEquivalent.isEmpty)
        XCTAssertEqual(row.keyEquivalentModifierMask, [])
    }

    /// Clicking a pane row moves the window to that pane, so it opens where
    /// the key is rather than wherever it was left — and every row lands on
    /// its own pane, which is what a mis-paired selector would break.
    func testEveryPaneRow_selectsItsOwnPane() throws {
        for doorway in Self.doorways {
            guard let pane = doorway.action.settingsPane else { continue }
            controller.settings.selectedSettingsPane = pane == .general ? .appearance : .general

            try select(doorway.selector)

            XCTAssertEqual(controller.settings.selectedSettingsPane, pane)
        }

        XCTAssertEqual(settingsShownCount, SettingsPane.allCases.count)
    }

    /// The check needs the window it will answer in, so the row opens it — on
    /// 一般, where the update rows live — and only then starts the check. The
    /// other order would leave the answer with nowhere to appear, or would take
    /// the user's focus seconds after they had gone back to typing.
    ///
    /// The window is up by the time the command returns, while the fetch is
    /// still a task that has not run: asserting before the wait is what pins
    /// that order.
    func testCheckForUpdates_opensTheGeneralPaneThenStartsTheCheck() throws {
        let fetched = expectation(description: "the checker fetched")
        controller.settings.selectedSettingsPane = .customDictionary
        controller.updateCheckerOverride = UpdateChecker(
            settings: SettingsStore(userDefaults: userDefaults),
            installedVersionText: "3.6.5",
            fetchManifest: {
                fetched.fulfill()
                throw UpdateManifest.ManifestError.malformed
            },
            // Never the shipped presenter: it would put a sheet on a real
            // window belonging to whoever is running the tests.
            presentManualOutcome: { _, _ in },
        )

        try select(Self.checkForUpdates)

        XCTAssertEqual(controller.settings.selectedSettingsPane, .general)
        XCTAssertEqual(settingsShownCount, 1)
        wait(for: [fetched], timeout: 2)
    }

    /// The generic 開啟設定 row and its ⌃⇧, chord went away on 2026-08-24
    /// (USER): once every pane has a row, a row for "whichever pane was last
    /// used" is a second key for what 一般 already does. Pinned so it cannot
    /// come back by accident — a row here would take a host key again.
    func testMenu_hasNoOpenSettingsRow() throws {
        for row in try menu().items {
            XCTAssertNotEqual(row.action, Self.showPreferences, row.title)
        }
    }

    /// The override behind that retired row stays: it is the selector the
    /// system reserves for this command (`IMKInputController.h:165-170`), and
    /// a generic "preferences" command reopens where the user left off rather
    /// than jumping them to a pane they did not ask for.
    func testShowPreferences_stillOpensTheWindowOnTheStoredPane() {
        controller.settings.selectedSettingsPane = .customDictionary

        controller.showPreferences(nil)

        XCTAssertEqual(controller.settings.selectedSettingsPane, .customDictionary)
        XCTAssertEqual(settingsShownCount, 1)
    }

    /// The menu is rebuilt on every draw, which is what lets it follow a language change with no
    /// refresh wiring of its own.
    func testMenu_redrawnAfterALanguageChange_readsInTheNewLanguage() throws {
        XCTAssertEqual(try item(action: Self.openAppearancePane, in: menu()).title, "外觀")

        try XCTUnwrap(controller.displayLanguageOverride).setLanguage(.english)

        XCTAssertEqual(try item(action: Self.openAppearancePane, in: menu()).title, "Appearance")
    }

    private static let showPreferences = Selector(("showPreferences:"))
    private static let checkForUpdates = Selector(("checkForUpdates:"))
    private static let openAppearancePane = Selector(("openAppearancePane:"))

    /// The menu's first group, mirrored: the action each row sends and the
    /// selector IMK routes it by. A literal rather than a read of the
    /// controller's own table, so a row that moved would fail here instead of
    /// agreeing with itself. The pane is not restated — `settingsPane` is the
    /// one table for that, pinned by `ShortcutActionsTests`.
    private static let doorways: [(action: ShortcutAction, selector: Selector)] = [
        (.openGeneralPane, Selector(("openGeneralPane:"))),
        (.openAppearancePane, openAppearancePane),
        (.openShortcutPane, Selector(("openShortcutPane:"))),
        (.openCustomDictionaryPane, Selector(("openCustomDictionaryPane:"))),
        (.openDictionarySourcesPane, Selector(("openDictionarySourcesPane:"))),
    ]

    /// A cleared shortcut claims no key equivalent at all — the menu item stays,
    /// the chord does not, and no host key is taken for a command the user
    /// unbound.
    func testMenu_claimsNoChordWhenTheShortcutIsCleared() throws {
        KeyboardShortcuts.setShortcut(nil, for: .openAppearancePane)

        let row = try item(action: Self.openAppearancePane, in: menu())

        XCTAssertEqual(row.title, "外觀")
        XCTAssertEqual(row.keyEquivalent, "")
    }

    /// ⌘, belongs to the application being typed into. A key equivalent claimed
    /// from here is taken away from the host for as long as this input source
    /// is selected, so claiming that one would cost the user their app's own
    /// settings shortcut.
    func testMenu_doesNotClaimCommandComma() throws {
        KeyboardShortcuts.reset(ShortcutAction.allCases.map(\.name))

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
