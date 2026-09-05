package com.siansiansu.taigikeyboard.ime.text.keyboard

import android.os.Handler
import android.os.Looper
import android.view.MotionEvent
import com.siansiansu.taigikeyboard.ime.popup.KeyAnchor
import com.siansiansu.taigikeyboard.ime.popup.PopupHost
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyType

/**
 * Adapter the coordinator uses to talk back to the IME. Lets [KeyboardLayout]
 * supply runtime callbacks (key-press dispatch, IME picker, vibration / sound)
 * without coupling this file to `TaigiKeyboard` or `TextInputManager`.
 */
interface KeyEventDispatcher {
    fun dispatchKeyPress(data: KeyData)

    fun showInputMethodPicker()

    fun keyPressVibrate()

    fun keyPressSound(data: KeyData)

    val longPressDelayMs: Long

    /** Resolves the [KeyAnchor] for popup show / extend. Done by the caller
     *  (not the coordinator) because anchor construction needs label
     *  formatting + popup-cell resolution that depend on `TaigiKeyboard` state.
     *  [keyboardWidth] is the current keyboard root width in pixels — needed
     *  for the extended-popup anchor-side selection. [desiredKeyWidth] and
     *  [desiredKeyHeight] are the solver-derived per-key cell dims (legacy
     *  `KeyboardView.desiredKeyWidth/Height`); they drive popup sizing in
     *  [KeyboardLayoutSolver.solvePopupDimensions]. */
    fun resolveAnchor(
        bounds: KeyBounds,
        keyboardWidth: Int,
        desiredKeyWidth: Int,
        desiredKeyHeight: Int,
    ): KeyAnchor
}

/**
 * Per-key bounds resolved by [KeyboardLayout] during measure/place. Values are
 * relative to the [KeyboardLayout] root in pixel coordinates. Hit-box widens
 * horizontally to the keyboard edge for first/last-in-row keys and includes
 * the key margin on top/bottom — mirrors legacy `KeyView.updateTouchHitBox`
 * so `INVARIANT_keyboard_touch_hit_box_matches_visible_bounds` is upheld.
 */
data class KeyBounds(
    val keyId: Long,
    val data: KeyData,
    val visible: Bounds,
    val hit: Bounds,
    val rowIndex: Int,
    /** True when this is the first key in [rowIndex]. Set at layout time so
     *  the per-MotionEvent drift check in [KeyTouchCoordinator.handleMove]
     *  doesn't have to scan the bounds list to recompute first/last
     *  membership on every pointer move. */
    val isFirstInRow: Boolean,
    val isLastInRow: Boolean,
)

data class Bounds(
    val left: Int,
    val top: Int,
    val right: Int,
    val bottom: Int,
) {
    val width: Int get() = right - left
    val height: Int get() = bottom - top

    /** Half-open `[left, right) × [top, bottom)` to match `android.graphics.Rect.contains`
     *  semantics — boundary pixels resolve to the next-key side, not the
     *  previous key. Mirrors the legacy `KeyView.touchHitBox.contains` path. */
    fun contains(
        x: Int,
        y: Int,
    ): Boolean = x >= left && x < right && y >= top && y < bottom
}

/**
 * Centralized MotionEvent → key dispatch state machine. Owns:
 *
 * - Active pointer ID + active key reference (single-pointer model — when a
 *   second pointer arrives mid-press, the first is force-released).
 * - Long-press scheduling (popup `extend` + special long-press paths for
 *   SPACE / LANGUAGE_SWITCH → IME picker, DELETE → repeat).
 * - Popup `show` / `extend` / `hide` / `propagateMotionEvent` hand-off.
 *
 * Mirrors the legacy `KeyboardView.onTouchEvent` + `KeyView.onFlorisTouchEvent`
 * pair as one cohesive state machine. Uses raw `MotionEvent` semantics
 * (`actionMasked`, pointer IDs, `ACTION_POINTER_*`) reached via Compose
 * `Modifier.pointerInteropFilter` — Compose's native pointer-input API does
 * not expose `actionMasked` / `actionIndex` cleanly enough to port the
 * legacy multi-pointer first-cancel rule (matches AOSP LatinIME
 * PointerTracker; pinned by §16 invariants).
 */
