import Foundation
import KeyboardKit

class SharedSettings {
    let userDefaults: UserDefaults

    private static let appGroupId = "group.com.siansiansu.TaigiKeyboard"

    private static var _sharedUserDefaults: UserDefaults?

    static var sharedUserDefaults: UserDefaults {
        if _sharedUserDefaults == nil {
            do {
                if let appGroupDefaults = UserDefaults(suiteName: appGroupId) {
                    let testKey = "test_key_\(UUID().uuidString)"
                    appGroupDefaults.set("test", forKey: testKey)
                    _ = appGroupDefaults.string(forKey: testKey)
                    appGroupDefaults.removeObject(forKey: testKey)

                    _sharedUserDefaults = appGroupDefaults
                } else {
                    throw NSError(domain: "SharedSettings", code: 1, userInfo: [NSLocalizedDescriptionKey: "Failed to create App Group UserDefaults"])
                }
            } catch {
                _sharedUserDefaults = UserDefaults.standard
            }
        }
        return _sharedUserDefaults!
    }

    private enum Keys {
        static let inputMode = "inputMode"
        static let enableDoubleTapOO = "enableDoubleTapOO"
        static let enableDoubleTapNN = "enableDoubleTapNN"
        static let isTranslateSwapped = "isTranslateSwapped"
        static let outputBothScripts = "outputBothScripts"
        static let fontType = "fontType"
        static let fullAccessEnabled = "fullAccessEnabled"
        static let autoCapitalizationEnabled = "autoCapitalizationEnabled"
        static let autoSpaceEnabled = "autoSpaceEnabled"
        static let phahTaigiLayoutEnabled = "phahTaigiLayoutEnabled"
        static let variantSearchEnabled = "variantSearchEnabled"
    }

    static let shared = SharedSettings()

    private init() {
        userDefaults = Self.sharedUserDefaults
    }

    var inputMode: InputMode {
        get {
            let rawValue = userDefaults.string(forKey: Keys.inputMode) ?? "tl"
            return InputMode(rawValue: rawValue) ?? InputMode.tl
        }
        set {
            userDefaults.set(newValue.rawValue, forKey: Keys.inputMode)
        }
    }

    var enableDoubleTapOO: Bool {
        get {
            userDefaults.object(forKey: Keys.enableDoubleTapOO) as? Bool ?? true
        }
        set {
            userDefaults.set(newValue, forKey: Keys.enableDoubleTapOO)
        }
    }

    var enableDoubleTapNN: Bool {
        get {
            userDefaults.object(forKey: Keys.enableDoubleTapNN) as? Bool ?? true
        }
        set {
            userDefaults.set(newValue, forKey: Keys.enableDoubleTapNN)
        }
    }

    var isTranslateSwapped: Bool {
        get {
            userDefaults.object(forKey: Keys.isTranslateSwapped) as? Bool ?? false
        }
        set {
            userDefaults.set(newValue, forKey: Keys.isTranslateSwapped)
            userDefaults.synchronize()
        }
    }

    var fontType: FontType {
        get {
            let rawValue = userDefaults.string(forKey: Keys.fontType) ?? FontType.openHuninn.rawValue
            return FontType(rawValue: rawValue) ?? .openHuninn
        }
        set {
            userDefaults.set(newValue.rawValue, forKey: Keys.fontType)
        }
    }

    var isFullAccessEnabled: Bool {
        get {
            userDefaults.bool(forKey: Keys.fullAccessEnabled)
        }
        set {
            userDefaults.set(newValue, forKey: Keys.fullAccessEnabled)
        }
    }

    var isAutoCapitalizationEnabled: Bool {
        get {
            userDefaults.object(forKey: Keys.autoCapitalizationEnabled) as? Bool ?? true
        }
        set {
            userDefaults.set(newValue, forKey: Keys.autoCapitalizationEnabled)
        }
    }

    var isAutoSpaceEnabled: Bool {
        get {
            userDefaults.object(forKey: Keys.autoSpaceEnabled) as? Bool ?? false
        }
        set {
            userDefaults.set(newValue, forKey: Keys.autoSpaceEnabled)
        }
    }

    var phahTaigiLayoutEnabled: Bool {
        get {
            userDefaults.object(forKey: Keys.phahTaigiLayoutEnabled) as? Bool ?? true
        }
        set {
            userDefaults.set(newValue, forKey: Keys.phahTaigiLayoutEnabled)
        }
    }

    var outputBothScripts: Bool {
        get {
            userDefaults.object(forKey: Keys.outputBothScripts) as? Bool ?? false
        }
        set {
            userDefaults.set(newValue, forKey: Keys.outputBothScripts)
        }
    }

    // 異用字搜尋：關閉時只搜尋原始詞，開啟時搜尋全部（含異用字）
    var variantSearchEnabled: Bool {
        get {
            userDefaults.object(forKey: Keys.variantSearchEnabled) as? Bool ?? false
        }
        set {
            userDefaults.set(newValue, forKey: Keys.variantSearchEnabled)
        }
    }

    func resetToDefaults() {
        inputMode = .tl
        enableDoubleTapOO = true
        enableDoubleTapNN = true
        isTranslateSwapped = false
        outputBothScripts = false
        fontType = .openHuninn
        isAutoCapitalizationEnabled = true
        isAutoSpaceEnabled = false
        phahTaigiLayoutEnabled = true
        variantSearchEnabled = false
    }
}

extension SharedSettings {
    func syncToKeyboardContext(_ context: KeyboardKit.KeyboardContext) {
        context.settings.spaceLongPressBehavior = .moveInputCursor

        // 同步自動大寫設定
        // KeyboardKit 會根據此設定自動處理，不需要手動強制重置 keyboardCase
        // 保留手動 shift (.uppercased) 和 caps lock (.capsLocked) 的狀態
        context.settings.isAutocapitalizationEnabled = isAutoCapitalizationEnabled
    }
}

// MARK: - Font Manager

import SwiftUI

/// 字體管理器（負責根據設定切換顯示字體）
class FontManager: ObservableObject {
    static let shared = FontManager()

    @Published var currentFontType: FontType

    private init() {
        currentFontType = SharedSettings.shared.fontType
    }

    /// 更新字體類型
    func updateFontType(_ fontType: FontType) {
        SharedSettings.shared.fontType = fontType
        currentFontType = fontType
    }

    /// 重新載入字體設定（從 UserDefaults）
    func reloadFontType() {
        currentFontType = SharedSettings.shared.fontType
    }

    /// 取得指定大小的字體
    func font(size: CGFloat) -> Font {
        KeyboardModels.Fonts.font(for: currentFontType, size: size)
    }
}

// MARK: - View Extension for Font Environment

extension View {
    /// 套用全域字體環境
    func withFontEnvironment() -> some View {
        modifier(FontEnvironmentModifier())
    }
}

/// 字體環境 Modifier
struct FontEnvironmentModifier: ViewModifier {
    @ObservedObject private var fontManager = FontManager.shared

    func body(content: Content) -> some View {
        content
            .environmentObject(fontManager)
    }
}
