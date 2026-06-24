// 中文: 內建主題靜態表 — 三個「按鍵風格」family(經典 / 框線 / 簡潔),共用同一組 7 色(預設 + 5 light 漸層 + 1 dark 暗眠山貓)。
// 中文: 三 family 顏色完全相同,差別只在按鍵風格:經典 = 一般填色鍵;框線 = 透明鍵 + 邊框;簡潔 = 透明鍵無框。
// 中文: 透明鍵 = normalKeyFill/specialKeyFill 設成 clear(alpha 0)→ 鍵盤背景(平塗/漸層)從鍵面透出 = 「鍵=背景」。
// 中文: 5 漸層 light-only(dark = nil,深色模式仍顯 light);暗眠山貓 dark-only(light = nil,淺色模式仍顯 dark);預設保持 adaptive(bg/text 留 nil,render 端依系統 colorScheme 解析)。
// 中文: id 為非 UUID 字串。經典維持既有 id(default sentinel / standardPink…);框線 = framed*、簡潔 = clean*。

import Foundation

/// One built-in theme family — a section header (`title`) plus its variant
/// themes, shown as a horizontal shelf in the theme picker. The three families
/// (經典 / 框線 / 簡潔) are a key-STYLE axis over one shared set of 7 colors.
// 中文: 單一內建主題 family — section 標題 + variant 主題(主題頁一條橫向 shelf)。三 family = 按鍵風格軸。
struct BuiltInThemeFamily: Equatable {
    let titleKey: StringKey
    let themes: [BuiltInTheme]
}

/// The app-bundled, read-only theme catalog. Three key-style families, each with
/// the same 7 colors (adaptive 預設 + 5 light-only gradients + 1 dark-only 暗眠山貓).
/// 經典 keeps filled keys; 框線 / 簡潔 make keys transparent (background shows
/// through), 框線 adding an outline.
// 中文: 內建主題目錄 — 三個按鍵風格 family,各 7 個相同顏色的主題。
enum BuiltInThemes {
    /// Shelf order shown to the user (after `Default`, before user themes).
    // 中文: 顯示順序(在 Default 之後、使用者自訂主題之前)。
    static let families: [BuiltInThemeFamily] = [
        BuiltInThemeFamily(titleKey: .themeFamilyClassic, themes: familyThemes(.classic)),
        BuiltInThemeFamily(titleKey: .themeFamilyFramed, themes: familyThemes(.framed)),
        BuiltInThemeFamily(titleKey: .themeFamilyClean, themes: familyThemes(.clean)),
    ]

    /// Flattened lookup roster — used by `ThemeResolver` / `SharedSettings` to
    /// resolve a selected id.
    // 中文: 攤平後的查表清單(供 resolver/SharedSettings 以 id 解析)。
    static let all: [BuiltInTheme] = families.flatMap(\.themes)

    /// Looks up a built-in by id; `nil` when the id is not a built-in.
    static func theme(id: String) -> BuiltInTheme? {
        all.first { $0.id == id }
    }

    // MARK: - Key style

    /// The per-family key-style axis. All three families share one set of colors;
    /// only the key rendering differs.
    // 中文: 每個 family 的按鍵風格軸。三 family 共用顏色,只差按鍵呈現。
    private enum KeyStyle {
        case classic // filled keys (white over a gradient, adaptive for 預設)
        case framed // transparent keys + outline border
        case clean // transparent keys, no border

        /// Keys are transparent (background shows through) for framed / clean.
        var hasTransparentKeys: Bool { self != .classic }
        /// Only the framed family draws the key outline.
        var isBordered: Bool { self == .framed }
        /// id prefix per family — 經典 keeps the legacy `standard*` ids.
        var idPrefix: String {
            switch self {
            case .classic: "standard"
            case .framed: "framed"
            case .clean: "clean"
            }
        }
    }

    // MARK: - Shared colors

