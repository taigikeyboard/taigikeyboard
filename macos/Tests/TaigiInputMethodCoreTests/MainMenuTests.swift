import AppKit
@testable import TaigiInputMethodCore
import XCTest

/// The main menu this process builds for itself, since it has no nib and shows
/// no menu bar. What it is for is the keyboard shortcuts: AppKit dispatches the
/// standard editing commands through it.
final class MainMenuTests: XCTestCase {
    private func submenu(titled title: String, in menu: NSMenu) throws -> NSMenu {
        try XCTUnwrap(
            menu.items.first { $0.title == title }?.submenu,
            "no \(title) menu — the menu bar was \(menu.items.map(\.title))",
        )
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
        let file = try submenu(titled: String(localized: "檔案"), in: MainMenu.make())

        let close = try XCTUnwrap(file.items.first { $0.keyEquivalent == "w" })
        XCTAssertEqual(close.action, #selector(NSWindow.performClose(_:)))
        XCTAssertEqual(close.keyEquivalentModifierMask, [.command])
    }

    /// Inert while the settings form is toggles and a picker, and the reason
    /// the menu is built at all once a text field lands in it.
    func testMainMenu_bindsTheStandardEditingShortcuts() throws {
        let edit = try submenu(titled: String(localized: "編輯"), in: MainMenu.make())

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
        let mainMenu = MainMenu.make()

        for submenu in mainMenu.items.compactMap(\.submenu) {
            for item in submenu.items {
                XCTAssertNil(item.target, "\(item.title) must act on the first responder")
            }
        }
    }

    /// ⌘Q while the settings window is key would quit the input method the user
    /// is typing with — and a ⌘Q meant for the app they were typing in lands
    /// here whenever our window has focus.
    func testMainMenu_hasNoQuitItem() {
        let mainMenu = MainMenu.make()

        for submenu in mainMenu.items.compactMap(\.submenu) {
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
    /// that role.
    func testMainMenu_leadsWithTheApplicationMenu() {
        let mainMenu = MainMenu.make()

        XCTAssertEqual(mainMenu.items.first?.title, "TaigiKeyboard")
        XCTAssertEqual(
            mainMenu.items.map(\.title),
            ["TaigiKeyboard", String(localized: "檔案"), String(localized: "編輯")],
        )
    }
}
