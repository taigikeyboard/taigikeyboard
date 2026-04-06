import Foundation
import KeyboardKit
import LocalAuthentication
import SwiftUI
import UIKit

// MARK: - Codable Color

/// A color value that persists a single static RGBA to UserDefaults.
///
/// This deliberately stores one color for both light and dark modes.
/// When no custom color is set (`KeyboardColorSettings` field is `nil`),
/// the keyboard falls back to KeyboardKit's dynamic adaptive colors.
struct CodableColor: Codable, Equatable {
    let red: Double
    let green: Double
    let blue: Double
    let alpha: Double

    var color: Color {
        Color(red: red, green: green, blue: blue, opacity: alpha)
    }

    init(_ color: Color) {
        let uiColor = UIColor(color)
        var r: CGFloat = 0, g: CGFloat = 0, b: CGFloat = 0, a: CGFloat = 0
        uiColor.getRed(&r, green: &g, blue: &b, alpha: &a)
        self.red = Double(r)
        self.green = Double(g)
        self.blue = Double(b)
        self.alpha = Double(a)
    }
}

// MARK: - Keyboard Color Settings

struct KeyboardColorSettings: Codable, Equatable {
    var backgroundColor: CodableColor?
    var keyTextColor: CodableColor?
    var normalKeyFillColor: CodableColor?
    var specialKeyFillColor: CodableColor?
    var candidateTextColor: CodableColor?
    var candidateBackgroundColor: CodableColor?

    static let `default` = KeyboardColorSettings()
}

/// 鍵盤佈局類型
enum KeyboardLayoutType: String, CaseIterable {
    case phahTaigi = "phahTaigi"  // PhahTaigi 佈局
    case qwerty = "qwerty"        // 標準 QWERTY 佈局
    case tps = "tps"              // 方音符號佈局
    case moe1 = "moe1"            // 教育部輸入法佈局1
    case moe2 = "moe2"            // 教育部輸入法佈局2
}

