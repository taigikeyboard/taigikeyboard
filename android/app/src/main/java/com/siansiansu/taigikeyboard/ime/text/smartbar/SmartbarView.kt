
package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.content.res.ColorStateList
import android.util.AttributeSet
import android.util.Log
import android.util.TypedValue
import android.view.View
import android.widget.ImageButton
import android.widget.LinearLayout
import androidx.compose.runtime.getValue
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ui.theme.TaigiKeyboardTheme

/**
 * Smartbar 視圖：候選詞、英文三欄、數字列、Toolbar 容器。
 * 候選 strip 由 Compose 渲染（Taigi LazyRow + English Row）。
 */
class SmartbarView : LinearLayout {
    // A7: `SmartbarView` is only inflated inside the IME input view tree, so
    // `context` is always the IME service. Previewed screens do not use it.
    private val smartbarManager: SmartbarManager
        get() = (context as TaigiKeyboard).smartbarManager

    // 候選詞容器（ToolbarManager visibility-swap target — keep）
    var candidatesContainer: LinearLayout? = null
        private set
    var candidatesComposeView: ComposeView? = null
        private set

    // 展開收合相關視圖
    var expandToggleButton: ImageButton? = null
        private set
    var dividerView: View? = null
        private set

    // 其他容器
    var numberRowView: LinearLayout? = null
        private set

    // 英文三欄式候選詞容器（ToolbarManager visibility-swap target — keep）
    var englishCandidatesContainer: LinearLayout? = null
        private set
    var englishCandidatesComposeView: ComposeView? = null
        private set

    // Toolbar views
    var toolbarContainer: LinearLayout? = null
        private set
    var toolbarToggleButton: ImageButton? = null
        private set
    var toolbarGlobeButton: ImageButton? = null
        private set

    constructor(context: Context) : this(context, null)
    constructor(context: Context, attrs: AttributeSet?) : this(context, attrs, 0)
    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(context, attrs, defStyleAttr)

    override fun onAttachedToWindow() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onAttachedToWindow()")

        super.onAttachedToWindow()

        // 候選詞容器 + Compose 子視圖
        candidatesContainer = findViewById(R.id.candidates_container)
        candidatesComposeView = findViewById(R.id.candidates_compose)

        // 展開收合按鈕與分隔線
        expandToggleButton = findViewById(R.id.expand_toggle_button)
        dividerView = findViewById(R.id.candidate_divider)

        // 初始化展開按鈕 tint
        applyExpandButtonTint()

        // 其他視圖
        numberRowView = findViewById(R.id.number_row)

        // 英文三欄式候選詞容器 + Compose 子視圖
        englishCandidatesContainer = findViewById(R.id.english_candidates_container)
        englishCandidatesComposeView = findViewById(R.id.english_candidates_compose)

        // Toolbar views
        toolbarContainer = findViewById(R.id.toolbar_container)
        toolbarToggleButton = findViewById(R.id.toolbar_toggle_button)
        toolbarGlobeButton = findViewById(R.id.toolbar_globe_button)

        installCandidateComposeContent()

        smartbarManager.registerSmartbarView(this)
    }

    private fun installCandidateComposeContent() {
        // Disposal strategy matches Phase A root host (TaigiKeyboard.kt:271):
        // configChange detaches the input view while the IME service Lifecycle
        // stays alive, so DisposeOnViewTreeLifecycleDestroyed would NOT dispose
        // stale compositions. DisposeOnDetachedFromWindow is the correct
        // boundary for hot-path candidate rendering.
        candidatesComposeView?.apply {
            setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnDetachedFromWindow)
            setContent {
                TaigiKeyboardTheme {
                    val state by smartbarManager.candidateStripState
                        .collectAsStateWithLifecycle()
                    TaigiCandidateStrip(
                        state = state,
                        onCandidateClick = { word, index ->
                            smartbarManager.onTaigiCandidateClicked(word, index)
                        },
                    )
                }
            }
        }
        englishCandidatesComposeView?.apply {
            setViewCompositionStrategy(ViewCompositionStrategy.DisposeOnDetachedFromWindow)
            setContent {
                TaigiKeyboardTheme {
                    val state by smartbarManager.candidateStripState
                        .collectAsStateWithLifecycle()
                    EnglishCandidateStrip(
                        state = state,
                        onEnglishCandidateClick = { index ->
                            smartbarManager.onEnglishCandidateClicked(index)
                        },
                    )
                }
            }
        }
    }

    /**
     * Apply custom candidate background color from appearance settings.
     */
    fun applyCustomBackgroundColor(color: Int?) {
        if (color != null) {
            candidatesContainer?.setBackgroundColor(color)
        }
    }

    /**
     * 設定展開按鈕可見性
     * 只在有候選詞時顯示
     */
    fun setExpandButtonVisible(visible: Boolean) {
        expandToggleButton?.visibility = if (visible) View.VISIBLE else View.GONE
        dividerView?.visibility = if (visible) View.VISIBLE else View.GONE
    }

    /**
     * 更新展開按鈕圖示狀態
     * @param isExpanded true 顯示向上箭頭，false 顯示向下箭頭
     */
    fun setExpandButtonState(isExpanded: Boolean) {
        expandToggleButton?.apply {
            setImageResource(
                if (isExpanded) {
                    R.drawable.ic_keyboard_arrow_up
                } else {
                    R.drawable.ic_keyboard_arrow_down
                },
            )
            // 根據鍵盤主題動態設定圖示顏色
            val typedValue = TypedValue()
            context.theme.resolveAttribute(R.attr.smartbar_fgColor, typedValue, true)
            imageTintList = ColorStateList.valueOf(typedValue.data)
        }
    }

    /**
     * Multiplies the default smartbar height with the given [factor] and sets it.
     */
    fun setHeightFactor(factor: Float) {
        val baseSize = resources.getDimension(R.dimen.smartbar_height)
        val size = (baseSize * factor).toInt()
        layoutParams?.height = size
    }

    /**
     * 根據鍵盤主題設定展開按鈕的 tint
     */
    private fun applyExpandButtonTint() {
        expandToggleButton?.apply {
            val typedValue = TypedValue()
            context.theme.resolveAttribute(R.attr.smartbar_fgColor, typedValue, true)
            imageTintList = ColorStateList.valueOf(typedValue.data)
        }
    }
}