    /// One of the 7 shared color identities. `gradient == nil` is the adaptive
    /// 預設 head; the next 5 are soft light single-hue gradients; the last is the
    /// dark-only 暗眠山貓 (Catppuccin Mocha) gradient (`isDarkPalette`), which lands
    /// in the `dark` variant slot with light text over a dark gradient.
    // 中文: 7 個共用顏色之一。gradient == nil = adaptive 預設;接著 5 個是柔和單色相 light 漸層;最後 1 個是 dark-only 暗眠山貓(Catppuccin Mocha),放 dark slot。
    private struct BaseColor {
        let key: String
        let displayNameKey: StringKey
        let gradient: (top: UInt32, bottom: UInt32)?
        /// Dark palette (e.g. 暗眠山貓/Catppuccin): light text over a dark gradient,
        /// with a dark neutral key fill; builds into the `dark` variant slot
        /// (`light` nil) so it stays dark regardless of system scheme.
        // 中文: dark 主題:深漸層配 light 字 + 深中性鍵色,放 dark slot(light=nil),不隨系統明暗變。
        var isDarkPalette: Bool = false
    }

    // 中文: 7 色順序:預設(adaptive)→ 櫻花 → 金煌 → 海風 → 翠青 → 藤紫(以上 light)→ 暗眠山貓(dark)。漸層 hex 為視覺估值。
    private static let baseColors: [BaseColor] = [
        BaseColor(key: "default", displayNameKey: .themePaletteDefault, gradient: nil),
        BaseColor(key: "pink", displayNameKey: .themePalettePink, gradient: (0xE6C2D0, 0xEADCE2)),
        BaseColor(key: "gold", displayNameKey: .themePaletteGold, gradient: (0xEAD9A6, 0xECE4D2)),
        BaseColor(key: "blue", displayNameKey: .themePaletteBlue, gradient: (0xBFD2EA, 0xDCE2EC)),
        BaseColor(key: "green", displayNameKey: .themePaletteGreen, gradient: (0xC3D8C8, 0xDCE5DD)),
        BaseColor(key: "purple", displayNameKey: .themePalettePurple, gradient: (0xCDC4E4, 0xDEDAEA)),
        // 暗眠山貓: Catppuccin Mocha — 背景 Base→Mantle 漸層(比鍵深),鍵 Surface0,字 Text。
        BaseColor(key: "catppuccin", displayNameKey: .themePaletteCatppuccin, gradient: (0x1E1E2E, 0x181825), isDarkPalette: true),
    ]

    // 中文: 漸層中性鍵色 + 鍵字色 — light 主題(經典白鍵 / ≈經典黑字)與 dark 主題(暗眠山貓:Catppuccin Mocha Surface0 鍵 / Text 字)各一組。
    // CROSS-PLATFORM INVARIANT — mirrors android .../ime/core/BuiltInThemes.kt LIGHT_KEY_FILL/LIGHT_KEY_TEXT/DARK_KEY_FILL/DARK_KEY_TEXT.
    // Drift causes silent divergence (iOS/Android theme key colors differ).
    private static let lightKeyFill: UInt32 = 0xFFFFFF
    private static let lightKeyText: UInt32 = 0x1C1C1E
    private static let darkKeyFill: UInt32 = 0x313244 // Catppuccin Mocha Surface0
    private static let darkKeyText: UInt32 = 0xCDD6F4 // Catppuccin Mocha Text

    // CROSS-PLATFORM INVARIANT — mirrors android .../ime/core/BuiltInThemes.kt OUTLINED_KEY_BORDER_WIDTH.
    // Drift causes silent divergence. 框線 family 的鍵邊框寬度。
    private static let outlinedKeyBorderWidth: Double = 1.0

    // MARK: - Builders

