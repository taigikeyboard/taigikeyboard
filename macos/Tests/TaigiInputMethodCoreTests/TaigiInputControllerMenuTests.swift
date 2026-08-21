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

    /// The menu IS the shortcut roster: every row the 快捷鍵 pane draws, in the
    /// same order, each printing the key it currently answers to. The
    /// romanization CHOICE is deliberately absent — two checkmarked rows were a
    /// setting rather than a shortcut, and the same switch is here as an action
    /// with a key of its own (USER 2026-08-21).
    func testMenu_listsEveryShortcutAndNothingElse() throws {
        let titles = try menu().items.filter { !$0.isSeparatorItem }.map(\.title)

        // The literal oracle for this surface: the copy is the authored Hanji,
        // and it reads the same here as it does in the shortcut pane. The keys
        // are absent from the titles because AppKit lays them out from each
        // row's `keyEquivalent` — except the slot row, whose key is a range.
        XCTAssertEqual(titles, [
            "開啟設定", "切換 台羅/白話字", "切換 漢羅對調",
            "換下一个候選字", "換頂一个候選字", "後一頁候選字", "頭前一頁候選字",
            "選字鍵  ⌃1 – ⌃9",
            "送出選著的候選字", "送出原本拍的字", "直接送出漢字", "直接送出羅馬字",
        ])
        XCTAssertEqual(
            titles.count,
            ShortcutAction.allCases.count + ComposingAction.allCases.count + 1,
            "a shortcut the pane can set must be a shortcut the menu can show",
        )
    }

    /// Rules between the groups, never at either end: a leading or trailing
    /// separator draws a line with nothing under it. Four groups, so three.
    func testMenu_separatesTheGroups() throws {
        let items = try menu().items

        XCTAssertFalse(items.first?.isSeparatorItem ?? true, "no rule above the first row")
        XCTAssertFalse(items.last?.isSeparatorItem ?? true, "no rule below the last row")
        XCTAssertEqual(
            items.filter(\.isSeparatorItem).count,
            ShortcutAction.groups.count + ComposingAction.groups.count - 1,
            "one rule between each pair of groups",
        )
    }

    /// The composing rows carry their key as a real `keyEquivalent`, and lead to
    /// where it is set: those keys need a composition, and a menu is open when
    /// there is none.
    func testTheComposingRows_carryTheirKeyAndOpenTheShortcutPane() throws {
        let items = try menu().items.filter { !$0.isSeparatorItem }
        let confirm = try XCTUnwrap(items.first { $0.title == "送出選著的候選字" })

        XCTAssertEqual(confirm.keyEquivalent, "\r", "Return is what a fresh install commits with")
        XCTAssertEqual(confirm.keyEquivalentModifierMask, [])
        XCTAssertEqual(confirm.action, Self.openShortcutSettings)
    }

    /// The key is right-aligned and dimmed by AppKit, which is the ONLY way to
    /// get that layout here: this menu is vended to the system's text-input menu
    /// agent, and an `attributedTitle` does not survive the trip — the agent
    /// draws the plain title, which lays out as-is (dogfood 2026-08-21). So
    /// every row that has a chord claims it as a `keyEquivalent`.
    func testEveryRowWithAChord_claimsItAsAKeyEquivalent() throws {
        let expected: [String: (String, NSEvent.ModifierFlags)] = [
            "開啟設定": (",", [.control, .shift]),
            "切換 台羅/白話字": ("r", [.control, .command]),
            "切換 漢羅對調": ("h", [.control, .command]),
            "換下一个候選字": (" ", []),
            "換頂一个候選字": ("\t", .shift),
            "後一頁候選字": ("]", []),
            "頭前一頁候選字": ("[", []),
            "送出選著的候選字": ("\r", []),
            "送出原本拍的字": ("\r", .shift),
            "直接送出漢字": ("\r", .control),
            "直接送出羅馬字": ("\r", .option),
        ]

        for item in try menu().items where !item.isSeparatorItem {
            guard let (key, modifiers) = expected[item.title] else {
                XCTAssertTrue(
                    item.title.hasPrefix("選字鍵"),
                    "\(item.title) is not a row this case knows",
                )
                continue
            }
            XCTAssertEqual(item.keyEquivalent, key, item.title)
            XCTAssertEqual(item.keyEquivalentModifierMask, modifiers, item.title)
        }
    }

    /// An unset `keyEquivalentModifierMask` is ⌘, so a bare Space would draw —
    /// and answer to — ⌘Space.
    func testABareChord_claimsNoModifier() throws {
        let space = try XCTUnwrap(
            menu().items.first { $0.title == "換下一个候選字" },
        )

        XCTAssertEqual(space.keyEquivalentModifierMask, [])
    }

    /// The slot row is the one key AppKit cannot lay out: nine chords behind a
    /// single setting. It prints its range in the title instead.
    func testTheSlotRow_printsItsRangeInTheTitle() throws {
        let slots = try XCTUnwrap(
            menu().items.first { $0.title.hasPrefix("選字鍵") },
        )

        XCTAssertEqual(slots.title, "選字鍵  ⌃1 – ⌃9")
        XCTAssertEqual(slots.keyEquivalent, "")
    }

    /// Every row a user could click to find out where its key lives goes to the
    /// same place, including the one whose key is nine chords.
    func testEveryRowThatIsNotAGlobalAction_opensTheShortcutPane() throws {
        let globalActions = [Self.showPreferences, Self.toggleRomanization, Self.toggleTranslateSwapped]
        let rows = try menu().items.filter { item in
            !item.isSeparatorItem && !globalActions.contains { $0 == item.action }
        }

        XCTAssertEqual(rows.count, ComposingAction.allCases.count + 1)
        for row in rows {
            XCTAssertEqual(row.action, Self.openShortcutSettings, row.title)
        }
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
    private static let toggleRomanization = Selector(("toggleRomanizationFromMenu:"))
    private static let toggleTranslateSwapped = Selector(("toggleTranslateSwappedFromMenu:"))

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

    /// The system calls `menu()` every time the menu is drawn so the input
    /// method can reflect its current state (`IMKInputController.h:307-310`).
    /// A menu built once and cached would print the chords that were recorded
    /// when the process started.
    func testMenu_redrawnAfterRecording_showsTheNewChord() throws {
        // Started from empty rather than assumed empty: the library writes to
        // the test process's own defaults domain, which survives the run.
        KeyboardShortcuts.setShortcut(nil, for: .toggleRomanization)
        XCTAssertEqual(try item(action: Self.toggleRomanization, in: menu()).keyEquivalent, "")

        KeyboardShortcuts.setShortcut(.init(.r, modifiers: [.control, .option]), for: .toggleRomanization)

        let item = try item(action: Self.toggleRomanization, in: menu())
        XCTAssertEqual(item.keyEquivalent, "r")
        XCTAssertEqual(item.keyEquivalentModifierMask, [.control, .option])
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

    /// The menu rows run the same path their chords do, so a setting behaves
    /// the same whichever surface changed it.
    func testTheRomanizationRow_togglesTheModeLikeItsChordDoes() throws {
        XCTAssertEqual(controller.settings.inputMode, .tl)

        try select(Self.toggleRomanization)

        XCTAssertEqual(controller.settings.inputMode, .poj)

        try select(Self.toggleRomanization)

        XCTAssertEqual(controller.settings.inputMode, .tl)
    }

    func testTheSwapRow_togglesTheSettingLikeItsChordDoes() throws {
        XCTAssertFalse(controller.settings.isTranslateSwapped)

        try select(Self.toggleTranslateSwapped)

        XCTAssertTrue(controller.settings.isTranslateSwapped)
    }
}
