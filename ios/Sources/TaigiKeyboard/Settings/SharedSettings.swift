// 中文: 整個 app + keyboard extension 共用的設定持久化 facade。
// 中文: 後端為 App Group UserDefaults,跨程序共享;部分欄位代理到 KeyboardSettings.store (KeyboardKit 自有 store)。
// 中文: 同時實作 KeyboardEnvironment (UI 端可寫) 與 EngineSettings/EngineSettingsProvider (引擎端唯讀 + live-read)。

import Foundation
import KeyboardKit

// 中文: 設定中心 singleton。所有持久化都走 App Group UserDefaults,
// 中文: inputMode 與 keyboardLayoutType 之間透過 TPSSyncCoordinator 維持 1:1 連動。
final class SharedSettings {
    private let userDefaults: UserDefaults

    // 中文: App Group identifier;sharedContainerURL 與 sharedUserDefaults 都掛在這個 group。
    static let appGroupId = "group.com.siansiansu.TaigiKeyboard"

    /// App Group shared container URL, accessible by both main app and keyboard extension.
    // 中文: App Group 共享容器 URL — 主 app 與 keyboard extension 共用,放使用者 DB 與資源檔。
    static var sharedContainerURL: URL? {
        FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: appGroupId)
    }

    // 中文: App Group 共享 UserDefaults。建立失敗 (Provisioning 沒設好) 時回退到 .standard,避免 crash。
    static let sharedUserDefaults = UserDefaults(suiteName: appGroupId) ?? .standard

    // 中文: UserDefaults 持久化欄位 key 集合。所有 key 都是 raw string,變動會破壞舊版相容性。
    private enum Keys {
        static let inputMode = "inputMode"
        static let enableDoubleTapOO = "enableDoubleTapOO"
        static let enableDoubleTapNN = "enableDoubleTapNN"
        static let isTranslateSwapped = "isTranslateSwapped"
        static let outputBothScripts = "outputBothScripts"
        static let fontType = "fontType"
        static let fullAccessEnabled = "fullAccessEnabled"
        static let autoSpaceEnabled = "autoSpaceEnabled"
        static let keyboardLayoutType = "keyboardLayoutType"
        static let inputModeBeforeTps = "inputModeBeforeTps"
        static let layoutBeforeTps = "layoutBeforeTps"
        static let frequencyRecordingEnabled = "frequencyRecordingEnabled"
        static let associationRecordingEnabled = "associationRecordingEnabled"
        static let customDictEnabled = "customDictEnabled"
        // Dictionary toggles
        static let moeDictEnabled = "moeDictEnabled"
        static let newwordDictEnabled = "newwordDictEnabled"
        static let kunggeDictEnabled = "kunggeDictEnabled"
        static let iTaigiDictEnabled = "iTaigiDictEnabled"
        static let taiwanJapanDictEnabled = "taiwanJapanDictEnabled"
        static let taiHuaDictEnabled = "taiHuaDictEnabled"
        static let taiwanPlantDictEnabled = "taiwanPlantDictEnabled"
        static let sttiDictEnabled = "sttiDictEnabled"
        static let khpooDictEnabled = "khpooDictEnabled"
        static let variantEnabled = "variantEnabled"
        static let khiin = "khiin"
        static let lkkDictEnabled = "lkkDictEnabled"
        /// TPS settings
        static let tpsOrMapsToER = "tpsOrMapsToER"
        /// Toolbar settings
        static let toolbarAutoCollapse = "toolbarAutoCollapse"
        /// Globe key
        static let isGlobeKeyEnabled = "isGlobeKeyEnabled"
        // Appearance settings
        static let keyHeightScale = "keyHeightScale"
        static let colorSettings = "colorSettings"
        static let keyFontSizeScale = "keyFontSizeScale"
        static let candidateTextSizeScale = "candidateTextSizeScale"
        static let keyCornerRadius = "keyCornerRadius"
        static let keyBorderWidth = "keyBorderWidth"
    }

    // 中文: process 內 singleton。整個 app + extension 共用同一份設定 facade。
    static let shared = SharedSettings()

    /// Guards the mutual recursion between inputMode and keyboardLayoutType
    /// setters when TPS ↔ layout auto-sync fires.
    // 中文: 防止 inputMode <-> keyboardLayoutType setter 互相觸發無限遞迴的 re-entry guard。
    private let tpsSync = TPSSyncCoordinator()

    private init() {
        userDefaults = Self.sharedUserDefaults
    }

    // 中文: 目前輸入模式。setter 會啟動 TPS 雙向連動 — 切到 .tps 時自動把 layout 切成 .tps,
    // 中文: 並備份原 layout 到 layoutBeforeTps;離開 .tps 時還原。
    var inputMode: InputMode {
        get {
            let rawValue = userDefaults.string(forKey: Keys.inputMode) ?? "tl"
            return InputMode(rawValue: rawValue) ?? InputMode.tl
        }
        set {
            let oldValue = inputMode
            userDefaults.set(newValue.rawValue, forKey: Keys.inputMode)

            tpsSync.sync {
                if newValue == .tps, oldValue != .tps {
                    if keyboardLayoutType != .tps {
                        layoutBeforeTps = keyboardLayoutType
                        keyboardLayoutType = .tps
                    }
                } else if newValue != .tps, oldValue == .tps {
                    if keyboardLayoutType == .tps {
                        keyboardLayoutType = layoutBeforeTps
                    }
                }
            }
        }
    }

    // 中文: POJ「雙擊 OO」預處理開關。預設 true。供 ToneToggles 打包後給 ToneConverter。
    var isDoubleTapOOEnabled: Bool {
        get {
            userDefaults.object(forKey: Keys.enableDoubleTapOO) as? Bool ?? true
        }
        set {
            userDefaults.set(newValue, forKey: Keys.enableDoubleTapOO)
        }
    }

    // 中文: POJ「雙擊 NN」預處理開關。預設 true。
    var isDoubleTapNNEnabled: Bool {
        get {
            userDefaults.object(forKey: Keys.enableDoubleTapNN) as? Bool ?? true
        }
        set {
            userDefaults.set(newValue, forKey: Keys.enableDoubleTapNN)
        }
    }

    // 中文: 翻譯方向是否反轉 (台↔英)。預設 false。
    var isTranslateSwapped: Bool {
        get {
            userDefaults.object(forKey: Keys.isTranslateSwapped) as? Bool ?? false
        }
        set {
            userDefaults.set(newValue, forKey: Keys.isTranslateSwapped)
        }
    }

    // 中文: 鍵盤字體選擇。預設 .openHuninn (jf open 粉圓)。
    var fontType: FontType {
        get {
            let rawValue = userDefaults.string(forKey: Keys.fontType) ?? FontType.openHuninn.rawValue
            return FontType(rawValue: rawValue) ?? .openHuninn
        }
        set {
            userDefaults.set(newValue.rawValue, forKey: Keys.fontType)
        }
    }

    // 中文: Full Access 開關 (是否取得網路 / 剪貼簿等完整存取)。預設 false。
    var isFullAccessEnabled: Bool {
        get {
            userDefaults.bool(forKey: Keys.fullAccessEnabled)
        }
        set {
            userDefaults.set(newValue, forKey: Keys.fullAccessEnabled)
        }
    }

    // isAutoCapitalizationEnabled moved to KeyboardKit's KeyboardSettings
    // Access via state.keyboardContext.settings.isAutocapitalizationEnabled

    // 中文: 自動空格開關 (上字後是否補空白)。預設 false。
    var isAutoSpaceEnabled: Bool {
        get {
            userDefaults.object(forKey: Keys.autoSpaceEnabled) as? Bool ?? false
        }
        set {
            userDefaults.set(newValue, forKey: Keys.autoSpaceEnabled)
        }
    }

    // 中文: 目前鍵盤排版。setter 會啟動 TPS 雙向連動 — 切到 .tps 時自動把 inputMode 切成 .tps,
    // 中文: 並備份原 inputMode 到 inputModeBeforeTps;離開 .tps 時還原。
    var keyboardLayoutType: KeyboardLayoutType {
        get {
            let rawValue = userDefaults.string(forKey: Keys.keyboardLayoutType) ?? KeyboardLayoutType.phahTaigi.rawValue
            return KeyboardLayoutType(rawValue: rawValue) ?? .phahTaigi
        }
        set {
            let oldValue = keyboardLayoutType
            userDefaults.set(newValue.rawValue, forKey: Keys.keyboardLayoutType)

            tpsSync.sync {
                if newValue == .tps, oldValue != .tps {
                    let currentInputMode = inputMode
                    if currentInputMode != .tps {
                        inputModeBeforeTps = currentInputMode
                    }
                    inputMode = .tps
                } else if newValue != .tps, oldValue == .tps {
                    inputMode = inputModeBeforeTps
                }
            }
        }
    }

    /// Stores the inputMode before switching to TPS, so it can be restored when leaving TPS
    // 中文: 切換到 TPS 之前的 inputMode 備份。離開 TPS 時用來還原。
    private var inputModeBeforeTps: InputMode {
        get {
            let rawValue = userDefaults.string(forKey: Keys.inputModeBeforeTps) ?? "tl"
            return InputMode(rawValue: rawValue) ?? .tl
        }
        set {
            userDefaults.set(newValue.rawValue, forKey: Keys.inputModeBeforeTps)
        }
    }

    /// Stores the layout before switching to TPS, so it can be restored when leaving TPS
    // 中文: 切換到 TPS 之前的 layout 備份。離開 TPS 時用來還原。
    private var layoutBeforeTps: KeyboardLayoutType {
        get {
            let rawValue = userDefaults.string(forKey: Keys.layoutBeforeTps) ?? KeyboardLayoutType.phahTaigi.rawValue
            return KeyboardLayoutType(rawValue: rawValue) ?? .phahTaigi
        }
        set {
            userDefaults.set(newValue.rawValue, forKey: Keys.layoutBeforeTps)
        }
    }

    // 中文: 「同時輸出漢字 + 羅馬字」開關。預設 false。
    var isOutputBothScripts: Bool {
        get {
            userDefaults.object(forKey: Keys.outputBothScripts) as? Bool ?? false
        }
        set {
            userDefaults.set(newValue, forKey: Keys.outputBothScripts)
        }
    }

    // MARK: - Frequency Recording (default: on)

    // 中文: 是否記錄使用者選字頻率 (供 user_frequency.db 排序加權)。預設 true。
    var isFrequencyRecordingEnabled: Bool {
        get { userDefaults.object(forKey: Keys.frequencyRecordingEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.frequencyRecordingEnabled) }
    }

    // MARK: - Association Recording (default: on)

    // 中文: 是否記錄選字關聯 (供 NextWord 推薦使用)。預設 true。
    var isAssociationRecordingEnabled: Bool {
        get { userDefaults.object(forKey: Keys.associationRecordingEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.associationRecordingEnabled) }
    }

    // MARK: - Custom Dictionary

    // 中文: 自訂詞庫開關。預設 true。
    var isCustomDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.customDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.customDictEnabled) }
    }

    // MARK: - Dictionary Toggles

    // 中文: 各內建詞典開關。每個欄位獨立持久化於 UserDefaults,預設值見每行 fallback。
    // 中文: 預設開啟:MOE / Newword / Kungge / STTI / Khpoo;預設關閉:iTaigi / TaiwanJapan / TaiHua / TaiwanPlant / Variant / Khiin / LKK。

    // 中文: 教育部詞典 (MOE) 開關。預設 true。
    var isMoeDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.moeDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.moeDictEnabled) }
    }

    // 中文: 新詞詞典開關。預設 true。
    var isNewwordDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.newwordDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.newwordDictEnabled) }
    }

    // 中文: 公語詞典開關。預設 true。
    var isKunggeDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.kunggeDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.kunggeDictEnabled) }
    }

    // 中文: iTaigi 詞典開關。預設 false。
    var isITaigiDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.iTaigiDictEnabled) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.iTaigiDictEnabled) }
    }

    // 中文: 台日大辭典開關。預設 false。
    var isTaiwanJapanDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.taiwanJapanDictEnabled) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.taiwanJapanDictEnabled) }
    }

    // 中文: 台華對照辭典開關。預設 false。
    var isTaiHuaDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.taiHuaDictEnabled) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.taiHuaDictEnabled) }
    }

    // 中文: 台灣植物名彙開關。預設 false。
    var isTaiwanPlantDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.taiwanPlantDictEnabled) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.taiwanPlantDictEnabled) }
    }

    // 中文: 教育部臺灣台語常用詞辭典 (STTI) 開關。預設 true。
    var isSttiDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.sttiDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.sttiDictEnabled) }
    }

    // 中文: 教育部閩南語推薦用字 (Khpoo) 開關。預設 true。
    var isKhpooDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.khpooDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.khpooDictEnabled) }
    }

    /// Variant characters toggle (default: off)
    // 中文: 異體字候選開關。預設 false。
    var isVariantEnabled: Bool {
        get { userDefaults.object(forKey: Keys.variantEnabled) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.variantEnabled) }
    }

    /// Khiin supplementary data toggle (default: off)
    // 中文: Khiin 補充資料開關。預設 false。
    var isKhiinEnabled: Bool {
        get { userDefaults.object(forKey: Keys.khiin) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.khiin) }
    }

    /// LKK Hàn-lô mixed script suggestions (default: on)
    // 中文: LKK 漢羅混寫候選開關。預設 true。
    var isLkkDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.lkkDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.lkkDictEnabled) }
    }

    // MARK: - Toolbar Settings

    /// Toolbar auto-collapse toggle (default: true = auto-collapse on composing/mode change)
    // 中文: 工具列自動收合開關。true 時組字或切模式會自動收合工具列。預設 true。
    var isToolbarAutoCollapse: Bool {
        get { userDefaults.object(forKey: Keys.toolbarAutoCollapse) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.toolbarAutoCollapse) }
    }

    // MARK: - Globe Key

    /// Globe key toggle. Default depends on device type for backward compatibility:
    /// iPad/iPhone SE (Touch ID) = true, regular iPhone = false.
    // 中文: 地球鍵開關。預設值由 DeviceCapabilities.prefersGlobeKeyByDefault 決定 (iPad / Touch ID iPhone 為 true)。
    // 中文: 一旦使用者寫入過值,後續以儲存值為準,不再走 device-default fallback。
    var isGlobeKeyEnabled: Bool {
        get {
            guard let stored = userDefaults.object(forKey: Keys.isGlobeKeyEnabled) as? Bool else {
                return DeviceCapabilities.prefersGlobeKeyByDefault
            }
            return stored
        }
        set { userDefaults.set(newValue, forKey: Keys.isGlobeKeyEnabled) }
    }

    // MARK: - TPS Settings

    /// TPS "or" maps to ㄜ (default: on). When off, "or" maps to ㄛ.
    // 中文: TPS 中 "or" 映射對象開關。true 時映射到ㄜ,false 時映射到ㄛ。預設 true。
    var isTpsOrMappedToER: Bool {
        get { userDefaults.object(forKey: Keys.tpsOrMapsToER) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.tpsOrMapsToER) }
    }

    // MARK: - Appearance (scale factor, default 1.0)

    // 中文: 鍵盤高度縮放係數。預設 1.0。
    var keyHeightScale: CGFloat {
        get { userDefaults.object(forKey: Keys.keyHeightScale) as? Double ?? 1.0 }
        set { userDefaults.set(newValue, forKey: Keys.keyHeightScale) }
    }

    // 中文: 鍵帽字體大小縮放係數。預設 1.0。
    var keyFontSizeScale: CGFloat {
        get { userDefaults.object(forKey: Keys.keyFontSizeScale) as? Double ?? 1.0 }
        set { userDefaults.set(newValue, forKey: Keys.keyFontSizeScale) }
    }

    // 中文: 候選列文字大小縮放係數。預設 1.0。
    var candidateTextSizeScale: CGFloat {
        get { userDefaults.object(forKey: Keys.candidateTextSizeScale) as? Double ?? 1.0 }
        set { userDefaults.set(newValue, forKey: Keys.candidateTextSizeScale) }
    }

    // 中文: 鍵帽圓角半徑 (point)。預設 6.0。
    var keyCornerRadius: CGFloat {
        get { userDefaults.object(forKey: Keys.keyCornerRadius) as? Double ?? 6.0 }
        set { userDefaults.set(newValue, forKey: Keys.keyCornerRadius) }
    }

    // 中文: 鍵帽邊框寬度 (point)。預設 0 (無邊框)。
    var keyBorderWidth: CGFloat {
        get { userDefaults.object(forKey: Keys.keyBorderWidth) as? Double ?? 0 }
        set { userDefaults.set(newValue, forKey: Keys.keyBorderWidth) }
    }

    // 中文: 鍵盤顏色組合。以 JSON 序列化進 UserDefaults;decode 失敗或未設定時回傳 .default (全 nil)。
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
    // 中文: 取得渲染週期一致快照。每次渲染 (~50 個鍵) 呼叫一次,避免每個鍵都打 UserDefaults。
    func snapshot() -> SettingsSnapshot {
        SettingsSnapshot(
            inputMode: inputMode,
            fontType: fontType,
            keyboardLayoutType: keyboardLayoutType,
            isTranslateSwapped: isTranslateSwapped,
            isTpsOrMappedToER: isTpsOrMappedToER,
            keyFontSizeScale: keyFontSizeScale,
            keyCornerRadius: keyCornerRadius,
            colorSettings: colorSettings,
        )
    }

    // 中文: 把 SharedSettings 自有的所有欄位重設為原廠預設值。
    // 中文: 不負責 KeyboardKit-owned defaults — 那些走 SettingsResetCoordinator.resetKeyboardKitDefaults()。
    func resetToDefaults() {
        inputMode = .tl
        isDoubleTapOOEnabled = true
        isDoubleTapNNEnabled = true
        isTranslateSwapped = false
        isOutputBothScripts = false
        fontType = .openHuninn
        isAutoSpaceEnabled = false
        keyboardLayoutType = .phahTaigi
        // Dictionary toggles (iTaigi, TaiHua default off)
        isMoeDictEnabled = true
        isNewwordDictEnabled = true
        isKunggeDictEnabled = true
        isITaigiDictEnabled = false
        isTaiwanJapanDictEnabled = false
        isTaiHuaDictEnabled = false
        isTaiwanPlantDictEnabled = false
        isSttiDictEnabled = true
        isKhpooDictEnabled = true
        isVariantEnabled = false
        isKhiinEnabled = false
        isLkkDictEnabled = true
        // Toolbar
        isToolbarAutoCollapse = true
        // Globe key: remove stored value so device-based default takes effect
        userDefaults.removeObject(forKey: Keys.isGlobeKeyEnabled)
        // TPS
        isTpsOrMappedToER = true
        // Appearance
        keyHeightScale = 1.0
        keyFontSizeScale = 1.0
        candidateTextSizeScale = 1.0
        keyCornerRadius = 6.0
        keyBorderWidth = 0
        colorSettings = .default

        // KeyboardKit-owned defaults live in a separate store; reset via
        // `SettingsResetCoordinator.resetAll()` when you need both sides.
    }
}

