
package com.siansiansu.taigikeyboard.ime.text.key

import android.annotation.SuppressLint
import android.content.res.Configuration
import android.graphics.*
import android.graphics.drawable.Drawable
import android.os.Handler
import android.os.Looper
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.view.ViewOutlineProvider
import android.view.inputmethod.EditorInfo
import androidx.core.content.ContextCompat.getDrawable
import androidx.core.graphics.BlendModeColorFilterCompat
import androidx.core.graphics.BlendModeCompat
import androidx.core.view.children
import com.google.android.flexbox.FlexboxLayout
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardView
import com.siansiansu.taigikeyboard.util.getColorFromAttr
import com.siansiansu.taigikeyboard.util.setBackgroundTintColor
import com.siansiansu.taigikeyboard.settings.AppTexts
import com.siansiansu.taigikeyboard.settings.DisplayLanguage
import java.util.*

@SuppressLint("ViewConstructor")
class KeyView(
    private val keyboardView: KeyboardView,
    val data: KeyData
) : View(keyboardView.context) {

    companion object {
        // 共享 Paint 物件以減少記憶體使用
        private val sharedLabelPaint: Paint = Paint().apply {
            alpha = 255
            color = 0
            isAntiAlias = true
            isFakeBoldText = false
            textAlign = Paint.Align.CENTER
            typeface = Typeface.DEFAULT
        }
    }

    private var isKeyPressed: Boolean = false
        set(value) {
            if (field != value) {
                field = value
                updateKeyPressedBackground()
                invalidate()
            }
        }
    private var osHandler: Handler? = null
    private var osTimer: Timer? = null
    private var shouldBlockNextKeyCode: Boolean = false

    private var drawable: Drawable? = null
    private var drawableColor: Int = 0
    private var drawablePadding: Int = 0
    private var label: String? = null

    // 快取計算結果，避免每幀重複計算
    private var cachedIsComposing: Boolean = false
    private var cachedImeAction: Int = 0
    private var cachedCaps: Boolean = false
    private var needsRedraw: Boolean = true

    var taigikeyboard: TaigiKeyboard? = null
    var touchHitBox: Rect = Rect(-1, -1, -1, -1)

    init {
        layoutParams = FlexboxLayout.LayoutParams(
            FlexboxLayout.LayoutParams.WRAP_CONTENT, FlexboxLayout.LayoutParams.WRAP_CONTENT
        ).apply {
            setMargins(
                resources.getDimension((R.dimen.key_marginH)).toInt(),
                resources.getDimension(R.dimen.key_marginV).toInt(),
                resources.getDimension((R.dimen.key_marginH)).toInt(),
                resources.getDimension(R.dimen.key_marginV).toInt()
            )
            flexShrink = when (keyboardView.computedLayout?.mode) {
                KeyboardMode.NUMERIC,
                KeyboardMode.NUMERIC_ADVANCED,
                KeyboardMode.PHONE,
                KeyboardMode.PHONE2 -> 1.0f
                else -> when (data.code) {
                    KeyCode.SHIFT,
                    KeyCode.VIEW_CHARACTERS,
                    KeyCode.VIEW_SYMBOLS,
                    KeyCode.VIEW_SYMBOLS2,
                    KeyCode.DELETE,
                    KeyCode.ENTER -> 0.0f
                    else -> 1.0f
                }
            }
            flexGrow = when (keyboardView.computedLayout?.mode) {
                KeyboardMode.NUMERIC,
                KeyboardMode.PHONE,
                KeyboardMode.PHONE2 -> 0.0f
                KeyboardMode.NUMERIC_ADVANCED -> when (data.type) {
                    KeyType.NUMERIC -> 1.0f
                    else -> 0.0f
                }
                else -> when (data.code) {
                    KeyCode.SPACE -> 1.0f
                    else -> 0.0f
                }
            }
        }
        setPadding(0, 0, 0, 0)

        background = getDrawable(context, R.drawable.shape_rect_rounded)
        elevation = 0.0f

        updateKeyPressedBackground()

        // 初始化時更新按鍵內容
        updateKeyContent()
    }

    /**
     * Creates a label text from the given [keyData].
     *
     * @param keyData Optional. The key data to generate the label from. Defaults to [data].
     * @return The generated label.
     */
    fun getComputedLetter(keyData: KeyData = data): String {
        if (keyData.code == KeyCode.URI_COMPONENT_TLD) {
            return when (taigikeyboard?.textInputManager?.caps) {
                true -> keyData.label.uppercase(Locale.getDefault())
                else -> keyData.label.lowercase(Locale.getDefault())
            }
        }
        // Use label if it's different from code (for multi-codepoint characters like o͘)
        // Otherwise use code for standard single characters
        val baseLabel = if (keyData.label.isNotEmpty() &&
            keyData.label != keyData.code.toChar().toString()) {
            keyData.label
        } else {
            keyData.code.toChar().toString()
        }
        return when {
            taigikeyboard?.textInputManager?.caps ?: false -> baseLabel.uppercase(Locale.getDefault())
            else -> baseLabel
        }
    }

    /**
     * 覆寫 invalidate 以標記需要重繪
     */
    override fun invalidate() {
        needsRedraw = true
        super.invalidate()
    }

    /**
     * Disable receiving touch events by the Android system. All touch events should be handled
     * only by the parent [KeyboardView].
     *
     * @see [onFlorisTouchEvent] for an explanation why.
     */
    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(event: MotionEvent?): Boolean {
        return false
    }

    /**
     * Basically the same as [onTouchEvent], but is only called by the parent [KeyboardView].
     * The parent [KeyboardView] can at any time send an [MotionEvent.ACTION_CANCEL], which means
     * the pointer has lost interest in this key and thus this key should return back to the
     * default, non-pressed state. An [MotionEvent.ACTION_CANCEL] can also be requested by this
     * [KeyView] itself, if it notices that the pointer moved to far from the key and/or from the
     * eventually showing extended popup.
     *
     * The reason why a custom onTouch event listener is being used is that in the default
     * implementation of the Android touch system there isn't really a way for a child view to tell
     * its parent that it has lost interest in having the focus of the parent and the parent should
     * go look at which child the pointer is actually above.
     */
    fun onFlorisTouchEvent(event: MotionEvent?): Boolean {
        event ?: return false
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                keyboardView.popupManager.show(this)
                isKeyPressed = true
                taigikeyboard?.keyPressVibrate(this)
                taigikeyboard?.keyPressSound(data)
                if (data.code == KeyCode.DELETE && data.type == KeyType.ENTER_EDITING) {
                    osTimer = Timer()
                    osTimer?.scheduleAtFixedRate(object : TimerTask() {
                        override fun run() {
                            taigikeyboard?.textInputManager?.sendKeyPress(data)
                            if (!isKeyPressed) {
                                osTimer?.cancel()
                                osTimer = null
                            }
                        }
                    }, 500, 50)
                }
                val delayMillis = keyboardView.prefs.longPressDelay
                if (osHandler == null) {
                    osHandler = Handler(Looper.getMainLooper())
                }
                osHandler?.postDelayed({
                    if (data.popup.isNotEmpty()) {
                        keyboardView.popupManager.extend(this)
                    }
                    if (data.code == KeyCode.SPACE) {
                        taigikeyboard?.textInputManager?.sendKeyPress(
                            KeyData(
                                KeyCode.SHOW_INPUT_METHOD_PICKER,
                                type = KeyType.FUNCTION
                            )
                        )
                        shouldBlockNextKeyCode = true
                    }
                    if (data.code == KeyCode.LANGUAGE_SWITCH) {
                        // 長按顯示輸入法選單
                        taigikeyboard?.textInputManager?.sendKeyPress(
                            KeyData(
                                KeyCode.SHOW_INPUT_METHOD_PICKER,
                                type = KeyType.FUNCTION
                            )
                        )
                        shouldBlockNextKeyCode = true
                    }
                }, delayMillis.toLong())
            }
            MotionEvent.ACTION_MOVE -> {
                if (keyboardView.popupManager.isShowingExtendedPopup) {
                    val isPointerWithinBounds =
                        keyboardView.popupManager.propagateMotionEvent(this, event)
                    if (!isPointerWithinBounds && !shouldBlockNextKeyCode) {
                        keyboardView.dismissActiveKeyViewReference()
                    }
                } else {
                    val parent = parent as ViewGroup
                    if ((event.x < -0.1f * measuredWidth && parent.children.first() != this)
                        || (event.x > 1.1f * measuredWidth && parent.children.last() != this)
                        || event.y < -0.35f * measuredHeight
                        || event.y > 1.35f * measuredHeight
                    ) {
                        if (!shouldBlockNextKeyCode) {
                            keyboardView.dismissActiveKeyViewReference()
                        }
                    }
                }
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                isKeyPressed = false
                osHandler?.removeCallbacksAndMessages(null)
                osTimer?.cancel()
                osTimer = null
                val retData = keyboardView.popupManager.getActiveKeyData(this)
                keyboardView.popupManager.hide()
                if (event.actionMasked != MotionEvent.ACTION_CANCEL && !shouldBlockNextKeyCode && retData != null) {
                    taigikeyboard?.textInputManager?.sendKeyPress(retData)
                    performClick()
                } else {
                    shouldBlockNextKeyCode = false
                }
            }
            else -> return false
        }
        return true
    }

    /**
     * Solution base from this great StackOverflow answer which explained and helped a lot
     * for handling onMeasure():
     *  https://stackoverflow.com/a/12267248/6801193
     *  by Devunwired
     */
    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val desiredWidth = when (keyboardView.computedLayout?.mode) {
            KeyboardMode.NUMERIC,
            KeyboardMode.PHONE,
            KeyboardMode.PHONE2 -> (keyboardView.desiredKeyWidth * 2.68f).toInt()
            KeyboardMode.NUMERIC_ADVANCED -> when (data.code) {
                44, 46 -> keyboardView.desiredKeyWidth
                KeyCode.VIEW_SYMBOLS, 61 -> (keyboardView.desiredKeyWidth * 1.34f).toInt()
                else -> (keyboardView.desiredKeyWidth * 1.56f).toInt()
            }
            else -> when (data.code) {
                KeyCode.SHIFT,
                KeyCode.VIEW_CHARACTERS,
                KeyCode.VIEW_SYMBOLS,
                KeyCode.VIEW_SYMBOLS2,
                KeyCode.DELETE,
                KeyCode.ENTER -> (keyboardView.desiredKeyWidth * 1.56f).toInt()
                KeyCode.SPACE -> when (keyboardView.computedLayout?.mode) {
                    KeyboardMode.SYMBOLS -> (keyboardView.desiredKeyWidth * 0.56f).toInt()
                    else -> keyboardView.desiredKeyWidth
                }
                else -> keyboardView.desiredKeyWidth
            }
        }
        val desiredHeight = keyboardView.desiredKeyHeight

        val widthMode = MeasureSpec.getMode(widthMeasureSpec)
        val widthSize = MeasureSpec.getSize(widthMeasureSpec)
        val heightMode = MeasureSpec.getMode(heightMeasureSpec)
        val heightSize = MeasureSpec.getSize(heightMeasureSpec)

        // Measure Width
        val width = when (widthMode) {
            MeasureSpec.EXACTLY -> {
                // Must be this size
                widthSize
            }
            MeasureSpec.AT_MOST -> {
                // Can't be bigger than...
                desiredWidth.coerceAtMost(widthSize)
            }
            else -> {
                // Be whatever you want
                desiredWidth
            }
        }

        // Measure Height
        val height = when (heightMode) {
            MeasureSpec.EXACTLY -> {
                // Must be this size
                heightSize
            }
            MeasureSpec.AT_MOST -> {
                // Can't be bigger than...
                desiredHeight.coerceAtMost(heightSize)
            }
            else -> {
                // Be whatever you want
                desiredHeight
            }
        }

        drawablePadding = (0.15f * height).toInt()

        // MUST CALL THIS
        setMeasuredDimension(width, height)
    }

    override fun onLayout(changed: Boolean, left: Int, top: Int, right: Int, bottom: Int) {
        super.onLayout(changed, left, top, right, bottom)
        updateTouchHitBox()
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        super.onSizeChanged(w, h, oldw, oldh)
        outlineProvider = KeyViewOutline(w, h)
    }

    /**
     * Updates the background depending on [isKeyPressed] and [data].
     * 當 translate 按鍵處於 swapped 狀態時，顯示啟用的背景色
     *
     * 優化：只在狀態改變時調用，不在 onDraw 中重複執行
     */
    private fun updateKeyPressedBackground() {
        if (data.code == KeyCode.ENTER) {
            setBackgroundTintColor(
                this, R.attr.colorPrimary
            )
        } else {
            // 檢查是否為 translate 按鍵且處於 swapped 狀態
            // 使用 SmartbarManager 的快取值避免 DataStore 非同步讀取問題
            val isTranslateSwapped = data.code == KeyCode.TRANSLATE &&
                com.siansiansu.taigikeyboard.ime.text.smartbar.SmartbarManager.getInstance().getCachedIsTranslateSwapped()

            setBackgroundTintColor(
                this, when {
                    isTranslateSwapped -> R.attr.key_bgColorActive
                    else -> R.attr.key_bgColor
                }
            )
        }
    }

    /**
     * 更新按鍵內容（文字/圖示）
     * 只在必要時重新計算，避免每幀都執行
     */
    private fun updateKeyContent() {
        // 檢查 Shift 狀態是否改變（影響字母大小寫）
        val currentCaps = taigikeyboard?.textInputManager?.caps ?: false
        if (cachedCaps != currentCaps) {
            cachedCaps = currentCaps
            needsRedraw = true
        }

        if (data.type == KeyType.CHARACTER && data.code != KeyCode.SPACE
            || data.type == KeyType.NUMERIC
        ) {
            label = getComputedLetter()
            drawable = null
        } else {
            when (data.code) {
                KeyCode.TRANSLATE -> {
                    drawable = getDrawable(context, R.drawable.ic_translate)
                    drawableColor = getColorFromAttr(context, R.attr.key_fgColor)
                    label = null
                }
                KeyCode.DELETE -> {
                    drawable = getDrawable(context, R.drawable.ic_backspace)
                    drawableColor = getColorFromAttr(context, R.attr.key_fgColor)
                    label = null
                }
                KeyCode.ENTER -> {
                    // 檢查是否處於組字模式
                    val isComposing = taigikeyboard?.textInputManager?.getComposingManager()?.isComposing() == true

                    // 記錄狀態是否改變（在更新 cachedIsComposing 之前）
                    val composingStateChanged = cachedIsComposing != isComposing

                    // 只在狀態改變時更新
                    if (composingStateChanged) {
                        cachedIsComposing = isComposing
                        needsRedraw = true
                    }

                    if (isComposing) {
                        // 組字模式：只顯示「確定」文字
                        // showHanjiMode 固定為 true
                        val displayLanguage = when {
                            keyboardView.prefs.isTranslateSwapped -> DisplayLanguage.HANJI
                            keyboardView.prefs.inputMode == "poj" -> DisplayLanguage.POJ
                            else -> DisplayLanguage.TL
                        }
                        label = AppTexts.confirm.text(displayLanguage)
                        drawable = null
                    } else {
                        // 非組字模式：只顯示圖示
                        label = null
                        val action = taigikeyboard?.currentInputEditorInfo?.imeOptions ?: 0

                        // 需要更新 drawable 的條件：
                        // 1. IME action 改變
                        // 2. 從組字模式切回來（composingStateChanged && !isComposing）
                        val needUpdateDrawable = (cachedImeAction != action) || (composingStateChanged && !isComposing)
                        if (needUpdateDrawable) {
                            cachedImeAction = action
                            drawable = getDrawable(context, when (action and EditorInfo.IME_MASK_ACTION) {
                                EditorInfo.IME_ACTION_DONE -> R.drawable.ic_done
                                EditorInfo.IME_ACTION_GO -> R.drawable.ic_arrow_right_alt
                                EditorInfo.IME_ACTION_NEXT -> R.drawable.ic_arrow_right_alt
                                EditorInfo.IME_ACTION_NONE -> R.drawable.ic_keyboard_return
                                EditorInfo.IME_ACTION_PREVIOUS -> R.drawable.ic_arrow_right_alt
                                EditorInfo.IME_ACTION_SEARCH -> R.drawable.ic_search
                                EditorInfo.IME_ACTION_SEND -> R.drawable.ic_send
                                else -> R.drawable.ic_arrow_right_alt
                            })
                            drawableColor = getColorFromAttr(context, R.attr.key_enter_fgColor)
                            if (action and EditorInfo.IME_FLAG_NO_ENTER_ACTION > 0) {
                                drawable = getDrawable(context, R.drawable.ic_keyboard_return)
                            }
                        }
                    }
                }
                KeyCode.LANGUAGE_SWITCH -> {
                    drawable = getDrawable(context, R.drawable.ic_language)
                    drawableColor = getColorFromAttr(context, R.attr.key_fgColor)
                    label = null
                }
                KeyCode.PHONE_PAUSE -> {
                    label = resources.getString(R.string.key__phone_pause)
                    drawable = null
                }
                KeyCode.PHONE_WAIT -> {
                    label = resources.getString(R.string.key__phone_wait)
                    drawable = null
                }
                KeyCode.SHIFT -> {
                    label = null
                    drawable = getDrawable(context, when {
                        taigikeyboard?.textInputManager?.caps ?: false && taigikeyboard?.textInputManager?.capsLock ?: false -> {
                            drawableColor = getColorFromAttr(context, R.attr.colorAccent)
                            R.drawable.ic_keyboard_capslock
                        }
                        taigikeyboard?.textInputManager?.caps ?: false && !(taigikeyboard?.textInputManager?.capsLock ?: false) -> {
                            drawableColor = getColorFromAttr(context, R.attr.key_fgColor)
                            R.drawable.ic_keyboard_capslock
                        }
                        else -> {
                            drawableColor = getColorFromAttr(context, R.attr.key_fgColor)
                            R.drawable.ic_keyboard_arrow_up
                        }
                    })
                }
                KeyCode.SPACE -> {
                    when (keyboardView.computedLayout?.mode) {
                        KeyboardMode.NUMERIC,
                        KeyboardMode.NUMERIC_ADVANCED,
                        KeyboardMode.PHONE,
                        KeyboardMode.PHONE2 -> {
                            drawable = getDrawable(context, R.drawable.ic_space_bar)
                            drawableColor = getColorFromAttr(context, R.attr.key_fgColor)
                            label = null
                        }
                        KeyboardMode.CHARACTERS -> {
                            // 空白鍵不顯示文字
                            drawable = null
                            label = null
                        }
                        else -> {
                            drawable = null
                            label = null
                        }
                    }
                }
                KeyCode.SWITCH_TO_MEDIA_CONTEXT -> {
                    drawable = getDrawable(context, R.drawable.ic_sentiment_satisfied)
                    drawableColor = getColorFromAttr(context, R.attr.key_fgColor)
                    label = null
                }
                KeyCode.SWITCH_TO_TEXT_CONTEXT,
                KeyCode.VIEW_CHARACTERS -> {
                    label = resources.getString(R.string.key__view_characters)
                    drawable = null
                }
                KeyCode.VIEW_NUMERIC -> {
                    label = resources.getString(R.string.key__view_numeric)
                    drawable = null
                }
                KeyCode.VIEW_NUMERIC_ADVANCED -> {
                    // 在 symbol 鍵盤中，根據 isTranslateSwapped 狀態決定顯示內容
                    val isTranslateSwapped = com.siansiansu.taigikeyboard.ime.text.smartbar.SmartbarManager.getInstance().getCachedIsTranslateSwapped()
                    if (isTranslateSwapped && keyboardView.computedLayout?.mode == KeyboardMode.SYMBOLS) {
                        label = "、"
                        drawable = null
                    } else {
                        label = resources.getString(R.string.key__view_numeric)
                        drawable = null
                    }
                }
                KeyCode.VIEW_PHONE -> {
                    label = resources.getString(R.string.key__view_phone)
                    drawable = null
                }
                KeyCode.VIEW_PHONE2 -> {
                    label = resources.getString(R.string.key__view_phone2)
                    drawable = null
                }
                KeyCode.VIEW_SYMBOLS -> {
                    label = resources.getString(R.string.key__view_symbols)
                    drawable = null
                }
                KeyCode.VIEW_SYMBOLS2 -> {
                    label = resources.getString(R.string.key__view_symbols2)
                    drawable = null
                }
            }
        }
    }

    /**
     * Updates the touch hit box of this [KeyView] with its current absolute position within the
     * parent [KeyboardView].
     */
    private fun updateTouchHitBox() {
        if (visibility == GONE) {
            touchHitBox.set(-1, -1, -1, -1)
        } else {
            val parent = parent as ViewGroup
            val keyMarginH = resources.getDimension((R.dimen.key_marginH)).toInt()
            val keyMarginV = resources.getDimension((R.dimen.key_marginV)).toInt()

            touchHitBox.apply {
                left = when (this@KeyView) {
                    parent.children.first() -> 0
                    else -> (parent.x + x - keyMarginH).toInt()
                }
                right = when (this@KeyView) {
                    parent.children.last() -> keyboardView.measuredWidth
                    else -> (parent.x + x + measuredWidth + keyMarginH).toInt()
                }
                top = (parent.y + y - keyMarginV).toInt()
                bottom = (parent.y + y + measuredHeight + keyMarginV).toInt()
            }
        }
    }

    /**
     * Updates the visibility of this [KeyView] by checking the current key variation of the parent
     * TextInputManager.
     */
    fun updateVisibility() {
        when (data.code) {
            // SWITCH_TO_MEDIA_CONTEXT 和 LANGUAGE_SWITCH 不再互斥，都保持可見
            else -> if (data.variation != KeyVariation.ALL) {
                val keyVariation = taigikeyboard?.textInputManager?.keyVariation ?: KeyVariation.NORMAL
                val newVisibility =
                    if (data.variation == KeyVariation.NORMAL && (keyVariation == KeyVariation.NORMAL
                                || keyVariation == KeyVariation.PASSWORD)
                    ) {
                        VISIBLE
                    } else if (data.variation == keyVariation) {
                        VISIBLE
                    } else {
                        GONE
                    }
                if (data.label == "-" && data.code == 45) {
                    android.util.Log.d("KeyView", "Hyphen key updateVisibility: variation=${data.variation}, keyVariation=$keyVariation, newVisibility=$newVisibility")
                }
                visibility = newVisibility
                if (data.label == "-" && data.code == 45) {
                    android.util.Log.d("KeyView", "Hyphen key after set: visibility=$visibility, width=$width, height=$height, measuredWidth=$measuredWidth, isShown=$isShown")
                }
                updateTouchHitBox()
            }
        }
    }

    /**
     * Draw the key label / drawable.
     *
     * 優化：移除每幀重複計算，只繪製已快取的內容
     */
    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)

        // 只在需要時更新內容
        if (needsRedraw) {
            updateKeyContent()
            needsRedraw = false
        }

        // Draw drawable
        val drawable = drawable
        if (drawable != null) {
            var marginV = 0
            var marginH = 0
            if (measuredWidth > measuredHeight) {
                marginH = (measuredWidth - measuredHeight) / 2
            } else {
                marginV = (measuredHeight - measuredWidth) / 2
            }
            drawable.setBounds(
                marginH + drawablePadding,
                marginV + drawablePadding,
                measuredWidth - marginH - drawablePadding,
                measuredHeight - marginV - drawablePadding)
            drawable.colorFilter = BlendModeColorFilterCompat.createBlendModeColorFilterCompat(
                drawableColor,
                BlendModeCompat.SRC_ATOP
            )
            drawable.draw(canvas)
        }

        // Draw label
        val label = label
        if (label != null) {
            // 使用共享的 Paint 物件，設定當前按鍵的屬性
            sharedLabelPaint.textSize = when {
                // ?123 按鍵使用專屬字體大小
                data.code == KeyCode.VIEW_SYMBOLS -> {
                    resources.getDimension(R.dimen.key_symbols_textSize)
                }
                // Enter 鍵組字模式的「確定」文字使用較小字體
                data.code == KeyCode.ENTER && label.isNotEmpty() -> {
                    resources.getDimension(R.dimen.key_enter_confirm_textSize)
                }
                // VIEW_NUMERIC_ADVANCED: 根據顯示內容決定字體大小
                data.code == KeyCode.VIEW_NUMERIC_ADVANCED -> {
                    // 如果顯示「、」符號，使用一般按鍵大小；否則使用數字鍵大小
                    if (label == "、") {
                        resources.getDimension(R.dimen.key_textSize)
                    } else {
                        resources.getDimension(R.dimen.key_numeric_textSize)
                    }
                }
                // 數字鍵和空白鍵
                data.code == KeyCode.VIEW_NUMERIC ||
                data.code == KeyCode.SPACE -> {
                    resources.getDimension(R.dimen.key_numeric_textSize)
                }
                // 一般按鍵
                else -> {
                    resources.getDimension(R.dimen.key_textSize)
                }
            }

            // 根據設定設定字體
            sharedLabelPaint.typeface = com.siansiansu.taigikeyboard.util.FontUtils.getKeyFont(
                customFontEnabled = keyboardView.prefs.customFontEnabled,
                context = context
            )

            // Enter 鍵使用專屬顏色，其他按鍵使用一般顏色
            sharedLabelPaint.color = if (data.code == KeyCode.ENTER) {
                getColorFromAttr(context, R.attr.key_enter_fgColor)
            } else {
                getColorFromAttr(context, R.attr.key_fgColor)
            }

            sharedLabelPaint.alpha = if (keyboardView.computedLayout?.mode == KeyboardMode.CHARACTERS &&
                data.code == KeyCode.SPACE) { 120 } else { 255 }

            val centerX = measuredWidth / 2.0f
            val centerY = measuredHeight / 2.0f + (sharedLabelPaint.textSize - sharedLabelPaint.descent()) / 2

            if (label.contains("\n")) {
                // Even if more lines may be existing only the first 2 are shown
                val labelLines = label.split("\n")
                canvas.drawText(labelLines[0], centerX, centerY * 0.70f, sharedLabelPaint)
                canvas.drawText(labelLines[1], centerX, centerY * 1.30f, sharedLabelPaint)
            } else {
                canvas.drawText(label, centerX, centerY, sharedLabelPaint)
            }
        }
    }

    /**
     * Custom Outline Provider, needed for the [KeyView] elevation rendering.
     */
    private class KeyViewOutline(
        private val width: Int,
        private val height: Int
    ) : ViewOutlineProvider() {

        override fun getOutline(view: View?, outline: Outline?) {
            view ?: return
            outline ?: return
            outline.setRoundRect(
                0,
                0,
                width,
                height,
                view.resources.getDimension(R.dimen.key_borderRadius)
            )
        }
    }
}
