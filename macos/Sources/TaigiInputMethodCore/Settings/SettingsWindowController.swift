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

    /// Keeps the window's light/dark override following the 外觀 setting.
    /// A window-level fact, so it is owned here with the rest of them —
    /// the split controller owns what is inside the window, not its chrome.
    private var appearanceObservation: AnyObject?

    /// The window a sheet belongs on, or `nil` when there is none to put one
    /// on: before the window has first been shown, and after it is closed.
    ///
    /// Both halves matter. The window is held rather than released when it
    /// closes, so "there is a window" and "the user can see it" are different
    /// questions — and a sheet on a closed window is one nobody answers. Every
    /// sheet this app raises asks here (`UserDataFilePanels.withSettingsWindow`,
    /// `UpdateAlertPresenter`), so which window that is stays one fact.
    var windowForSheets: NSWindow? {
        guard let window, window.isVisible else { return nil }
        return window
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
        if self.window == nil {
            // Once, on the window this instance keeps: `makeWindow` is static
            // so a test can build a window without going through the shared
            // controller, and an observation belongs to whoever holds the
            // window it updates.
            appearanceObservation = SettingsStore().observeChanges(of: SettingsStore.Keys.appearanceMode) {
                // Fires on whichever thread wrote the value, and carries none —
                // hop, then re-read.
                Task { @MainActor in Self.applyAppearance(to: window) }
            }
        }
        self.window = window

        window.makeKeyAndOrderFront(nil)
        // After the window is up, and only here: the notification offer needs a
        // frontmost app to be seen, and this is the one moment this process is
        // frontmost because the user put it there rather than because it
        // interrupted them. Gated before the task, so every later open costs
        // one defaults read instead of an allocation and a hop.
        let settings = SettingsStore()
        if UpdateNotificationOffer.isPending(in: settings) {
            Task { await UpdateNotificationOffer.offerIfNeeded(in: settings, parent: window) }
        }
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
    /// The titlebar is bound, not written: `SettingsSplitViewController` names
    /// the pane it is showing in its own `title`, and `NSWindow` documents this
    /// binding as the way that reaches the titlebar. So nothing here has to be
    /// told when the pane or the display language changes.
    static func makeWindow(language: DisplayLanguageStore) -> NSWindow {
        let window = NSWindow(
            contentRect: .zero,
            // `.resizable` since the dictionary panes carry lists and the
            // window has to grow taller for them; the width is pinned below,
            // so what stays resizable is the height.
            //
            // `.fullSizeContentView` with a transparent titlebar is what lets
            // the sidebar's material run the full height of the window, the
            // way System Settings' does. `NSSplitViewItem`'s
            // `allowsFullHeightLayout` is on by default but only takes effect
            // under this style mask.
            styleMask: [.titled, .closable, .miniaturizable, .resizable, .fullSizeContentView],
            backing: .buffered,
            defer: false,
        )
        window.titlebarAppearsTransparent = true
        let splitViewController = SettingsSplitViewController(
            stores: ComposingSessionCoordinator.shared.userDataStores,
            language: language,
        )
        window.contentViewController = splitViewController
        window.bind(.title, to: splitViewController, withKeyPath: "title")
        applyAppearance(to: window)
        // The default is to release the window when it closes, which would turn
        // the second open into a message to a freed object.
        window.isReleasedWhenClosed = false
        // The window is the single authority on its own width now: the two
        // columns are child hosting controllers, and the SwiftUI write-back
        // that used to overwrite these limits only happens for a hosting view
        // used AS the window's content view. `.greatestFiniteMagnitude` for
        // the height, which has no ceiling.
        window.contentMinSize = NSSize(
            width: SettingsPaneLayout.contentWidth, height: SettingsPaneLayout.minimumContentHeight,
        )
        window.contentMaxSize = NSSize(width: SettingsPaneLayout.contentWidth, height: .greatestFiniteMagnitude)
        window.setContentSize(NSSize(
            width: SettingsPaneLayout.contentWidth, height: SettingsPaneLayout.initialContentHeight,
        ))
        window.center()
        // After `center()`: restoring a saved frame should win over centering,
        // and saving at all is what returns the user to the height they chose.
        window.setFrameAutosaveName("TaigiSettingsWindow")
        // Last, so it sees the restored frame: `contentMinSize`/`contentMaxSize`
        // bound what the user can DRAG the window to, and neither one resizes a
        // frame `setFrameAutosaveName` has just restored. The split view keeps
        // no autosave name of its own — one would persist a divider position
        // for a divider that cannot move.
        applyContentSizeLimits(window)
        return window
    }

    /// Puts the 外觀 setting on the window: 淺色 / 深色 force it, 自動 leaves
    /// it nil, which is an `NSWindow` resolving against the system.
    static func applyAppearance(to window: NSWindow) {
        let appearance = SettingsStore().appearanceMode.forcedAppearance
        if window.appearance != appearance {
            window.appearance = appearance
        }
    }

    /// Puts a restored frame inside the limits `makeWindow` declares: the
    /// width to `contentWidth` whichever side of it the frame sits on, and the
    /// height up to the floor if it is under it. Never shrinks a height the
    /// user chose.
    ///
    /// Both width directions matter, and each has shipped: a build with a
    /// smaller floor autosaved a narrower frame, and the build before this one
    /// let the window be dragged wider than `contentWidth`.
    ///
    /// Internal so a test can drive it with a deliberately off-size frame
    /// without staging an autosaved one in `UserDefaults`.
    static func applyContentSizeLimits(_ window: NSWindow) {
        // `contentRect(forFrameRect:)`, NOT `contentLayoutRect`: under
        // `.fullSizeContentView` the layout rect excludes the titlebar, so
        // reading it here would shrink the window by a titlebar's height on
        // every call — and `setContentSize` speaks the other measure.
        let currentHeight = window.contentRect(forFrameRect: window.frame).height
        window.setContentSize(NSSize(
            width: SettingsPaneLayout.contentWidth,
            height: max(currentHeight, SettingsPaneLayout.minimumContentHeight),
        ))
    }
}
