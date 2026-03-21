
package com.siansiansu.taigikeyboard.ime.popup

import android.content.res.Configuration
import android.util.TypedValue
import android.view.Gravity
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.widget.*
import androidx.core.content.ContextCompat.getDrawable
import com.google.android.flexbox.FlexboxLayout
import com.google.android.flexbox.JustifyContent
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.media.emoji.EmojiKeyData
import com.siansiansu.taigikeyboard.ime.media.emoji.EmojiKeyboardView
import com.siansiansu.taigikeyboard.ime.text.key.KeyCode
import com.siansiansu.taigikeyboard.ime.text.key.KeyData
import com.siansiansu.taigikeyboard.ime.text.key.KeyView
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardView

class KeyPopupManager<T_KBD: View, T_KV: View>(private val keyboardView: T_KBD) {

    private var anchorLeft: Boolean = false
    private var anchorRight: Boolean = false
    private var anchorOffset: Int = 0
    private var activeExtIndex: Int? = null
    private val exceptionsForKeyCodes = listOf(
        KeyCode.ENTER,
        KeyCode.LANGUAGE_SWITCH,
        KeyCode.SWITCH_TO_TEXT_CONTEXT,
        KeyCode.SWITCH_TO_MEDIA_CONTEXT
    )
    private var keyPopupWidth: Int
    private var keyPopupHeight: Int
    private var keyPopupDiffX: Int = 0
    private val popupView: LinearLayout
    private val popupViewExt: FlexboxLayout
    private var row0count: Int = 0
    private var row1count: Int = 0
    private var window: PopupWindow
    private var windowExt: PopupWindow

    /** Is true if the preview popup is visible to the user, else false */
    val isShowingPopup: Boolean
        get() = popupView.visibility == View.VISIBLE
    /** Is true if the extended popup is visible to the user, else false */
    val isShowingExtendedPopup: Boolean
        get() = windowExt.isShowing

    init {
        keyPopupWidth = keyboardView.resources.getDimension(R.dimen.key_width).toInt()
        keyPopupHeight = keyboardView.resources.getDimension(R.dimen.key_height).toInt()
        popupView = View.inflate(
            keyboardView.context,
            R.layout.key_popup, null
        ) as LinearLayout
        popupView.visibility = View.INVISIBLE
        popupViewExt = View.inflate(
            keyboardView.context,
            R.layout.key_popup_extended, null
        ) as FlexboxLayout
        window = createPopupWindow(popupView)
        windowExt = createPopupWindow(popupViewExt)
    }

    /**
     * Helper function to create a [KeyPopupExtendedSingleView] and preconfigure it.
     *
     * @param keyView Reference to the keyView currently controlling the popup.
     * @param k The index of the key in the key data popup array.
     * @param isInitActive If it should initially be marked as active.
     * @param isWrapBefore If the [FlexboxLayout] should wrap before this view.
     * @return A preconfigured [KeyPopupExtendedSingleView].
     */
    private fun createTextView(
        keyView: T_KV,
        k: Int,
        isInitActive: Boolean = false,
        isWrapBefore: Boolean = false
    ): KeyPopupExtendedSingleView? {
        val textView = KeyPopupExtendedSingleView(keyView.context, isInitActive)
        val lp = FlexboxLayout.LayoutParams(keyPopupWidth, keyView.measuredHeight)
        lp.isWrapBefore = isWrapBefore
        textView.layoutParams = lp
        textView.gravity = Gravity.CENTER
        val textSize = keyboardView.resources.getDimension(R.dimen.key_popup_textSize)
        if (keyView is KeyView) {
            when (keyView.data.popup[k].code) {
                KeyCode.SETTINGS -> {
                    textView.iconDrawable = getDrawable(
                        keyView.context, R.drawable.ic_settings
                    )
                }
                KeyCode.SWITCH_TO_TEXT_CONTEXT -> {
                    textView.text = keyView.resources.getString(R.string.key__view_characters)
                }
                KeyCode.SWITCH_TO_MEDIA_CONTEXT -> {
                    textView.iconDrawable = getDrawable(
                        keyView.context, R.drawable.ic_sentiment_satisfied
                    )
                }
                else -> {
                    textView.setTextSize(
                        TypedValue.COMPLEX_UNIT_PX, when (keyView.data.popup[k].code) {
                            KeyCode.URI_COMPONENT_TLD,
                            KeyCode.SWITCH_TO_TEXT_CONTEXT -> textSize * 0.6f
                            else -> textSize
                        }
                    )
                    val computedLetter = keyView.getComputedLetter(keyView.data.popup[k])
                    textView.text = computedLetter

                    // 設定字體：根據 fontType 決定
                    val prefs = com.siansiansu.taigikeyboard.ime.core.PrefHelper(keyView.context)
                    textView.typeface = com.siansiansu.taigikeyboard.util.FontUtils.getTypefaceByType(
                        fontType = prefs.fontType,
                        context = keyView.context
                    )
                }
            }
        } // EmojiKeyView is no longer used (Compose implementation)
        return textView
    }

