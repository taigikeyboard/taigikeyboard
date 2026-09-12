package com.siansiansu.taigikeyboard.ime.text

import android.os.Handler
import android.text.InputType
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard

/**
 * Manages keyboard caps/shift state extracted from TextInputManager.
 *
 * Handles auto-capitalization, single shift, caps lock, and
 * cursor-based caps mode detection.
 */
class CapsStateManager(
    private val taigikeyboard: TaigiKeyboard,
    private val onInvalidateAllKeys: () -> Unit,
    private val onInvalidateCharacterKeys: () -> Unit,
) {
    // Dedicated handler for caps double-tap detection — avoids interference with
    // TextInputManager's osHandler (double-space period, etc.)
    private val capsHandler: Handler = Handler(android.os.Looper.getMainLooper())
    var caps: Boolean = false
        private set
    var capsLock: Boolean = false
        private set
    private var hasCapsRecentlyChanged: Boolean = false
    var hasSpaceRecentlyPressed: Boolean = false

    fun getCapsState(): Pair<Boolean, Boolean> = Pair(caps, capsLock)

    /**
     * Handle shift key press (single tap / double tap for caps lock).
     */
    fun handleShift() {
        if (hasCapsRecentlyChanged) {
            capsHandler.removeCallbacksAndMessages(null)
            caps = true
            capsLock = true
            hasCapsRecentlyChanged = false
        } else {
            caps = !caps
            capsLock = false
            hasCapsRecentlyChanged = true
            capsHandler.postDelayed({
                hasCapsRecentlyChanged = false
            }, 300)
        }
        onInvalidateCharacterKeys()
    }

    /**
     * Re-derive auto-caps from the cursor position. Called on every cursor
     * update (`CURSOR_UPDATE_MONITOR`), so it must stay cheap: caps lock
     * owns the shift state outright, and with auto-capitalization off the
     * answer is always "no caps" — neither case touches the editor.
     */
    fun updateCapsState() {
        if (capsLock) return
        applyAutoCaps()
    }

    /**
     * Reset caps after single character output (non-capslock).
     */
    fun resetSingleShift() {
        if (caps && !capsLock) {
            applyAutoCaps()
        }
    }

    // `getCursorCapsMode` is a blocking IPC round-trip to the editor's
    // process and `onInvalidateAllKeys` rebuilds the keyboard appearance,
    // so both run only when they can change what the user sees.
    private fun applyAutoCaps() {
        val wantsCaps =
            taigikeyboard.prefs.autoCapitalizationEnabled &&
                fetchCurrentCursorCapsMode() != CapsMode.NONE
        if (caps == wantsCaps) return
        caps = wantsCaps
        onInvalidateAllKeys()
    }

    private fun fetchCurrentCursorCapsMode(): CapsMode {
        val ic = taigikeyboard.currentInputConnection
        val info = taigikeyboard.currentInputEditorInfo
        val capsFlags = ic?.getCursorCapsMode(info.inputType) ?: 0
        return parseCapsModeFromFlags(capsFlags)
    }

    private fun parseCapsModeFromFlags(flags: Int): CapsMode =
        when {
            flags and InputType.TYPE_TEXT_FLAG_CAP_CHARACTERS > 0 -> CapsMode.ALL
            flags and InputType.TYPE_TEXT_FLAG_CAP_SENTENCES > 0 -> CapsMode.SENTENCES
            flags and InputType.TYPE_TEXT_FLAG_CAP_WORDS > 0 -> CapsMode.WORDS
            else -> CapsMode.NONE
        }

    enum class CapsMode {
        ALL,
        NONE,
        SENTENCES,
        WORDS,
    }
}
