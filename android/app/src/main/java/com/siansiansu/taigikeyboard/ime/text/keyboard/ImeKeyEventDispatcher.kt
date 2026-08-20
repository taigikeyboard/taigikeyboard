// Adapter the keyboard-body Composable uses to call back into the IME service.
// Lives next to the KeyEventDispatcher interface it satisfies.
// The compose-host provider preserves the legacy null-tolerant behaviour
// (popup anchor resolution stays a no-op until the keyboard view is mounted).

package com.siansiansu.taigikeyboard.ime.text.keyboard

import android.content.Context
import android.view.View
import android.view.inputmethod.InputMethodManager
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.popup.KeyAnchor
import com.siansiansu.taigikeyboard.ime.popup.buildPopupCells
import com.siansiansu.taigikeyboard.ime.popup.tpsPopupWithBaseGlyph
import com.siansiansu.taigikeyboard.ime.text.CapsStateManager
import com.siansiansu.taigikeyboard.ime.text.key.KeyData

internal class ImeKeyEventDispatcher(
    private val taigikeyboard: TaigiKeyboard,
    private val prefs: PrefHelper,
    private val capsStateManager: CapsStateManager,
    private val composeHostProvider: () -> View?,
    private val onDispatchKeyPress: (KeyData) -> Unit,
) : KeyEventDispatcher {
    private val locationScratch = IntArray(2)

    override fun dispatchKeyPress(data: KeyData) = onDispatchKeyPress(data)

    override fun showInputMethodPicker() {
        val im = taigikeyboard.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        im.showInputMethodPicker()
    }

    override fun keyPressVibrate() = taigikeyboard.keyPressVibrate()

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
        val inputMode = prefs.inputMode
        val computedLabel = computeKeyLetter(
            bounds.data,
            inputMode,
            capsStateManager.caps,
            capsStateManager.capsLock,
        )
        // TPS glyph keys get their own glyph prepended to the long-press popup
        // so the base letter stays selectable (base-first; parity with iOS
        // Callouts.TPSCallouts.calloutChars). Applied at anchor resolution, not
        // in the layout JSON, because the keycap hint draws data.popup directly.
        val anchorData = tpsPopupWithBaseGlyph(bounds.data, prefs.isTpsLayout)

        // Skip popup-cell resolution when the key has no popup variants — most
        // presses never trigger long-press extend. KeyTouchCoordinator already
        // gates the long-press path on data.popup.isNotEmpty().
        val popupCells = if (anchorData.popup.isEmpty()) {
            emptyList()
        } else {
            buildPopupCells(
                anchorData,
                inputMode,
                capsStateManager.caps,
                capsStateManager.capsLock,
                taigikeyboard.resources,
            )
        }

        return KeyAnchor(
            data = anchorData,
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

