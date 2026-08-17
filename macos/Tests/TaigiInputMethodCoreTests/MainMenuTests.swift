import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// The main menu this process builds for itself, since it has no nib and shows
/// no menu bar. What it is for is the keyboard shortcuts: AppKit dispatches the
/// standard editing commands through it.
///
/// Items are found by selector, key equivalent or position — never by their titles, which now
/// follow the display language and would make every case here depend on which language it ran in.
@MainActor
final class MainMenuTests: XCTestCase {
    private var suiteName = ""
    private var userDefaults = UserDefaults.standard

    override func setUpWithError() throws {
        try super.setUpWithError()
        suiteName = "MainMenuTests.\(UUID().uuidString)"
        userDefaults = try XCTUnwrap(UserDefaults(suiteName: suiteName))
    }

    override func tearDown() {
        userDefaults.removePersistentDomain(forName: suiteName)
        super.tearDown()
    }

    private func makeMenu(_ language: DisplayLanguage = .hanji) -> NSMenu {
        MainMenu.make(TestFixtures.makeDisplayLanguageStore(language, userDefaults: userDefaults))
    }

    /// The menu bar's shape, which every other case indexes into: the application menu AppKit
    /// claims by position, then File, then Edit.
    private enum Position {
        static let file = 1
        static let edit = 2
    }

    private func submenu(at index: Int, in menu: NSMenu) throws -> NSMenu {
        try XCTUnwrap(menu.items[index].submenu, "no submenu at \(index)")
    }

    /// Every shortcut in `menu`, as the pair that has to be right for it to do
    /// anything: the keystroke, and the command it sends the first responder.
    /// Asserting the keystroke alone would pass against a Paste item wired to
    /// `cut(_:)`.
    private func shortcuts(in menu: NSMenu) -> [String: Selector?] {
        menu.items.reduce(into: [:]) { shortcuts, item in
            guard !item.keyEquivalent.isEmpty else { return }
            XCTAssertEqual(
                item.keyEquivalentModifierMask, [.command],
                "\(item.title) is not a ⌘ shortcut",
            )
            shortcuts[item.keyEquivalent] = item.action
        }
    }

    /// The window has a close button, but a window an input method opens over
    /// someone else's document is one the user will reach for ⌘W to dismiss.
    func testMainMenu_bindsCommandWToClosingTheWindow() throws {
        let file = try submenu(at: Position.file, in: makeMenu())

        let close = try XCTUnwrap(file.items.first { $0.keyEquivalent == "w" })
        XCTAssertEqual(close.action, #selector(NSWindow.performClose(_:)))
        XCTAssertEqual(close.keyEquivalentModifierMask, [.command])
    }

    /// Inert while the settings form is toggles and a picker, and the reason
    /// the menu is built at all once a text field lands in it.
    func testMainMenu_bindsTheStandardEditingShortcuts() throws {
        let edit = try submenu(at: Position.edit, in: makeMenu())

        XCTAssertEqual(
            shortcuts(in: edit),
            [
                // Undo and redo are the responder chain's own commands and have
                // no Swift-visible selector to name.
                "z": Selector(("undo:")),
                "Z": Selector(("redo:")),
                "x": #selector(NSText.cut(_:)),
                "c": #selector(NSText.copy(_:)),
                "v": #selector(NSText.paste(_:)),
                "a": #selector(NSText.selectAll(_:)),
            ],
        )
    }

    /// Nothing may target the first responder that is not a first-responder
    /// action: these items act on whatever text field or window the user is in,
    /// which is why none of them carries a target.
    func testMainMenu_leavesEveryItemToTheFirstResponder() {
        for submenu in makeMenu().items.compactMap(\.submenu) {
            for item in submenu.items {
                XCTAssertNil(item.target, "\(item.title) must act on the first responder")
            }
        }
    }

    /// ⌘Q while the settings window is key would quit the input method the user
    /// is typing with — and a ⌘Q meant for the app they were typing in lands
    /// here whenever our window has focus.
    func testMainMenu_hasNoQuitItem() {
        for submenu in makeMenu().items.compactMap(\.submenu) {
            for item in submenu.items {
                XCTAssertNotEqual(item.action, #selector(NSApplication.terminate(_:)))
                XCTAssertFalse(
                    item.keyEquivalent == "q" && item.keyEquivalentModifierMask == [.command],
                    "\(item.title) claims ⌘Q",
                )
            }
        }
    }

    /// AppKit reads the first submenu as the application menu whatever it is
    /// called, so a menu bar that opened with File would lose File's items to
    /// that role. The product name is not localized — it names the app in every language.
    func testMainMenu_leadsWithTheApplicationMenu() {
        XCTAssertEqual(makeMenu().items.first?.title, "TaigiKeyboard")
    }

    /// The literal oracle: the titles must be the authored copy for the language asked for, not
    /// merely whatever the resolver happens to answer — a production surface reading the wrong key
    /// would otherwise agree with an expectation that read the same wrong key.
    func testMainMenu_titlesAreTheAuthoredCopyForTheActiveLanguage() throws {
        let hanji = makeMenu(.hanji)
        XCTAssertEqual(hanji.items[Position.file].title, "檔案")
        XCTAssertEqual(hanji.items[Position.edit].title, "編輯")
        XCTAssertEqual(try submenu(at: Position.file, in: hanji).items.first?.title, "關閉")

        let english = makeMenu(.english)
        XCTAssertEqual(english.items[Position.file].title, "File")
        XCTAssertEqual(english.items[Position.edit].title, "Edit")
        XCTAssertEqual(try submenu(at: Position.file, in: english).items.first?.title, "Close")
    }

    /// Rebuilding is how the menu follows a language change, so a rebuild must carry the new
    /// language while leaving the commands exactly where they were.
    func testMainMenu_rebuiltUnderAnotherLanguage_keepsEveryCommandInPlace() throws {
        let store = TestFixtures.makeDisplayLanguageStore(.hanji, userDefaults: userDefaults)
        let before = MainMenu.make(store)

        store.setLanguage(.japanese)
        let after = MainMenu.make(store)

        XCTAssertNotEqual(after.items[Position.edit].title, before.items[Position.edit].title)
        XCTAssertEqual(after.items[Position.edit].title, "編集")
        XCTAssertEqual(
            try shortcuts(in: submenu(at: Position.edit, in: after)),
            try shortcuts(in: submenu(at: Position.edit, in: before)),
        )
    }
}
