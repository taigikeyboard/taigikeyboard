// 解析後主題外觀的字串相等快取 — 只在輸入(themeId / colorSettings / userThemes / isDark / 5 尺寸)變動時重解析。
// 鍵盤渲染端(KeyboardAppearanceResolver)與候選列(SmartbarManager)各持一份,避免每次按鍵重 parse JSON。

package com.siansiansu.taigikeyboard.ime.core

import android.content.Context
import android.content.res.Configuration

/**
 * Caches the resolved [ThemeAppearance] so the per-keystroke render path does not
 * re-parse the colorSettings + userThemes JSON on every call. Re-resolves through
 * [ThemeResolver] ONLY when one of the resolver inputs flips: the selected theme id,
 * the colorSettings JSON, the userThemes JSON, the night-mode flag, or any of the
 * five size scalars (which feed `PrefHelper.legacyAppearance`).
 *
 * One instance per consumer ([KeyboardAppearanceResolver] for keys, [SmartbarManager]
 * for the candidate strip). Resolution is pure + deterministic, so the two caches stay
 * in lockstep without sharing mutable state. `fontType` / `heightFactor` are NOT theme
 * inputs and stay outside this cache.
 */
internal class ThemeAppearanceCache(
    private val prefs: PrefHelper,
) {
    private var cachedKey: Key? = null
    private var cached: ThemeAppearance = ThemeAppearance.DEFAULT

    fun resolve(isDark: Boolean): ThemeAppearance {
        val key = Key(
            selectedThemeId = prefs.selectedThemeId,
            colorSettingsJson = prefs.colorSettings,
            userThemesJson = prefs.userThemes,
            isDark = isDark,
            keyHeightScale = prefs.keyHeightScale,
            keyFontSizeScale = prefs.keyFontSizeScale,
            candidateTextSizeScale = prefs.candidateTextSizeScale,
            keyCornerRadius = prefs.keyCornerRadius,
            keyBorderWidth = prefs.keyBorderWidth,
        )
        if (key != cachedKey) {
            cachedKey = key
            // Resolve through the single PrefHelper entry point so the resolver-input
            // wiring (legacyAppearance + userThemes) lives in ONE place; the cache only
            // adds the re-resolve gate that entry point's doc says it lacks.
            cached = prefs.resolvedAppearance(key.isDark)
        }
        return cached
    }

    /** Snapshot of every [ThemeResolver] input; equality drives the re-resolve gate. */
    private data class Key(
        val selectedThemeId: String,
        val colorSettingsJson: String,
        val userThemesJson: String,
        val isDark: Boolean,
        val keyHeightScale: Float,
        val keyFontSizeScale: Float,
        val candidateTextSizeScale: Float,
        val keyCornerRadius: Float,
        val keyBorderWidth: Float,
    )
}

/**
 * Whether the keyboard should render its dark-mode theme variant. Reads the live
 * configuration so a night-mode flip is picked up on the next appearance push.
 */
internal fun isKeyboardNightMode(context: Context): Boolean =
    (context.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) ==
        Configuration.UI_MODE_NIGHT_YES
