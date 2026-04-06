import Foundation
import OSLog

/// Resolves the bundle containing dictionary resources.
///
/// Dictionary files (dictionary.db, dictionary.trie) live only in the
/// keyboard extension bundle to avoid duplication. When running inside
/// the main app, this resolver locates the embedded .appex bundle.
/// When running inside the extension, it returns the extension's own bundle.
enum ResourceBundleResolver {
    private static let logger = Logger(
        subsystem: LexiconConstants.Logging.subsystem,
        category: "ResourceBundleResolver",
    )

    /// Returns the bundle containing dictionary resources.
    ///
    /// Resolution order:
    /// 1. Extension's own bundle (when running as keyboard extension)
    /// 2. Embedded .appex inside the main app bundle
    /// 3. Falls back to Bundle(for:) for test targets
    static var dictionaryBundle: Bundle {
        // Fast path: extension has the files in its own bundle
        if Bundle.main.bundlePath.hasSuffix(".appex") {
            return Bundle.main
        }

        // Running in the main app: reach into the embedded extension
        let appexURL = Bundle.main.bundleURL
            .appendingPathComponent("PlugIns")
            .appendingPathComponent("TaigiKeyboardExtension.appex")

        if let appexBundle = Bundle(url: appexURL) {
            return appexBundle
        }

        // Fallback for unit tests or unexpected configurations
        #if DEBUG
            logger.warning("[RESOLVE] Could not locate extension bundle, falling back to Bundle(for:)")
        #endif
        return Bundle(for: DictionaryRepository.self)
    }
}
