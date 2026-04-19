import Foundation
import KeyboardKit

final class SharedSettings {
    private let userDefaults: UserDefaults

    static let appGroupId = "group.com.siansiansu.TaigiKeyboard"

    /// App Group shared container URL, accessible by both main app and keyboard extension.
    static var sharedContainerURL: URL? {
        FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: appGroupId)
    }

    static let sharedUserDefaults = UserDefaults(suiteName: appGroupId) ?? .standard

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

    static let shared = SharedSettings()

    /// Guards the mutual recursion between inputMode and keyboardLayoutType
    /// setters when TPS ↔ layout auto-sync fires.
    private let tpsSync = TPSSyncCoordinator()

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

    var isDoubleTapOOEnabled: Bool {
        get {
            userDefaults.object(forKey: Keys.enableDoubleTapOO) as? Bool ?? true
        }
        set {
            userDefaults.set(newValue, forKey: Keys.enableDoubleTapOO)
        }
    }

    var isDoubleTapNNEnabled: Bool {
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

    // isAutoCapitalizationEnabled moved to KeyboardKit's KeyboardSettings
    // Access via state.keyboardContext.settings.isAutocapitalizationEnabled

    var isAutoSpaceEnabled: Bool {
        get {
            userDefaults.object(forKey: Keys.autoSpaceEnabled) as? Bool ?? false
        }
        set {
            userDefaults.set(newValue, forKey: Keys.autoSpaceEnabled)
        }
    }

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
    private var layoutBeforeTps: KeyboardLayoutType {
        get {
            let rawValue = userDefaults.string(forKey: Keys.layoutBeforeTps) ?? KeyboardLayoutType.phahTaigi.rawValue
            return KeyboardLayoutType(rawValue: rawValue) ?? .phahTaigi
        }
        set {
            userDefaults.set(newValue.rawValue, forKey: Keys.layoutBeforeTps)
        }
    }

    var isOutputBothScripts: Bool {
        get {
            userDefaults.object(forKey: Keys.outputBothScripts) as? Bool ?? false
        }
        set {
            userDefaults.set(newValue, forKey: Keys.outputBothScripts)
        }
    }

    // MARK: - Frequency Recording (default: on)

    var isFrequencyRecordingEnabled: Bool {
        get { userDefaults.object(forKey: Keys.frequencyRecordingEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.frequencyRecordingEnabled) }
    }

    // MARK: - Association Recording (default: on)

    var isAssociationRecordingEnabled: Bool {
        get { userDefaults.object(forKey: Keys.associationRecordingEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.associationRecordingEnabled) }
    }

    // MARK: - Custom Dictionary

    var isCustomDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.customDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.customDictEnabled) }
    }

    // MARK: - Dictionary Toggles

    var isMoeDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.moeDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.moeDictEnabled) }
    }

    var isNewwordDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.newwordDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.newwordDictEnabled) }
    }

    var isKunggeDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.kunggeDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.kunggeDictEnabled) }
    }

    var isITaigiDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.iTaigiDictEnabled) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.iTaigiDictEnabled) }
    }

    var isTaiwanJapanDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.taiwanJapanDictEnabled) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.taiwanJapanDictEnabled) }
    }

    var isTaiHuaDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.taiHuaDictEnabled) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.taiHuaDictEnabled) }
    }

    var isTaiwanPlantDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.taiwanPlantDictEnabled) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.taiwanPlantDictEnabled) }
    }

    var isSttiDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.sttiDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.sttiDictEnabled) }
    }

    var isKhpooDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.khpooDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.khpooDictEnabled) }
    }

    /// Variant characters toggle (default: off)
    var isVariantEnabled: Bool {
        get { userDefaults.object(forKey: Keys.variantEnabled) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.variantEnabled) }
    }

    /// Khiin supplementary data toggle (default: off)
    var isKhiinEnabled: Bool {
        get { userDefaults.object(forKey: Keys.khiin) as? Bool ?? false }
        set { userDefaults.set(newValue, forKey: Keys.khiin) }
    }

    /// LKK Hàn-lô mixed script suggestions (default: on)
    var isLkkDictEnabled: Bool {
        get { userDefaults.object(forKey: Keys.lkkDictEnabled) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.lkkDictEnabled) }
    }

    // MARK: - Toolbar Settings

    /// Toolbar auto-collapse toggle (default: true = auto-collapse on composing/mode change)
    var isToolbarAutoCollapse: Bool {
        get { userDefaults.object(forKey: Keys.toolbarAutoCollapse) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.toolbarAutoCollapse) }
    }

    // MARK: - Globe Key

    /// Globe key toggle. Default depends on device type for backward compatibility:
    /// iPad/iPhone SE (Touch ID) = true, regular iPhone = false.
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
    var isTpsOrMappedToER: Bool {
        get { userDefaults.object(forKey: Keys.tpsOrMapsToER) as? Bool ?? true }
        set { userDefaults.set(newValue, forKey: Keys.tpsOrMapsToER) }
    }

    // MARK: - Appearance (scale factor, default 1.0)

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
            isTranslateSwapped: isTranslateSwapped,
            isTpsOrMappedToER: isTpsOrMappedToER,
            keyFontSizeScale: keyFontSizeScale,
            keyCornerRadius: keyCornerRadius,
            colorSettings: colorSettings,
        )
    }

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
        isLkkDictEnabled = false
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
    var isAutoCap: Bool {
        KeyboardSettings.store.bool(
            forKey: "com.keyboardkit.settings.keyboard.isAutocapitalizationEnabled",
        )
    }

    /// Live-reads the two underlying booleans per call, matching the
    /// `EngineSettingsProvider.current` live-read contract.
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
    var current: EngineSettings {
        self
    }
}
