// 中文: 內建主題靜態表 — KeyboardKit 風格目錄,依 family(Standard / Swifty / Minimal)分區。
// 中文: Standard family 的 Blue/Green/Purple 已上柔和漸層配色;Swifty/Minimal 仍 scaffold(無配色,light/dark = nil)。
// 中文: USER 流程 — 排版固定後逐一補各 theme 配色;漸層主題見 gradientTheme helper。
// 中文: id 為非 UUID、非 "default" 字串(SharedSettings 以 UUID 判斷是否讀檔)。
// 中文: family/variant 名稱取自 KeyboardKit 官網(KK Pro themes 閉源,本地 clone 無此表);USER 之後自行增刪。

import Foundation

/// One built-in theme family — a section header (`title`) plus its variant
/// themes, shown as a horizontal shelf in the theme picker (mirrors the
/// KeyboardKit demo's Standard / Swifty / Minimal grouping).
// 中文: 單一內建主題 family — section 標題 + 該家族的 variant 主題(主題頁一條橫向 shelf)。
struct BuiltInThemeFamily: Equatable {
    let title: String
    let themes: [BuiltInTheme]
}

/// The app-bundled, read-only theme catalog, grouped by family. Scaffold stage:
/// themes carry a `previewImageName` (card screenshot slot) but no palette yet.
// 中文: 內建主題目錄,依 family 分組。scaffold 階段:有截圖槽,無配色。
enum BuiltInThemes {
    /// Shelf order shown to the user (after `Default`, before user themes).
    // 中文: 顯示順序(在 Default 之後、使用者自訂主題之前)。
    static let families: [BuiltInThemeFamily] = [
        BuiltInThemeFamily(title: "經典", themes: [
            // The Standard head IS the app default (adaptive). Its id is the
            // `ThemeId.default` sentinel so selecting it = reset-to-default, and
            // it shows selected whenever no other theme is chosen. Resolver /
            // SharedSettings early-return on this id before any catalog lookup.
            // 中文: Standard 第一張 = 預設(adaptive)。id 用 ThemeId.default sentinel,選它=回預設、無選取時它就是亮的。
            BuiltInTheme(
                id: ThemeId.default,
                displayName: "預設",
                light: nil,
                dark: nil,
                previewImageName: "theme_standard_preview",
            ),
            // Soft single-hue gradient themes matching the KeyboardKit standard
            // theme look (examined the reference shots): a gentle tint at the TOP
            // (candidate bar) fading DOWN to a very pale version of the SAME hue —
            // it stays tinted to the bottom (no neutral gray). White keys ride on
            // top. Values are visual estimates; fine-tune on device. Shelf order:
            // 櫻花 → 稻穗 → 海風 → 翠青 → 藤紫 (after the adaptive 預設 head).
            gradientTheme(
                "standardPink", "櫻花",
                lightTop: 0xE6C2D0, lightBottom: 0xEADCE2,
                darkTop: 0x4A303C, darkBottom: 0x36242E,
            ),
            gradientTheme(
                "standardGold", "稻穗",
                lightTop: 0xEAD9A6, lightBottom: 0xECE4D2,
                darkTop: 0x423B22, darkBottom: 0x322C1A,
            ),
            gradientTheme(
                "standardBlue", "海風",
                lightTop: 0xBFD2EA, lightBottom: 0xDCE2EC,
                darkTop: 0x323E58, darkBottom: 0x262E40,
            ),
            gradientTheme(
                "standardGreen", "翠青",
                lightTop: 0xC3D8C8, lightBottom: 0xDCE5DD,
                darkTop: 0x324235, darkBottom: 0x28342A,
            ),
            gradientTheme(
                "standardPurple", "藤紫",
                lightTop: 0xCDC4E4, lightBottom: 0xDEDAEA,
                darkTop: 0x3A3252, darkBottom: 0x2C2640,
            ),
        ]),
        BuiltInThemeFamily(title: "Swifty", themes: [
            scaffold("swifty", "Swifty"),
            scaffold("swiftyBlue", "Swifty Blue"),
            scaffold("swiftyGreen", "Swifty Green"),
            scaffold("swiftyPurple", "Swifty Purple"),
        ]),
        BuiltInThemeFamily(title: "Minimal", themes: [
            scaffold("minimal", "Minimal"),
            scaffold("minimalBlue", "Blue"),
            scaffold("minimalGreen", "Green"),
            scaffold("minimalSunset", "Sunset"),
        ]),
    ]