    /**
     * Helper function for a convenient way of creating a [PopupWindow].
     *
     * @param view The view to set as content view of the [PopupWindow].
     * @return A new [PopupWindow] already preconfigured and ready-to-go.
     */
    private fun createPopupWindow(view: View): PopupWindow {
        return PopupWindow(keyboardView.context).apply {
            animationStyle = 0
            contentView = view
            enterTransition = null
            exitTransition = null
            isClippingEnabled = false
            isFocusable = false
            isTouchable = false
            setBackgroundDrawable(null)
        }
    }

    /**
     * Shows a preview popup for the passed [keyView]. Ignores show requests for key views which
     * key code is equal to or less than [KeyCode.SPACE]. KeyViews with a code defined in
     * [exceptionsForKeyCodes] will only shadow-calculating the size of the key popup, as these
     * sizes are needed for the extended popup. No popup will be shown to the user in this case.
     *
     * @param keyView Reference to the keyView currently controlling the popup.
     */
    fun show(keyView: T_KV) {
        if (keyView is KeyView && keyView.data.code <= KeyCode.SPACE
            && !exceptionsForKeyCodes.contains(keyView.data.code)
            && keyView.data.popup.isEmpty()) {
            return
        }

        // Update keyPopupWidth and keyPopupHeight
        if (keyboardView is KeyboardView) {
            when (keyboardView.resources.configuration.orientation) {
                Configuration.ORIENTATION_LANDSCAPE -> {
                    keyPopupWidth = (keyboardView.desiredKeyWidth * 0.6f).toInt()
                    keyPopupHeight = (keyboardView.desiredKeyHeight * 3.0f).toInt()
                }
                else -> {
                    keyPopupWidth = (keyboardView.desiredKeyWidth * 1.1f).toInt()
                    keyPopupHeight = (keyboardView.desiredKeyHeight * 2.5f).toInt()
                }
            }
        } // EmojiKeyboardView no longer uses PopupManager (Compose implementation)
        keyPopupDiffX = (keyView.measuredWidth - keyPopupWidth) / 2
        // Calculating is done, so exit show() here if this key view is a special one.
        if (keyView is KeyView && exceptionsForKeyCodes.contains(keyView.data.code)) {
            return
        }

        val keyPopupX = keyPopupDiffX
        val keyPopupY = -keyPopupHeight
        if (window.isShowing) {
            window.update(keyView, keyPopupX, keyPopupY, keyPopupWidth, keyPopupHeight)
        } else {
            window.width = keyPopupWidth
            window.height = keyPopupHeight
            window.showAsDropDown(keyView, keyPopupX, keyPopupY, Gravity.NO_GRAVITY)
        }
        if (keyView is KeyView) {
            popupView.findViewById<TextView>(R.id.key_popup_text)?.text = keyView.getComputedLetter()
            popupView.findViewById<ImageView>(R.id.key_popup_threedots)?.visibility = when {
                keyView.data.popup.isEmpty() -> View.INVISIBLE
                else -> View.VISIBLE
            }
        } // EmojiKeyView is no longer used (Compose implementation)
        popupView.visibility = View.VISIBLE
    }

