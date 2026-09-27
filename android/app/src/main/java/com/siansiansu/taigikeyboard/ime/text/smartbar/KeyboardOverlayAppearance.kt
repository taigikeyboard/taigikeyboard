// Resolves the theme appearance keyboard overlays paint with: theme background + role-first
// foreground + chrome accent. Extends the candidate overlay's surface-continuity + PR #425
// role-first foreground to the symbol / layout / settings panels.

package com.siansiansu.taigikeyboard.ime.text.smartbar

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.dimensionResource
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.ThemeAppearanceCache
import com.siansiansu.taigikeyboard.ime.core.ThemeSurface
import com.siansiansu.taigikeyboard.ime.core.isKeyboardNightMode

/**
 * The theme colors a keyboard overlay panel paints with.
 *
 * Single seam for the symbol / layout / settings overlays so they match the candidate overlay:
 * a custom theme repaints its [surface] (solid, gradient or photo — the panels are siblings of
 * the painted keyboard parent, not children, so they must paint it themselves); the adaptive
 * default falls back to [solidBackground] (`?keyboard_bgColor`). [foreground] is role-first
 * ([candidateTextColor] over the `?smartbar_fgColor` attr) so a light-only gradient theme stays
 * readable in system dark mode (the PR #425 invariant, here extended to these panels). [accent]
 * keeps the chrome accent. Paint with `Modifier.themeBackground(surface, solidBackground,
 * topInsetPx)` — [rememberSmartbarInsetPx] for the panels mounted below the smartbar, 0 for the
 * candidate overlay that spans the full keyboard.
 *
 * [calloutFill] / [calloutForeground] paint a popup that floats over the keys (the one-handed
 * menu): a fixed key palette uses its key fill + key text, like the key long-press popup;
 * otherwise the panel [solidBackground] / [foreground].
 */
data class KeyboardOverlayAppearance(
    val solidBackground: Color,
    val surface: ThemeSurface?,
    val foreground: Color,
    val accent: Color,
    val calloutFill: Color,
    val calloutForeground: Color,
)

/**
 * Resolves [KeyboardOverlayAppearance] from the live keyboard theme.
 *
 * @param refreshKey bump on overlay show() so a live keyboard-color change is picked up without
 *   recreating the composition (same cadence as [rememberKeyboardChromeColors]).
 *
 * The [ThemeAppearanceCache] is remembered on [prefs] (one instance per consumer, matching its
 * design) — NOT on refreshKey, so the re-resolve gate inside the cache keeps working.
 */
@Composable
fun rememberKeyboardOverlayAppearance(
    prefs: PrefHelper,
    refreshKey: Int = 0,
): KeyboardOverlayAppearance {
    val context = LocalContext.current
    val chrome = rememberKeyboardChromeColors(refreshKey)
    val cache = remember(prefs) { ThemeAppearanceCache(prefs) }
    return remember(refreshKey, context, chrome, cache) {
        val colors = cache.resolve(isKeyboardNightMode(context)).colors
        val foreground = colors.candidateTextColor?.let { Color(it) } ?: chrome.foreground
        // CROSS-PLATFORM INVARIANT — mirrors iOS Callouts/KeyboardCalloutStyle+Taigi.swift themed(by:):
        // key fill + key text together, or neither.
        val keyFill = colors.fixedKeyFill
        val keyText = colors.keyTextColor
        val keyPalette = if (keyFill != null && keyText != null) Color(keyFill) to Color(keyText) else null
        KeyboardOverlayAppearance(
            solidBackground = chrome.background,
            surface = colors.surface,
            foreground = foreground,
            accent = chrome.accent,
            calloutFill = keyPalette?.first ?: chrome.background,
            calloutForeground = keyPalette?.second ?: foreground,
        )
    }
}

/**
 * Smartbar height in px — the `themeBackground` top inset for the offset overlay panels
 * (symbol / layout / settings), which are mounted below the smartbar. The candidate overlay spans
 * the full keyboard and passes 0 instead.
 */
@Composable
fun rememberSmartbarInsetPx(): Float {
    val density = LocalDensity.current
    val smartbarHeight = dimensionResource(R.dimen.smartbar_height)
    return remember(density, smartbarHeight) { with(density) { smartbarHeight.toPx() } }
}
