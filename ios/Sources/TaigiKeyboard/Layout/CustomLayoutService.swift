import KeyboardKit
import OSLog

#if DEBUG
private let layoutLogger = Logger(
    subsystem: LexiconConstants.Logging.subsystem,
    category: "CustomLayoutService"
)
#endif

/// KeyboardKit 10 相容的 Layout Service
///
/// 根據鍵盤類型、設定、裝置選擇對應的佈局，
/// 並透過 LayoutConverter 轉換為 KeyboardLayout。
class CustomLayoutService {

    /// 根據 context 建構鍵盤 layout
    func keyboardLayout(for context: KeyboardContext) -> KeyboardLayout {
        var config = KeyboardLayout.DeviceConfiguration.standard(for: context)
        let settings = SharedSettings.shared
        config.rowHeight *= (0.87 * settings.keyHeightScale)
        config.buttonCornerRadius = settings.keyCornerRadius
        let converter = LayoutConverter(context: context, config: config)
        let keyDefs = selectLayout(for: context)

        #if DEBUG
        layoutLogger.debug("[LAYOUT] keyboardType=\(String(describing: context.keyboardType), privacy: .public) rows=\(keyDefs.count)")
        #endif

        return converter.convert(keyDefs)
    }

    // MARK: - Private

    /// 根據 keyboardType、設定、裝置選擇對應的佈局
    private func selectLayout(for context: KeyboardContext) -> [[KeyDef]] {
        let settings = SharedSettings.shared
        let needsGlobe = needsGlobeKey(for: context)

        switch context.keyboardType {
        case .alphabetic:
            return selectAlphabeticLayout(settings: settings, needsGlobe: needsGlobe)
        case .numeric:
            return needsGlobe ? TaigiLayouts.Numeric.withGlobe : TaigiLayouts.Numeric.iPhone
        case .symbolic:
            return needsGlobe ? TaigiLayouts.Symbolic.withGlobe : TaigiLayouts.Symbolic.iPhone
        default:
            return TaigiLayouts.Alphabetic.qwerty_TL_iPhone
        }
    }

    /// 判斷是否需要 globe 鍵（iPhone SE 或 iPad）
    private func needsGlobeKey(for context: KeyboardContext) -> Bool {
        let device = DeviceConfiguration(context: context)
        return device.isIPad || device.isSmallIPhone
    }

    /// 選擇 Alphabetic 鍵盤佈局
    private func selectAlphabeticLayout(
        settings: SharedSettings,
        needsGlobe: Bool
    ) -> [[KeyDef]] {
        // English 模式（Apple 標準英文鍵盤）
        if settings.inputMode == .english {
            return needsGlobe
                ? TaigiLayouts.Alphabetic.qwerty_English_withGlobe
                : TaigiLayouts.Alphabetic.qwerty_English_iPhone
        }

        // 根據 keyboardLayoutType 選擇佈局
        switch settings.keyboardLayoutType {
        case .tps:
            // 方音符號佈局
            return needsGlobe
                ? TaigiLayouts.Alphabetic.tps_withGlobe
                : TaigiLayouts.Alphabetic.tps_iPhone

        case .phahTaigi:
            // PhahTaigi 佈局
            return needsGlobe
                ? TaigiLayouts.Alphabetic.phahTaigi_withGlobe
                : TaigiLayouts.Alphabetic.phahTaigi_iPhone

        case .moe1:
            // 教育部輸入法佈局1（根據 inputMode 選擇 TL 或 POJ）
            if settings.inputMode == .poj {
                return needsGlobe
                    ? TaigiLayouts.Alphabetic.moe1_POJ_withGlobe
                    : TaigiLayouts.Alphabetic.moe1_POJ_iPhone
            }
            return needsGlobe
                ? TaigiLayouts.Alphabetic.moe1_TL_withGlobe
                : TaigiLayouts.Alphabetic.moe1_TL_iPhone

        case .moe2:
            // 教育部輸入法佈局2（根據 inputMode 選擇 TL 或 POJ）
            if settings.inputMode == .poj {
                return needsGlobe
                    ? TaigiLayouts.Alphabetic.moe2_POJ_withGlobe
                    : TaigiLayouts.Alphabetic.moe2_POJ_iPhone
            }
            return needsGlobe
                ? TaigiLayouts.Alphabetic.moe2_TL_withGlobe
                : TaigiLayouts.Alphabetic.moe2_TL_iPhone

        case .qwerty:
            // QWERTY 佈局（根據 inputMode 選擇 POJ 或 TL）
            if settings.inputMode == .poj {
                return needsGlobe
                    ? TaigiLayouts.Alphabetic.qwerty_POJ_withGlobe
                    : TaigiLayouts.Alphabetic.qwerty_POJ_iPhone
            }
            // TL 模式（預設）
            return needsGlobe
                ? TaigiLayouts.Alphabetic.qwerty_TL_withGlobe
                : TaigiLayouts.Alphabetic.qwerty_TL_iPhone
        }
    }
}
