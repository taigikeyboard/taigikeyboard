// 中文: 解析詞典資源所在的 Bundle。Keyboard extension 與主 app 進程都需要存取
// 中文: dictionary.bin / association.bin / dictionary.fst,且檔案只放在 .appex 裡。

import Foundation

/// Resolves the bundle containing dictionary resources.
///
/// Dictionary files (dictionary.bin, association.bin, dictionary.fst) live only in the
/// keyboard extension bundle to avoid duplication. When running inside
/// the main app, this resolver locates the embedded .appex bundle.
/// When running inside the extension, it returns the extension's own bundle.
// 中文: 詞典資源 Bundle 解析器 — 解決 extension / 主 app / 測試 三種執行情境的路徑差異。
enum ResourceBundleResolver {
    private static let logger = DebugLogger(category: "ResourceBundleResolver")

    /// Returns the bundle containing dictionary resources.
    ///
    /// Resolution order:
    /// 1. Extension's own bundle (when running as keyboard extension)
    /// 2. Embedded .appex inside the main app bundle
    /// 3. Falls back to Bundle(for:) for test targets
    // 中文: 詞典資源 Bundle — 依序嘗試 extension 自身 → 主 app 內嵌 .appex → Bundle(for:) 測試 fallback。
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

        // Fallback for unit tests or unexpected configurations.
        // `LexiconBitmaskBundleAnchor` is a private class colocated with the
        // dictionary resources in the same bundle, so `Bundle(for:)` resolves
        // to the right asset directory.
        logger.warning("[RESOLVE] Could not locate extension bundle, falling back to Bundle(for:)")
        return Bundle(for: type(of: LexiconBitmaskBundleAnchor()))
    }
}

/// Anchor class for `Bundle(for:)` — needs to be a class (not an enum), and
/// must live in the same bundle as the dictionary resources. Stays in this
/// file to keep the dependency local.
private final class LexiconBitmaskBundleAnchor {}
