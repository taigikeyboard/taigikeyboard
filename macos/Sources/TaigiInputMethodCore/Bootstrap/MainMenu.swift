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
///
/// Titles come from the display-language resolver, so the menu follows the app's own language
/// picker. The assembled bundle carries `.lproj` directories, but they hold nothing except the
/// `InfoPlist.strings` naming the input source — this package ships no `Localizable.strings` for
/// AppKit to resolve a menu title from, so the alternative here is not "follows the system language"
/// but "frozen in one language forever".
@MainActor
enum MainMenu {
    /// The menu the application runs with, ready to assign to `NSApp.mainMenu`.
    ///
    /// Deliberately has no Quit item. Quitting an input method mid-composition
    /// takes the user's keyboard away from them, and a ⌘Q aimed at the app they
    /// were typing in would land here whenever the settings window is key.
    ///
    /// Rebuilt rather than relabelled when the language changes: the items carry no state — a
    /// selector, a key equivalent and a title each — so there is nothing a rebuild loses.
    static func make(_ language: DisplayLanguageStore) -> NSMenu {
        let mainMenu = NSMenu()
        // The first submenu is the application menu by convention, whether or
        // not it has items; without it AppKit reads the File menu as one.
        mainMenu.addItem(submenu(NSMenu(title: appMenuTitle)))
        mainMenu.addItem(submenu(fileMenu(language)))
        mainMenu.addItem(submenu(editMenu(language)))
        return mainMenu
    }

    /// The process name, not a localized string: it names the product in every language.
    private static let appMenuTitle = "TaigiKeyboard"

    private static func fileMenu(_ language: DisplayLanguageStore) -> NSMenu {
        let menu = NSMenu(title: language.string(.macosMenuFile))
        menu.addItem(
            withTitle: language.string(.macosMenuClose),
            action: #selector(NSWindow.performClose(_:)),
            keyEquivalent: "w",
        )
        return menu
    }

    private static func editMenu(_ language: DisplayLanguageStore) -> NSMenu {
        let menu = NSMenu(title: language.string(.macosMenuEdit))
        // Undo and redo are declared by `NSUndoManager`'s responder chain and
        // have no Swift-visible selector to name, unlike the four below.
        menu.addItem(withTitle: language.string(.macosMenuUndo), action: Selector(("undo:")), keyEquivalent: "z")
        menu.addItem(withTitle: language.string(.macosMenuRedo), action: Selector(("redo:")), keyEquivalent: "Z")
        menu.addItem(.separator())
        menu.addItem(withTitle: language.string(.macosMenuCut), action: #selector(NSText.cut(_:)), keyEquivalent: "x")
        menu.addItem(withTitle: language.string(.macosMenuCopy), action: #selector(NSText.copy(_:)), keyEquivalent: "c")
        menu.addItem(withTitle: language.string(.macosMenuPaste), action: #selector(NSText.paste(_:)), keyEquivalent: "v")
        menu.addItem(
            withTitle: language.string(.macosMenuSelectAll),
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
