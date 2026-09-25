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
    /// and put back in `tearDown` — the menu prints the doorway's chord, and a
    /// case may clear or re-record any of them.
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
        // The doorway row ends in a window. Counted here rather than shown: an
        // input method's settings window ordered in mid-test lands in front of
        // whoever is running the tests.
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

    /// The global shortcuts a click can stand in for, then the doorway, then —
    /// past the rule — the two commands with somewhere to go rather than
    /// somewhere to be: the check, and the 關於 page the sidebar does not list
    /// (USER 2026-09-19: the menu is where a user looks up the chords they
    /// last recorded; USER 2026-09-20: 關於 lives only here). Each shortcut
    /// row carries the 快捷鍵 pane's own name for it; the doorway is
    /// 台語齒盤設定. The same rows as Windows and Linux: this literal is the one
    /// `taigi_desktop_core::keys::input_method_menu` asserts. No composing key appears here; the menu stopped
    /// being that roster when the agent proved unable to display one without
    /// also dispatching it.
    func testMenu_isTheGlobalShortcutsThenTheSettingsDoorwayThenCheckForUpdatesAndAbout() throws {
        let items = try menu().items

        // The literal oracle for this surface: the copy is the authored Hanji.
        XCTAssertEqual(
            items.map(\.title),
            ["切換台羅/白話字", "切換候選詞顯示", "", "台語齒盤設定", "", "檢查更新", "關於齒盤"],
        )
        XCTAssertEqual(items.filter(\.isSeparatorItem).count, 2)
        XCTAssertFalse(try XCTUnwrap(items.first).isSeparatorItem)
        XCTAssertFalse(try XCTUnwrap(items.last).isSeparatorItem)
        XCTAssertEqual(
            items.filter { !$0.isSeparatorItem }.map(\.action),
            [
                Self.toggleRomanization, Self.cycleCandidateDisplayMode, Self.openSettings, Self.checkForUpdates,
                Self.showAbout,
            ],
        )
    }

    /// The 漢羅對調 swap stays off the menu: its default is the bare backtick,
    /// and a bare key equivalent here would be eaten by the agent everywhere
    /// this input source is selected — so the row could never print the one
    /// chord it is known by. The picker stays off too: it needs the caret a
    /// click has no hold of. And the Telex guide: a row for the few who use
    /// the scheme is a row everyone else reads past (USER 2026-09-20).
    func testMenu_hasNoRowForTheSwapThePickerOrTheGuide() throws {
        let titles = try menu().items.map(\.title)

        for action in [ShortcutAction.toggleTranslateSwapped, .showSymbolPicker, .showTelexGuide] {
            XCTAssertFalse(try titles.contains(action.label(language())), "\(action)")
        }
    }

    /// Each shortcut row prints its recorded chord, like the doorway does.
    func testTheShortcutRows_printTheirDefaultChords() throws {
        let menu = try menu()

        for (action, key) in [(Self.toggleRomanization, "c"), (Self.cycleCandidateDisplayMode, "h")] {
            let row = try item(action: action, in: menu)
            XCTAssertEqual(row.keyEquivalent, key, "\(action)")
            XCTAssertEqual(row.keyEquivalentModifierMask, [.control, .command], "\(action)")
        }
    }

    /// A row re-recorded onto a bare key prints nothing: the Carbon hotkey
    /// behind it is armed only while a session holds the engine, but a key
    /// equivalent here is dispatched by the agent for as long as this input
    /// source is selected, and a bare letter claimed that way is a letter the
    /// user can no longer type. The pane still shows the key.
    func testAShortcutRow_onABareKey_claimsNoKeyEquivalent() throws {
        KeyboardShortcuts.setShortcut(.init(.z), for: .toggleRomanization)

        let row = try item(action: Self.toggleRomanization, in: menu())

        XCTAssertEqual(row.title, "切換台羅/白話字")
        XCTAssertEqual(row.keyEquivalent, "")
        XCTAssertEqual(row.keyEquivalentModifierMask, [])
    }

    /// A shortcut row follows a re-recording and a clearing the way the
    /// doorway does: the menu is rebuilt on every draw.
    func testAShortcutRow_printsTheRecordedChord_andNothingOnceCleared() throws {
        KeyboardShortcuts.setShortcut(.init(.j, modifiers: [.control, .option]), for: .cycleCandidateDisplayMode)
        let recorded = try item(action: Self.cycleCandidateDisplayMode, in: menu())
        XCTAssertEqual(recorded.keyEquivalent, "j")
        XCTAssertEqual(recorded.keyEquivalentModifierMask, [.control, .option])

        KeyboardShortcuts.setShortcut(nil, for: .cycleCandidateDisplayMode)
        let cleared = try item(action: Self.cycleCandidateDisplayMode, in: menu())
        XCTAssertEqual(cleared.title, "切換候選詞顯示")
        XCTAssertEqual(cleared.keyEquivalent, "")
        XCTAssertEqual(cleared.keyEquivalentModifierMask, [])
    }

    /// With no session armed the click goes nowhere, as the chord does: there
    /// is no composition for the switch to apply to.
    func testTheRomanizationRow_withNoSession_changesNothing() throws {
        XCTAssertEqual(controller.settings.inputMode, .tl)

        try select(Self.toggleRomanization)

        XCTAssertEqual(controller.settings.inputMode, .tl)
    }

    /// Clicking a shortcut row does what its chord does, through the same
    /// doorway: with the session armed, the romanization flips.
    func testTheRomanizationRow_switchesTheRomanization() throws {
        // Recorded rather than shown: a real flash is a panel ordered in
        // front of whoever is running the tests.
        controller.modeFlashOverride = { _ in }
        controller.activateServer(RecordingTextInputClient())
        defer { controller.deactivateServer(nil) }
        XCTAssertEqual(controller.settings.inputMode, .tl)

        try select(Self.toggleRomanization)

        XCTAssertEqual(controller.settings.inputMode, .poj)
    }

    private func language() throws -> DisplayLanguageStore {
        try XCTUnwrap(controller.displayLanguageOverride)
    }

    /// A row per pane, each named after its own sidebar row, from 2026-08-21
    /// until 2026-08-26. They went the way the ⌃⇧ chords behind them had
    /// (USER): five rows into one window are five names for the same thing, and
    /// naming the panes is the sidebar's job. Pinned so none returns by
    /// accident — each was a row that could take a host key.
    func testMenu_hasNoPerPaneRows() throws {
        let retired = [
            "openGeneralPane:", "openAppearancePane:", "openShortcutPane:",
            "openCustomDictionaryPane:", "openDictionarySourcesPane:",
        ].map { Selector(($0)) }

        for row in try menu().items {
            XCTAssertFalse(
                row.action.map(retired.contains) ?? false,
                "\(row.title) still opens a named pane straight from the menu",
            )
        }
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

    /// 關於 is a command too: no chord, same rule.
    func testTheAboutRow_claimsNoKeyEquivalent() throws {
        let row = try item(action: Self.showAbout, in: menu())

        XCTAssertEqual(row.keyEquivalent, "")
        XCTAssertEqual(row.keyEquivalentModifierMask, [])
    }

    /// The row opens the settings window on the one pane the sidebar does not
    /// list — the only way there.
    func testSelectingAbout_opensTheSettingsWindowOnTheAboutPane() throws {
        controller.settings.selectedSettingsPane = .customDictionary

        try select(Self.showAbout)

        XCTAssertEqual(controller.settings.selectedSettingsPane, .about)
        XCTAssertEqual(settingsShownCount, 1)
    }

    /// The general form of the rule above: only a global-shortcut row may
    /// claim a key. A row that claims one the user cannot see and re-record in
    /// the 快捷鍵 pane is a key taken from the host that no surface admits to.
    func testOnlyTheShortcutRows_claimAKey() throws {
        let shortcutRows = [Self.toggleRomanization, Self.cycleCandidateDisplayMode, Self.openSettings]
        for row in try menu().items where !row.keyEquivalent.isEmpty {
            XCTAssertTrue(
                row.action.map(shortcutRows.contains) ?? false,
                "\(row.title) claims \(row.keyEquivalent)",
            )
        }
    }

    /// The row prints the chord the 快捷鍵 pane holds for it right now, not the
    /// one it shipped with: the menu is rebuilt on every draw, which is what
    /// lets it follow a re-recording with no refresh wiring of its own.
    func testTheSettingsRow_printsTheRecordedChord() throws {
        KeyboardShortcuts.setShortcut(
            .init(.j, modifiers: [.control, .option]),
            for: .openLastSettingsPane,
        )

        let row = try item(action: Self.openSettings, in: menu())

        XCTAssertEqual(row.keyEquivalent, "j")
        XCTAssertEqual(row.keyEquivalentModifierMask, [.control, .option])
    }

    /// And nothing at all once the user has cleared that row: a key equivalent
    /// drawn here is taken from the host application for as long as this input
    /// source is selected, so an action with no chord must spend none.
    func testTheSettingsRow_claimsNoKeyWithNothingRecorded() throws {
        KeyboardShortcuts.setShortcut(nil, for: .openLastSettingsPane)

        let row = try item(action: Self.openSettings, in: menu())

        XCTAssertEqual(row.title, "台語齒盤設定")
        XCTAssertEqual(row.keyEquivalent, "")
        XCTAssertEqual(row.keyEquivalentModifierMask, [])
    }

    /// Clicking the row opens the window where the user left it, which is what
    /// the chord it prints does — a row that jumped to a fixed pane would land
    /// somewhere other than its own key.
    func testTheSettingsRow_reopensTheStoredPane() throws {
        controller.settings.selectedSettingsPane = .customDictionary

        try select(Self.openSettings)

        XCTAssertEqual(controller.settings.selectedSettingsPane, .customDictionary)
        XCTAssertEqual(settingsShownCount, 1)
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
            presentManualOutcome: { _ in },
        )

        try select(Self.checkForUpdates)

        XCTAssertEqual(controller.settings.selectedSettingsPane, .general)
        XCTAssertEqual(settingsShownCount, 1)
        wait(for: [fetched], timeout: 2)
    }

    /// The menu is rebuilt on every draw, which is what lets it follow a language change with no
    /// refresh wiring of its own.
    func testMenu_redrawnAfterALanguageChange_readsInTheNewLanguage() throws {
        XCTAssertEqual(try item(action: Self.openSettings, in: menu()).title, "台語齒盤設定")

        try XCTUnwrap(controller.displayLanguageOverride).setLanguage(.english)

        XCTAssertEqual(try item(action: Self.openSettings, in: menu()).title, "TaigiKeyboard Settings")
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

    private static let checkForUpdates = Selector(("checkForUpdates:"))
    private static let showAbout = Selector(("showAbout:"))
    private static let toggleRomanization = Selector(("toggleRomanization:"))
    private static let cycleCandidateDisplayMode = Selector(("cycleCandidateDisplayMode:"))
    /// The doorway's command: `showPreferences:` is the selector the system
    /// reserves for it (`IMKInputController.h:165-170`), so the row sends that
    /// rather than a second one of the controller's own.
    private static let openSettings = Selector(("showPreferences:"))

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
