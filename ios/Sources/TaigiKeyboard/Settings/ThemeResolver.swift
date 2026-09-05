// 主題解析器 — 把選定主題 id + colorScheme 解析成渲染端要用的完整外觀(ThemeAppearance)。
// 處理 "default"(回既有全域外觀)、user-theme id、built-in id(依 colorScheme 取 light/dark)。

import Foundation
import SwiftUI

/// Resolves the active theme id into a `ThemeAppearance` for rendering.
///
/// Pure + deterministic (no keyboard runtime needed) so it is directly
/// unit-testable. Handles three cases:
/// - `ThemeId.default` → the legacy free-pick appearance (`legacyAppearance`),
///   so uncustomized users stay all-nil (Liquid Glass) and customized users
///   keep their look with no migration.
/// - a known `UserTheme` id → that theme's full appearance.
/// - a known built-in id → factory sizes with the built-in's
///   `colorScheme`-appropriate color variant.
///
/// Unknown ids — a deleted `UserTheme` still selected, or a stale/unknown
/// built-in id — fall back to the full legacy appearance so the keyboard never
/// renders an empty/broken theme.
// 純函式主題解析。未知 id(已刪除的自訂主題、或未知 built-in)fallback 回完整 legacy 外觀,確保不渲染空主題。
enum ThemeResolver {
    static func resolved(
        themeId: String,
        colorScheme: ColorScheme,
        legacyAppearance: ThemeAppearance,
        userThemes: [UserTheme],
        builtInThemes: [BuiltInTheme] = BuiltInThemes.all,
    ) -> ThemeAppearance {
        if themeId == ThemeId.default {
            return legacyAppearance
        }
        if let theme = userThemes.first(where: { $0.id.uuidString == themeId }) {
            return theme.appearance
        }
        if let builtIn = builtInThemes.first(where: { $0.id == themeId }) {
            // Built-in themes are colors-first → factory sizes + their
            // colorScheme-appropriate color variant, plus an optional per-theme
            // appearance override (`keyBorderWidth`, used by the 框線 family).
            var appearance = ThemeAppearance.default
            appearance.colors = builtIn.colors(for: colorScheme)
            appearance.keyBorderWidth = builtIn.keyBorderWidth ?? ThemeAppearance.default.keyBorderWidth
            return appearance
        }
        // Unknown id → fall back to the full legacy appearance (Liquid Glass
        // preserved for default users; customized users keep their look).
        return legacyAppearance
    }
}
