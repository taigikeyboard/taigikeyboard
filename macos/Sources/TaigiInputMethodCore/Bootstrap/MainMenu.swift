// The main menu an input-method process has to build for itself.

import AppKit

/// Builds `NSApp.mainMenu`.
///
/// An `LSUIElement` process shows no menu bar and this package has no nib to
/// load one from, so without this there is no main menu at all — and AppKit
/// dispatches keyboard shortcuts for the standard editing commands through the
/// main menu. No menu means the settings window cannot be closed with ⌘W and
/// any text field in it would have no ⌘X/⌘C/⌘V/⌘A. That is MacishType's stated
/// reason for building the same menu
/// (`references/MacishType/macos/MacishType/AppDelegate.swift:97-127`);
/// azooKey-Desktop builds one too, without saying why
/// (`references/azooKey-Desktop/azooKeyMac/AppDelegate.swift:114-132`).
///
/// The items carry no target: they act on the first responder, which is the
/// text field or window the user is actually in.
enum MainMenu {
    /// The menu the application runs with, ready to assign to `NSApp.mainMenu`.
    ///
    /// Deliberately has no Quit item. Quitting an input method mid-composition
    /// takes the user's keyboard away from them, and a ⌘Q aimed at the app they
    /// were typing in would land here whenever the settings window is key.
    static func make() -> NSMenu {
        let mainMenu = NSMenu()
        // The first submenu is the application menu by convention, whether or
        // not it has items; without it AppKit reads the File menu as one.
        mainMenu.addItem(submenu(NSMenu(title: appMenuTitle)))
        mainMenu.addItem(submenu(fileMenu()))
        mainMenu.addItem(submenu(editMenu()))
        return mainMenu
    }

    private static let appMenuTitle = "TaigiKeyboard"

    private static func fileMenu() -> NSMenu {
        let menu = NSMenu(title: String(localized: "檔案"))
        menu.addItem(
            withTitle: String(localized: "關閉"),
            action: #selector(NSWindow.performClose(_:)),
            keyEquivalent: "w",
        )
        return menu
    }

    private static func editMenu() -> NSMenu {
        let menu = NSMenu(title: String(localized: "編輯"))
        // Undo and redo are declared by `NSUndoManager`'s responder chain and
        // have no Swift-visible selector to name, unlike the four below.
        menu.addItem(withTitle: String(localized: "還原"), action: Selector(("undo:")), keyEquivalent: "z")
        menu.addItem(withTitle: String(localized: "重做"), action: Selector(("redo:")), keyEquivalent: "Z")
        menu.addItem(.separator())
        menu.addItem(withTitle: String(localized: "剪下"), action: #selector(NSText.cut(_:)), keyEquivalent: "x")
        menu.addItem(withTitle: String(localized: "拷貝"), action: #selector(NSText.copy(_:)), keyEquivalent: "c")
        menu.addItem(withTitle: String(localized: "貼上"), action: #selector(NSText.paste(_:)), keyEquivalent: "v")
        menu.addItem(
            withTitle: String(localized: "全選"),
            action: #selector(NSText.selectAll(_:)),
            keyEquivalent: "a",
        )
        return menu
    }

    /// A top-level menu bar entry. AppKit reads submenus from the item, not
    /// from the menu, so each one needs a carrier item whose title it takes.
    private static func submenu(_ menu: NSMenu) -> NSMenuItem {
        let item = NSMenuItem(title: menu.title, action: nil, keyEquivalent: "")
        item.submenu = menu
        return item
    }
}