    /**
     * Extends the currently showing key preview popup if there are popup keys defined in the
     * key data of the passed [keyView]. Ignores extend requests for key views which key code
     * is equal to or less than [KeyCode.SPACE]. An exception is made for the codes defined in
     * [exceptionsForKeyCodes], as they most likely have special keys bound to them.
     *
     * Layout of the extended key popup: (n = keyView.data.popup.size)
     *   when n <= 5: single line, row0 only
     *     _ _ _ _ _
     *     K K K K K
     *   when n > 5 && n % 2 == 1: multi line, row0 has 1 more key than row1, empty space position
     *     is depending on the current anchor
     *     anchorLeft           anchorRight
     *     K K ... K _         _ K ... K K
     *     K K ... K K         K K ... K K
     *   when n > 5 && n % 2 == 0: multi line, both same length
     *     K K ... K K
     *     K K ... K K
     *
     * @param keyView Reference to the keyView currently controlling the popup.
     */
    fun extend(keyView: T_KV) {
        if (keyView is KeyView && keyView.data.code <= KeyCode.SPACE
            && !exceptionsForKeyCodes.contains(keyView.data.code)
            && keyView.data.popup.isEmpty()) {
            return
        }

        // Anchor left if keyView is in left half of keyboardView, else anchor right
        if (keyView is KeyView) {
            anchorLeft = keyView.x < keyboardView.measuredWidth / 2
        } // EmojiKeyView is no longer used (Compose implementation)
        anchorRight = !anchorLeft

        // 決定每一行的按鍵數量
        val n = when (keyView) {
            is KeyView -> keyView.data.popup.size
            else -> 0 // EmojiKeyView is no longer used (Compose implementation)
        }
        when {
            n <= 10 -> {
                row1count = 0
                row0count = n
            }
            n > 10 && n % 2 == 1 -> {
                row1count = (n - 1) / 2
                row0count = (n + 1) / 2
            }
            else -> {
                row1count = n / 2
                row0count = n / 2
            }
        }

        // Calculate anchor offset (always positive int, direction depends on anchorLeft and
        // anchorRight state)
        anchorOffset = when {
            row0count <= 1 -> 0
            else -> {
                var offset = when {
                    row0count % 2 == 1 -> (row0count - 1) / 2
                    row0count % 2 == 0 -> (row0count / 2) - 1
                    else -> 0
                }
                val availableSpace = when {
                    anchorLeft -> keyView.x.toInt() + keyPopupDiffX
                    anchorRight -> keyboardView.measuredWidth -
                            (keyView.x.toInt() + keyPopupDiffX + keyPopupWidth)
                    else -> 0
                }
                while (offset > 0) {
                    if (availableSpace >= offset * keyPopupWidth) {
                        break
                    } else {
                        offset -= 1
                    }
                }
                offset
            }
        }

        // Build UI
        popupViewExt.removeAllViews()
        val indices = when (keyView) {
            is KeyView -> keyView.data.popup.indices
            else -> IntRange(0, 0) // EmojiKeyView is no longer used (Compose implementation)
        }
        var idx = 0
        for (k in indices) {
            val isInitActive =
                anchorLeft && (idx - row1count == anchorOffset) ||
                anchorRight && (idx - row1count == row0count - 1 - anchorOffset)
            popupViewExt.addView(
                createTextView(
                    keyView, k, isInitActive, (row1count > 0) && (idx - row1count == 0)
                )
            )
            if (isInitActive) {
                activeExtIndex = idx
            }
            idx++
        }
        popupView.findViewById<ImageView>(R.id.key_popup_threedots)?.visibility = View.INVISIBLE

        // Calculate layout params
        val extWidth = row0count * keyPopupWidth
        val extHeight = when {
            row1count > 0 -> keyView.measuredHeight * 2
            else -> keyView.measuredHeight
        }
        popupViewExt.justifyContent = if (anchorLeft) {
            JustifyContent.FLEX_START
        } else {
            JustifyContent.FLEX_END
        }
        if (popupViewExt.layoutParams == null) {
            popupViewExt.layoutParams = ViewGroup.LayoutParams(extWidth, extHeight)
        } else {
            popupViewExt.layoutParams.apply {
                width = extWidth
                height = extHeight
            }
        }
        val x = ((keyView.measuredWidth - keyPopupWidth) / 2) + when {
            anchorLeft -> -anchorOffset * keyPopupWidth
            anchorRight -> -extWidth + keyPopupWidth + anchorOffset * keyPopupWidth
            else -> 0
        }
        val y = -keyPopupHeight - when {
            row1count > 0 -> keyView.measuredHeight
            else -> 0
        }

        // Position and show popup window
        if (windowExt.isShowing) {
            windowExt.update(keyView, x, y, extWidth, extHeight)
        } else {
            windowExt.width = extWidth
            windowExt.height = extHeight
            windowExt.showAsDropDown(keyView, x, y, Gravity.NO_GRAVITY)
        }
    }

