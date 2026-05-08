package com.siansiansu.taigikeyboard.ime.popup

import android.view.MotionEvent
import com.siansiansu.taigikeyboard.ime.text.key.KeyData

/**
 * Window-relative anchor descriptor used to drive popup show / extend / drag /
 * commit lookups without coupling the popup layer to any specific [android.view.View]
 * subclass.
 *
 * `xInKeyboard` is the keyboard-root-relative X used by
 * [com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardLayoutSolver.solveExtendedPopupGeometry]
 * for screen-edge clamp side selection. `xInWindow` / `yInWindow` are the
 * window-absolute (top-left) coords used by `PopupWindow.showAtLocation`.
 *
 * `computedLabel` is pre-resolved by the caller so the popup layer never
 * reaches back into the keyboard layer for label formatting (input-mode
 * + caps/capsLock + KeyLabelCaseCache lookups stay on the caller side).
 *
 * `popupCells` is also pre-resolved — Phase C's `buildPopupCell` logic moves
 * to the caller side so the popup layer stays a pure renderer of state.
 */
data class KeyAnchor(
    val data: KeyData,
    val measuredWidth: Int,
    val measuredHeight: Int,
    val xInKeyboard: Int,
    val keyboardWidth: Int,
    val xInWindow: Int,
    val yInWindow: Int,
    val computedLabel: String,
    val popupCells: List<PopupCell>,
    val isLandscape: Boolean,
    /** Solver-derived per-key cell dims (legacy
     *  `KeyboardView.desiredKeyWidth/desiredKeyHeight`). Drives popup
     *  sizing in [com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardLayoutSolver.solvePopupDimensions]
     *  so width/height-scale prefs and landscape orientation flow through
     *  to the preview popup rather than being baked from the static
     *  `R.dimen.key_width / key_height` resource values. */
    val desiredKeyWidth: Int,
    val desiredKeyHeight: Int,
)

/**
 * Surface the keyboard body uses to drive popups. Backed by [KeyPopupManager]
 * in production; supply a no-op implementation in preview / test paths.
 *
 * Manager tracks the most-recently-anchored [KeyAnchor] internally so the body
 * can call [propagateMotionEvent] / [getActiveKeyData] without re-passing the
 * anchor — matches the legacy `KeyPopupManager` ergonomics.
 */
interface PopupHost {
    val isShowingPopup: Boolean
    val isShowingExtendedPopup: Boolean

    fun show(anchor: KeyAnchor)
    fun extend(anchor: KeyAnchor)
    fun hide()
    fun dismissAllPopups()

    fun propagateMotionEvent(event: MotionEvent): Boolean
    fun activeKeyData(): KeyData?
}

/** No-op PopupHost for preview / test paths. */
object NoOpPopupHost : PopupHost {
    override val isShowingPopup: Boolean = false
    override val isShowingExtendedPopup: Boolean = false
    override fun show(anchor: KeyAnchor) = Unit
    override fun extend(anchor: KeyAnchor) = Unit
    override fun hide() = Unit
    override fun dismissAllPopups() = Unit
    override fun propagateMotionEvent(event: MotionEvent): Boolean = false
    override fun activeKeyData(): KeyData? = null
}
