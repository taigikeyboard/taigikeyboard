// Resolves the theme appearance keyboard overlays paint with: gradient backdrop + role-first
// foreground + chrome accent. Extends the candidate overlay's gradient-continuity + PR #425
// role-first foreground to the symbol / layout / settings panels.

package com.siansiansu.taigikeyboard.ime.text.smartbar

import androidx.compose.foundation.background
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.drawWithCache
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.res.dimensionResource
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.ThemeAppearanceCache
import com.siansiansu.taigikeyboard.ime.core.isKeyboardNightMode

/**
 * The theme colors a keyboard overlay panel paints with.
 *
 * Single seam for the symbol / layout / settings overlays so they match the candidate overlay:
 * a gradient theme repaints the gradient backdrop ([gradientStops]); a flat theme falls back to
 * [solidBackground] (`?keyboard_bgColor`). [foreground] is role-first ([candidateTextColor] over
 * the `?smartbar_fgColor` attr) so a light-only gradient theme stays readable in system dark mode
 * (the PR #425 invariant, here extended to these panels). [accent] keeps the chrome accent.
 */
data class KeyboardOverlayAppearance(
    val solidBackground: Color,
    val gradientStops: List<Int>?,
    val foreground: Color,
    val accent: Color,
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
        KeyboardOverlayAppearance(
            solidBackground = chrome.background,
            gradientStops = colors.gradientStops()?.toList(),
            foreground = colors.candidateTextColor?.let { Color(it) } ?: chrome.foreground,
            accent = chrome.accent,
        )
    }
}

/**
 * Paints a keyboard overlay panel's theme backdrop: a vertical gradient when [gradientStops] has
 * >=2 stops, else the flat [solid] color. Shared by the candidate / symbol / layout / settings
 * overlays so the gradient-vs-flat branch lives in one place.
 *
 * [topInsetPx] is the height of any chrome ABOVE the panel (the smartbar, for the symbol / layout /
 * settings panels which are offset below it). The gradient is sliced to span the FULL keyboard
 * — `startY = -topInsetPx` puts the gradient top at the keyboard top, not the panel top — so the
 * panel shows its true `[topInset, keyboardBottom]` slice and stays continuous with the gradient
 * keyboard. The candidate overlay spans the full keyboard, so it passes the default 0.
 */
fun Modifier.keyboardOverlayBackdrop(
    gradientStops: List<Int>?,
    solid: Color,
    topInsetPx: Float = 0f,
): Modifier =
    if (gradientStops != null && gradientStops.size >= 2) {
        val colors = gradientStops.map { Color(it) }
        // drawWithCache rebuilds the brush only when the draw size changes, not every frame (the
        // candidate overlay scrolls). startY = -topInsetPx slices the gradient to span the FULL
        // keyboard so the panel's slice stays continuous with the gradient above it.
        drawWithCache {
            val brush = Brush.verticalGradient(colors, startY = -topInsetPx, endY = size.height)
            onDrawBehind { drawRect(brush) }
        }
    } else {
        background(solid)
    }

/**
 * Smartbar height in px — the [keyboardOverlayBackdrop] top inset for the offset overlay panels
 * (symbol / layout / settings), which are mounted below the smartbar. The candidate overlay spans
 * the full keyboard and passes 0 instead.
 */
@Composable
fun rememberSmartbarInsetPx(): Float {
    val density = LocalDensity.current
    val smartbarHeight = dimensionResource(R.dimen.smartbar_height)
    return remember(density, smartbarHeight) { with(density) { smartbarHeight.toPx() } }
}
