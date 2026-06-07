// 中文: KeyboardKit 10 的 Layout Service 實作。
// 中文: 根據 keyboardType / 使用者設定 / 裝置決定 [[KeyDef]],再交給 LayoutConverter 轉成 KeyboardLayout。

import KeyboardKit

private let layoutLogger = DebugLogger(category: "CustomLayoutService")

/// KeyboardKit 10 相容的 Layout Service
///
/// 根據鍵盤類型、設定、裝置選擇對應的佈局，
/// 並透過 LayoutConverter 轉換為 KeyboardLayout。
class CustomLayoutService {
    /// 根據 context 建構鍵盤 layout
    ///
    /// `appearance` 預設 nil = 解析當前選定主題的全域外觀(keyboard extension 用)。
    /// 外觀編輯器預覽傳入 draft `ThemeAppearance`,讓 row height / corner 跟著草稿走,
    /// 不必先寫進 SharedSettings(編輯器是 draft-and-save)。
    // 中文: appearance nil → 走全域選定主題;傳入則用該草稿外觀(預覽專用)。
    func keyboardLayout(
        for context: KeyboardContext,
        appearance: ThemeAppearance? = nil,
    ) -> KeyboardLayout {
        var config = KeyboardLayout.DeviceConfiguration.standard(for: context)
        // Layout geometry follows the active (or draft) theme (per-theme key height /
        // corner radius). Non-appearance reads (inputMode / layoutType / globe) stay on
        // the live settings below.
        let appearance = appearance ?? SharedSettings.shared.resolvedAppearance(for: context.colorScheme)
        config.rowHeight *= (0.87 * appearance.keyHeightScale)
        config.buttonCornerRadius = appearance.keyCornerRadius
        let converter = LayoutConverter(context: context, config: config)
        let keyDefs = selectLayout(for: context)

        layoutLogger.debug("[LAYOUT] keyboardType=\(String(describing: context.keyboardType)) rows=\(keyDefs.count)")

        return converter.convert(keyDefs)
    }

    // MARK: - Private

    /// Returns the globe or iPhone variant based on device requirements
    // 中文: 依是否需要地球鍵,在 withGlobe / iPhone 兩種變體之間擇一。
    private func resolveLayout(
        withGlobe: [[KeyDef]],
        iPhone: [[KeyDef]],
        needsGlobe: Bool,
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

    /// Determines whether the globe key should be shown.
    /// English mode: preserves device-dependent behavior (iPad/iPhone SE).
    /// Other modes: uses user toggle setting.
    // 中文: 是否要顯示地球鍵。English 模式依裝置(iPad / iPhone SE)決定;其他模式看使用者開關。
    // 中文: TPS 佈局按鍵已多,永遠不顯示地球鍵。
    private func needsGlobeKey(for context: KeyboardContext) -> Bool {
        let settings = SharedSettings.shared
        if context.keyboardType == .alphabetic, settings.inputMode == .english {
            let device = DeviceConfiguration()
            return device.isIPad || device.isSmallIPhone
        }
        // TPS layout has more keys — never show globe key
        if settings.inputMode == .tps { return false }
        return settings.isGlobeKeyEnabled
    }

    /// 選擇 Alphabetic 鍵盤佈局
    private func selectAlphabeticLayout(
        settings: SharedSettings,
        needsGlobe: Bool,
    ) -> [[KeyDef]] {
        let layouts = TaigiLayouts.Alphabetic.self

        // English mode (Apple standard English keyboard)
        if settings.inputMode == .english {
            return resolveLayout(withGlobe: layouts.qwerty_English_withGlobe, iPhone: layouts.qwerty_English_iPhone, needsGlobe: needsGlobe)
        }

        // TPS mode — inputMode takes priority over keyboardLayoutType
        if settings.inputMode == .tps {
            return resolveLayout(withGlobe: layouts.tps_withGlobe, iPhone: layouts.tps_iPhone, needsGlobe: needsGlobe)
        }

        switch settings.keyboardLayoutType {
        case .tps:
            return resolveLayout(withGlobe: layouts.tps_withGlobe, iPhone: layouts.tps_iPhone, needsGlobe: needsGlobe)

        case .phahTaigi:
            return resolveLayout(withGlobe: layouts.phahTaigi_withGlobe, iPhone: layouts.phahTaigi_iPhone, needsGlobe: needsGlobe)

        case .moe1:
            if settings.inputMode == .poj {
                return resolveLayout(withGlobe: layouts.moe1_POJ_withGlobe, iPhone: layouts.moe1_POJ_iPhone, needsGlobe: needsGlobe)
            }
            return resolveLayout(withGlobe: layouts.moe1_TL_withGlobe, iPhone: layouts.moe1_TL_iPhone, needsGlobe: needsGlobe)

        case .moe2:
            if settings.inputMode == .poj {
                return resolveLayout(withGlobe: layouts.moe2_POJ_withGlobe, iPhone: layouts.moe2_POJ_iPhone, needsGlobe: needsGlobe)
            }
            return resolveLayout(withGlobe: layouts.moe2_TL_withGlobe, iPhone: layouts.moe2_TL_iPhone, needsGlobe: needsGlobe)

        case .qwerty:
            if settings.inputMode == .poj {
                return resolveLayout(withGlobe: layouts.qwerty_POJ_withGlobe, iPhone: layouts.qwerty_POJ_iPhone, needsGlobe: needsGlobe)
            }
            return resolveLayout(withGlobe: layouts.qwerty_TL_withGlobe, iPhone: layouts.qwerty_TL_iPhone, needsGlobe: needsGlobe)
        }
    }
}
