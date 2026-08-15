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
        installLexiconEngine()

        server = IMKServer(
            name: Bundle.main.infoDictionary?["InputMethodConnectionName"] as? String,
            bundleIdentifier: Bundle.main.bundleIdentifier,
        )
        if server == nil {
            logger.error("IMKServer creation failed — this process receives no key events")
        }
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
                dictionaryVersion: version
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
    static func isInputMethodsCopy(_ bundleURL: URL) -> Bool {
        bundleURL
            .resolvingSymlinksInPath()
            .standardizedFileURL
            .deletingLastPathComponent()
            .lastPathComponent == "Input Methods"
    }
}