/// Immutable snapshot of settings needed during a single render cycle.
/// Avoids repeated UserDefaults reads when rendering ~50 keys.
struct SettingsSnapshot {
    let inputMode: InputMode
    let fontType: FontType
    let keyboardLayoutType: KeyboardLayoutType
    let keyFontSizeScale: CGFloat
    let keyCornerRadius: CGFloat
    let colorSettings: KeyboardColorSettings
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
            if let appGroupDefaults = UserDefaults(suiteName: appGroupId) {
                _sharedUserDefaults = appGroupDefaults
            } else {
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
        static let inputModeBeforeTps = "inputModeBeforeTps"
        static let layoutBeforeTps = "layoutBeforeTps"
        // 詞頻紀錄開關
        static let frequencyRecordingEnabled = "frequencyRecordingEnabled"
        // 詞關聯紀錄開關
        static let associationRecordingEnabled = "associationRecordingEnabled"
        // 自訂詞庫開關
        static let customDictEnabled = "customDictEnabled"
        // 詞庫開關
        static let moeDictEnabled = "moeDictEnabled"
        static let newwordDictEnabled = "newwordDictEnabled"
        static let kunggeDictEnabled = "kunggeDictEnabled"
        static let iTaigiDictEnabled = "iTaigiDictEnabled"
        static let taiwanJapanDictEnabled = "taiwanJapanDictEnabled"
        static let taiHuaDictEnabled = "taiHuaDictEnabled"
        static let taiwanPlantDictEnabled = "taiwanPlantDictEnabled"
        static let sttiDictEnabled = "sttiDictEnabled"
        static let khpooDictEnabled = "khpooDictEnabled"
        // 異用字開關
        static let variantEnabled = "variantEnabled"
        // 在來字開關
        static let khiin = "khiin"
        // LKK漢羅合用建議用字
        static let lkkDictEnabled = "lkkDictEnabled"
        // 方音符號設定
        static let tpsOrMapsToER = "tpsOrMapsToER"
        // 家私櫥設定
        static let toolbarAutoCollapse = "toolbarAutoCollapse"
        // 切換鍵盤鍵
        static let isGlobeKeyEnabled = "isGlobeKeyEnabled"
        // 外觀設定
        static let keyHeightScale = "keyHeightScale"
        static let colorSettings = "colorSettings"
        static let keyFontSizeScale = "keyFontSizeScale"
        static let candidateTextSizeScale = "candidateTextSizeScale"
        static let keyCornerRadius = "keyCornerRadius"
        static let keyBorderWidth = "keyBorderWidth"
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
            let oldValue = inputMode
            userDefaults.set(newValue.rawValue, forKey: Keys.inputMode)

            // TPS ↔ layout 1:1 sync (reverse direction)
            // Write directly to userDefaults to avoid recursion with keyboardLayoutType setter
            if newValue == .tps && oldValue != .tps {
                // Entering TPS mode: switch layout to TPS
                if keyboardLayoutType != .tps {
                    layoutBeforeTps = keyboardLayoutType
                    userDefaults.set(KeyboardLayoutType.tps.rawValue, forKey: Keys.keyboardLayoutType)
                    phahTaigiLayoutEnabled = false
                }
            } else if newValue != .tps && oldValue == .tps {
                // Leaving TPS mode: restore previous layout
                if keyboardLayoutType == .tps {
                    let restored = layoutBeforeTps
                    userDefaults.set(restored.rawValue, forKey: Keys.keyboardLayoutType)
                    phahTaigiLayoutEnabled = (restored == .phahTaigi)
                }
            }
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
            let oldValue = keyboardLayoutType
            // TPS ↔ inputMode 1:1 sync
            if newValue == .tps && oldValue != .tps {
                // Entering TPS: save current inputMode, then switch to tps
                let currentInputMode = inputMode
                if currentInputMode != .tps {
                    inputModeBeforeTps = currentInputMode
                }
                inputMode = .tps
            } else if newValue != .tps && oldValue == .tps {
                // Leaving TPS: restore previous inputMode
                inputMode = inputModeBeforeTps
            }
            userDefaults.set(newValue.rawValue, forKey: Keys.keyboardLayoutType)
            // 同步舊的 phahTaigiLayoutEnabled 設定（向後相容）
            phahTaigiLayoutEnabled = (newValue == .phahTaigi)
        }
    }

    // Stores the inputMode before switching to TPS, so it can be restored when leaving TPS
    private var inputModeBeforeTps: InputMode {
        get {
            let rawValue = userDefaults.string(forKey: Keys.inputModeBeforeTps) ?? "tl"
            return InputMode(rawValue: rawValue) ?? .tl
        }
        set {
            userDefaults.set(newValue.rawValue, forKey: Keys.inputModeBeforeTps)
        }
    }

    // Stores the layout before switching to TPS, so it can be restored when leaving TPS
    private var layoutBeforeTps: KeyboardLayoutType {
        get {
            let rawValue = userDefaults.string(forKey: Keys.layoutBeforeTps) ?? KeyboardLayoutType.phahTaigi.rawValue
            return KeyboardLayoutType(rawValue: rawValue) ?? .phahTaigi
        }
        set {
            userDefaults.set(newValue.rawValue, forKey: Keys.layoutBeforeTps)
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

    // MARK: - 詞頻紀錄開關（預設開啟）

    var frequencyRecordingEnabled: Bool {
        get { userDefaults.object(forKey: Keys.frequencyRecordingEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.frequencyRecordingEnabled) }
    }

    // MARK: - 詞關聯紀錄開關（預設開啟）

    var associationRecordingEnabled: Bool {
        get { userDefaults.object(forKey: Keys.associationRecordingEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.associationRecordingEnabled) }
    }

    // MARK: - 自訂詞庫開關

    var customDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.customDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.customDictEnabled) }
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
        get { userDefaults.object(forKey: Keys.taiwanJapanDictEnabled) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.taiwanJapanDictEnabled) }
    }

    var taiHuaDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.taiHuaDictEnabled) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.taiHuaDictEnabled) }
    }

    var taiwanPlantDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.taiwanPlantDictEnabled) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.taiwanPlantDictEnabled) }
    }

    var sttiDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.sttiDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.sttiDictEnabled) }
    }

    var khpooDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.khpooDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.khpooDictEnabled) }
    }

    // 異用字開關（預設關閉）
    var variantEnabled: Bool {
        get { userDefaults.object(forKey: Keys.variantEnabled) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.variantEnabled) }
    }

    // 在來字開關（預設關閉）
    var khiin: Bool {
        get { userDefaults.object(forKey: Keys.khiin) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.khiin) }
    }

    // LKK漢羅合用建議用字（預設開啟）
    var lkkDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.lkkDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.lkkDictEnabled) }
    }

    // MARK: - 家私櫥設定

    /// Toolbar auto-collapse toggle (default: true = auto-collapse on composing/mode change)
    var isToolbarAutoCollapse: Bool {
        get { userDefaults.object(forKey: Keys.toolbarAutoCollapse) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.toolbarAutoCollapse) }
    }

    // MARK: - 切換鍵盤鍵

    /// Globe key toggle. Default depends on device type for backward compatibility:
    /// iPad/iPhone SE (Touch ID) = true, regular iPhone = false.
    var isGlobeKeyEnabled: Bool {
        get {
            guard let stored = userDefaults.object(forKey: Keys.isGlobeKeyEnabled) as? Bool else {
                return Self.defaultGlobeKeyEnabled
            }
            return stored
        }
        set { userDefaults.set(newValue, forKey: Keys.isGlobeKeyEnabled) }
    }

    /// Device-based default: iPad or iPhone SE (Touch ID) = true, otherwise false
    private static var defaultGlobeKeyEnabled: Bool {
        if UIDevice.current.userInterfaceIdiom == .pad { return true }
        guard UIDevice.current.userInterfaceIdiom == .phone else { return false }
        let laContext = LAContext()
        var error: NSError?
        _ = laContext.canEvaluatePolicy(.deviceOwnerAuthentication, error: &error)
        return laContext.biometryType == .touchID
    }

    // MARK: - 方音符號設定

    // or 對應 ㄜ（預設開啟，關閉時 or → ㄛ）
    var tpsOrMapsToER: Bool {
        get { userDefaults.object(forKey: Keys.tpsOrMapsToER) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.tpsOrMapsToER) }
    }

    // MARK: - 外觀設定（scale factor, default 1.0）

    var keyHeightScale: CGFloat {
        get { userDefaults.object(forKey: Keys.keyHeightScale) as? Double ?? 1.0 }
        set { userDefaults.set(newValue, forKey: Keys.keyHeightScale) }
    }

    var keyFontSizeScale: CGFloat {
        get { userDefaults.object(forKey: Keys.keyFontSizeScale) as? Double ?? 1.0 }
        set { userDefaults.set(newValue, forKey: Keys.keyFontSizeScale) }
    }

    var candidateTextSizeScale: CGFloat {
        get { userDefaults.object(forKey: Keys.candidateTextSizeScale) as? Double ?? 1.0 }
        set { userDefaults.set(newValue, forKey: Keys.candidateTextSizeScale) }
    }

    var keyCornerRadius: CGFloat {
        get { userDefaults.object(forKey: Keys.keyCornerRadius) as? Double ?? 6.0 }
        set { userDefaults.set(newValue, forKey: Keys.keyCornerRadius) }
    }

    var keyBorderWidth: CGFloat {
        get { userDefaults.object(forKey: Keys.keyBorderWidth) as? Double ?? 0 }
        set { userDefaults.set(newValue, forKey: Keys.keyBorderWidth) }
    }

    var colorSettings: KeyboardColorSettings {
        get {
            guard let data = userDefaults.data(forKey: Keys.colorSettings),
                  let settings = try? JSONDecoder().decode(KeyboardColorSettings.self, from: data)
            else { return .default }
            return settings
        }
        set {
            if let data = try? JSONEncoder().encode(newValue) {
                userDefaults.set(data, forKey: Keys.colorSettings)
            }
        }
    }

    /// Creates an immutable snapshot of render-relevant settings.
    /// Call once per render cycle to avoid repeated UserDefaults reads.
    func snapshot() -> SettingsSnapshot {
        SettingsSnapshot(
            inputMode: inputMode,
            fontType: fontType,
            keyboardLayoutType: keyboardLayoutType,
            keyFontSizeScale: keyFontSizeScale,
            keyCornerRadius: keyCornerRadius,
            colorSettings: colorSettings
        )
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
        // 詞庫開關預設（iTaigi、台華線頂對照典 預設關閉）
        moeDictEnabled = true
        newwordDictEnabled = true
        kunggeDictEnabled = true
        iTaigiDictEnabled = false
        taiwanJapanDictEnabled = false
        taiHuaDictEnabled = false
        taiwanPlantDictEnabled = false
        sttiDictEnabled = true
        khpooDictEnabled = true
        variantEnabled = false
        khiin = false
        lkkDictEnabled = false
        // 家私櫥設定
        isToolbarAutoCollapse = true
        // 切換鍵盤鍵（移除儲存值，讓裝置預設邏輯生效）
        userDefaults.removeObject(forKey: Keys.isGlobeKeyEnabled)
        // 方音符號設定
        tpsOrMapsToER = true
        // 外觀設定
        keyHeightScale = 1.0
        keyFontSizeScale = 1.0
        candidateTextSizeScale = 1.0
        keyCornerRadius = 6.0
        keyBorderWidth = 0
        colorSettings = .default

        // 重設 KeyboardKit 設定
        KeyboardSettings.store.set(true, forKey: "com.keyboardkit.settings.keyboard.isAutocapitalizationEnabled")
        KeyboardSettings.store.set(true, forKey: "com.keyboardkit.settings.feedback.isAudioFeedbackEnabled")
        KeyboardSettings.store.set(true, forKey: "com.keyboardkit.settings.feedback.isHapticFeedbackEnabled")
    }
}

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