    /// Flattened lookup roster — used by `ThemeResolver` / `SharedSettings` to
    /// resolve a selected id. A scaffold id resolves to `.default` colors until
    /// its palette is authored.
    // 中文: 攤平後的查表清單(供 resolver/SharedSettings 以 id 解析);scaffold id 暫解析為 .default 配色。
    static let all: [BuiltInTheme] = families.flatMap(\.themes)

    /// Looks up a built-in by id; `nil` when the id is not a built-in.
    static func theme(id: String) -> BuiltInTheme? {
        all.first { $0.id == id }
    }

    // MARK: - Helper

    /// Builds a colorless scaffold theme: a name + a card screenshot slot
    /// (`theme_<id>_preview`), no palette. Palettes are authored later, per theme.
    // 中文: 組一個無配色的 scaffold 主題 — 名稱 + 截圖槽(theme_<id>_preview),配色之後再補。
    private static func scaffold(_ id: String, _ displayName: String) -> BuiltInTheme {
        BuiltInTheme(
            id: id,
            displayName: displayName,
            light: nil,
            dark: nil,
            previewImageName: "theme_\(id)_preview",
        )
    }

    // 中文: 漸層主題的中性鍵色。功能鍵與字母鍵同色(光面白 / 暗面 soft dark)— 對齊 iOS 26 Liquid Glass
    // 中文: 預設主題的白功能鍵觀感(非經典灰),漸層是唯一色相,鍵保持中性 → 柔和。
    private static let lightKeyFill: UInt32 = 0xFFFFFF
    private static let lightKeyText: UInt32 = 0x1C1C1E
    private static let darkKeyFill: UInt32 = 0x3A3A3C
    private static let darkKeyText: UInt32 = 0xFFFFFF

    /// Builds a soft single-hue gradient theme — a top→bottom background gradient
    /// over neutral keys (white in light, soft dark in dark). Letter and function
    /// keys share one fill so function keys read white, matching the iOS 26 Liquid
    /// Glass default (not the classic gray). The gradient is the only hue.
    // 中文: 組柔和單色相漸層主題 — top→bottom 漸層 + 中性鍵;功能鍵=字母鍵同色(白),對齊 Liquid Glass 預設。
    private static func gradientTheme(
        _ id: String,
        _ displayName: String,
        lightTop: UInt32, lightBottom: UInt32,
        darkTop: UInt32, darkBottom: UInt32,
    ) -> BuiltInTheme {
        BuiltInTheme(
            id: id,
            displayName: displayName,
            light: softGradientColors(top: lightTop, bottom: lightBottom, keyFill: lightKeyFill, keyText: lightKeyText),
            dark: softGradientColors(top: darkTop, bottom: darkBottom, keyFill: darkKeyFill, keyText: darkKeyText),
            previewImageName: "theme_\(id)_preview",
        )
    }

    /// One scheme variant for a gradient theme: the 2-stop background gradient + a
    /// single neutral key fill (used for BOTH normal and special keys, so function
    /// keys are white like the Liquid Glass default) + key/candidate text color.
    /// `backgroundColor` and `candidateBackgroundColor` stay nil — the gradient owns
    /// the background and the candidate bar is made transparent in
    /// `TaigiKeyboardView.candidateStyle` so the gradient is continuous candidate→bottom.
    // 中文: 漸層主題單一 scheme variant — 2-stop 漸層 + 單一中性鍵色(normal+special 共用 → 功能鍵白)+ 字色;bg/候選背景留 nil。
    private static func softGradientColors(
        top: UInt32, bottom: UInt32,
        keyFill: UInt32, keyText: UInt32,
    ) -> KeyboardColorSettings {
        var colors = KeyboardColorSettings()
        colors.backgroundGradient = ThemeGradient(stops: [CodableColor(hex: top), CodableColor(hex: bottom)])
        let fill = CodableColor(hex: keyFill)
        colors.normalKeyFillColor = fill
        colors.specialKeyFillColor = fill
        colors.keyTextColor = CodableColor(hex: keyText)
        colors.candidateTextColor = CodableColor(hex: keyText)
        return colors
    }
}
