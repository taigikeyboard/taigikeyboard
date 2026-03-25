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

    /// Returns the globe or iPhone variant based on device requirements
    private func resolveLayout(
        withGlobe: [[KeyDef]],
        iPhone: [[KeyDef]],
        needsGlobe: Bool
    ) -> [[KeyDef]] {
        needsGlobe ? withGlobe : iPhone
    }

    /// 根據 keyboardType、設定、裝置選擇對應的佈局
    private func selectLayout(for context: KeyboardContext) -> [[KeyDef]] {
        let settings = SharedSettings.shared
        let needsGlobe = needsGlobeKey(for: context)

        switch context.keyboardType {
        case .alphabetic:
            return selectAlphabeticLayout(settings: settings, needsGlobe: needsGlobe)
        case .numeric:
            return resolveLayout(withGlobe: TaigiLayouts.Numeric.withGlobe, iPhone: TaigiLayouts.Numeric.iPhone, needsGlobe: needsGlobe)
        case .symbolic:
            return resolveLayout(withGlobe: TaigiLayouts.Symbolic.withGlobe, iPhone: TaigiLayouts.Symbolic.iPhone, needsGlobe: needsGlobe)
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
        let A = TaigiLayouts.Alphabetic.self

        // English mode (Apple standard English keyboard)
        if settings.inputMode == .english {
            return resolveLayout(withGlobe: A.qwerty_English_withGlobe, iPhone: A.qwerty_English_iPhone, needsGlobe: needsGlobe)
        }

        // TPS mode — inputMode takes priority over keyboardLayoutType
        if settings.inputMode == .tps {
            return resolveLayout(withGlobe: A.tps_withGlobe, iPhone: A.tps_iPhone, needsGlobe: needsGlobe)
        }

        switch settings.keyboardLayoutType {
        case .tps:
            return resolveLayout(withGlobe: A.tps_withGlobe, iPhone: A.tps_iPhone, needsGlobe: needsGlobe)

        case .phahTaigi:
            return resolveLayout(withGlobe: A.phahTaigi_withGlobe, iPhone: A.phahTaigi_iPhone, needsGlobe: needsGlobe)

        case .moe1:
            if settings.inputMode == .poj {
                return resolveLayout(withGlobe: A.moe1_POJ_withGlobe, iPhone: A.moe1_POJ_iPhone, needsGlobe: needsGlobe)
            }
            return resolveLayout(withGlobe: A.moe1_TL_withGlobe, iPhone: A.moe1_TL_iPhone, needsGlobe: needsGlobe)

        case .moe2:
            if settings.inputMode == .poj {
                return resolveLayout(withGlobe: A.moe2_POJ_withGlobe, iPhone: A.moe2_POJ_iPhone, needsGlobe: needsGlobe)
            }
            return resolveLayout(withGlobe: A.moe2_TL_withGlobe, iPhone: A.moe2_TL_iPhone, needsGlobe: needsGlobe)

        case .qwerty:
            if settings.inputMode == .poj {
                return resolveLayout(withGlobe: A.qwerty_POJ_withGlobe, iPhone: A.qwerty_POJ_iPhone, needsGlobe: needsGlobe)
            }
            return resolveLayout(withGlobe: A.qwerty_TL_withGlobe, iPhone: A.qwerty_TL_iPhone, needsGlobe: needsGlobe)
        }
    }
}
