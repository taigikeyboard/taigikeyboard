// Resolves a KeyboardAppearance snapshot from live prefs + IME state. Caches
// the colour-settings JSON parse and typeface lookup; rebuilt only when the
// underlying string flips.

package com.siansiansu.taigikeyboard.ime.text

import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.ThemeAppearanceCache
import com.siansiansu.taigikeyboard.ime.core.isKeyboardNightMode
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardAppearance
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardHeightFactor
import com.siansiansu.taigikeyboard.typeface.TypefaceLoader

/**
 * Builds [KeyboardAppearance] snapshots for [TextInputManager.pushAppearance].
 * Owns the colour-settings + typeface parse caches that the legacy
 * `KeyboardView.getColorSettings` cache mirrored.
 *
 * Caches re-parse only when the underlying preference string changes; the
 * snapshot itself is recreated every call (cheap data-class composition) so
 * the equality check inside `_keyboardUi.update` is the single
 * recomposition-gating point.
 */
internal class KeyboardAppearanceResolver(
    private val prefs: PrefHelper,
    private val taigikeyboard: TaigiKeyboard,
    private val capsStateManager: CapsStateManager,
    private val isComposingProvider: () -> Boolean,
    private val translateSwappedProvider: () -> Boolean,
) {
    private val themeCache = ThemeAppearanceCache(prefs)
    private var cachedFontType: String = ""
    private var cachedTypeface: android.graphics.Typeface = android.graphics.Typeface.DEFAULT

    fun snapshot(): KeyboardAppearance {
        // Resolve the active theme (default theme -> legacy colorSettings + scalars +
        // flat shadow, so the default path stays byte-identical). The cache only
        // re-parses JSON when a theme input flips, keeping the per-keystroke path cheap.
        val theme = themeCache.resolve(isKeyboardNightMode(taigikeyboard))
        return KeyboardAppearance(
            keyboardLayoutType = prefs.keyboardLayoutType,
            inputMode = prefs.inputMode,
            caps = capsStateManager.caps,
            capsLock = capsStateManager.capsLock,
            isComposing = isComposingProvider(),
            isTranslateSwapped = translateSwappedProvider(),
            imeOptions = taigikeyboard.currentInputEditorInfo?.imeOptions ?: 0,
            confirmKeyLabel = KeyboardAppearance.confirmKeyLabel(prefs.inputMode, prefs.isTranslateSwapped),
            colorSettings = theme.colors,
            typeface = resolveTypeface(),
            keyFontSizeScale = theme.keyFontSizeScale,
            keyCornerRadius = theme.keyCornerRadius,
            keyBorderWidth = theme.keyBorderWidth,
            keyShadowIntensity = theme.keyShadowIntensity,
            heightFactor = KeyboardHeightFactor.fromPreferenceString(prefs.heightFactor),
            keyHeightScale = theme.keyHeightScale,
        )
    }

    /** The resolved theme colors only — for the View-layer gradient/transparency
     *  apply (KeyboardThemeSurfaceController), which does not need the full
     *  KeyboardAppearance. Shares the same cache as [snapshot]. */
    fun resolvedColors(): com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings =
        themeCache.resolve(isKeyboardNightMode(taigikeyboard)).colors

    private fun resolveTypeface(): android.graphics.Typeface {
        val fontType = prefs.fontType
        if (fontType != cachedFontType) {
            cachedFontType = fontType
            cachedTypeface = TypefaceLoader.getTypefaceByType(fontType, taigikeyboard)
        }
        return cachedTypeface
    }
}
