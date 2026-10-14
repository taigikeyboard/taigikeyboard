import KeyboardKit

private let layoutLogger = DebugLogger(category: "CustomLayoutService")

/// KeyboardKit 10-compatible layout service. Picks the `[[KeyDef]]` set from keyboard type,
/// settings and device, then converts it to a `KeyboardLayout` via `LayoutConverter`.
class CustomLayoutService {
    /// Builds the keyboard layout for `context`. A nil `appearance` resolves the globally selected
    /// theme's appearance (what the keyboard extension uses); the appearance editor's preview passes
    /// a draft `ThemeAppearance` so row height / corner follow it without writing to SharedSettings.
    func keyboardLayout(
        for context: KeyboardContext,
        appearance: ThemeAppearance? = nil,
    ) -> KeyboardLayout {
        var config = KeyboardLayoutConfiguration.standard(for: context)
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
    private func resolveLayout(
        withGlobe: [[KeyDef]],
        iPhone: [[KeyDef]],
        needsGlobe: Bool,
    ) -> [[KeyDef]] {
        needsGlobe ? withGlobe : iPhone
    }

    private func selectLayout(for context: KeyboardContext) -> [[KeyDef]] {
        let settings = SharedSettings.shared
        let needsGlobe = needsGlobeKey(for: context)

        switch context.keyboardType {
        case .alphabetic:
            return selectAlphabeticLayout(settings: settings, needsGlobe: needsGlobe, inputType: context.keyboardInputType)
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
    private func needsGlobeKey(for context: KeyboardContext) -> Bool {
        let settings = SharedSettings.shared
        if context.keyboardType == .alphabetic, settings.inputMode == .english {
            let device = DeviceConfiguration()
            return device.isIPad || device.isSmallIPhone
        }
        // TPS layout has more keys — never show globe key
        if settings.inputMode == .tps {
            return false
        }
        return settings.isGlobeKeyEnabled
    }

    private func selectAlphabeticLayout(
        settings: SharedSettings,
        needsGlobe: Bool,
        inputType: Keyboard.InputType,
    ) -> [[KeyDef]] {
        let layouts = TaigiLayouts.Alphabetic.self
        let rows: [[KeyDef]]

        // English mode (Apple standard English keyboard)
        if settings.inputMode == .english {
            rows = resolveLayout(withGlobe: layouts.qwerty_English_withGlobe, iPhone: layouts.qwerty_English_iPhone, needsGlobe: needsGlobe)
            return Self.applyInputTypeKey(rows, inputType: inputType)
        }

        // TPS mode — inputMode takes priority over keyboardLayoutType. TPS rows get no
        // input-type key (Android `tps_halfwidth.json` has none).
        if settings.inputMode == .tps {
            return resolveLayout(withGlobe: layouts.tps_withGlobe, iPhone: layouts.tps_iPhone, needsGlobe: needsGlobe)
        }

        switch settings.keyboardLayoutType {
        case .tps:
            return resolveLayout(withGlobe: layouts.tps_withGlobe, iPhone: layouts.tps_iPhone, needsGlobe: needsGlobe)

        case .phahTaigi:
            rows = resolveLayout(withGlobe: layouts.phahTaigi_withGlobe, iPhone: layouts.phahTaigi_iPhone, needsGlobe: needsGlobe)

        case .moe1:
            rows = settings.inputMode == .poj
                ? resolveLayout(withGlobe: layouts.moe1_POJ_withGlobe, iPhone: layouts.moe1_POJ_iPhone, needsGlobe: needsGlobe)
                : resolveLayout(withGlobe: layouts.moe1_TL_withGlobe, iPhone: layouts.moe1_TL_iPhone, needsGlobe: needsGlobe)

        case .moe2:
            rows = settings.inputMode == .poj
                ? resolveLayout(withGlobe: layouts.moe2_POJ_withGlobe, iPhone: layouts.moe2_POJ_iPhone, needsGlobe: needsGlobe)
                : resolveLayout(withGlobe: layouts.moe2_TL_withGlobe, iPhone: layouts.moe2_TL_iPhone, needsGlobe: needsGlobe)

        case .qwerty:
            rows = settings.inputMode == .poj
                ? resolveLayout(withGlobe: layouts.qwerty_POJ_withGlobe, iPhone: layouts.qwerty_POJ_iPhone, needsGlobe: needsGlobe)
                : resolveLayout(withGlobe: layouts.qwerty_TL_withGlobe, iPhone: layouts.qwerty_TL_iPhone, needsGlobe: needsGlobe)
        }
        return Self.applyInputTypeKey(rows, inputType: inputType)
    }

    /// Puts `@` (email field) or `/` (URL field) on the bottom row: it replaces the
    /// `,` key where the row has one, else sits right after the emoji key. Other
    /// input types return `rows` unchanged. The key is half-width in every mode.
    // CROSS-PLATFORM INVARIANT — mirrors the Android `mod/*.json` bottom-row
    // `email_address` / `uri` variants: default / english / moe2 swap `,`;
    // moe1 / phah_taigi add the key in those fields only.
    static func applyInputTypeKey(_ rows: [[KeyDef]], inputType: Keyboard.InputType) -> [[KeyDef]] {
        let key: String
        switch inputType {
        case .emailAddress: key = "@"
        case .url: key = "/"
        default: return rows
        }
        guard var bottomRow = rows.last else { return rows }
        var commaIndex: Int?
        var emojiIndex: Int?
        for (index, keyDef) in bottomRow.enumerated() {
            switch keyDef {
            case .char(",", _): commaIndex = index
            case .emoji: emojiIndex = index
            default: break
            }
        }
        if let commaIndex {
            bottomRow[commaIndex] = .char(key)
        } else if let emojiIndex {
            bottomRow.insert(.char(key), at: emojiIndex + 1)
        } else {
            return rows
        }
        return rows.dropLast() + [bottomRow]
    }
}
