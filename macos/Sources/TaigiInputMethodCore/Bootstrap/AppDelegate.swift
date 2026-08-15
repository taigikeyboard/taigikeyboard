// Application bootstrap: install guard, Rust logger sink, IMKServer.

import AppKit
import InputMethodKit

/// Starts the IMKServer that feeds `TaigiInputController`.
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

        server = IMKServer(
            name: Bundle.main.infoDictionary?["InputMethodConnectionName"] as? String,
            bundleIdentifier: Bundle.main.bundleIdentifier,
        )
        if server == nil {
            logger.error("IMKServer creation failed — this process receives no key events")
        }
    }

    /// True when `bundleURL` sits directly inside an `Input Methods` directory,
    /// which is the only place an input method is meant to run from. Symlinks
    /// are resolved first so a symlinked install location still matches, and
    /// the check is on the parent directory so both `~/Library` and `/Library`
    /// qualify without hardcoding a home path (which the sandbox rewrites).
    static func isInputMethodsCopy(_ bundleURL: URL) -> Bool {
        bundleURL
            .resolvingSymlinksInPath()
            .standardizedFileURL
            .deletingLastPathComponent()
            .lastPathComponent == "Input Methods"
    }
}
