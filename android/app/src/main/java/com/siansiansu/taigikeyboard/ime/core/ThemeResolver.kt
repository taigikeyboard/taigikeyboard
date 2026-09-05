package com.siansiansu.taigikeyboard.ime.core

/**
 * Resolves the active theme id into a [ThemeAppearance] for rendering. Pure +
 * deterministic (no keyboard runtime needed) so it is directly unit-testable.
 * Mirrors iOS ThemeResolver. Handles:
 *
 * - [ThemeId.DEFAULT] -> the legacy free-pick appearance (uncustomized users
 *   stay adaptive, customized users keep their look, no migration).
 * - a known user-theme id -> that theme's full appearance.
 * - a known built-in id -> factory sizes + the built-in's night-mode color variant.
 * - an unknown id (deleted user theme / stale built-in) -> the legacy appearance,
 *   so the keyboard never renders an empty/broken theme.
 */
object ThemeResolver {
    fun resolved(
        themeId: String,
        isDark: Boolean,
        legacyAppearance: ThemeAppearance,
        userThemes: List<UserTheme>,
        builtInThemes: List<BuiltInTheme> = BuiltInThemes.all,
    ): ThemeAppearance {
        if (themeId == ThemeId.DEFAULT) return legacyAppearance
        // Gate the user branch on a UUID-shaped id (mirrors iOS `id.uuidString == themeId`):
        // a built-in id like "standardBlue" can never resolve to a user theme, so a corrupt
        // persisted user theme carrying a built-in id cannot shadow the built-in.
        if (ThemeId.isUserTheme(themeId)) {
            userThemes.firstOrNull { it.id == themeId }?.let { return it.appearance }
        }
        builtInThemes.firstOrNull { it.id == themeId }?.let { builtIn ->
            // Built-in themes are colors-first -> factory sizes + their night-mode
            // color variant, plus an optional per-theme appearance override
            // (keyBorderWidth, used by the 框線 family).
            return ThemeAppearance.DEFAULT.copy(
                colors = builtIn.colors(isDark),
                keyBorderWidth = builtIn.keyBorderWidth ?: ThemeAppearance.DEFAULT.keyBorderWidth,
            )
        }
        return legacyAppearance
    }
}