class KeyTouchCoordinator(
    private val popupHost: PopupHost,
    private val dispatcher: KeyEventDispatcher,
) {
    private val osHandler = Handler(Looper.getMainLooper())
    private var bounds: List<KeyBounds> = emptyList()
    private var keyboardWidth: Int = 0
    private var desiredKeyWidth: Int = 0
    private var desiredKeyHeight: Int = 0

    private var activeKey: KeyBounds? = null
    private var activePointerId: Int = -1
    private var activeX: Int = 0
    private var activeY: Int = 0
    private var pressedKeyId: Long? = null
    private var shouldBlockNextKeyCode: Boolean = false
    private var deleteRepeatRunnable: Runnable? = null
    private var longPressRunnable: Runnable? = null

    /** Pressed-key id callback so [KeyContent] composables can flip their
     *  visual pressed state without re-querying the coordinator. */
    var onPressedKeyChanged: ((Long?) -> Unit)? = null

    fun updateBounds(
        newBounds: List<KeyBounds>,
        keyboardWidth: Int,
        desiredKeyWidth: Int,
        desiredKeyHeight: Int,
    ) {
        bounds = newBounds
        this.keyboardWidth = keyboardWidth
        this.desiredKeyWidth = desiredKeyWidth
        this.desiredKeyHeight = desiredKeyHeight
    }

    /** Read-only preview hit test exposed for [KeyboardLayout] when running
     *  in `isPreview = true` mode. The preview path does not feed
     *  [onMotionEvent], so it needs to consult the resolved bounds without
     *  going through the full state machine. Returns the key id under the
     *  pointer (or null if no key contains the point). */
    fun previewHitTest(
        x: Int,
        y: Int,
    ): Long? = findKeyAt(x, y)?.keyId

    fun cancelActiveKey() {
        cancelScheduled()
        if (pressedKeyId != null) {
            pressedKeyId = null
            onPressedKeyChanged?.invoke(null)
        }
        popupHost.hide()
        activeKey = null
        activePointerId = -1
        shouldBlockNextKeyCode = false
    }

    fun reset() {
        cancelActiveKey()
        // Defensive: cancelScheduled() in cancelActiveKey already removes the
        // two known runnables (delete-repeat + long-press). Cleared again
        // here so future Runnable additions inside this coordinator do not
        // silently leak into the IME-service handler queue.
        osHandler.removeCallbacksAndMessages(null)
        popupHost.dismissAllPopups()
    }

    /** Drives the state machine from a raw MotionEvent fed by
     *  `Modifier.pointerInteropFilter`. */
    fun onMotionEvent(event: MotionEvent): Boolean {
        val pointerIndex = event.actionIndex
        val pointerId = event.getPointerId(pointerIndex)
        return when (event.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                handleDown(event, pointerIndex, pointerId)
                true
            }
            MotionEvent.ACTION_MOVE -> {
                handleMove(event)
                true
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP, MotionEvent.ACTION_CANCEL -> {
                handleUpOrCancel(event, pointerId, isCancel = event.actionMasked == MotionEvent.ACTION_CANCEL)
                true
            }
            else -> false
        }
    }

    private fun handleDown(
        event: MotionEvent,
        pointerIndex: Int,
        pointerId: Int,
    ) {
        val newX = event.getX(pointerIndex).toInt()
        val newY = event.getY(pointerIndex).toInt()

        if (activePointerId == -1) {
            activePointerId = pointerId
            activeX = newX
            activeY = newY
            startKeyPress(findKeyAt(newX, newY))
        } else if (activePointerId != pointerId) {
            // Multi-touch handoff — commit the previous pointer's active key
            // (legacy `KeyboardView.onTouchEvent` synthesizes ACTION_UP here,
            // which `KeyView.onFlorisTouchEvent` interprets as a press
            // dispatch when not blocked by long-press / popup state). Pins
            // `INVARIANT_keyboard_multi_touch_first_pointer_cancels`.
            endKeyPress(committed = true)
            activePointerId = pointerId
            activeX = newX
            activeY = newY
            startKeyPress(findKeyAt(newX, newY))
        } else {
            // Same pointer ID with stale state (UP/CANCEL was missed) —
            // recover without committing the prior press. Mirrors legacy
            // `KeyboardView.onTouchEvent` "stale pointerId" recovery branch.
            endKeyPress(committed = false)
            activePointerId = pointerId
            activeX = newX
            activeY = newY
            startKeyPress(findKeyAt(newX, newY))
        }
    }

    private fun handleMove(event: MotionEvent) {
        if (activePointerId == -1) return
        val pIdx = event.findPointerIndex(activePointerId)
        if (pIdx < 0) return
        activeX = event.getX(pIdx).toInt()
        activeY = event.getY(pIdx).toInt()
        val key = activeKey

        if (popupHost.isShowingExtendedPopup && key != null) {
            // Inside extended popup: convert root-relative event to
            // key-relative coords and route to popup.propagateMotionEvent.
            // Pins INVARIANT_keyboard_popup_drag_select_tracks_pointer.
            val relX = (activeX - key.visible.left).toFloat()
            val relY = (activeY - key.visible.top).toFloat()
            val relEvent = MotionEvent.obtain(
                event.downTime,
                event.eventTime,
                MotionEvent.ACTION_MOVE,
                relX,
                relY,
                0,
            )
            try {
                val withinBounds = popupHost.propagateMotionEvent(relEvent)
                if (!withinBounds && !shouldBlockNextKeyCode) {
                    cancelActiveKey()
                }
            } finally {
                relEvent.recycle()
            }
        } else if (key != null) {
            // No extended popup: dismiss when finger drifts too far from the
            // active key. Mirrors KeyView.onFlorisTouchEvent ACTION_MOVE
            // drift check (KeyView.kt:407-425).
            val width = key.visible.width
            val height = key.visible.height
            val relX = activeX - key.visible.left
            val relY = activeY - key.visible.top
            val driftedHoriz = (!key.isFirstInRow && relX < -0.1f * width) ||
                (!key.isLastInRow && relX > 1.1f * width)
            val driftedVert = relY < -0.35f * height || relY > 1.35f * height
            if ((driftedHoriz || driftedVert) && !shouldBlockNextKeyCode) {
                cancelActiveKey()
            }
        }
        // No `else`: when `activeKey == null` MOVE is a no-op. A touch whose
        // initial DOWN hit-test missed (e.g., started above the keyboard on
        // the smartbar or in a layout gap) does NOT get promoted to a press
        // by drifting onto a key. Aligns with AOSP LatinIME
        // `PointerTracker.onMoveEvent` semantics. Pins
        // `INVARIANT_keyboard_press_starts_only_on_down`.
    }

    private fun handleUpOrCancel(
        event: MotionEvent,
        pointerId: Int,
        isCancel: Boolean,
    ) {
        if (isCancel) {
            // ACTION_CANCEL is a gesture-level abort: the framework does not
            // guarantee `actionIndex` reports the active pointer id, so the
            // pointer-id gate used for ACTION_UP would skip cleanup and
            // leave pressed state / popup / DELETE-repeat lingering until an
            // unrelated event resets things. Pins
            // `INVARIANT_keyboard_action_cancel_unconditional_cleanup`.
            if (activePointerId == -1) return
            endKeyPress(committed = false)
            activePointerId = -1
            return
        }
        if (activePointerId != pointerId) return
        endKeyPress(committed = true)
        activePointerId = -1
    }

    private fun startKeyPress(key: KeyBounds?) {
        cancelScheduled()
        activeKey = key
        if (key == null) return
        pressedKeyId = key.keyId
        onPressedKeyChanged?.invoke(key.keyId)
        dispatcher.keyPressVibrate()
        dispatcher.keyPressSound(key.data)

        val anchor = dispatcher.resolveAnchor(key, keyboardWidth, desiredKeyWidth, desiredKeyHeight)
        popupHost.show(anchor)

        if (key.data.code == KeyCode.DELETE && key.data.type == KeyType.ENTER_EDITING) {
            val runnable = object : Runnable {
                override fun run() {
                    if (pressedKeyId == key.keyId) {
                        dispatcher.dispatchKeyPress(key.data)
                        osHandler.postDelayed(this, DELETE_REPEAT_INTERVAL_MS)
                    }
                }
            }
            deleteRepeatRunnable = runnable
            osHandler.postDelayed(runnable, DELETE_REPEAT_INITIAL_DELAY_MS)
        }

        // Long-press → popup extend + special paths (SPACE / LANGUAGE_SWITCH
        // → IME picker, with shouldBlockNextKeyCode = true so the eventual
        // UP does not also dispatch a SPACE / language-switch keypress).
        // Pins INVARIANT_keyboard_long_press_delay_unchanged. The press-start
        // anchor is reused so the long-press path skips a second
        // [resolveAnchor] (which would re-run popup-cell + label resolution).
        val longPress = Runnable {
            if (pressedKeyId != key.keyId) return@Runnable
            if (key.data.popup.isNotEmpty()) {
                popupHost.extend(anchor)
            }
            if (key.data.code == KeyCode.SPACE || key.data.code == KeyCode.LANGUAGE_SWITCH) {
                dispatcher.showInputMethodPicker()
                shouldBlockNextKeyCode = true
            }
        }
        longPressRunnable = longPress
        osHandler.postDelayed(longPress, dispatcher.longPressDelayMs)
    }

    private fun endKeyPress(committed: Boolean) {
        val key = activeKey
        cancelScheduled()
        if (pressedKeyId != null) {
            pressedKeyId = null
            onPressedKeyChanged?.invoke(null)
        }
        val keyDataOnUp: KeyData? = popupHost.activeKeyData() ?: key?.data
        popupHost.hide()
        if (committed && !shouldBlockNextKeyCode && keyDataOnUp != null) {
            dispatcher.dispatchKeyPress(keyDataOnUp)
        }
        shouldBlockNextKeyCode = false
        activeKey = null
    }

    private fun cancelScheduled() {
        deleteRepeatRunnable?.let { osHandler.removeCallbacks(it) }
        longPressRunnable?.let { osHandler.removeCallbacks(it) }
        deleteRepeatRunnable = null
        longPressRunnable = null
    }

    private fun findKeyAt(
        x: Int,
        y: Int,
    ): KeyBounds? = bounds.firstOrNull { it.hit.contains(x, y) }

    companion object {
        private const val DELETE_REPEAT_INITIAL_DELAY_MS = 500L
        private const val DELETE_REPEAT_INTERVAL_MS = 50L
    }
}