    /**
     * Updates the current selected key in extended popup according to the passed [event].
     * This function does nothing if the extended popup is not showing and will return false.
     *
     * @param keyView Reference to the keyView currently controlling the popup.
     * @param event The [MotionEvent] passed from the parent keyboard view's onTouch event.
     * @return True if the pointer movement is within the elements bounds, false otherwise.
     */
    fun propagateMotionEvent(keyView: T_KV, event: MotionEvent): Boolean {
        if (!isShowingExtendedPopup) {
            return false
        }

        val kX: Float = event.x / keyPopupWidth.toFloat()

        // Check if out of boundary on y-axis
        if (event.y < -keyPopupHeight || event.y > 0.9f * keyPopupHeight) {
            return false
        }

        activeExtIndex = when {
            anchorLeft -> when {
                // check if out of boundary on x-axis
                event.x < keyPopupDiffX - (anchorOffset + 1) * keyPopupWidth ||
                event.x > (keyPopupDiffX + (row0count + 1 - anchorOffset) * keyPopupWidth) -> {
                    return false
                }
                // row 1
                event.y < 0 && row1count > 0 -> when {
                    kX >= row1count - anchorOffset -> row1count - 1
                    kX < -anchorOffset -> 0
                    kX < 0 -> kX.toInt() - 1 + anchorOffset
                    else -> kX.toInt() + anchorOffset
                }
                // row 0
                else -> when {
                    kX >= row0count - anchorOffset -> row1count + row0count - 1
                    kX < -anchorOffset -> row1count
                    kX < 0 -> row1count + kX.toInt() - 1 + anchorOffset
                    else -> row1count + kX.toInt() + anchorOffset
                }
            }
            anchorRight -> when {
                // check if out of boundary on x-axis
                event.x > keyView.measuredWidth - keyPopupDiffX + (anchorOffset + 1) * keyPopupWidth ||
                event.x < (keyView.measuredWidth -
                        keyPopupDiffX - (row0count + 1 - anchorOffset) * keyPopupWidth) -> {
                    return false
                }
                // row 1
                event.y < 0 && row1count > 0 -> when {
                    kX >= anchorOffset -> row1count - 1
                    kX < -(row1count - 1 - anchorOffset) -> 0
                    kX < 0 -> row1count - 2 + kX.toInt() - anchorOffset
                    else -> row1count - 1 + kX.toInt() - anchorOffset
                }
                // row 0
                else -> when {
                    kX >= anchorOffset -> row1count + row0count - 1
                    kX < -(row0count - 1 - anchorOffset) -> row1count
                    kX < 0 -> row1count + row0count - 2 + kX.toInt() - anchorOffset
                    else -> row1count + row0count - 1 + kX.toInt() - anchorOffset
                }
            }
            else -> -1
        }

        if (keyView is KeyView) {
            for (k in keyView.data.popup.indices) {
                val view = popupViewExt.getChildAt(k)
                if (view != null) {
                    val textView = view as KeyPopupExtendedSingleView
                    textView.isActive = k == activeExtIndex
                }
            }
        } // EmojiKeyView is no longer used (Compose implementation)

        return true
    }

    /**
     * Gets the [KeyData] of the currently active key. May be either the key of the popup preview
     * or one of the keys in extended popup, if shown. Returns null if type parameter [T_KV]
     * is not [KeyView].
     *
     * @param keyView Reference to the keyView currently controlling the popup.
     * @return The [KeyData] object of the currently active key or null.
     */
    fun getActiveKeyData(keyView: T_KV): KeyData? {
        return if (keyView is KeyView) {
            keyView.data.popup.getOrNull(activeExtIndex ?: -1) ?: keyView.data
        } else {
            null
        }
    }

    /**
     * Gets the [EmojiKeyData] of the currently active key. May be either the key of the popup
     * preview or one of the keys in extended popup, if shown. Returns null (EmojiKeyView is
     * no longer used - Compose implementation).
     *
     * @param keyView Reference to the keyView currently controlling the popup.
     * @return The [EmojiKeyData] object of the currently active key or null.
     */
    fun getActiveEmojiKeyData(keyView: T_KV): EmojiKeyData? {
        return null // EmojiKeyView is no longer used (Compose implementation)
    }

    /**
     * Hides the key preview popup as well as the extended popup.
     */
    fun hide() {
        popupView.visibility = View.INVISIBLE
        if (windowExt.isShowing) {
            windowExt.dismiss()
        }

        activeExtIndex = null
    }

    /**
     * Dismisses all currently shown popups. Should be called by the parent keyboard view when it
     * is closing.
     */
    fun dismissAllPopups() {
        if (window.isShowing) {
            window.dismiss()
        }
        if (windowExt.isShowing) {
            windowExt.dismiss()
        }

        activeExtIndex = null
    }
}
