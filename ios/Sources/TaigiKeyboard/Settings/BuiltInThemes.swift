// 中文: 內建主題靜態表 — KeyboardKit 風格目錄,依 family(Standard / Swifty / Minimal)分區。
// 中文: 排版 scaffold 階段:每個 theme 只有名稱 + 截圖 asset 槽,尚無顏色(light/dark = nil)。
// 中文: USER 流程 — 先把主題頁排版固定,之後再逐一補各 theme 的實際配色。
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
                displayName: "經典",
                light: nil,
                dark: nil,
                previewImageName: "theme_standard_preview",
            ),
            scaffold("standardBlue", "Blue"),
            scaffold("standardGreen", "Green"),
            scaffold("standardPurple", "Purple"),
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
}
