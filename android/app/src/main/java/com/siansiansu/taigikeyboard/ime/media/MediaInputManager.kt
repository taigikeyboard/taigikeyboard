
package com.siansiansu.taigikeyboard.ime.media

import android.annotation.SuppressLint
import android.os.Handler
import android.os.Looper
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.widget.*
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.InputView
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.logging.TraceContext
import com.siansiansu.taigikeyboard.ime.core.logging.TraceId
import com.siansiansu.taigikeyboard.ime.media.emoji.EmojiKeyData
import com.siansiansu.taigikeyboard.ime.media.emoji.EmojiKeyboardView
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyType
import kotlinx.coroutines.*

private const val TAG = "MediaInputManager"

class MediaInputManager(
    private val taigikeyboard: TaigiKeyboard,
) : CoroutineScope by MainScope(),
    TaigiKeyboard.EventListener {
    private val logger get() = taigikeyboard.compositionRoot.logger
    private var osHandler: Handler? = null
    private var isDeletePressed: Boolean = false
    private var emojiKeyboardView: ViewFlipper? = null

    var mediaViewGroup: LinearLayout? = null

    /**
     * Called when a new input view has been registered. Used to initialize all media-relevant
     * views and layouts.
     * TODO: evaluate if the view initializing process can be optimized.
     */
    @SuppressLint("ClickableViewAccessibility")
    override fun onRegisterInputView(inputView: InputView) {
        logger.i(TAG, "onRegisterInputView(inputView)")

        launch(Dispatchers.Default) {
            mediaViewGroup = inputView.findViewById(R.id.media_input)
            emojiKeyboardView = inputView.findViewById(R.id.media_input_view_flipper)

            // Init bottom buttons
            inputView
                .findViewById<Button>(R.id.media_input_switch_to_text_input_button)
                .setOnTouchListener { view, event -> onBottomButtonEvent(view, event) }
            inputView
                .findViewById<ImageButton>(R.id.media_input_backspace_button)
                .setOnTouchListener { view, event -> onBottomButtonEvent(view, event) }

            try {
                // 直接建立並加入 EmojiKeyboardView
                val emojiView = EmojiKeyboardView(taigikeyboard.context)
                withContext(Dispatchers.Main) {
                    val layoutParams =
                        ViewGroup.LayoutParams(
                            ViewGroup.LayoutParams.MATCH_PARENT,
                            ViewGroup.LayoutParams.WRAP_CONTENT,
                        )
                    emojiKeyboardView?.addView(emojiView, layoutParams)
                }
            } catch (e: Exception) {
                logger.e(TAG, "Error initializing media input views", e)
            }
        }
    }

    /**
     * Clean-up of resources and stopping all coroutines.
     */
    override fun onDestroy() {
        logger.i(TAG, "onDestroy()")

        cancel()
    }

    /**
     * Handles clicks on the bottom buttons.
     */
    private fun onBottomButtonEvent(
        view: View,
        event: MotionEvent?,
    ): Boolean {
        event ?: return false
        val data =
            when (view.id) {
                R.id.media_input_switch_to_text_input_button -> {
                    KeyData(KeyCode.SWITCH_TO_TEXT_CONTEXT)
                }

                R.id.media_input_backspace_button -> {
                    KeyData(KeyCode.DELETE, type = KeyType.ENTER_EDITING)
                }

                else -> {
                    null
                }
            }
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                taigikeyboard.keyPressVibrate(view)
                taigikeyboard.keyPressSound(data)
                if (data?.code == KeyCode.DELETE && data.type == KeyType.ENTER_EDITING) {
                    isDeletePressed = true
                    if (osHandler == null) {
                        osHandler = Handler(Looper.getMainLooper())
                    }
                    // 使用 Handler 替代 Timer，確保回調在主執行緒執行
                    val repeatDelete =
                        object : Runnable {
                            override fun run() {
                                if (isDeletePressed) {
                                    TraceContext.withTrace(TraceId.next()) {
                                        taigikeyboard.textInputManager.sendKeyPress(data)
                                    }
                                    osHandler?.postDelayed(this, 50)
                                }
                            }
                        }
                    osHandler?.postDelayed(repeatDelete, 500)
                }
            }

            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                isDeletePressed = false
                osHandler?.removeCallbacksAndMessages(null)
                if (event.actionMasked != MotionEvent.ACTION_CANCEL && data != null) {
                    TraceContext.withTrace(TraceId.next()) {
                        taigikeyboard.textInputManager.sendKeyPress(data)
                    }
                }
            }
        }
        // MUST return false here so the background selector for showing a transparent bg works
        return false
    }

    /**
     * Sends a given [emojiKeyData] to the current input editor.
     *
     * Routes through [com.siansiansu.taigikeyboard.ime.text.composing.ComposingManager]
     * so an active Taigi preedit is committed atomically together with the
     * emoji in a single `InputConnection.commitText` call. Pre-A5 behavior
     * (direct `finishComposingText` + `commitText`) was flagged as a
     * deferred parity violation in `composing-state-boundary.md` §11.6 —
     * this path now pins `INVARIANT_composing_external_insert_commits_preedit_atomically`
     * (see `behavioral-invariants.md` §13). Falls back to the direct commit
     * only when `ComposingManager` is unavailable (pre-init / post-destroy
     * edges).
     */
    fun sendEmojiKeyPress(emojiKeyData: EmojiKeyData) {
        val ic = taigikeyboard.currentInputConnection ?: return
        val emoji = emojiKeyData.getCodePointsAsString()
        val composingManager = taigikeyboard.textInputManager.getComposingManager()
        if (composingManager != null) {
            composingManager.commitPreeditThenInsertExternal(emoji, ic)
        } else {
            ic.commitText(emoji, 1)
        }
    }
}
