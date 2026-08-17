// The window the settings form lives in, and the activation dance an
// input-method process needs to put it in front of the app being typed into.

import AppKit
import SwiftUI

/// Shows the one settings window this input method has.
///
/// A dedicated controller rather than a general window registry: there is
/// exactly one window, and a keyed collection would be a lookup table with one
/// entry (`references/MacishType/macos/MacishType/WindowManager.swift:5-18` is
/// the general version, and it exists because that project has three windows).
@MainActor
final class SettingsWindowController {
    /// Process-wide, because the window is: an input method has one settings
    /// window no matter how many client sessions are open.
    static let shared = SettingsWindowController()

    private static let logger = DebugLogger(category: "SettingsWindow")

    /// Held rather than recreated, so reopening returns the user to the window
    /// where they left it instead of a fresh one in the middle of the screen.
    private var window: NSWindow?

    /// The store the window was built with, so a later relabel reads the same one its tab
    /// controller does rather than whichever store the caller happens to hold.
    private var windowLanguage: DisplayLanguageStore?

    /// The window the settings pages are being shown in, for the file panels
    /// they open as sheets on it. `nil` before the window has ever been shown,
    /// which is a state no page can be visible in.
    var presentedWindow: NSWindow? {
        window
    }

    private init() {}

    /// Re-reads the chrome AppKit copied rather than bound — the window title and the tab labels.
    /// A window that was never shown has nothing to update.
    func refreshLocalizedChrome() {
        guard let window, let language = windowLanguage else { return }
        window.title = language.string(.macosWindowTitle)
        (window.contentViewController as? SettingsTabViewController)?.applyLocalizedLabels()
    }

    /// Brings the settings window up, creating it the first time.
    func show() {
        Self.logger.debug("show settings window")

        // Before the window is built or shown: under Automatic the persisted tag stays "system"
        // while the OS language can have changed underneath it, and nothing writes the key in that
        // case — so this is the refresh point that catches it.
        DisplayLanguageStore.shared.syncFromSettings()

        // Before anything is ordered in. This process is an `LSUIElement`
        // accessory that is never the active application, and a window ordered
        // in without activating first belongs to an app the user has not
        // switched to — it appears behind the document they are typing in.
        //
        // `activate()` only, never `activate(ignoringOtherApps:)`: the latter
        // is `API_DEPRECATED` in the SDK ("Use NSApp.activate instead") and
        // this target is macOS 14, where the replacement is available.
        // Activation is a request rather than a command, which is what the
        // `orderFrontRegardless()` below covers.
        NSApp.activate()

        let window = window ?? Self.makeWindow(language: DisplayLanguageStore.shared)
        self.window = window
        windowLanguage = DisplayLanguageStore.shared

        window.makeKeyAndOrderFront(nil)
        // If activation was deferred or refused, `makeKeyAndOrderFront` orders
        // the window only within this app's own layer — where nothing else is —
        // and the user would be left looking at a window that never appeared.
        // The candidate panel needs the same call for the same reason
        // (`CandidatePanel.swift`).
        window.orderFrontRegardless()
    }

    /// Builds the window without showing it. `static` and internal so a test
    /// can inspect the chrome — style mask, toolbar style, tab identity —
    /// without ordering a window in front of whoever is running the tests.
    static func makeWindow(language: DisplayLanguageStore) -> NSWindow {
        let window = NSWindow(
            contentRect: .zero,
            // `.resizable` since the 詞庫 tab carries lists; the floor per tab
            // is enforced by `SettingsTabViewController`, which only ever
            // grows the window and never touches this mask.
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false,
        )
        window.title = language.string(.macosWindowTitle)
        // The tab controller before the toolbar style: `.preference` acts on
        // the toolbar the tab controller attaches, and the SDK marks it "For
        // Settings windows only" (`NSWindow.h:239`).
        let tabController = SettingsTabViewController(language: language)
        window.contentViewController = tabController
        window.toolbarStyle = .preference
        // The default is to release the window when it closes, which would turn
        // the second open into a message to a freed object.
        window.isReleasedWhenClosed = false
        window.center()
        // After `center()`: restoring a saved frame should win over centering,
        // and saving at all is what returns the user to the size they chose
        // now that the window is resizable.
        window.setFrameAutosaveName("TaigiSettingsWindow")
        // Last, so it sees the restored frame: the first tab was selected
        // while the controller had no window, so nothing has put a floor under
        // the tab the window opens on yet — and an autosaved frame from a
        // narrower build could be below it.
        tabController.applyMinimumSizeForSelectedTab()
        return window
    }
}
