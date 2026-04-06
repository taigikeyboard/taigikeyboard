import CoreText
import Foundation
import OSLog

/// Registers custom fonts at runtime when they are not in the current bundle.
///
/// Used by the keyboard extension to load fonts from the containing app bundle,
/// avoiding the need to duplicate font files in both targets.
enum FontRegistration {
    private static let logger = Logger(
        subsystem: LexiconConstants.Logging.subsystem,
        category: "FontRegistration",
    )

    private static let fontFileNames = [
        "jf-openhuninn-2.1.ttf",
        "Iansui-Regular.ttf",
    ]

    /// Registers fonts from the containing app bundle.
    ///
    /// Call this early in the extension lifecycle (viewDidLoad)
    /// before any UI that uses custom fonts is rendered.
    /// No-op when running inside the main app (UIAppFonts handles it).
    static func registerFontsIfNeeded() {
        guard Bundle.main.bundlePath.hasSuffix(".appex") else {
            return
        }

        // Navigate from .appex to containing app
        let containingAppURL = Bundle.main.bundleURL
            .deletingLastPathComponent() // PlugIns/
            .deletingLastPathComponent() // TaigiKeyboard.app/

        for fileName in fontFileNames {
            let fontURL = containingAppURL.appendingPathComponent(fileName)

            guard FileManager.default.fileExists(atPath: fontURL.path) else {
                #if DEBUG
                    logger.warning("[FONT] Not found: \(fileName, privacy: .public)")
                #endif
                continue
            }

            var error: Unmanaged<CFError>?
            let success = CTFontManagerRegisterFontsForURL(
                fontURL as CFURL,
                .process,
                &error,
            )

            #if DEBUG
                if success {
                    logger.debug("[FONT] Registered: \(fileName, privacy: .public)")
                } else if let cfError = error?.takeRetainedValue() {
                    // kCTFontManagerErrorAlreadyRegistered is expected on extension reuse
                    let nsError = cfError as Error
                    logger.debug("[FONT] \(fileName, privacy: .public): \(nsError.localizedDescription, privacy: .public)")
                }
            #endif
        }
    }
}
