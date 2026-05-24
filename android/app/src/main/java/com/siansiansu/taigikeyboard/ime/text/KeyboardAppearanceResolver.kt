// Resolves a KeyboardAppearance snapshot from live prefs + IME state. Caches
// the colour-settings JSON parse and typeface lookup; rebuilt only when the
// underlying string flips.

package com.siansiansu.taigikeyboard.ime.text

import com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardAppearance
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardHeightFactor
import com.siansiansu.taigikeyboard.localization.SettingsTexts
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
    private var cachedColorSettingsJson: String = ""
    private var cachedColorSettings: KeyboardColorSettings = KeyboardColorSettings()
    private var cachedFontType: String = ""
    private var cachedTypeface: android.graphics.Typeface = android.graphics.Typeface.DEFAULT

    fun snapshot(): KeyboardAppearance =
        KeyboardAppearance(
            keyboardLayoutType = prefs.keyboardLayoutType,
            inputMode = prefs.inputMode,
            caps = capsStateManager.caps,
            capsLock = capsStateManager.capsLock,
            isComposing = isComposingProvider(),
            isTranslateSwapped = translateSwappedProvider(),
            imeOptions = taigikeyboard.currentInputEditorInfo?.imeOptions ?: 0,
            confirmKeyLabel = SettingsTexts.confirmKeyLabel(prefs.inputMode, prefs.isTranslateSwapped),
            colorSettings = resolveColorSettings(),
            typeface = resolveTypeface(),
            keyFontSizeScale = prefs.keyFontSizeScale,
            keyCornerRadius = prefs.keyCornerRadius,
            keyBorderWidth = prefs.keyBorderWidth,
            heightFactor = KeyboardHeightFactor.fromPreferenceString(prefs.heightFactor),
            keyHeightScale = prefs.keyHeightScale,
        )

    private fun resolveColorSettings(): KeyboardColorSettings {
        val json = prefs.colorSettings
        if (json != cachedColorSettingsJson) {
            cachedColorSettingsJson = json
            cachedColorSettings = KeyboardColorSettings.fromJson(json)
        }
        return cachedColorSettings
    }

    private fun resolveTypeface(): android.graphics.Typeface {
        val fontType = prefs.fontType
        if (fontType != cachedFontType) {
            cachedFontType = fontType
            cachedTypeface = TypefaceLoader.getTypefaceByType(fontType, taigikeyboard)
        }
        return cachedTypeface
    }
}
