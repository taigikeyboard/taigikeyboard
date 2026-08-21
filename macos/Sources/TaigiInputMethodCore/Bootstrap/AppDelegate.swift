// Application bootstrap: install guard, Rust logger sink, IMKServer.

import AppKit
import InputMethodKit

/// Starts the IMKServer that feeds `TaigiInputController`.
@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    /// Strong reference. `IMKServer` owns the Mach connection that delivers key
    /// events; letting it deallocate leaves a live-looking process that never
    /// receives input.
    private var server: IMKServer?

    private let logger = DebugLogger(category: "Bootstrap")

    func applicationDidFinishLaunching(_: Notification) {
        guard AppDelegate.isInputMethodsCopy(Bundle.main.bundleURL) else {
            // A build product carries the same bundle ID, so LaunchServices can
            // start it instead of the installed copy; the two then fight over
            // one IMKServer connection name and the loser gets no key events.
            logger.error(
                "launched from \(Bundle.main.bundleURL.path), not an Input Methods directory — exiting rather than competing with the installed copy",
            )
            exit(0)
        }

        RustEngineBridge.installLoggerSink()
        installLexiconEngine()
        // Opening is asynchronous, so this only starts it. A composition typed
        // before it finishes ranks without the user's history — one keystroke
        // ordered as it would be on a fresh install, which is why this runs at
        // launch rather than lazily on the first commit.
        ComposingSessionCoordinator.shared.openUserDataStores()

        // At launch rather than with the settings window: this is process-wide
        // AppKit configuration, and the menu has to exist before any window of
        // ours becomes key for its shortcuts to reach the first responder.
        //
        // The chrome AppKit copies rather than binds is re-rendered from HERE and nowhere else when
        // the display language changes (see behavioral-invariants.md §37, Rendering split). SwiftUI
        // needs nothing: it observes the store directly. Asserted single-assigner, because a second
        // one would silently replace this and freeze whichever surface it owned.
        assert(DisplayLanguageStore.shared.languageDidChange == nil)
        DisplayLanguageStore.shared.languageDidChange = Self.refreshLocalizedChrome
        Self.refreshLocalizedChrome()

        // Before the handlers register, so nothing re-persists what it clears.
        RetiredSettingsCleanup.run()

        // Before the first keystroke can read the bindings, so a user upgrading
        // mid-session does not type one composition under the old settings and
        // the next under the new ones.
        ComposingShortcutMigration.run()

        // The hotkey handlers exist for the process's life; whether they FIRE
        // is the coordinator's call, made as sessions register and release
        // their shortcut endpoint. Assigned here rather than defaulted inside
        // the coordinator so tests exercising it never touch Carbon.
        ShortcutHotkeys.registerHandlers()
        ComposingSessionCoordinator.shared.shortcutAvailabilityDidChange = ShortcutHotkeys.setEnabled

        server = IMKServer(
            name: Bundle.main.infoDictionary?["InputMethodConnectionName"] as? String,
            bundleIdentifier: Bundle.main.bundleIdentifier,
        )
        if server == nil {
            logger.error("IMKServer creation failed — this process receives no key events")
        }
    }

    /// Rebuilds the AppKit UI that reads its text once and keeps a copy.
    ///
    /// The main menu is rebuilt rather than relabelled — its items carry a selector, a key
    /// equivalent and a title, and none of that is state a rebuild can lose. The settings window
    /// needs nothing: its content is SwiftUI observing the store, and its titlebar follows the
    /// selected pane's `navigationTitle` through the hosting controller's scene bridging.
    @MainActor
    private static func refreshLocalizedChrome() {
        NSApp.mainMenu = MainMenu.make(DisplayLanguageStore.shared)
    }

    /// Loads the dictionary data the bundle ships with. Failures are logged and
    /// left alone: an uninstalled engine returns no candidates, which is a
    /// keyboard that types romanization but suggests nothing — far better than
    /// refusing to launch and leaving the user with no input method at all.
    private func installLexiconEngine() {
        guard let resourceURL = Bundle.main.resourceURL else {
            logger.error("bundle has no resource directory — lexicon not installed")
            return
        }
        do {
            let artifacts = try DictionaryArtifacts(baseURL: resourceURL)
            // The bundle version doubles as the dictionary stamp: the data is
            // rebuilt and re-bundled by the same release that bumps it.
            let version = (Bundle.main.infoDictionary?["CFBundleVersion"] as? String)
                .flatMap(UInt32.init) ?? 1
            guard let stats = RustEngineBridge.lexiconInstall(
                artifacts: artifacts,
                dictionaryVersion: version,
            ) else {
                logger.error("lexicon install returned no stats — engine not installed")
                return
            }
            logger.info(
                "lexicon installed: records=\(stats.dictionaryRecordCount) prefixEntries=\(stats.prefixIndexEntryCount) version=\(version)",
            )
        } catch {
            logger.error("lexicon not installed: \(error)")
        }
    }

    /// True when `bundleURL` sits directly inside an `Input Methods` directory,
    /// which is the only place an input method is meant to run from. Symlinks
    /// are resolved first so a symlinked install location still matches, and
    /// the check is on the parent directory so both `~/Library` and `/Library`
    /// qualify without hardcoding a home path (which the sandbox rewrites).
    /// `nonisolated`: a pure path test with no AppKit state, called from the
    /// launch guard and from tests that have no main actor to hop to.
    nonisolated static func isInputMethodsCopy(_ bundleURL: URL) -> Bool {
        bundleURL
            .resolvingSymlinksInPath()
            .standardizedFileURL
            .deletingLastPathComponent()
            .lastPathComponent == "Input Methods"
    }
}
