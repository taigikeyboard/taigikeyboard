import Foundation
import KeyboardKit

/// 鍵盤佈局類型
enum KeyboardLayoutType: String, CaseIterable {
    case phahTaigi = "phahTaigi"  // PhahTaigi 佈局
    case qwerty = "qwerty"        // 標準 QWERTY 佈局
    case flick = "flick"          // Flick 聲調佈局
    case tps = "tps"              // 台灣注音（方音符號）佈局
}

class SharedSettings {
    let userDefaults: UserDefaults

    static let appGroupId = "group.com.siansiansu.TaigiKeyboard"

    /// 取得 App Group 共用資料夾路徑
    /// Main App 和 Keyboard Extension 都可存取
    static func getSharedContainerURL() -> URL? {
        FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: appGroupId)
    }

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
        static let autoSpaceEnabled = "autoSpaceEnabled"
        static let phahTaigiLayoutEnabled = "phahTaigiLayoutEnabled"
        static let keyboardLayoutType = "keyboardLayoutType"
        // 詞庫開關
        static let moeDictEnabled = "moeDictEnabled"
        static let newwordDictEnabled = "newwordDictEnabled"
        static let kunggeDictEnabled = "kunggeDictEnabled"
        static let iTaigiDictEnabled = "iTaigiDictEnabled"
        static let taiwanJapanDictEnabled = "taiwanJapanDictEnabled"
        static let taiHuaDictEnabled = "taiHuaDictEnabled"
        static let taiwanPlantDictEnabled = "taiwanPlantDictEnabled"
        // 異用字開關
        static let variantEnabled = "variantEnabled"
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

    // isAutoCapitalizationEnabled 已移至 KeyboardKit 的 KeyboardSettings
    // 使用 state.keyboardContext.settings.isAutocapitalizationEnabled 存取

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

    /// 鍵盤佈局類型
    var keyboardLayoutType: KeyboardLayoutType {
        get {
            let rawValue = userDefaults.string(forKey: Keys.keyboardLayoutType) ?? KeyboardLayoutType.phahTaigi.rawValue
            return KeyboardLayoutType(rawValue: rawValue) ?? .phahTaigi
        }
        set {
            userDefaults.set(newValue.rawValue, forKey: Keys.keyboardLayoutType)
            // 同步舊的 phahTaigiLayoutEnabled 設定（向後相容）
            phahTaigiLayoutEnabled = (newValue == .phahTaigi)
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

    // MARK: - 詞庫開關設定

    var moeDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.moeDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.moeDictEnabled) }
    }

    var newwordDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.newwordDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.newwordDictEnabled) }
    }

    var kunggeDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.kunggeDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.kunggeDictEnabled) }
    }

    var iTaigiDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.iTaigiDictEnabled) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.iTaigiDictEnabled) }
    }

    var taiwanJapanDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.taiwanJapanDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.taiwanJapanDictEnabled) }
    }

    var taiHuaDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.taiHuaDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.taiHuaDictEnabled) }
    }

    var taiwanPlantDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.taiwanPlantDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.taiwanPlantDictEnabled) }
    }

    // 異用字開關（預設關閉）
    var variantEnabled: Bool {
        get { userDefaults.object(forKey: Keys.variantEnabled) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.variantEnabled) }
    }

    func resetToDefaults() {
        inputMode = .tl
        enableDoubleTapOO = true
        enableDoubleTapNN = true
        isTranslateSwapped = false
        outputBothScripts = false
        fontType = .openHuninn
        isAutoSpaceEnabled = false
        phahTaigiLayoutEnabled = true
        keyboardLayoutType = .phahTaigi
        // 詞庫開關預設（iTaigi 預設關閉）
        moeDictEnabled = true
        newwordDictEnabled = true
        kunggeDictEnabled = true
        iTaigiDictEnabled = false
        taiwanJapanDictEnabled = true
        taiHuaDictEnabled = true
        taiwanPlantDictEnabled = true
        variantEnabled = false

        // 重設 KeyboardKit 設定
        KeyboardSettings.store.set(true, forKey: "com.keyboardkit.settings.keyboard.isAutocapitalizationEnabled")
    }
}

import OSLog

private let settingsLogger = Logger(
    subsystem: LexiconConstants.Logging.subsystem,
    category: "SharedSettings"
)

extension SharedSettings {
    /// 同步台語鍵盤專屬設定到 KeyboardContext
    ///
    /// 注意：isAutocapitalizationEnabled 由 KeyboardKit 自動管理，
    /// 透過 KeyboardSettings.setupStore() 使用 App Group 持久化。
    func syncToKeyboardContext(_ context: KeyboardKit.KeyboardContext) {
        // KeyboardKit 10: 設定空白鍵長按行為
        context.settings.spacebarLongPressBehavior = .moveInputCursor

        // 自動大寫設定由 KeyboardKit 的 KeyboardSettings 自動管理
        // 不需要手動同步，KeyboardKit 會自動讀取持久化的值
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
