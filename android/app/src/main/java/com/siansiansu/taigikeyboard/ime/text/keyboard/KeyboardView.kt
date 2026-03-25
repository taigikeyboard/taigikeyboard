
package com.siansiansu.taigikeyboard.ime.text.keyboard

import android.annotation.SuppressLint
import android.content.Context
import android.content.res.Configuration
import android.graphics.Canvas
import android.graphics.drawable.ColorDrawable
import android.util.AttributeSet
import android.util.Log
import android.view.HapticFeedbackConstants
import android.view.MotionEvent
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.LinearLayout
import androidx.core.view.children
import com.google.android.flexbox.FlexboxLayout
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.core.PrefHelper
import com.siansiansu.taigikeyboard.ime.popup.KeyPopupManager
import com.siansiansu.taigikeyboard.ime.text.key.KeyView
import com.siansiansu.taigikeyboard.ime.text.layout.ComputedLayoutData
import com.siansiansu.taigikeyboard.util.getColorFromAttr

class KeyboardView : LinearLayout {
    companion object {
        private const val TAG = "KeyboardView"
    }

    private var activeKeyView: KeyView? = null
    private var activePointerId: Int? = null
    private var activeX: Float = 0.0f
    private var activeY: Float = 0.0f

    private var colorDrawable: ColorDrawable
    var computedLayout: ComputedLayoutData? = null
        set(v) {
            if (BuildConfig.DEBUG) Log.d(TAG, "[LAYOUT] computedLayout setter called: name=${v?.name}, mode=${v?.mode}")
            field = v
            buildLayout()
        }
    var desiredKeyWidth: Int = resources.getDimension(R.dimen.key_width).toInt()
    var desiredKeyHeight: Int = resources.getDimension(R.dimen.key_height).toInt()
    var taigikeyboard: TaigiKeyboard? = null
    var isPreviewMode: Boolean = false
    var popupManager = KeyPopupManager<KeyboardView, KeyView>(this)
    lateinit var prefs: PrefHelper

    // Cached parsed color settings — avoids JSON parsing in onDraw/KeyView.onDraw
    private var cachedColorSettingsJson: String = ""
    var cachedColorSettings: com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings =
        com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings()
        private set

    /** Returns the cached KeyboardColorSettings, re-parsing only when the JSON string changes. */
    fun getColorSettings(): com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings {
        val json = prefs.colorSettings
        if (json != cachedColorSettingsJson) {
            cachedColorSettingsJson = json
            cachedColorSettings = com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings.fromJson(json)
        }
        return cachedColorSettings
    }

    // Cached typeface — avoids FontUtils.getTypefaceByType() call on every onDraw
    private var cachedFontType: String = ""
    var cachedTypeface: android.graphics.Typeface = android.graphics.Typeface.DEFAULT
        private set

    /** Returns the cached Typeface, re-resolving only when fontType changes. */
    fun getTypeface(): android.graphics.Typeface {
        val fontType = prefs.fontType
        if (fontType != cachedFontType) {
            cachedFontType = fontType
            cachedTypeface = com.siansiansu.taigikeyboard.util.FontUtils.getTypefaceByType(fontType, context)
        }
        return cachedTypeface
    }

    constructor(context: Context) : this(context, null)
    constructor(context: Context, attrs: AttributeSet?) : this(context, attrs, 0)
    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(context, attrs, defStyleAttr) {
        colorDrawable = ColorDrawable(getColorFromAttr(context, R.attr.keyboard_bgColor))
        background = colorDrawable
        orientation = VERTICAL
        layoutParams = layoutParams ?: FrameLayout.LayoutParams(
            FrameLayout.LayoutParams.MATCH_PARENT,
            FrameLayout.LayoutParams.WRAP_CONTENT
        )
    }

