// 中文: 內建主題靜態表 — KeyboardKit 風格目錄,依 family(Standard / Swifty / Minimal)分區。
// 中文: Standard family 的漸層主題只帶 light 配色(dark = nil)→ 深色模式刻意維持 light 觀感(USER:這些主題色不隨深色變);Swifty/Minimal 仍 scaffold(無配色,light/dark = nil)。
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
            // 櫻花 → 金煌 → 海風 → 翠青 → 藤紫 (after the adaptive 預設 head).
            // Light-only by design (dark == nil): these themes keep their light
            // palette in dark mode, so colors(for: .dark) falls back to light.
            gradientTheme(
                "standardPink", "櫻花",
                top: 0xE6C2D0, bottom: 0xEADCE2,
            ),
            gradientTheme(
                "standardGold", "金煌",
                top: 0xEAD9A6, bottom: 0xECE4D2,
            ),
            gradientTheme(
                "standardBlue", "海風",
                top: 0xBFD2EA, bottom: 0xDCE2EC,
            ),
            gradientTheme(
                "standardGreen", "翠青",
                top: 0xC3D8C8, bottom: 0xDCE5DD,
            ),
            gradientTheme(
                "standardPurple", "藤紫",
                top: 0xCDC4E4, bottom: 0xDEDAEA,
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

    // 中文: 漸層主題的中性鍵色。功能鍵與字母鍵同色(白)— 對齊 iOS 26 Liquid Glass 預設白功能鍵(非經典灰)。
    // 中文: 漸層主題 light-only,深色模式維持 light 觀感,故只需 light 鍵色。
    private static let lightKeyFill: UInt32 = 0xFFFFFF
    private static let lightKeyText: UInt32 = 0x1C1C1E

    /// Builds a soft single-hue gradient theme — a top→bottom background gradient
    /// over neutral white keys. Letter and function keys share one fill so function
    /// keys read white, matching the iOS 26 Liquid Glass default (not the classic
    /// gray). The gradient is the only hue. Light-only (`dark` is nil): in dark mode
    /// `colors(for: .dark)` falls back to this light palette, so the theme keeps its
    /// light look (USER request — these themes don't darken with the system).
    // 中文: 組柔和單色相漸層主題 — top→bottom 漸層 + 白色中性鍵;功能鍵=字母鍵同色(白)。
    // 中文: light-only(dark = nil),深色模式由 colors(for:) fallback 回 light → 主題不隨系統變深。
    private static func gradientTheme(
        _ id: String,
        _ displayName: String,
        top: UInt32, bottom: UInt32,
    ) -> BuiltInTheme {
        BuiltInTheme(
            id: id,
            displayName: displayName,
            light: softGradientColors(top: top, bottom: bottom, keyFill: lightKeyFill, keyText: lightKeyText),
            dark: nil,
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
