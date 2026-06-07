// 中文: 主題解析器 — 把選定主題 id + colorScheme 解析成渲染端要用的顏色 + 陰影。
// 中文: 處理 "default"(回既有自由配色 buffer)、user-theme id、built-in id(依 colorScheme 取 light/dark)。

import Foundation
import SwiftUI

/// Resolves the active theme id into a `ResolvedKeyboardTheme` for rendering.
///
/// Pure + deterministic (no keyboard runtime needed) so it is directly
/// unit-testable. Handles three cases:
/// - `ThemeId.default` → the legacy free-pick buffer (`colorSettings`), so
///   uncustomized users stay all-nil (Liquid Glass) and customized users keep
///   their look with no migration.
/// - a known `UserTheme` id → that theme's colors + shadow.
/// - a known built-in id → that theme's `colorScheme`-appropriate variant.
///
/// Unknown ids — a deleted `UserTheme` still selected, or a stale/unknown
/// built-in id — fall back to the legacy buffer so the keyboard never renders
/// an empty/broken theme.
// 中文: 純函式主題解析。未知 id(已刪除的自訂主題、或未知 built-in)fallback 回 legacy buffer,確保不渲染空主題。
enum ThemeResolver {
    static func resolved(
        themeId: String,
        colorScheme: ColorScheme,
        legacyColorSettings: KeyboardColorSettings,
        userThemes: [UserTheme],
        builtInThemes: [BuiltInTheme] = BuiltInThemes.all,
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
        if let builtIn = builtInThemes.first(where: { $0.id == themeId }) {
            return ResolvedKeyboardTheme(
                colors: builtIn.colors(for: colorScheme),
                keyShadowIntensity: 0,
            )
        }
        // Unknown id → fall back to the legacy buffer (Liquid Glass preserved
        // for default users; customized users keep their colors).
        return ResolvedKeyboardTheme(colors: legacyColorSettings, keyShadowIntensity: 0)
    }
}