    /**
     * Builds the UI layout based on the [computedLayout].
     */
    private fun buildLayout() {
        destroyLayout()
        val computedLayout = computedLayout ?: return
        if (BuildConfig.DEBUG && computedLayout.name.contains("phah_taigi")) {
            Log.d(TAG, "[LAYOUT] Building phahTaigi layout: ${computedLayout.name}")
        }
        for ((rowIndex, row) in computedLayout.arrangement.withIndex()) {
            val rowView = KeyboardRowView(context)
            for ((keyIndex, key) in row.withIndex()) {
                val keyView = KeyView(this, key)
                keyView.taigikeyboard = taigikeyboard
                rowView.addView(keyView)
                if (BuildConfig.DEBUG && computedLayout.name.contains("phah_taigi") && rowIndex == 2 && keyIndex >= 8) {
                    Log.d(TAG, "[LAYOUT]     Row $rowIndex Key $keyIndex: label='${key.label}' code=${key.code} type=${key.type} variation=${key.variation}")
                }
            }
            if (BuildConfig.DEBUG && computedLayout.name.contains("phah_taigi")) {
                Log.d(TAG, "[LAYOUT]   Row $rowIndex: added ${row.size} KeyViews to rowView (childCount=${rowView.childCount})")
                if (rowIndex == 2) {
                    post {
                        Log.d(TAG, "[LAYOUT]   Row $rowIndex after layout: rowView width=${rowView.width}, childCount=${rowView.childCount}")
                        for (i in 0 until rowView.childCount) {
                            val child = rowView.getChildAt(i)
                            if (child is KeyView && child.data.label == "-") {
                                Log.d(TAG, "[LAYOUT]     Hyphen KeyView: visibility=${child.visibility}, width=${child.width}, left=${child.left}, right=${child.right}, isShown=${child.isShown}")
                            }
                        }
                    }
                }
            }
            addView(rowView)
        }
    }

    /**
     * Removes all keys.
     */
    private fun destroyLayout() {
        removeAllViews()
    }

