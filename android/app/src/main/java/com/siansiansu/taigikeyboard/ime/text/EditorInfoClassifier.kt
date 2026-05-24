// Pure classification of a non-null EditorInfo into (KeyboardMode, KeyVariation,
// isComposingEnabled). Extracted from TextInputManager.onStartInputView so the
// mapping is unit-testable.
//
// Null EditorInfo is handled by the caller: the legacy `onStartInputView` null
// branch returned `CHARACTERS` WITHOUT touching `keyVariation`, so a prior
// password session stayed PASSWORD/non-composing. Moving the null branch into
// this object would erase that state, so callers must keep the null path inline.

package com.siansiansu.taigikeyboard.ime.text

import android.text.InputType
import android.view.inputmethod.EditorInfo
import com.siansiansu.taigikeyboard.ime.text.key.KeyVariation
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode

internal object EditorInfoClassifier {
    data class Classification(
        val mode: KeyboardMode,
        val keyVariation: KeyVariation,
        val isComposingEnabled: Boolean,
    )

    fun classify(info: EditorInfo): Classification {
        val (mode, keyVariation) = resolveModeAndVariation(info)
        val isComposingEnabled =
            when (mode) {
                KeyboardMode.NUMERIC,
                KeyboardMode.PHONE,
                KeyboardMode.PHONE2,
                -> false

                else -> keyVariation != KeyVariation.PASSWORD
            }
        return Classification(mode, keyVariation, isComposingEnabled)
    }

    private fun resolveModeAndVariation(info: EditorInfo): Pair<KeyboardMode, KeyVariation> =
        when (info.inputType and InputType.TYPE_MASK_CLASS) {
            InputType.TYPE_CLASS_NUMBER -> KeyboardMode.NUMERIC to KeyVariation.NORMAL
            InputType.TYPE_CLASS_PHONE -> KeyboardMode.PHONE to KeyVariation.NORMAL
            InputType.TYPE_CLASS_TEXT -> KeyboardMode.CHARACTERS to resolveTextVariation(info.inputType)
            else -> KeyboardMode.CHARACTERS to KeyVariation.NORMAL
        }

    private fun resolveTextVariation(inputType: Int): KeyVariation =
        when (inputType and InputType.TYPE_MASK_VARIATION) {
            InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS,
            InputType.TYPE_TEXT_VARIATION_WEB_EMAIL_ADDRESS,
            -> KeyVariation.EMAIL_ADDRESS

            InputType.TYPE_TEXT_VARIATION_PASSWORD,
            InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD,
            InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD,
            -> KeyVariation.PASSWORD

            InputType.TYPE_TEXT_VARIATION_URI -> KeyVariation.URI

            else -> KeyVariation.NORMAL
        }
}
