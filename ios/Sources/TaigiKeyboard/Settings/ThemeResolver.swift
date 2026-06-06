// 中文: 主題解析器 — 把選定主題 id 解析成渲染端實際要用的顏色 + 陰影。
// 中文: PR-2a 處理 "default"(回既有自由配色 buffer)與 user-theme id;built-in id 於 PR-2b 加入。

import Foundation

/// Resolves the active theme id into a `ResolvedKeyboardTheme` for rendering.
///
/// Pure + Foundation-only so it is unit-testable without a keyboard runtime.
/// PR-2a handles two cases:
/// - `ThemeId.default` → the legacy free-pick buffer (`colorSettings`), so
///   uncustomized users stay all-nil (Liquid Glass) and customized users keep
///   their look with no migration.
/// - a known `UserTheme` id → that theme's colors + shadow.
///
/// Unknown ids — a built-in id before PR-2b lands, or a `UserTheme` that was
/// deleted while still selected — fall back to the legacy buffer so the
/// keyboard never renders an empty/broken theme.
// 中文: 純函式主題解析。未知 id(尚未支援的 built-in、或已刪除的自訂主題)fallback 回 legacy buffer,確保鍵盤不會渲染空主題。
enum ThemeResolver {
    static func resolved(
        themeId: String,
        legacyColorSettings: KeyboardColorSettings,
        userThemes: [UserTheme],
    ) -> ResolvedKeyboardTheme {
        if themeId == ThemeId.default {
            return ResolvedKeyboardTheme(colors: legacyColorSettings, keyShadowIntensity: 0)
        }
        if let theme = userThemes.first(where: { $0.id.uuidString == themeId }) {
            return ResolvedKeyboardTheme(
                colors: theme.colors,
                keyShadowIntensity: theme.keyShadowIntensity,
            )
        }
        // Unknown id → fall back to the legacy buffer (Liquid Glass preserved
        // for default users; customized users keep their colors).
        return ResolvedKeyboardTheme(colors: legacyColorSettings, keyShadowIntensity: 0)
    }
}
