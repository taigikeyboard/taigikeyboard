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

    /// The floor under the window: the sidebar's minimum plus the widest
    /// pane's list content. One floor for the whole window — the sidebar
    /// shows every pane, so there is no per-tab size to switch between.
    static let minimumContentSize = NSSize(width: 760, height: 470)

    /// What a first launch opens at. Explicit rather than derived: a `.zero`
    /// window under a flexible `NavigationSplitView` is not guaranteed to
    /// settle on a sensible size on its own.
    static let initialContentSize = NSSize(width: 860, height: 560)

    /// Held rather than recreated, so reopening returns the user to the window
    /// where they left it instead of a fresh one in the middle of the screen.
    private var window: NSWindow?

    /// The window the settings pages are being shown in, for the file panels
    /// they open as sheets on it. `nil` before the window has ever been shown,
    /// which is a state no page can be visible in.
    var presentedWindow: NSWindow? {
        window
    }

    private init() {}

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

        window.makeKeyAndOrderFront(nil)
        // If activation was deferred or refused, `makeKeyAndOrderFront` orders
        // the window only within this app's own layer — where nothing else is —
        // and the user would be left looking at a window that never appeared.
        // The candidate panel needs the same call for the same reason
        // (`CandidatePanel.swift`).
        window.orderFrontRegardless()
    }

    /// Builds the window without showing it. `static` and internal so a test
    /// can inspect the chrome — style mask, content size, hosting root —
    /// without ordering a window in front of whoever is running the tests.
    ///
    /// No `window.title` is written here or anywhere: the hosting controller
    /// bridges the detail pane's `navigationTitle` into the titlebar, and a
    /// manual write would compete with it. That also covers language changes —
    /// the pane titles re-render from the store SwiftUI observes.
    static func makeWindow(language: DisplayLanguageStore) -> NSWindow {
        let window = NSWindow(
            contentRect: .zero,
            // `.resizable` since the dictionary panes carry lists; the floor
            // is `minimumContentSize`, applied below.
            styleMask: [.titled, .closable, .miniaturizable, .resizable],
            backing: .buffered,
            defer: false,
        )
        let hostingController = NSHostingController(
            rootView: SettingsRootView(
                stores: ComposingSessionCoordinator.shared.userDataStores,
                settingsProvider: SettingsStore(),
                language: language,
            ),
        )
        // Explicit rather than defaulted, pinning the contract: `.all` is what
        // carries the detail's `navigationTitle` into the titlebar, and any
        // toolbar content the split view declares into the window toolbar.
        hostingController.sceneBridgingOptions = .all
        window.contentViewController = hostingController
        // The default is to release the window when it closes, which would turn
        // the second open into a message to a freed object.
        window.isReleasedWhenClosed = false
        window.contentMinSize = minimumContentSize
        window.setContentSize(initialContentSize)
        window.center()
        // After `center()`: restoring a saved frame should win over centering,
        // and saving at all is what returns the user to the size they chose.
        window.setFrameAutosaveName("TaigiSettingsWindow")
        // Last, so it sees the restored frame: `contentMinSize` stops future
        // shrinking but does not grow a frame autosaved by a build with a
        // smaller floor — the tabbed window this layout replaced had one.
        growToMinimum(window)
        return window
    }

    /// Grows the window to the content floor if a restored frame sits below
    /// it, in either dimension. Never shrinks a size the user chose.
    /// Internal so a test can drive it with a deliberately small frame
    /// without staging an autosaved one in `UserDefaults`.
    static func growToMinimum(_ window: NSWindow) {
        let current = window.contentLayoutRect.size
        let grown = NSSize(
            width: max(current.width, minimumContentSize.width),
            height: max(current.height, minimumContentSize.height),
        )
        if grown != current {
            window.setContentSize(grown)
        }
    }
}
