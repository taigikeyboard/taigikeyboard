// 中文: 整個 app + keyboard extension 共用的設定持久化 facade。
// 中文: 後端為 App Group UserDefaults,跨程序共享;部分欄位代理到 KeyboardSettings.store (KeyboardKit 自有 store)。
// 中文: 同時實作 KeyboardEnvironment (UI 端可寫) 與 EngineSettings/EngineSettingsProvider (引擎端唯讀 + live-read)。

import Foundation
import KeyboardKit
import SwiftUI

// 中文: 設定中心 singleton。所有持久化都走 App Group UserDefaults,
// 中文: inputMode 與 keyboardLayoutType 透過 setInputMode / setKeyboardLayoutType 狀態機維持 1:1 連動。
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

    // MARK: - Typed key descriptors
    //
    // Each persisted field has a `SettingsKey<T>` descriptor pairing the raw
    // UserDefaults key with its default value + codec. Key spellings are
    // frozen for binary compatibility — anything stored on disk by an older
    // build must keep round-tripping. Typos (`tpsOrMapsToER`, `khiin`,
    // `enableDoubleTapOO/NN`) are deliberate; renaming them silently abandons
    // the user's existing preference.
    // 中文: 持久化欄位描述符。每個 key 字串都凍結 (含拼字錯誤) — 改了就會丟失既有使用者設定。

    private static let inputModeKey: SettingsKey<InputMode> = .rawRep("inputMode", default: .tl)
    private static let inputModeBeforeTpsKey: SettingsKey<InputMode> = .rawRep("inputModeBeforeTps", default: .tl)
    private static let keyboardLayoutTypeKey: SettingsKey<KeyboardLayoutType> = .rawRep("keyboardLayoutType", default: .phahTaigi)
    private static let layoutBeforeTpsKey: SettingsKey<KeyboardLayoutType> = .rawRep("layoutBeforeTps", default: .phahTaigi)
    private static let fontTypeKey: SettingsKey<FontType> = .rawRep("fontType", default: .openHuninn)

    private static let isDoubleTapOOEnabledKey: SettingsKey<Bool> = .bool("enableDoubleTapOO", default: true)
    private static let isDoubleTapNNEnabledKey: SettingsKey<Bool> = .bool("enableDoubleTapNN", default: true)
    private static let isTranslateSwappedKey: SettingsKey<Bool> = .bool("isTranslateSwapped", default: false)
    private static let isOutputBothScriptsKey: SettingsKey<Bool> = .bool("outputBothScripts", default: false)
    private static let isFullAccessEnabledKey: SettingsKey<Bool> = .bool("fullAccessEnabled", default: false)
    private static let isAutoSpaceEnabledKey: SettingsKey<Bool> = .bool("autoSpaceEnabled", default: false)
    private static let isFrequencyRecordingEnabledKey: SettingsKey<Bool> = .bool("frequencyRecordingEnabled", default: true)
    private static let isAssociationRecordingEnabledKey: SettingsKey<Bool> = .bool("associationRecordingEnabled", default: true)
    private static let isLiteralRomanCandidateEnabledKey: SettingsKey<Bool> = .bool("literalRomanCandidateEnabled", default: true)
    private static let isCustomDictEnabledKey: SettingsKey<Bool> = .bool("customDictEnabled", default: true)

    private static let isMoeDictEnabledKey: SettingsKey<Bool> = .bool("moeDictEnabled", default: true)
    private static let isNewwordDictEnabledKey: SettingsKey<Bool> = .bool("newwordDictEnabled", default: true)
    private static let isKunggeDictEnabledKey: SettingsKey<Bool> = .bool("kunggeDictEnabled", default: true)
    private static let isITaigiDictEnabledKey: SettingsKey<Bool> = .bool("iTaigiDictEnabled", default: false)
    private static let isTaiwanJapanDictEnabledKey: SettingsKey<Bool> = .bool("taiwanJapanDictEnabled", default: false)
    private static let isTaiHuaDictEnabledKey: SettingsKey<Bool> = .bool("taiHuaDictEnabled", default: false)
    private static let isTaiwanPlantDictEnabledKey: SettingsKey<Bool> = .bool("taiwanPlantDictEnabled", default: false)
    private static let isSttiDictEnabledKey: SettingsKey<Bool> = .bool("sttiDictEnabled", default: true)
    private static let isKhpooDictEnabledKey: SettingsKey<Bool> = .bool("khpooDictEnabled", default: true)
    private static let isVariantEnabledKey: SettingsKey<Bool> = .bool("variantEnabled", default: false)
    private static let isKhiinEnabledKey: SettingsKey<Bool> = .bool("khiin", default: false)
    private static let isLkkDictEnabledKey: SettingsKey<Bool> = .bool("lkkDictEnabled", default: true)
    // 中文: 開發者補充辭典 (詞庫增補檔案) 開關。預設 true。
    private static let isDevDictEnabledKey: SettingsKey<Bool> = .bool("devDictEnabled", default: true)

    // kautian subcollection toggles (nested under the kautian master).
    // 10 accents default ON — opt-out model, upgrade is zero behaviour change (DD5).
    // Name appendix defaults ON — opt-out model; a user who never toggled it gets
    // surname candidates after upgrade, an explicit OFF stored value is preserved.
    // Accent key order mirrors config.yaml `dialect_columns` (= subtag bit - 1).
    // 中文: kautian subcollection 子開關 (10 腔調 + 姓名附錄)。腔調預設開 (DD5 opt-out),姓名附錄預設開 (opt-out)。腔調順序對齊 config.yaml dialect_columns。
    private static let isKautianAccentLukangEnabledKey: SettingsKey<Bool> = .bool("kautianAccentLukangEnabled", default: true)
    private static let isKautianAccentSansiaEnabledKey: SettingsKey<Bool> = .bool("kautianAccentSansiaEnabled", default: true)
    private static let isKautianAccentTaipakEnabledKey: SettingsKey<Bool> = .bool("kautianAccentTaipakEnabled", default: true)
    private static let isKautianAccentGilanEnabledKey: SettingsKey<Bool> = .bool("kautianAccentGilanEnabled", default: true)
    private static let isKautianAccentTainanEnabledKey: SettingsKey<Bool> = .bool("kautianAccentTainanEnabled", default: true)
    private static let isKautianAccentKaohsiungEnabledKey: SettingsKey<Bool> = .bool("kautianAccentKaohsiungEnabled", default: true)
    private static let isKautianAccentKinmenEnabledKey: SettingsKey<Bool> = .bool("kautianAccentKinmenEnabled", default: true)
    private static let isKautianAccentMakungEnabledKey: SettingsKey<Bool> = .bool("kautianAccentMakungEnabled", default: true)
    private static let isKautianAccentSintikEnabledKey: SettingsKey<Bool> = .bool("kautianAccentSintikEnabled", default: true)
    private static let isKautianAccentTaichungEnabledKey: SettingsKey<Bool> = .bool("kautianAccentTaichungEnabled", default: true)
    private static let isKautianNameAppendixEnabledKey: SettingsKey<Bool> = .bool("kautianNameAppendixEnabled", default: true)

    private static let isTpsOrMappedToERKey: SettingsKey<Bool> = .bool("tpsOrMapsToER", default: true)
    private static let isToolbarAutoCollapseKey: SettingsKey<Bool> = .bool("toolbarAutoCollapse", default: true)

    /// Globe key has a device-dependent default (`DeviceCapabilities.prefersGlobeKeyByDefault`)
    /// so the getter is hand-written; the descriptor is reused for writes
    /// and the reset-to-default `removeObject(forKey:)` path.
    // 中文: 地球鍵描述符僅供寫入與 reset 用;讀取走 isGlobeKeyEnabled getter 因預設依機型。
    private static let isGlobeKeyEnabledKey: SettingsKey<Bool> = .bool("isGlobeKeyEnabled", default: false)

    private static let keyHeightScaleKey: SettingsKey<CGFloat> = .cgFloat("keyHeightScale", default: 1.0)
    private static let keyFontSizeScaleKey: SettingsKey<CGFloat> = .cgFloat("keyFontSizeScale", default: 1.0)
    private static let candidateTextSizeScaleKey: SettingsKey<CGFloat> = .cgFloat("candidateTextSizeScale", default: 1.0)
    private static let keyCornerRadiusKey: SettingsKey<CGFloat> = .cgFloat("keyCornerRadius", default: 6.0)
    private static let keyBorderWidthKey: SettingsKey<CGFloat> = .cgFloat("keyBorderWidth", default: 0)

    private static let colorSettingsKey: SettingsKey<KeyboardColorSettings> = .codable("colorSettings", default: .default)

    // 中文: 選定主題 id。"default" = 既有自由配色 buffer(colorSettings);其餘為 UserTheme UUID / built-in id。
    private static let selectedThemeIdKey: SettingsKey<String> = .string("selectedThemeId", default: ThemeId.default)
    // 中文: 主題版本計數。每次 user-theme 檔變更 +1,寫入觸發 didChangeNotification,讓 extension 重新解析。
    private static let themeRevisionKey: SettingsKey<Int> = .int("themeRevision", default: 0)

    // 中文: process 內 singleton。整個 app + extension 共用同一份設定 facade。
    static let shared = SharedSettings()

    private init() {
        userDefaults = Self.sharedUserDefaults
    }

    /// Test-only seam: builds a `SharedSettings` against a caller-supplied
    /// `UserDefaults` (typically a fresh suite name) so the TPS state machine
    /// and default-value behavior can be exercised in isolation. Production
    /// code must keep using `SharedSettings.shared`.
    ///
    /// Scope caveat: `KeyboardEnvironment.settingsUserDefaults` continues to
    /// return the static App Group store regardless of the injected
    /// `userDefaults`, so this seam is **not** a fully isolated facade.
    /// Tests must not exercise the `UserDefaults.didChangeNotification`
    /// path through `settingsUserDefaults`; only stored-value reads / writes
    /// flow through the injected store.
    // 中文: 測試專用 init。讓 SharedSettingsTests 可以注入獨立的 UserDefaults suite,
    // 中文: 在不污染 app group store 的前提下驗證 TPS 狀態機與預設值。
    // 中文: 範圍上限:settingsUserDefaults 仍回傳 static app group store,因此本 seam
    // 中文: 不覆蓋 didChangeNotification 鏈路 — 該路徑的測試需另想辦法。
    init(userDefaults: UserDefaults) {
        self.userDefaults = userDefaults
    }

    // 中文: 目前輸入模式。setter 轉發到 setInputMode(_:),由狀態機維持 inputMode ↔ keyboardLayoutType 連動。
    var inputMode: InputMode {
        get { userDefaults.value(for: Self.inputModeKey) }
        set { setInputMode(newValue) }
    }

    // 中文: POJ「雙擊 OO」預處理開關。預設 true。供 ToneToggles 打包後給 ToneConverter。
    var isDoubleTapOOEnabled: Bool {
        get { userDefaults.value(for: Self.isDoubleTapOOEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isDoubleTapOOEnabledKey) }
    }

    // 中文: POJ「雙擊 NN」預處理開關。預設 true。
    var isDoubleTapNNEnabled: Bool {
        get { userDefaults.value(for: Self.isDoubleTapNNEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isDoubleTapNNEnabledKey) }
    }

    // 中文: 翻譯方向是否反轉 (台↔英)。預設 false。
    var isTranslateSwapped: Bool {
        get { userDefaults.value(for: Self.isTranslateSwappedKey) }
        set { userDefaults.set(newValue, for: Self.isTranslateSwappedKey) }
    }

    // 中文: 鍵盤字體選擇。預設 .openHuninn (jf open 粉圓)。
    var fontType: FontType {
        get { userDefaults.value(for: Self.fontTypeKey) }
        set { userDefaults.set(newValue, for: Self.fontTypeKey) }
    }

    // 中文: Full Access 開關 (是否取得網路 / 剪貼簿等完整存取)。預設 false。
    var isFullAccessEnabled: Bool {
        get { userDefaults.value(for: Self.isFullAccessEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isFullAccessEnabledKey) }
    }

    // isAutoCapitalizationEnabled moved to KeyboardKit's KeyboardSettings
    // Access via state.keyboardContext.settings.isAutocapitalizationEnabled

    // 中文: 自動空格開關 (上字後是否補空白)。預設 false。
    var isAutoSpaceEnabled: Bool {
        get { userDefaults.value(for: Self.isAutoSpaceEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isAutoSpaceEnabledKey) }
    }

    // 中文: 目前鍵盤排版。setter 轉發到 setKeyboardLayoutType(_:),由狀態機維持 layout ↔ inputMode 連動。
    var keyboardLayoutType: KeyboardLayoutType {
        get { userDefaults.value(for: Self.keyboardLayoutTypeKey) }
        set { setKeyboardLayoutType(newValue) }
    }

    // MARK: - TPS state machine

    /// Writes `newMode` to `inputMode` and applies the TPS ↔ layout 1:1 sync.
    ///
    /// - Entering `.tps`: saves the current `keyboardLayoutType` into
    ///   `layoutBeforeTps` (skipped if the layout is already `.tps`) and
    ///   flips the layout to `.tps`.
    /// - Leaving `.tps`: restores the layout from `layoutBeforeTps`, but only
    ///   when the live layout is still `.tps` — a manual layout change
    ///   earlier in the same flow is preserved.
    /// - All cascading writes go through `userDefaults.set(_, for:)`, which
    ///   bypasses the property setter (no re-entry into the state machine).
    // 中文: 寫入 inputMode 並由狀態機保持 TPS 連動。進入 TPS 時備份 layout 並翻成 .tps,
    // 中文: 離開時若 layout 仍是 .tps 才還原。所有連動寫入都走 userDefaults.set(_, for:),
    // 中文: 直接打 UserDefaults,不會再觸發本物件的 setter,因此不再需要 re-entry guard。
    func setInputMode(_ newMode: InputMode) {
        let oldMode = inputMode
        userDefaults.set(newMode, for: Self.inputModeKey)

        if newMode == .tps, oldMode != .tps {
            let currentLayout = keyboardLayoutType
            if currentLayout != .tps {
                layoutBeforeTps = currentLayout
                userDefaults.set(.tps, for: Self.keyboardLayoutTypeKey)
            }
        } else if newMode != .tps, oldMode == .tps {
            if keyboardLayoutType == .tps {
                userDefaults.set(layoutBeforeTps, for: Self.keyboardLayoutTypeKey)
            }
        }
    }

    /// Writes `newLayout` to `keyboardLayoutType` and applies the TPS ↔
    /// input-mode 1:1 sync. Mirrors `setInputMode(_:)` with one deliberate
    /// asymmetry preserved from the pre-refactor cascade: leaving `.tps` on
    /// the layout side **unconditionally** restores `inputMode` from
    /// `inputModeBeforeTps`, whereas the input-side exit guards on
    /// `keyboardLayoutType == .tps` before restoring layout. This mirrors
    /// HEAD~1 byte-for-byte; any future symmetrization belongs in a
    /// separate slice.
    // 中文: 寫入 keyboardLayoutType 並由狀態機保持 TPS 連動。語意刻意對稱於 HEAD~1 而非完全對稱於 setInputMode —
    // 中文: layout 側離開 TPS 時無條件用 inputModeBeforeTps 還原 inputMode (即使當下 inputMode 已被外部改過),
    // 中文: 與 input 側的有條件還原相反。若未來要拉齊兩側,需獨立 slice 處理。
    func setKeyboardLayoutType(_ newLayout: KeyboardLayoutType) {
        let oldLayout = keyboardLayoutType
        userDefaults.set(newLayout, for: Self.keyboardLayoutTypeKey)

        if newLayout == .tps, oldLayout != .tps {
            let currentMode = inputMode
            if currentMode != .tps {
                inputModeBeforeTps = currentMode
                userDefaults.set(.tps, for: Self.inputModeKey)
            }
        } else if newLayout != .tps, oldLayout == .tps {
            userDefaults.set(inputModeBeforeTps, for: Self.inputModeKey)
        }
    }

    /// Stores the inputMode before switching to TPS, so it can be restored when leaving TPS
    // 中文: 切換到 TPS 之前的 inputMode 備份。離開 TPS 時用來還原。
    private var inputModeBeforeTps: InputMode {
        get { userDefaults.value(for: Self.inputModeBeforeTpsKey) }
        set { userDefaults.set(newValue, for: Self.inputModeBeforeTpsKey) }
    }

    /// Stores the layout before switching to TPS, so it can be restored when leaving TPS
    // 中文: 切換到 TPS 之前的 layout 備份。離開 TPS 時用來還原。
    private var layoutBeforeTps: KeyboardLayoutType {
        get { userDefaults.value(for: Self.layoutBeforeTpsKey) }
        set { userDefaults.set(newValue, for: Self.layoutBeforeTpsKey) }
    }

    // 中文: 「同時輸出漢字 + 羅馬字」開關。預設 false。
    var isOutputBothScripts: Bool {
        get { userDefaults.value(for: Self.isOutputBothScriptsKey) }
        set { userDefaults.set(newValue, for: Self.isOutputBothScriptsKey) }
    }

    // MARK: - Frequency Recording (default: on)

    // 中文: 是否記錄使用者選字頻率 (供 user_frequency.db 排序加權)。預設 true。
    var isFrequencyRecordingEnabled: Bool {
        get { userDefaults.value(for: Self.isFrequencyRecordingEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isFrequencyRecordingEnabledKey) }
    }

    // MARK: - Association Recording (default: on)

    // 中文: 是否記錄選字關聯 (供 NextWord 推薦使用)。預設 true。
    var isAssociationRecordingEnabled: Bool {
        get { userDefaults.value(for: Self.isAssociationRecordingEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isAssociationRecordingEnabledKey) }
    }

    // MARK: - Literal-Roman Candidate (§34/S22, default: on)

    // 中文: 顯示羅馬字開關 (§34/S22)。TL/POJ 組字時是否在候選列首位顯示字面 roman 候選。預設 true。
    var isLiteralRomanCandidateEnabled: Bool {
        get { userDefaults.value(for: Self.isLiteralRomanCandidateEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isLiteralRomanCandidateEnabledKey) }
    }

    // MARK: - Custom Dictionary

    // 中文: 自訂詞庫開關。預設 true。
    var isCustomDictEnabled: Bool {
        get { userDefaults.value(for: Self.isCustomDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isCustomDictEnabledKey) }
    }

    // MARK: - Dictionary Toggles

    // 中文: 各內建詞典開關。每個欄位獨立持久化於 UserDefaults,預設值見每行 descriptor。
    // 中文: 預設開啟:MOE / Newword / Kungge / STTI / Khpoo;預設關閉:iTaigi / TaiwanJapan / TaiHua / TaiwanPlant / Variant / Khiin / LKK。

    // 中文: 教育部詞典 (MOE) 開關。預設 true。
    var isMoeDictEnabled: Bool {
        get { userDefaults.value(for: Self.isMoeDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isMoeDictEnabledKey) }
    }

    // 中文: 新詞詞典開關。預設 true。
    var isNewwordDictEnabled: Bool {
        get { userDefaults.value(for: Self.isNewwordDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isNewwordDictEnabledKey) }
    }

    // 中文: 公語詞典開關。預設 true。
    var isKunggeDictEnabled: Bool {
        get { userDefaults.value(for: Self.isKunggeDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKunggeDictEnabledKey) }
    }

    // 中文: iTaigi 詞典開關。預設 false。
    var isITaigiDictEnabled: Bool {
        get { userDefaults.value(for: Self.isITaigiDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isITaigiDictEnabledKey) }
    }

    // 中文: 台日大辭典開關。預設 false。
    var isTaiwanJapanDictEnabled: Bool {
        get { userDefaults.value(for: Self.isTaiwanJapanDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isTaiwanJapanDictEnabledKey) }
    }

    // 中文: 台華對照辭典開關。預設 false。
    var isTaiHuaDictEnabled: Bool {
        get { userDefaults.value(for: Self.isTaiHuaDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isTaiHuaDictEnabledKey) }
    }

    // 中文: 台灣植物名彙開關。預設 false。
    var isTaiwanPlantDictEnabled: Bool {
        get { userDefaults.value(for: Self.isTaiwanPlantDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isTaiwanPlantDictEnabledKey) }
    }

    // 中文: 教育部臺灣台語常用詞辭典 (STTI) 開關。預設 true。
    var isSttiDictEnabled: Bool {
        get { userDefaults.value(for: Self.isSttiDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isSttiDictEnabledKey) }
    }

    // 中文: 教育部閩南語推薦用字 (Khpoo) 開關。預設 true。
    var isKhpooDictEnabled: Bool {
        get { userDefaults.value(for: Self.isKhpooDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKhpooDictEnabledKey) }
    }

    /// Variant characters toggle (default: off)
    // 中文: 異體字候選開關。預設 false。
    var isVariantEnabled: Bool {
        get { userDefaults.value(for: Self.isVariantEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isVariantEnabledKey) }
    }

    /// Khiin supplementary data toggle (default: off)
    // 中文: Khiin 補充資料開關。預設 false。
    var isKhiinEnabled: Bool {
        get { userDefaults.value(for: Self.isKhiinEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKhiinEnabledKey) }
    }

    /// LKK Hàn-lô mixed script suggestions (default: on)
    // 中文: LKK 漢羅混寫候選開關。預設 true。
    var isLkkDictEnabled: Bool {
        get { userDefaults.value(for: Self.isLkkDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isLkkDictEnabledKey) }
    }

    /// Developer supplement dictionary (詞庫增補檔案) toggle (default: on)
    // 中文: 開發者補充辭典 (詞庫增補檔案) 開關。預設 true。
    var isDevDictEnabled: Bool {
        get { userDefaults.value(for: Self.isDevDictEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isDevDictEnabledKey) }
    }

    // MARK: - Kautian Subcollections (nested under the kautian master, default on)

    // 中文: 鹿港偏泉腔 (語音差異)。預設 true。
    var isKautianAccentLukangEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentLukangEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentLukangEnabledKey) }
    }

    // 中文: 三峽偏泉腔 (語音差異)。預設 true。
    var isKautianAccentSansiaEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentSansiaEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentSansiaEnabledKey) }
    }

    // 中文: 臺北偏泉腔 (語音差異)。預設 true。
    var isKautianAccentTaipakEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentTaipakEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentTaipakEnabledKey) }
    }

    // 中文: 宜蘭偏漳腔 (語音差異)。預設 true。
    var isKautianAccentGilanEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentGilanEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentGilanEnabledKey) }
    }

    // 中文: 臺南混合腔 (語音差異)。預設 true。
    var isKautianAccentTainanEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentTainanEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentTainanEnabledKey) }
    }

    // 中文: 高雄混合腔 (語音差異)。預設 true。
    var isKautianAccentKaohsiungEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentKaohsiungEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentKaohsiungEnabledKey) }
    }

    // 中文: 金門偏泉腔 (語音差異)。預設 true。
    var isKautianAccentKinmenEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentKinmenEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentKinmenEnabledKey) }
    }

    // 中文: 馬公偏泉腔 (語音差異)。預設 true。
    var isKautianAccentMakungEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentMakungEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentMakungEnabledKey) }
    }

    // 中文: 新竹偏泉腔 (語音差異)。預設 true。
    var isKautianAccentSintikEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentSintikEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentSintikEnabledKey) }
    }

    // 中文: 臺中偏漳腔 (語音差異)。預設 true。
    var isKautianAccentTaichungEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianAccentTaichungEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianAccentTaichungEnabledKey) }
    }

    // 中文: 姓名附錄 (名 + 姓)。預設 true (opt-out)。
    var isKautianNameAppendixEnabled: Bool {
        get { userDefaults.value(for: Self.isKautianNameAppendixEnabledKey) }
        set { userDefaults.set(newValue, for: Self.isKautianNameAppendixEnabledKey) }
    }

    // MARK: - Toolbar Settings

    /// Toolbar auto-collapse toggle (default: true = auto-collapse on composing/mode change)
    // 中文: 工具列自動收合開關。true 時組字或切模式會自動收合工具列。預設 true。
    var isToolbarAutoCollapse: Bool {
        get { userDefaults.value(for: Self.isToolbarAutoCollapseKey) }
        set { userDefaults.set(newValue, for: Self.isToolbarAutoCollapseKey) }
    }

    // MARK: - Globe Key

    /// Globe key toggle. Default depends on device type for backward compatibility:
    /// iPad/iPhone SE (Touch ID) = true, regular iPhone = false.
    // 中文: 地球鍵開關。預設值由 DeviceCapabilities.prefersGlobeKeyByDefault 決定 (iPad / Touch ID iPhone 為 true)。
    // 中文: 一旦使用者寫入過值,後續以儲存值為準,不再走 device-default fallback。
    var isGlobeKeyEnabled: Bool {
        get {
            guard let stored = userDefaults.storedObject(for: Self.isGlobeKeyEnabledKey) as? Bool else {
                return DeviceCapabilities.prefersGlobeKeyByDefault
            }
            return stored
        }
        set { userDefaults.set(newValue, for: Self.isGlobeKeyEnabledKey) }
    }

    // MARK: - TPS Settings

    /// TPS "or" maps to ㄜ (default: on). When off, "or" maps to ㄛ.
    // 中文: TPS 中 "or" 映射對象開關。true 時映射到ㄜ,false 時映射到ㄛ。預設 true。
    var isTpsOrMappedToER: Bool {
        get { userDefaults.value(for: Self.isTpsOrMappedToERKey) }
        set { userDefaults.set(newValue, for: Self.isTpsOrMappedToERKey) }
    }

    // MARK: - Appearance (scale factor, default 1.0)

    // 中文: 鍵盤高度縮放係數。預設 1.0。
    var keyHeightScale: CGFloat {
        get { userDefaults.value(for: Self.keyHeightScaleKey) }
        set { userDefaults.set(newValue, for: Self.keyHeightScaleKey) }
    }

    // 中文: 鍵帽字體大小縮放係數。預設 1.0。
    var keyFontSizeScale: CGFloat {
        get { userDefaults.value(for: Self.keyFontSizeScaleKey) }
        set { userDefaults.set(newValue, for: Self.keyFontSizeScaleKey) }
    }

    // 中文: 候選列文字大小縮放係數。預設 1.0。
    var candidateTextSizeScale: CGFloat {
        get { userDefaults.value(for: Self.candidateTextSizeScaleKey) }
        set { userDefaults.set(newValue, for: Self.candidateTextSizeScaleKey) }
    }

    // 中文: 鍵帽圓角半徑 (point)。預設 6.0。
    var keyCornerRadius: CGFloat {
        get { userDefaults.value(for: Self.keyCornerRadiusKey) }
        set { userDefaults.set(newValue, for: Self.keyCornerRadiusKey) }
    }

    // 中文: 鍵帽邊框寬度 (point)。預設 0 (無邊框)。
    var keyBorderWidth: CGFloat {
        get { userDefaults.value(for: Self.keyBorderWidthKey) }
        set { userDefaults.set(newValue, for: Self.keyBorderWidthKey) }
    }

    // 中文: 鍵盤顏色組合(自由配色 buffer)。以 JSON 序列化進 UserDefaults;decode 失敗或未設定時回傳 .default (全 nil)。
    // 中文: 主題模型下,此欄位 = "default" 主題解析來源,也是外觀編輯器寫入目標(editor scratch)。
    var colorSettings: KeyboardColorSettings {
        get { userDefaults.value(for: Self.colorSettingsKey) }
        set { userDefaults.set(newValue, for: Self.colorSettingsKey) }
    }

    // 中文: 選定主題 id。"default" 時渲染走 colorSettings;其餘走 userThemeStore / built-in。
    var selectedThemeId: String {
        get { userDefaults.value(for: Self.selectedThemeIdKey) }
        set { userDefaults.set(newValue, for: Self.selectedThemeIdKey) }
    }

    // 中文: 使用者自訂主題儲存(App Group JSON 檔,排除備份)。變更時 bump themeRevision 觸發跨進程刷新。
    private lazy var userThemeStore = UserThemeStore(
        containerURL: Self.sharedContainerURL,
        onMutated: { [weak self] in self?.bumpThemeRevision() },
    )

    // 中文: 主題版本 +1。寫入 themeRevision 鍵 → 觸發 didChangeNotification,讓 keyboard extension 重新解析主題。
    private func bumpThemeRevision() {
        let next = userDefaults.value(for: Self.themeRevisionKey) &+ 1
        userDefaults.set(next, for: Self.themeRevisionKey)
    }

    // 中文: 渲染端消費的解析主題。"default" 走快路徑(免 file I/O,維持現有行為)。
    // 中文: 只有 id 為 UUID(user theme)才讀 user-theme 檔;非 UUID(built-in id)不讀檔 —
    // 中文: 避免 render 熱路徑無謂 I/O,built-in 由 ThemeResolver 查 BuiltInThemes 表並依 colorScheme 取 light/dark。
    func resolvedTheme(for colorScheme: ColorScheme) -> ResolvedKeyboardTheme {
        let id = selectedThemeId
        if id == ThemeId.default {
            return ResolvedKeyboardTheme(colors: colorSettings, keyShadowIntensity: 0)
        }
        let userThemes = UUID(uuidString: id) != nil ? userThemeStore.load() : []
        return ThemeResolver.resolved(
            themeId: id,
            colorScheme: colorScheme,
            legacyColorSettings: colorSettings,
            userThemes: userThemes,
        )
    }

    /// Creates an immutable snapshot of render-relevant settings.
    /// Call once per render cycle to avoid repeated UserDefaults reads.
    // 中文: 取得渲染週期一致快照。每次渲染 (~50 個鍵) 呼叫一次,避免每個鍵都打 UserDefaults。
    func snapshot(for colorScheme: ColorScheme) -> SettingsSnapshot {
        SettingsSnapshot(
            inputMode: inputMode,
            fontType: fontType,
            keyboardLayoutType: keyboardLayoutType,
            isTranslateSwapped: isTranslateSwapped,
            isTpsOrMappedToER: isTpsOrMappedToER,
            keyFontSizeScale: keyFontSizeScale,
            keyCornerRadius: keyCornerRadius,
            colorSettings: resolvedTheme(for: colorScheme).colors,
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
        isLiteralRomanCandidateEnabled = true
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
        isDevDictEnabled = true
        // Kautian subcollections (10 accents default on; name appendix default off)
        isKautianAccentLukangEnabled = true
        isKautianAccentSansiaEnabled = true
        isKautianAccentTaipakEnabled = true
        isKautianAccentGilanEnabled = true
        isKautianAccentTainanEnabled = true
        isKautianAccentKaohsiungEnabled = true
        isKautianAccentKinmenEnabled = true
        isKautianAccentMakungEnabled = true
        isKautianAccentSintikEnabled = true
        isKautianAccentTaichungEnabled = true
        isKautianNameAppendixEnabled = true
        // Toolbar
        isToolbarAutoCollapse = true
        // Globe key: remove stored value so device-based default takes effect
        userDefaults.remove(Self.isGlobeKeyEnabledKey)
        // TPS
        isTpsOrMappedToER = true
        // Appearance
        keyHeightScale = 1.0
        keyFontSizeScale = 1.0
        candidateTextSizeScale = 1.0
        keyCornerRadius = 6.0
        keyBorderWidth = 0
        colorSettings = .default
        // 中文: 回到 default 主題(走 colorSettings buffer);不刪除已存的 user themes。
        selectedThemeId = ThemeId.default

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
    /// engine-layer code (e.g. candidate capitalization via
    /// `RustEngineBridge.capitalizeCandidate`) can read it through
    /// `EngineSettings` without importing KeyboardKit.
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