    /// Builds the 7 themes for one key-style family. The 經典 head keeps the
    /// `ThemeId.default` sentinel (so reset shows it selected); framed / clean use
    /// `framedDefault` / `cleanDefault` ids. A light theme builds into the `light`
    /// slot (`dark` nil); a dark theme (暗眠山貓) builds into the `dark` slot
    /// (`light` nil) — mirror-symmetric. Preview slots mirror the family id prefix;
    /// missing assets fall back to a neutral placeholder until screenshots ship.
    // 中文: 組單一 family 的 7 主題。經典 head 用 default sentinel;框線/簡潔 用 framed*/clean* id。light 主題放 light slot、dark 主題(暗眠山貓)放 dark slot。
    private static func familyThemes(_ style: KeyStyle) -> [BuiltInTheme] {
        baseColors.map { base in
            let isDefault = base.gradient == nil
            let suffix = base.key.prefix(1).uppercased() + base.key.dropFirst()
            let id = isDefault
                ? (style == .classic ? ThemeId.default : "\(style.idPrefix)Default")
                : "\(style.idPrefix)\(suffix)"
            let previewName = isDefault
                ? "theme_\(style.idPrefix)_preview"
                : "theme_\(style.idPrefix)\(suffix)_preview"
            let scheme = colors(for: base, style: style)
            return BuiltInTheme(
                id: id,
                displayNameKey: base.displayNameKey,
                light: base.isDarkPalette ? nil : scheme,
                dark: base.isDarkPalette ? scheme : nil,
                previewImageName: previewName,
                keyBorderWidth: style.isBordered ? outlinedKeyBorderWidth : nil,
            )
        }
    }

    /// Resolves the color palette for one (color, key-style) pair. 經典 預設 stays
    /// fully adaptive (`nil`); framed / clean 預設 carry only transparent key fills
    /// so the adaptive background/text still show through and adapt to dark mode.
    // 中文: 解析 (顏色, 按鍵風格) 的配色。經典 預設 = 全 nil(adaptive);框線/簡潔 預設 = 只帶透明鍵填色。
    private static func colors(for base: BaseColor, style: KeyStyle) -> KeyboardColorSettings? {
        if let gradient = base.gradient {
            let keyText = base.isDarkPalette ? darkKeyText : lightKeyText
            let neutralFill = base.isDarkPalette ? darkKeyFill : lightKeyFill
            return gradientColors(top: gradient.top, bottom: gradient.bottom, keyText: keyText, neutralFill: neutralFill, transparentKeys: style.hasTransparentKeys)
        }
        guard style.hasTransparentKeys else { return nil } // 經典 預設 = adaptive
        var colors = KeyboardColorSettings()
        colors.normalKeyFillColor = CodableColor(.clear)
        colors.specialKeyFillColor = CodableColor(.clear)
        return colors
    }

    /// One scheme variant for a gradient color: the 2-stop background gradient +
    /// key/candidate text (`keyText`). Keys are either the neutral fill (`neutralFill`,
    /// 經典) or transparent so the gradient shows through (框線 / 簡潔). `backgroundColor`
    /// and `candidateBackgroundColor` stay nil — the gradient owns the background
    /// and the candidate bar is made transparent in `TaigiKeyboardView.candidateStyle`.
    // 中文: 漸層顏色單一 scheme variant — 2-stop 漸層 + 字色(keyText);鍵 = 中性填色(neutralFill,經典)或透明(框線/簡潔)。light/dark 主題傳入各自的 keyText/neutralFill。
    private static func gradientColors(top: UInt32, bottom: UInt32, keyText: UInt32, neutralFill: UInt32, transparentKeys: Bool) -> KeyboardColorSettings {
        var colors = KeyboardColorSettings()
        colors.backgroundGradient = ThemeGradient(stops: [CodableColor(hex: top), CodableColor(hex: bottom)])
        colors.keyTextColor = CodableColor(hex: keyText)
        colors.candidateTextColor = CodableColor(hex: keyText)
        if transparentKeys {
            colors.normalKeyFillColor = CodableColor(.clear)
            colors.specialKeyFillColor = CodableColor(.clear)
        } else {
            let fill = CodableColor(hex: neutralFill)
            colors.normalKeyFillColor = fill
            colors.specialKeyFillColor = fill
        }
        return colors
    }
}
