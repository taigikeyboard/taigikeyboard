
package com.siansiansu.taigikeyboard.ime.text.key

import android.annotation.SuppressLint
import android.content.res.Configuration
import android.graphics.*
import android.graphics.drawable.Drawable
import android.os.Build
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
import com.siansiansu.taigikeyboard.ime.core.logging.TraceContext
import com.siansiansu.taigikeyboard.ime.core.logging.TraceId
import com.siansiansu.taigikeyboard.ime.core.logging.tdebug
import com.siansiansu.taigikeyboard.ime.core.settings.InputMode
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardMode
import com.siansiansu.taigikeyboard.ime.text.keyboard.KeyboardView
import com.siansiansu.taigikeyboard.localization.SettingsTexts
import com.siansiansu.taigikeyboard.ime.theme.getColorFromAttr
import com.siansiansu.taigikeyboard.ime.theme.setBackgroundTintColor
import java.util.Locale

@SuppressLint("ViewConstructor")
class KeyView(
    private val keyboardView: KeyboardView,
    val data: KeyData,
) : View(keyboardView.context) {
    companion object {
        // 共享 Paint 物件以減少記憶體使用
        private val sharedLabelPaint: Paint =
            Paint().apply {
                alpha = 255
                color = 0
                isAntiAlias = true
                isFakeBoldText = false
                textAlign = Paint.Align.CENTER
                typeface = Typeface.DEFAULT
            }

        // Shared Paint for tone hint diacritics on number keys
        private val sharedHintPaint: Paint =
            Paint().apply {
                alpha = 150
                color = 0
                isAntiAlias = true
                isFakeBoldText = false
                textAlign = Paint.Align.CENTER
                typeface = Typeface.DEFAULT
            }

        // Tone number → standalone diacritic mapping (POJ and TL share all except tone 9)
        private val toneHints =
            mapOf(
                49 to "\u02CA", // 1 → space (no mark), handled below
                50 to "\u02CA", // 2 → ˊ MODIFIER LETTER ACUTE ACCENT
                51 to "\u02CB", // 3 → ˋ MODIFIER LETTER GRAVE ACCENT
                53 to "\u02C6", // 5 → ˆ MODIFIER LETTER CIRCUMFLEX ACCENT
                54 to "\u02C7", // 6 → ˇ CARON
                55 to "\u02C9", // 7 → ˉ MODIFIER LETTER MACRON
                56 to "\u02C8", // 8 → ˈ MODIFIER LETTER VERTICAL LINE
            )

        // Keys with no tone mark — use space placeholder for consistent layout
        private val noToneHintCodes = setOf(48, 49, 52) // 0, 1, 4

        /** Returns the tone hint diacritic for a number key, or null if not applicable. */
        fun toneHintForCode(
            code: Int,
            inputMode: String?,
        ): String? {
            if (inputMode == "english") return null
            if (code == 57) {
                // Tone 9: POJ uses breve, TL uses double acute
                return if (inputMode == "poj") "\u02D8" else "\u02BA"
            }
            if (noToneHintCodes.contains(code)) return " "
            return toneHints[code]
        }

        // MOE1 layout: punctuation key hints
        private val moe1Hints =
            mapOf(
                45 to "@", // - → @
                44 to ":;", // , (，) → :;
                46 to "!?", // . (。) → !?
            )

        /** Returns the hint for MOE1 punctuation keys, or null if not applicable. */
        fun moe1HintForCode(code: Int): String? = moe1Hints[code]
    }

    private var isKeyPressed: Boolean = false
        set(value) {
            if (field != value) {
                field = value
                // 同步 View 的 pressed state，讓 selector 的 state_pressed 能正確觸發
                isPressed = value
                updateKeyPressedBackground()
                invalidate()
            }
        }
    private var osHandler: Handler? = null
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
        layoutParams =
            FlexboxLayout
                .LayoutParams(
                    FlexboxLayout.LayoutParams.WRAP_CONTENT,
                    FlexboxLayout.LayoutParams.WRAP_CONTENT,
                ).apply {
                    setMargins(
                        resources.getDimension((R.dimen.key_marginH)).toInt(),
                        resources.getDimension(R.dimen.key_marginV).toInt(),
                        resources.getDimension((R.dimen.key_marginH)).toInt(),
                        resources.getDimension(R.dimen.key_marginV).toInt(),
                    )
                    flexShrink =
                        when (keyboardView.computedLayout?.mode) {
                            KeyboardMode.NUMERIC,
                            KeyboardMode.NUMERIC_ADVANCED,
                            KeyboardMode.PHONE,
                            KeyboardMode.PHONE2,
                            -> {
                                1.0f
                            }

                            else -> {
                                when (data.code) {
                                    KeyCode.SHIFT,
                                    KeyCode.VIEW_CHARACTERS,
                                    KeyCode.VIEW_SYMBOLS,
                                    KeyCode.VIEW_SYMBOLS2,
                                    KeyCode.DELETE,
                                    KeyCode.ENTER,
                                    KeyCode.TRANSLATE,
                                    -> 0.0f

                                    else -> 1.0f
                                }
                            }
                        }
                    flexGrow =
                        when (keyboardView.computedLayout?.mode) {
                            KeyboardMode.NUMERIC,
                            KeyboardMode.PHONE,
                            KeyboardMode.PHONE2,
                            -> {
                                0.0f
                            }

                            KeyboardMode.NUMERIC_ADVANCED -> {
                                when (data.type) {
                                    KeyType.NUMERIC -> 1.0f
                                    else -> 0.0f
                                }
                            }

                            else -> {
                                when (data.code) {
                                    KeyCode.SPACE -> 1.0f
                                    else -> 0.0f
                                }
                            }
                        }
                }
        setPadding(0, 0, 0, 0)

        applyAppearance()

        // 初始化時更新按鍵內容
        updateKeyContent()
    }

    /**
     * Apply background drawable, corner radius, border, and color tint from current prefs.
     * Safe to invoke repeatedly — also called from KeyboardView.applyAppearanceChanges()
     * when appearance settings are dragged live in the Layout tab preview.
     */
    internal fun applyAppearance() {
        val isFunctionKey =
            data.type == KeyType.MODIFIER || data.type == KeyType.ENTER_EDITING ||
                data.code == KeyCode.DELETE || data.code == KeyCode.SHIFT ||
                data.code == KeyCode.VIEW_NUMERIC || data.code == KeyCode.VIEW_NUMERIC_ADVANCED ||
                data.code == KeyCode.VIEW_SYMBOLS || data.code == KeyCode.VIEW_SYMBOLS2 ||
                data.code == KeyCode.VIEW_CHARACTERS
        // Inflate the background selector once per key — drawable type is static for a key's
        // lifetime, so re-inflating on every live appearance change wastes ~30 keys × ~30Hz
        // allocations during slider drag.
        if (background !is android.graphics.drawable.StateListDrawable) {
            background =
                when {
                    data.code == KeyCode.ENTER -> getDrawable(context, R.drawable.key_enter_background_selector)
                    isFunctionKey -> getDrawable(context, R.drawable.key_function_background_selector)
                    else -> getDrawable(context, R.drawable.key_background_selector)
                }
        }
        val radiusPx = keyboardView.prefs.keyCornerRadius * resources.displayMetrics.density
        val borderWidthPx = (keyboardView.prefs.keyBorderWidth * resources.displayMetrics.density).toInt()
        val borderColor = getColorFromAttr(context, R.attr.key_fgColor)
        val bg = background
        if (bg is android.graphics.drawable.StateListDrawable &&
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q
        ) {
            for (i in 0 until bg.stateCount) {
                val item = bg.getStateDrawable(i)
                if (item is android.graphics.drawable.GradientDrawable) {
                    item.cornerRadius = radiusPx
                    // Always re-apply stroke so dragging border-width to 0 visually clears it
                    item.setStroke(borderWidthPx, borderColor)
                }
            }
        }
        elevation = 0.0f

        val colors = keyboardView.getColorSettings()
        val isSpecialKey = isFunctionKey || data.code == KeyCode.ENTER
        val customFill = if (isSpecialKey) colors.specialKeyFillColor else colors.normalKeyFillColor
        // Always assign (null clears prior tint) so reset-to-default takes effect live
        backgroundTintList =
            customFill?.let {
                android.content.res.ColorStateList
                    .valueOf(it)
            }

        if (!keyboardView.isPreviewMode) {
            updateKeyPressedBackground()
        }

        // Refresh outline provider so cornerRadius changes apply without waiting for onSizeChanged
        if (width > 0 && height > 0) {
            outlineProvider = KeyViewOutline(width, height, keyboardView.prefs.keyCornerRadius)
        }
    }

    /**
     * Creates a label text from the given [keyData].
     *
     * @param keyData Optional. The key data to generate the label from. Defaults to [data].
     * @return The generated label.
     */
    fun getComputedLetter(keyData: KeyData = data): String {
        // TLD 使用標準大小寫（英文字母）
        if (keyData.code == KeyCode.URI_COMPONENT_TLD) {
            return when (taigikeyboard?.textInputManager?.caps) {
                true -> keyData.label.uppercase(Locale.getDefault())
                else -> keyData.label.lowercase(Locale.getDefault())
            }
        }
        // Use label if it's different from code (for multi-codepoint characters like o͘)
        // Otherwise use code for standard single characters
        val baseLabel =
            if (keyData.label.isNotEmpty() &&
                keyData.label != keyData.code.toChar().toString()
            ) {
                keyData.label
            } else {
                keyData.code.toChar().toString()
            }

        // Display override: "˙" → "·" (middle dot, more visible)
        if (baseLabel == "˙") return "·"

        // Display override: "nn" key shows nasal marker ⁿ/ᴺ in POJ mode
        // In TL mode, display as literal "nn" (falls through to normal case logic)
        if (baseLabel == "nn" && taigikeyboard?.prefs?.inputMode == "poj") {
            return if (taigikeyboard?.textInputManager?.caps == true) "\u1D3A" else "\u207F"
        }

        // 使用對照表正確轉換聲調字母（如 á → Á）
        val inputMode =
            when (taigikeyboard?.prefs?.inputMode) {
                "poj" -> InputMode.POJ
                "tl", "tps" -> InputMode.TL
                else -> InputMode.POJ
            }

        // Render-path FFI cache (R3 mitigation) — see KeyLabelCaseCache.kt.
        val caps = taigikeyboard?.textInputManager?.caps == true
        val capsLock = taigikeyboard?.textInputManager?.capsLock == true
        return KeyLabelCaseCache.getOrCompute(baseLabel, inputMode, caps, capsLock)
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
    override fun onTouchEvent(event: MotionEvent?): Boolean = false

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
                // 確保 Handler 在使用前初始化
                if (osHandler == null) {
                    osHandler = Handler(Looper.getMainLooper())
                }
                if (data.code == KeyCode.DELETE && data.type == KeyType.ENTER_EDITING) {
                    // 使用 Handler 替代 Timer，確保回調在主執行緒執行
                    val repeatDelete =
                        object : Runnable {
                            override fun run() {
                                if (isKeyPressed) {
                                    TraceContext.withTrace(TraceId.next()) {
                                        taigikeyboard?.compositionRoot?.logger?.tdebug("KeyView") {
                                            "[INPUT] fn=onFlorisTouchEvent gesture=repeat-delete code=${data.code} (${data.label})"
                                        }
                                        taigikeyboard?.textInputManager?.sendKeyPress(data)
                                    }
                                    osHandler?.postDelayed(this, 50)
                                }
                            }
                        }
                    osHandler?.postDelayed(repeatDelete, 500)
                }
                val delayMillis = keyboardView.prefs.longPressDelay
                osHandler?.postDelayed({
                    if (data.popup.isNotEmpty()) {
                        keyboardView.popupManager.extend(this)
                    }
                    if (data.code == KeyCode.SPACE) {
                        TraceContext.withTrace(TraceId.next()) {
                            taigikeyboard?.compositionRoot?.logger?.tdebug("KeyView") {
                                "[INPUT] fn=onFlorisTouchEvent gesture=long-press key=SPACE"
                            }
                            taigikeyboard?.textInputManager?.sendKeyPress(
                                KeyData(
                                    KeyCode.SHOW_INPUT_METHOD_PICKER,
                                    type = KeyType.FUNCTION,
                                ),
                            )
                        }
                        shouldBlockNextKeyCode = true
                    }
                    if (data.code == KeyCode.LANGUAGE_SWITCH) {
                        // 長按顯示輸入法選單
                        TraceContext.withTrace(TraceId.next()) {
                            taigikeyboard?.compositionRoot?.logger?.tdebug("KeyView") {
                                "[INPUT] fn=onFlorisTouchEvent gesture=long-press key=LANGUAGE_SWITCH"
                            }
                            taigikeyboard?.textInputManager?.sendKeyPress(
                                KeyData(
                                    KeyCode.SHOW_INPUT_METHOD_PICKER,
                                    type = KeyType.FUNCTION,
                                ),
                            )
                        }
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
                    if ((event.x < -0.1f * measuredWidth && parent.children.first() != this) ||
                        (event.x > 1.1f * measuredWidth && parent.children.last() != this) ||
                        event.y < -0.35f * measuredHeight ||
                        event.y > 1.35f * measuredHeight
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
                val retData = keyboardView.popupManager.getActiveKeyData(this)
                keyboardView.popupManager.hide()
                if (event.actionMasked != MotionEvent.ACTION_CANCEL && !shouldBlockNextKeyCode && retData != null) {
                    TraceContext.withTrace(TraceId.next()) {
                        taigikeyboard?.compositionRoot?.logger?.tdebug("KeyView") {
                            "[INPUT] fn=onFlorisTouchEvent gesture=ACTION_UP code=${retData.code} (${retData.label})"
                        }
                        taigikeyboard?.textInputManager?.sendKeyPress(retData)
                        performClick()
                    }
                } else {
                    if (com.siansiansu.taigikeyboard.BuildConfig.DEBUG) {
                        android.util.Log.w(
                            "KeyView",
                            "[TOUCH] UP → BLOCKED: " +
                                "code=${data.code} (${data.label}), " +
                                "cancel=${event.actionMasked == MotionEvent.ACTION_CANCEL}, " +
                                "blocked=$shouldBlockNextKeyCode, retData=${retData != null}",
                        )
                    }
                    shouldBlockNextKeyCode = false
                }
            }

            else -> {
                return false
            }
        }
        return true
    }

    /**
     * Solution base from this great StackOverflow answer which explained and helped a lot
     * for handling onMeasure():
     *  https://stackoverflow.com/a/12267248/6801193
     *  by Devunwired
     */
    override fun onMeasure(
        widthMeasureSpec: Int,
        heightMeasureSpec: Int,
    ) {
        val desiredWidth =
            when (keyboardView.computedLayout?.mode) {
                KeyboardMode.NUMERIC,
                KeyboardMode.PHONE,
                KeyboardMode.PHONE2,
                -> {
                    (keyboardView.desiredKeyWidth * 2.68f).toInt()
                }

                KeyboardMode.NUMERIC_ADVANCED -> {
                    when (data.code) {
                        44, 46 -> keyboardView.desiredKeyWidth
                        KeyCode.VIEW_SYMBOLS, 61 -> (keyboardView.desiredKeyWidth * 1.34f).toInt()
                        else -> (keyboardView.desiredKeyWidth * 1.56f).toInt()
                    }
                }

                else -> {
                    when (data.code) {
                        KeyCode.SHIFT,
                        KeyCode.VIEW_CHARACTERS,
                        KeyCode.VIEW_SYMBOLS,
                        KeyCode.VIEW_SYMBOLS2,
                        KeyCode.DELETE,
                        KeyCode.ENTER,
                        -> {
                            (keyboardView.desiredKeyWidth * 1.56f).toInt()
                        }

                        KeyCode.TRANSLATE -> {
                            val scale =
                                when (keyboardView.prefs.keyboardLayoutType) {
                                    "phahTaigi", "moe1" -> 2.0f
                                    else -> 1.5f
                                }
                            (keyboardView.desiredKeyWidth * scale).toInt()
                        }

                        KeyCode.SPACE -> {
                            when (keyboardView.computedLayout?.mode) {
                                KeyboardMode.SYMBOLS -> (keyboardView.desiredKeyWidth * 0.56f).toInt()
                                else -> keyboardView.desiredKeyWidth
                            }
                        }

                        else -> {
                            keyboardView.desiredKeyWidth
                        }
                    }
                }
            }
        val desiredHeight = keyboardView.desiredKeyHeight

        val widthMode = MeasureSpec.getMode(widthMeasureSpec)
        val widthSize = MeasureSpec.getSize(widthMeasureSpec)
        val heightMode = MeasureSpec.getMode(heightMeasureSpec)
        val heightSize = MeasureSpec.getSize(heightMeasureSpec)

        // Measure Width
        val width =
            when (widthMode) {
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
        val height =
            when (heightMode) {
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

    override fun onLayout(
        changed: Boolean,
        left: Int,
        top: Int,
        right: Int,
        bottom: Int,
    ) {
        super.onLayout(changed, left, top, right, bottom)
        updateTouchHitBox()
    }

    override fun onSizeChanged(
        w: Int,
        h: Int,
        oldw: Int,
        oldh: Int,
    ) {
        super.onSizeChanged(w, h, oldw, oldh)
        outlineProvider = KeyViewOutline(w, h, keyboardView.prefs.keyCornerRadius)
    }

    /**
     * Updates the background depending on [data].
     * 當 translate 按鍵處於 swapped 狀態時，顯示啟用的背景色
     *
     * 注意：一般按鍵的按下效果（pressed state）已由 selector 處理，
     * 此函式只處理特殊狀態（例如 translate swapped）
     *
     * 優化：只在狀態改變時調用，不在 onDraw 中重複執行
     */
    private fun updateKeyPressedBackground() {
        // 檢查是否為 translate 按鍵且處於 swapped 狀態
        // 使用 SmartbarManager 的快取值避免 DataStore 非同步讀取問題
        // A7: SmartbarManager 由 parent KeyboardView 推入；preview 模式為 null。
        val isTranslateSwapped =
            data.code == KeyCode.TRANSLATE &&
                keyboardView.smartbarManager?.getCachedIsTranslateSwapped() == true

        // 只有 translate 按鍵在 swapped 狀態時需要特殊處理
        // 其他按鍵（包括 ENTER、DELETE）的觸擊效果由 selector 自動處理
        if (isTranslateSwapped) {
            setBackgroundTintColor(this, R.attr.key_bgColorActive)
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

        if (data.type == KeyType.CHARACTER && data.code != KeyCode.SPACE ||
            data.type == KeyType.NUMERIC
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
                    // Preview mode: always show return icon
                    if (keyboardView.isPreviewMode) {
                        label = null
                        drawable = getDrawable(context, R.drawable.ic_keyboard_return)
                        drawableColor = getColorFromAttr(context, R.attr.key_enter_fgColor)
                    } else {
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
                            // 組字模式：依輸入模式顯示對應確認文字（選/soán/suán）
                            label =
                                SettingsTexts.confirmKeyLabel(
                                    keyboardView.prefs.inputMode,
                                    keyboardView.prefs.isTranslateSwapped,
                                )
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
                                drawable =
                                    getDrawable(
                                        context,
                                        when (action and EditorInfo.IME_MASK_ACTION) {
                                            EditorInfo.IME_ACTION_DONE -> R.drawable.ic_done
                                            EditorInfo.IME_ACTION_GO -> R.drawable.ic_arrow_right_alt
                                            EditorInfo.IME_ACTION_NEXT -> R.drawable.ic_arrow_right_alt
                                            EditorInfo.IME_ACTION_NONE -> R.drawable.ic_keyboard_return
                                            EditorInfo.IME_ACTION_PREVIOUS -> R.drawable.ic_arrow_right_alt
                                            EditorInfo.IME_ACTION_SEARCH -> R.drawable.ic_search
                                            EditorInfo.IME_ACTION_SEND -> R.drawable.ic_send
                                            else -> R.drawable.ic_arrow_right_alt
                                        },
                                    )
                                drawableColor = getColorFromAttr(context, R.attr.key_enter_fgColor)
                                if (action and EditorInfo.IME_FLAG_NO_ENTER_ACTION > 0) {
                                    drawable = getDrawable(context, R.drawable.ic_keyboard_return)
                                }
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
                    val isCaps = taigikeyboard?.textInputManager?.caps ?: false
                    val isCapsLock = taigikeyboard?.textInputManager?.capsLock ?: false
                    drawable =
                        getDrawable(
                            context,
                            when {
                                isCaps && isCapsLock -> {
                                    drawableColor = getColorFromAttr(context, R.attr.colorAccent)
                                    R.drawable.ic_keyboard_capslock
                                }

                                isCaps && !isCapsLock -> {
                                    drawableColor = getColorFromAttr(context, R.attr.key_fgColor)
                                    R.drawable.ic_keyboard_capslock
                                }

                                else -> {
                                    drawableColor = getColorFromAttr(context, R.attr.key_fgColor)
                                    R.drawable.ic_keyboard_arrow_up
                                }
                            },
                        )
                }

                KeyCode.SPACE -> {
                    when (keyboardView.computedLayout?.mode) {
                        KeyboardMode.NUMERIC,
                        KeyboardMode.NUMERIC_ADVANCED,
                        KeyboardMode.PHONE,
                        KeyboardMode.PHONE2,
                        -> {
                            drawable = getDrawable(context, R.drawable.ic_space_bar)
                            drawableColor = getColorFromAttr(context, R.attr.key_fgColor)
                            label = null
                        }

                        KeyboardMode.CHARACTERS -> {
                            drawable = null
                            label =
                                when (keyboardView.prefs.inputMode) {
                                    "poj" -> "POJ"
                                    "tl" -> "TL"
                                    "tps" -> "TPS"
                                    "english" -> "EN"
                                    else -> null
                                }
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
                KeyCode.VIEW_CHARACTERS,
                -> {
                    label = resources.getString(R.string.key__view_characters)
                    drawable = null
                }

                KeyCode.VIEW_NUMERIC -> {
                    label = resources.getString(R.string.key__view_numeric)
                    drawable = null
                }

                KeyCode.VIEW_NUMERIC_ADVANCED -> {
                    // 在 symbol 鍵盤中，根據 isTranslateSwapped 狀態決定顯示內容
                    // A7: SmartbarManager 由 parent KeyboardView 推入；preview 模式為 null。
                    val isTranslateSwapped =
                        keyboardView.smartbarManager?.getCachedIsTranslateSwapped() == true
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
                left =
                    when (this@KeyView) {
                        parent.children.first() -> 0
                        else -> (parent.x + x - keyMarginH).toInt()
                    }
                right =
                    when (this@KeyView) {
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
            else -> {
                if (data.variation != KeyVariation.ALL) {
                    val keyVariation = taigikeyboard?.textInputManager?.keyVariation ?: KeyVariation.NORMAL
                    val newVisibility =
                        if (data.variation == KeyVariation.NORMAL && (
                                keyVariation == KeyVariation.NORMAL ||
                                    keyVariation == KeyVariation.PASSWORD
                            )
                        ) {
                            VISIBLE
                        } else if (data.variation == keyVariation) {
                            VISIBLE
                        } else {
                            GONE
                        }
                    if (com.siansiansu.taigikeyboard.BuildConfig.DEBUG && data.label == "-" && data.code == 45) {
                        android.util.Log.d(
                            "KeyView",
                            "Hyphen key updateVisibility: variation=${data.variation}, keyVariation=$keyVariation, newVisibility=$newVisibility",
                        )
                    }
                    visibility = newVisibility
                    if (com.siansiansu.taigikeyboard.BuildConfig.DEBUG && data.label == "-" && data.code == 45) {
                        android.util.Log.d(
                            "KeyView",
                            "Hyphen key after set: visibility=$visibility, width=$width, height=$height, measuredWidth=$measuredWidth, isShown=$isShown",
                        )
                    }
                    updateTouchHitBox()
                }
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
                measuredHeight - marginV - drawablePadding,
            )
            // Apply custom key text color to icons (except ENTER which keeps its own color)
            val effectiveDrawableColor =
                if (data.code != KeyCode.ENTER) {
                    keyboardView.getColorSettings().keyTextColor ?: drawableColor
                } else {
                    drawableColor
                }
            drawable.colorFilter =
                BlendModeColorFilterCompat.createBlendModeColorFilterCompat(
                    effectiveDrawableColor,
                    BlendModeCompat.SRC_ATOP,
                )
            drawable.draw(canvas)
        }

        // Draw label
        val label = label
        if (label != null) {
            // Use shared Paint object, set properties for current key
            // Dynamically calculate text size: based on key height × ratio, adapts to different screen sizes
            val fontSizeScale = keyboardView.prefs.keyFontSizeScale
            val baseTextSize = measuredHeight * 0.58f * fontSizeScale
            sharedLabelPaint.textSize =
                when {
                    // ?123 key uses smaller font
                    data.code == KeyCode.VIEW_SYMBOLS -> {
                        baseTextSize * 0.80f
                    }

                    // Enter key in composing mode uses smaller font for confirmation text
                    data.code == KeyCode.ENTER && label.isNotEmpty() -> {
                        baseTextSize * 0.85f
                    }

                    // VIEW_NUMERIC_ADVANCED: determine font size based on display content
                    data.code == KeyCode.VIEW_NUMERIC_ADVANCED -> {
                        // If showing "、" symbol, use normal key size; otherwise use number key size
                        if (label == "、") baseTextSize else baseTextSize * 0.55f
                    }

                    // Number key and space key
                    data.code == KeyCode.VIEW_NUMERIC ||
                        data.code == KeyCode.SPACE -> {
                        baseTextSize * 0.55f
                    }

                    // MOE2 layout: only shrink 3+ char keys (tsh/chh) to fit within key width
                    data.type == KeyType.CHARACTER && keyboardView.prefs.keyboardLayoutType == "moe2" && label.length >= 3 -> {
                        baseTextSize *
                            0.75f
                    }

                    // Normal keys
                    else -> {
                        baseTextSize
                    }
                }

            // Set typeface based on user settings (cached at KeyboardView level)
            sharedLabelPaint.typeface = keyboardView.getTypeface()

            // Enter key always uses its dedicated color; other keys use custom color if set
            val customKeyTextColor = keyboardView.getColorSettings().keyTextColor
            sharedLabelPaint.color =
                if (data.code == KeyCode.ENTER) {
                    getColorFromAttr(context, R.attr.key_enter_fgColor)
                } else {
                    customKeyTextColor ?: getColorFromAttr(context, R.attr.key_fgColor)
                }

            sharedLabelPaint.alpha =
                if (keyboardView.computedLayout?.mode == KeyboardMode.CHARACTERS &&
                    data.code == KeyCode.SPACE
                ) {
                    120
                } else {
                    255
                }

            val centerX = measuredWidth / 2.0f
            val centerY = measuredHeight / 2.0f + (sharedLabelPaint.textSize - sharedLabelPaint.descent()) / 2

            // TPS layout: show main char + first popup variant stacked vertically
            // Only for TPS phonetic characters (code 0), not punctuation like comma
            val isTpsWithPopup =
                keyboardView.prefs.keyboardLayoutType == "tps" &&
                    data.type == KeyType.CHARACTER && data.code == 0 && data.popup.isNotEmpty()

            if (isTpsWithPopup) {
                // Main char: larger, at bottom
                sharedLabelPaint.textSize = baseTextSize * 0.78f
                val topY = measuredHeight * 0.28f
                val bottomY = measuredHeight * 0.80f
                // Popup variants on top (lighter)
                sharedHintPaint.color = sharedLabelPaint.color
                sharedHintPaint.alpha = 130
                sharedHintPaint.typeface = sharedLabelPaint.typeface
                if (data.popup.size >= 2) {
                    // Two callouts: top-left and top-right
                    sharedHintPaint.textSize = baseTextSize * 0.48f
                    val padding = measuredWidth * 0.12f
                    canvas.drawText(data.popup[0].label, padding, topY, sharedHintPaint.apply { textAlign = Paint.Align.LEFT })
                    canvas.drawText(
                        data.popup[1].label,
                        measuredWidth - padding,
                        topY,
                        sharedHintPaint.apply {
                            textAlign =
                                Paint.Align.RIGHT
                        },
                    )
                    sharedHintPaint.textAlign = Paint.Align.CENTER
                } else {
                    sharedHintPaint.textSize = baseTextSize * 0.52f
                    canvas.drawText(data.popup[0].label, centerX, topY, sharedHintPaint)
                }
                canvas.drawText(label, centerX, bottomY, sharedLabelPaint)
            } else if (label.contains("\n")) {
                // Even if more lines may be existing only the first 2 are shown
                val labelLines = label.split("\n")
                canvas.drawText(labelLines[0], centerX, centerY * 0.70f, sharedLabelPaint)
                canvas.drawText(labelLines[1], centerX, centerY * 1.30f, sharedLabelPaint)
            } else {
                canvas.drawText(label, centerX, centerY, sharedLabelPaint)
            }

            // TPS layout: show popup hint above punctuation keys (e.g., "。" above "，")
            if (keyboardView.prefs.keyboardLayoutType == "tps" &&
                keyboardView.computedLayout?.mode == KeyboardMode.CHARACTERS &&
                data.type == KeyType.CHARACTER && data.code != 0 && data.popup.isNotEmpty()
            ) {
                val hintLabel = data.popup[0].label
                sharedHintPaint.textSize = baseTextSize * 0.52f
                sharedHintPaint.color = sharedLabelPaint.color
                sharedHintPaint.alpha = 130
                sharedHintPaint.typeface = sharedLabelPaint.typeface
                val hintY = measuredHeight * 0.28f
                canvas.drawText(hintLabel, centerX, hintY, sharedHintPaint)
            }

            // Draw hint above keys (tone diacritics on number keys, punctuation hints on MOE1)
            if (data.type == KeyType.CHARACTER &&
                keyboardView.prefs.keyboardLayoutType != "tps"
            ) {
                val layoutType = keyboardView.prefs.keyboardLayoutType
                // Tone hints for number keys (all layouts except TPS)
                val hint =
                    toneHintForCode(data.code, keyboardView.prefs.inputMode)
                        // MOE1/MOE2 punctuation hints
                        ?: if (layoutType == "moe1" || layoutType == "moe2") moe1HintForCode(data.code) else null

                if (hint != null && hint != " ") {
                    val isMoe1TextHint = (layoutType == "moe1" || layoutType == "moe2") && moe1HintForCode(data.code) != null
                    sharedHintPaint.textSize = baseTextSize * if (isMoe1TextHint) 0.60f else 1.3f
                    sharedHintPaint.color = sharedLabelPaint.color
                    // MOE1 text hints: semi-transparent; tone diacritics: lighter
                    sharedHintPaint.alpha = if (isMoe1TextHint) 150 else 100
                    sharedHintPaint.typeface = Typeface.DEFAULT
                    // MOE1 text hints: position above main label; tone diacritics: overlap center
                    // Hyphen (-) has higher visual center than comma/period, so nudge its hint up
                    val moe1HintFactor = if (data.code == 45) 0.35f else 0.40f
                    val hintY = measuredHeight * if (isMoe1TextHint) moe1HintFactor else 0.66f
                    canvas.drawText(hint, centerX, hintY, sharedHintPaint)
                }
            }
        }
    }

    /**
     * Custom Outline Provider, needed for the [KeyView] elevation rendering.
     */
    private class KeyViewOutline(
        private val width: Int,
        private val height: Int,
        private val cornerRadiusDp: Float = -1f,
    ) : ViewOutlineProvider() {
        override fun getOutline(
            view: View?,
            outline: Outline?,
        ) {
            view ?: return
            outline ?: return
            val radius =
                if (cornerRadiusDp >= 0f) {
                    cornerRadiusDp * view.resources.displayMetrics.density
                } else {
                    view.resources.getDimension(R.dimen.key_borderRadius)
                }
            outline.setRoundRect(0, 0, width, height, radius)
        }
    }
}