    /**
     * Dismisses all shown key popups when keyboard is detached from window.
     */
    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        popupManager.dismissAllPopups()
        activeKeyView = null
        activePointerId = null
    }

    /**
     * Catch all events which are designated for child views.
     */
    override fun onInterceptTouchEvent(event: MotionEvent?): Boolean {
        return true
    }

    /**
     * This is the main logic for choosing which [KeyView] is the current active one.
     */
    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(event: MotionEvent?): Boolean {
        event ?: return false
        if (isPreviewMode) {
            return onPreviewTouchEvent(event)
        }
        val eventFloris = MotionEvent.obtainNoHistory(event)
        val pointerIndex = event.actionIndex
        var pointerId = event.getPointerId(pointerIndex)
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN,
            MotionEvent.ACTION_POINTER_DOWN -> {
                if (activePointerId == null) {
                    activePointerId = pointerId
                    activeX = event.getX(pointerIndex)
                    activeY = event.getY(pointerIndex)
                    searchForActiveKeyView()
                    if (BuildConfig.DEBUG) {
                        Log.d(TAG, "[TOUCH] DOWN → key=${activeKeyView?.data?.code} " +
                            "(${activeKeyView?.data?.label})")
                    }
                    sendFlorisTouchEvent(eventFloris, MotionEvent.ACTION_DOWN)
                } else if (activePointerId != pointerId) {
                    // New pointer arrived. Send ACTION_UP to current active view and move on
                    sendFlorisTouchEvent(eventFloris, MotionEvent.ACTION_UP)
                    activePointerId = pointerId
                    activeX = event.getX(pointerIndex)
                    activeY = event.getY(pointerIndex)
                    searchForActiveKeyView()
                    sendFlorisTouchEvent(eventFloris, MotionEvent.ACTION_DOWN)
                } else {
                    // Same pointer ID with stale state — previous UP/CANCEL was missed.
                    // Recover by treating as a fresh touch.
                    if (BuildConfig.DEBUG) {
                        Log.w(TAG, "[TOUCH] ACTION_DOWN with stale pointerId=$pointerId, recovering")
                    }
                    activeKeyView = null
                    activePointerId = pointerId
                    activeX = event.getX(pointerIndex)
                    activeY = event.getY(pointerIndex)
                    searchForActiveKeyView()
                    sendFlorisTouchEvent(eventFloris, MotionEvent.ACTION_DOWN)
                }
            }
            MotionEvent.ACTION_MOVE -> {
                for (index in 0 until event.pointerCount) {
                    pointerId = event.getPointerId(index)
                    if (activePointerId == pointerId) {
                        activeX = event.getX(index)
                        activeY = event.getY(index)
                        if (activeKeyView == null) {
                            searchForActiveKeyView()
                            sendFlorisTouchEvent(eventFloris, MotionEvent.ACTION_DOWN)
                        } else {
                            sendFlorisTouchEvent(eventFloris, MotionEvent.ACTION_MOVE)
                        }
                    }
                }
            }
            MotionEvent.ACTION_UP,
            MotionEvent.ACTION_CANCEL,
            MotionEvent.ACTION_POINTER_UP -> {
                if (activePointerId == pointerId) {
                    sendFlorisTouchEvent(eventFloris, MotionEvent.ACTION_UP)
                    activeKeyView = null
                    activePointerId = null
                } else if (BuildConfig.DEBUG) {
                    Log.d(TAG, "[TOUCH] ACTION_UP ignored: " +
                        "activePointerId=$activePointerId, eventPointerId=$pointerId")
                }
            }
            else -> return false
        }
        eventFloris.recycle()
        return true
    }

    /**
     * Sends a touch [event] to [activeKeyView] with action set to [actionParam]. Normalizes passed
     * actions (ACTION_POINTER_* will be converted to ACTION_*). Translates the absolute coords of
     * a passed [event] to relative ones so the [activeKeyView] can work with it.
     *
     * @param event The event to pass to [activeKeyView].
     * @param actionParam The action to set the [event] to.
     */
    private fun sendFlorisTouchEvent(event: MotionEvent, actionParam: Int) {
        val keyView = activeKeyView ?: run {
            if (BuildConfig.DEBUG) {
                Log.w(TAG, "[TOUCH] sendFlorisTouchEvent skipped: " +
                    "activeKeyView is null, action=$actionParam")
            }
            return
        }
        val keyViewParent = keyView.parent as ViewGroup
        keyView.onFlorisTouchEvent(event.apply {
            action = when (actionParam) {
                MotionEvent.ACTION_POINTER_DOWN -> MotionEvent.ACTION_DOWN
                MotionEvent.ACTION_POINTER_UP -> MotionEvent.ACTION_UP
                else -> actionParam
            }
            setLocation(
                activeX - keyViewParent.x - keyView.x,
                activeY - keyViewParent.y - keyView.y
            )
        })
    }

    /**
     * Searches for an active key view at [activeX]/[activeY].
     */
    private fun searchForActiveKeyView() {
        loop@ for (row in children) {
            if (row is FlexboxLayout) {
                for (keyView in row.children) {
                    if (keyView is KeyView) {
                        if (keyView.touchHitBox.contains(activeX.toInt(), activeY.toInt())) {
                            activeKeyView = keyView
                            break@loop
                        }
                    }
                }
            }
        }
    }

    /**
     * Simplified touch handler for preview mode: visual pressed state + haptic feedback only.
     * No text input, no popups, no long press, no repeat.
     */
    private fun onPreviewTouchEvent(event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                activeX = event.x
                activeY = event.y
                searchForActiveKeyView()
                activeKeyView?.let {
                    it.isPressed = true
                    performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
                }
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                activeKeyView?.isPressed = false
                activeKeyView = null
            }
        }
        return true
    }

    /**
     * Invalidates the current [activeKeyView] and sends a [MotionEvent.ACTION_CANCEL] to indicate
     * the loss of focus.
     */
    fun dismissActiveKeyViewReference() {
        if (BuildConfig.DEBUG) {
            Log.d(TAG, "[TOUCH] dismissActiveKeyViewReference: " +
                "keyCode=${activeKeyView?.data?.code}, pointerId=$activePointerId")
        }
        val cancelEvent = MotionEvent.obtain(
            0, 0, MotionEvent.ACTION_CANCEL, 0.0f, 0.0f, 0
        )
        activeKeyView?.onFlorisTouchEvent(cancelEvent)
        cancelEvent.recycle()
        activeKeyView = null
        activePointerId = null
    }

    /**
     * The desired key heights/widths are being calculated here.
     */
    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val widthSize = MeasureSpec.getSize(widthMeasureSpec)

        val keyMarginH = resources.getDimension((R.dimen.key_marginH)).toInt()
        desiredKeyWidth = (widthSize / 10) - (2 * keyMarginH)

        val factor = prefs.heightFactor
        val keyHeightFactor = when (resources.configuration.orientation) {
            Configuration.ORIENTATION_LANDSCAPE -> 0.85f
            else -> 1.0f
        } * when (factor) {
            "extra_short" -> 0.85f
            "short" -> 0.90f
            "mid_short" -> 0.95f
            "normal" -> 1.00f
            "mid_tall" -> 1.05f
            "tall" -> 1.10f
            "extra_tall" -> 1.15f
            else -> 1.00f
        } * prefs.keyHeightScale
        desiredKeyHeight = (resources.getDimension(R.dimen.key_height) * keyHeightFactor).toInt()
        taigikeyboard?.textInputManager?.smartbarManager?.smartbarView?.setHeightFactor(keyHeightFactor)

        super.onMeasure(widthMeasureSpec, heightMeasureSpec)
    }

    /**
     * Queues a layout request for all keys.
     */
    fun requestLayoutAllKeys() {
        for (row in children) {
            if (row is FlexboxLayout) {
                for (keyView in row.children) {
                    if (keyView is KeyView) {
                        keyView.requestLayout()
                    }
                }
            }
        }
    }

    /**
     * Queues a redraw for all keys.
     */
    fun invalidateAllKeys() {
        for (row in children) {
            if (row is FlexboxLayout) {
                for (keyView in row.children) {
                    if (keyView is KeyView) {
                        keyView.invalidate()
                    }
                }
            }
        }
    }

    /**
     * 只重繪符合指定 keyCode 的按鍵
     * 用於避免不必要的全鍵盤刷新
     */
    fun invalidateKeysByCode(vararg keyCodes: Int) {
        for (row in children) {
            if (row is FlexboxLayout) {
                for (keyView in row.children) {
                    if (keyView is KeyView && keyView.data.code in keyCodes) {
                        keyView.invalidate()
                    }
                }
            }
        }
    }

    /**
     * 重繪字母鍵（CHARACTER 類型）和 Shift 鍵
     * 用於 Shift 狀態改變時的高效刷新
     */
    fun invalidateCharacterKeys() {
        for (row in children) {
            if (row is FlexboxLayout) {
                for (keyView in row.children) {
                    if (keyView is KeyView) {
                        val isCharKey = keyView.data.type == com.siansiansu.taigikeyboard.ime.text.key.KeyType.CHARACTER
                        val isShiftKey = keyView.data.code == com.siansiansu.taigikeyboard.ime.text.key.KeyCode.SHIFT
                        if (isCharKey || isShiftKey) {
                            keyView.invalidate()
                        }
                    }
                }
            }
        }
    }

    /**
     * Syncs the current key variation with all keys and updates their visibility.
     */
    fun updateVisibility() {
        for (row in children) {
            if (row is FlexboxLayout) {
                for (keyView in row.children) {
                    if (keyView is KeyView) {
                        keyView.updateVisibility()
                    }
                }
            }
        }
    }

    override fun onDraw(canvas: Canvas) {
        super.onDraw(canvas)

        val customBgColor = getColorSettings().backgroundColor
        colorDrawable.color = customBgColor ?: getColorFromAttr(context, R.attr.keyboard_bgColor)
    }
}