// MARK: - EngineSettings Conformance

/// SharedSettings bridges the engine's settings needs to two stores:
/// its own `UserDefaults` (App Group container) for app-owned settings,
/// and `KeyboardSettings.store` (KeyboardKit-owned) for the shift-state
/// / auto-capitalization toggle. Engine-layer code cannot touch
/// `KeyboardSettings.store` directly (no `import KeyboardKit` allowed),
/// so this conformance bridges the two behind one protocol.
extension SharedSettings: EngineSettings {
    /// Mirrors KeyboardKit's `isAutocapitalizationEnabled` setting so
    /// engine-layer code (e.g. `CandidateProcessor.capitalize`) can read
    /// it through `EngineSettings` without importing KeyboardKit.
    // 中文: 鏡射 KeyboardKit 自有的 isAutocapitalizationEnabled 設定 — 引擎層因不能 import KeyboardKit,
    // 中文: 透過這個 conformance 從 EngineSettings 取得自動大寫狀態。
    var isAutoCap: Bool {
        KeyboardSettings.store.bool(
            forKey: "com.keyboardkit.settings.keyboard.isAutocapitalizationEnabled",
        )
    }

    /// Live-reads the two underlying booleans per call, matching the
    /// `EngineSettingsProvider.current` live-read contract.
    // 中文: 每次 access 都重讀兩個底層 bool,符合 EngineSettingsProvider 的 live-read 契約。
    var toneToggles: ToneToggles {
        ToneToggles(
            isDoubleTapOOEnabled: isDoubleTapOOEnabled,
            isDoubleTapNNEnabled: isDoubleTapNNEnabled,
        )
    }
}

// MARK: - EngineSettingsProvider Conformance

extension SharedSettings: EngineSettingsProvider {
    /// Returns `self` as `EngineSettings`. Each property access on the
    /// returned value re-reads `UserDefaults`, so the engine always sees
    /// the most recent values (critical for in-app setting changes to
    /// propagate without restarting the keyboard extension).
    // 中文: 直接回傳 self。引擎端後續對其 property 的 access 都會打到實際的 UserDefaults getter,
    // 中文: 因此 host app 改設定後不必重啟 extension 即可生效。
    var current: EngineSettings {
        self
    }
}
