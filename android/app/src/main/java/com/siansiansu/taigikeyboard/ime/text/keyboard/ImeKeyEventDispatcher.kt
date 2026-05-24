// Adapter the keyboard-body Composable uses to call back into the IME service.
// Lives next to the KeyEventDispatcher interface it satisfies.
// Constructor providers preserve the legacy null-tolerant behaviour for the
// host / compose-host views (popup anchor resolution stays a no-op until the
// keyboard view is mounted).

package com.siansiansu.taigikeyboard.ime.text.keyboard

import android.content.Context
import android.view.HapticFeedbackConstants
import android.view.View
import android.view.inputmethod.InputMethodManager
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.popup.KeyAnchor
import com.siansiansu.taigikeyboard.ime.popup.buildPopupCells
import com.siansiansu.taigikeyboard.ime.text.CapsStateManager
import com.siansiansu.taigikeyboard.ime.text.key.KeyData

internal class ImeKeyEventDispatcher(
    private val taigikeyboard: TaigiKeyboard,
    private val prefs: PrefHelper,
    private val capsStateManager: CapsStateManager,
    private val hostViewProvider: () -> View?,
    private val composeHostProvider: () -> View?,
    private val onDispatchKeyPress: (KeyData) -> Unit,
) : KeyEventDispatcher {
    private val locationScratch = IntArray(2)

    override fun dispatchKeyPress(data: KeyData) = onDispatchKeyPress(data)

    override fun showInputMethodPicker() {
        val im = taigikeyboard.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        im.showInputMethodPicker()
    }

    override fun keyPressVibrate() {
        if (!prefs.isVibrationFeedbackEnabled) return
        hostViewProvider()?.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
    }

    override fun keyPressSound(data: KeyData) = taigikeyboard.keyPressSound(data)

    override val longPressDelayMs: Long
        get() = prefs.longPressDelay.toLong()

    override fun resolveAnchor(
        bounds: KeyBounds,
        keyboardWidth: Int,
        desiredKeyWidth: Int,
        desiredKeyHeight: Int,
    ): KeyAnchor {
        // composeHost == keyboard ComposeView; window coords + key-relative
        // offset give the absolute window position PopupWindow.showAtLocation
        // needs. Null composeHost leaves the scratch untouched; the field
        // starts at [0, 0] so the pre-mount path resolves to window origin
        // (legacy behaviour).
        composeHostProvider()?.getLocationInWindow(locationScratch)
        val anchorTopXInWindow = locationScratch[0] + bounds.visible.left
        val anchorTopYInWindow = locationScratch[1] + bounds.visible.top

        val isLandscape = taigikeyboard.resources.configuration.orientation ==
            android.content.res.Configuration.ORIENTATION_LANDSCAPE
        val computedLabel = computeKeyLetter(
            bounds.data,
            prefs.inputMode,
            capsStateManager.caps,
            capsStateManager.capsLock,
        )
        // Skip popup-cell resolution when the key has no popup variants — most
        // presses never trigger long-press extend. KeyTouchCoordinator already
        // gates the long-press path on data.popup.isNotEmpty().
        val popupCells = if (bounds.data.popup.isEmpty()) {
            emptyList()
        } else {
            buildPopupCells(
                bounds.data,
                prefs.inputMode,
                capsStateManager.caps,
                capsStateManager.capsLock,
                taigikeyboard.resources,
            )
        }

        return KeyAnchor(
            data = bounds.data,
            measuredWidth = bounds.visible.width,
            measuredHeight = bounds.visible.height,
            xInKeyboard = bounds.visible.left,
            keyboardWidth = keyboardWidth,
            xInWindow = anchorTopXInWindow,
            yInWindow = anchorTopYInWindow,
            computedLabel = computedLabel,
            popupCells = popupCells,
            isLandscape = isLandscape,
            desiredKeyWidth = desiredKeyWidth,
            desiredKeyHeight = desiredKeyHeight,
        )
    }
}
