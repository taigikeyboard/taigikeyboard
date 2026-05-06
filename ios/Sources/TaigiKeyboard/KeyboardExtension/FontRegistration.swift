// 中文: 鍵盤擴充字型註冊器 — 從主 App bundle 載入自訂字型,避免同一份字型檔重複打包到 extension 內。

import CoreText
import Foundation

/// Registers custom fonts at runtime when they are not in the current bundle.
///
/// Used by the keyboard extension to load fonts from the containing app bundle,
/// avoiding the need to duplicate font files in both targets.
// 中文: 在 extension 啟動時動態註冊主 App bundle 內的字型。
enum FontRegistration {
    private static let logger = DebugLogger(category: "FontRegistration")

    private static let fontFileNames = [
        "jf-openhuninn-2.1.ttf",
        "Iansui-Regular.ttf",
        "GenYoMin2TW-R.otf",
        "GenYoGothic2TW-R.otf",
    ]

    /// Registers fonts from the containing app bundle.
    ///
    /// Call this early in the extension lifecycle (viewDidLoad)
    /// before any UI that uses custom fonts is rendered.
    /// No-op when running inside the main app (UIAppFonts handles it).
    // 中文: 把主 App bundle 的字型註冊到 process 內。在 extension viewDidLoad 早期呼叫;
    // 中文: 在主 App 執行時為 no-op(由 UIAppFonts 處理)。
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
