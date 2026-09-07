import CoreText
import Foundation

/// Registers custom fonts at runtime when they are not in the current bundle.
///
/// Used by the keyboard extension to load fonts from the containing app bundle,
/// avoiding the need to duplicate font files in both targets.
enum FontRegistration {
    private static let logger = DebugLogger(category: "FontRegistration")

    /// File names, not PostScript names — the two stopped matching when the
    /// typefaces moved to the shared `fonts/font/` directory, whose names follow
    /// Android's resource-naming rules so all four platforms can read one copy.
    /// Ask `KeyboardFonts` for the PostScript name.
    private static let fontFileNames = [
        "jf_openhuninn_2_1.ttf",
        "iansui_regular.ttf",
        "genyomin2tw_r.otf",
        "genyogothic2tw_r.otf",
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
                logger.warning("[FONT] Not found: \(fileName)")
                continue
            }

            var error: Unmanaged<CFError>?
            let success = CTFontManagerRegisterFontsForURL(
                fontURL as CFURL,
                .process,
                &error,
            )

            if success {
                logger.debug("[FONT] Registered: \(fileName)")
            } else if let cfError = error?.takeRetainedValue() {
                // kCTFontManagerErrorAlreadyRegistered is expected on extension reuse
                let nsError = cfError as Error
                logger.debug("[FONT] \(fileName): \(nsError.localizedDescription)")
            }
        }
    }
}
