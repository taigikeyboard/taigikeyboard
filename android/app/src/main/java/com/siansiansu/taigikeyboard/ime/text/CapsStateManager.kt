// 中文: 鍵盤大小寫/Shift 狀態機 — 從 TextInputManager 抽出獨立管理。
// 中文: 處理:auto-capitalization、單擊 Shift、雙擊 caps lock、依游標位置自動切大小寫模式。

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
    private var cursorCapsMode: CapsMode = CapsMode.NONE
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
     * Update caps state based on cursor position and editor info.
     */
    fun updateCapsState() {
        cursorCapsMode = fetchCurrentCursorCapsMode()
        if (!capsLock) {
            caps = if (taigikeyboard.prefs.autoCapitalizationEnabled) {
                cursorCapsMode != CapsMode.NONE
            } else {
                false
            }
            onInvalidateAllKeys()
        }
    }

    /**
     * Reset caps after single character output (non-capslock).
     */
    fun resetSingleShift() {
        if (caps && !capsLock) {
            caps = false
            updateCapsState()
        }
    }

    private fun fetchCurrentCursorCapsMode(): CapsMode {
        val ic = taigikeyboard.currentInputConnection
        val info = taigikeyboard.currentInputEditorInfo
        val capsFlags = ic?.getCursorCapsMode(info.inputType) ?: 0
        return parseCapsModeFromFlags(capsFlags)
    }

    private fun parseCapsModeFromFlags(flags: Int): CapsMode {
        return when {
            flags and InputType.TYPE_TEXT_FLAG_CAP_CHARACTERS > 0 -> CapsMode.ALL
            flags and InputType.TYPE_TEXT_FLAG_CAP_SENTENCES > 0 -> CapsMode.SENTENCES
            flags and InputType.TYPE_TEXT_FLAG_CAP_WORDS > 0 -> CapsMode.WORDS
            else -> CapsMode.NONE
        }
    }

    enum class CapsMode {
        ALL,
        NONE,
        SENTENCES,
        WORDS,
    }
}
