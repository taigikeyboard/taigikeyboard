// 中文: 內建主題靜態表 — 三個「按鍵風格」family(經典 / 框線 / 簡潔),共用同一組 6 色(預設 + 5 漸層)。
// 中文: 三 family 顏色完全相同,差別只在按鍵風格:經典 = 一般填色鍵;框線 = 透明鍵 + 邊框;簡潔 = 透明鍵無框。
// 中文: 透明鍵 = normalKeyFill/specialKeyFill 設成 clear(alpha 0)→ 鍵盤背景(平塗/漸層)從鍵面透出 = 「鍵=背景」。
// 中文: 漸層 + 框線/簡潔 皆 light-only(dark = nil);預設保持 adaptive(bg/text 留 nil,render 端依系統 colorScheme 解析)。
// 中文: id 為非 UUID 字串。經典維持既有 id(default sentinel / standardPink…);框線 = framed*、簡潔 = clean*。

import Foundation

/// One built-in theme family — a section header (`title`) plus its variant
/// themes, shown as a horizontal shelf in the theme picker. The three families
/// (經典 / 框線 / 簡潔) are a key-STYLE axis over one shared set of 6 colors.
// 中文: 單一內建主題 family — section 標題 + variant 主題(主題頁一條橫向 shelf)。三 family = 按鍵風格軸。
struct BuiltInThemeFamily: Equatable {
    let title: String
    let themes: [BuiltInTheme]
}

/// The app-bundled, read-only theme catalog. Three key-style families, each with
/// the same 6 colors (adaptive 預設 + 5 light-only gradients). 經典 keeps filled
/// keys; 框線 / 簡潔 make keys transparent (background shows through), 框線 adding
/// an outline.
// 中文: 內建主題目錄 — 三個按鍵風格 family,各 6 個相同顏色的主題。
enum BuiltInThemes {
    /// Shelf order shown to the user (after `Default`, before user themes).
    // 中文: 顯示順序(在 Default 之後、使用者自訂主題之前)。
    static let families: [BuiltInThemeFamily] = [
        BuiltInThemeFamily(title: "經典", themes: familyThemes(.classic)),
        BuiltInThemeFamily(title: "框線", themes: familyThemes(.framed)),
        BuiltInThemeFamily(title: "簡潔", themes: familyThemes(.clean)),
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

    /// One of the 6 shared color identities. `gradient == nil` is the adaptive
    /// 預設 head; the other 5 are soft single-hue gradients.
    // 中文: 6 個共用顏色之一。gradient == nil = adaptive 預設;其餘 5 個是柔和單色相漸層。
    private struct BaseColor {
        let key: String
        let displayName: String
        let gradient: (top: UInt32, bottom: UInt32)?
    }

    // 中文: 6 色順序:預設(adaptive)→ 櫻花 → 金煌 → 海風 → 翠青 → 藤紫。漸層 hex 為視覺估值。
    private static let baseColors: [BaseColor] = [
        BaseColor(key: "default", displayName: "預設", gradient: nil),
        BaseColor(key: "pink", displayName: "櫻花", gradient: (0xE6C2D0, 0xEADCE2)),
        BaseColor(key: "gold", displayName: "金煌", gradient: (0xEAD9A6, 0xECE4D2)),
        BaseColor(key: "blue", displayName: "海風", gradient: (0xBFD2EA, 0xDCE2EC)),
        BaseColor(key: "green", displayName: "翠青", gradient: (0xC3D8C8, 0xDCE5DD)),
        BaseColor(key: "purple", displayName: "藤紫", gradient: (0xCDC4E4, 0xDEDAEA)),
    ]

    // 中文: 漸層中性鍵色(經典白鍵)+ 鍵字色(≈經典黑);light-only,只需 light 鍵色。
    private static let lightKeyFill: UInt32 = 0xFFFFFF
    private static let lightKeyText: UInt32 = 0x1C1C1E

    // CROSS-PLATFORM INVARIANT — mirrors android .../ime/core/BuiltInThemes.kt OUTLINED_KEY_BORDER_WIDTH.
    // Drift causes silent divergence. 框線 family 的鍵邊框寬度。
    private static let outlinedKeyBorderWidth: Double = 1.0

    // MARK: - Builders

    /// Builds the 6 themes for one key-style family. The 經典 head keeps the
    /// `ThemeId.default` sentinel (so reset shows it selected); framed / clean use
    /// `framedDefault` / `cleanDefault` ids. Preview slots mirror the family id
    /// prefix; only 經典's assets ship today (framed / clean → neutral placeholder).
    // 中文: 組單一 family 的 6 主題。經典 head 用 default sentinel;框線/簡潔 用 framed*/clean* id。
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
            return BuiltInTheme(
                id: id,
                displayName: base.displayName,
                light: colors(for: base, style: style),
                dark: nil,
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
            return gradientColors(top: gradient.top, bottom: gradient.bottom, transparentKeys: style.hasTransparentKeys)
        }
        guard style.hasTransparentKeys else { return nil } // 經典 預設 = adaptive
        var colors = KeyboardColorSettings()
        colors.normalKeyFillColor = CodableColor(.clear)
        colors.specialKeyFillColor = CodableColor(.clear)
        return colors
    }

    /// One scheme variant for a gradient color: the 2-stop background gradient +
    /// key/candidate text. Keys are either the neutral white fill (經典) or
    /// transparent so the gradient shows through (框線 / 簡潔). `backgroundColor`
    /// and `candidateBackgroundColor` stay nil — the gradient owns the background
    /// and the candidate bar is made transparent in `TaigiKeyboardView.candidateStyle`.
    // 中文: 漸層顏色單一 scheme variant — 2-stop 漸層 + 字色;鍵 = 白(經典)或透明(框線/簡潔)。
    private static func gradientColors(top: UInt32, bottom: UInt32, transparentKeys: Bool) -> KeyboardColorSettings {
        var colors = KeyboardColorSettings()
        colors.backgroundGradient = ThemeGradient(stops: [CodableColor(hex: top), CodableColor(hex: bottom)])
        colors.keyTextColor = CodableColor(hex: lightKeyText)
        colors.candidateTextColor = CodableColor(hex: lightKeyText)
        if transparentKeys {
            colors.normalKeyFillColor = CodableColor(.clear)
            colors.specialKeyFillColor = CodableColor(.clear)
        } else {
            let fill = CodableColor(hex: lightKeyFill)
            colors.normalKeyFillColor = fill
            colors.specialKeyFillColor = fill
        }
        return colors
    }
}
