// 中文: Smartbar 平台 View 殼 — 候選 strip 內含為 ComposeView(LazyRow + English Row),
// 中文: 外層仍為 LinearLayout 以維持 IME inflate / Activity 預覽穩定性。

package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.content.res.ColorStateList
import android.graphics.Color
import android.util.AttributeSet
import android.view.View
import android.widget.Button
import android.widget.ImageButton
import android.widget.LinearLayout
import androidx.compose.runtime.getValue
import androidx.compose.ui.platform.ComposeView
import androidx.compose.ui.platform.ViewCompositionStrategy
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.core.CompositionRoot
import com.siansiansu.taigikeyboard.ime.core.KeyboardColorSettings
import com.siansiansu.taigikeyboard.ime.core.TaigiKeyboard
import com.siansiansu.taigikeyboard.ime.theme.getColorFromAttr
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

    // Resolved chrome foreground tint (light-only theme's candidateTextColor), or null
    // for the adaptive default theme (chrome then follows the night-aware attr). Set by
    // applyThemeSurface; read by setExpandButtonState/applyExpandButtonTint so an expand
    // toggle re-tint doesn't revert to the night attr over a light gradient.
    private var chromeForegroundTint: Int? = null

    constructor(context: Context) : this(context, null)
    constructor(context: Context, attrs: AttributeSet?) : this(context, attrs, 0)
    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(context, attrs, defStyleAttr)

    override fun onAttachedToWindow() {
        CompositionRoot.shared(context).logger.i("SmartbarView", "onAttachedToWindow()")

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
     * Applies the resolved theme to the smartbar chrome. A gradient theme paints
     * its background on the common parent (`text_input_content`), so the smartbar
     * root + toolbar-toggle + expand-toggle + candidate container all go transparent
     * to let the gradient show through. A flat/legacy theme restores the attr-backed
     * `?smartbar_bgColor` chrome and applies the theme's candidate background (null
     * candidate background -> cleared, matching the XML default).
     */
    fun applyThemeSurface(colors: KeyboardColorSettings) {
        val gradient = colors.hasBackgroundGradient
        val chromeBg = if (gradient) Color.TRANSPARENT else getColorFromAttr(context, R.attr.smartbar_bgColor)
        setBackgroundColor(chromeBg)
        toolbarToggleButton?.setBackgroundColor(chromeBg)
        expandToggleButton?.setBackgroundColor(chromeBg)

        val candidateBg = if (gradient) Color.TRANSPARENT else colors.candidateBackgroundColor
        if (candidateBg != null) {
            candidatesContainer?.setBackgroundColor(candidateBg)
        } else {
            candidatesContainer?.background = null
        }

        applyChromeForeground(colors.candidateTextColor)
    }

    /**
     * Tints the View-layer smartbar chrome (toolbar toggle, expand toggle, toolbar icon
     * buttons, mode text buttons) from the theme's `candidateTextColor` role. A light-only
     * theme keeps a fixed dark color so the glyphs stay readable over its light gradient in
     * system dark mode; a null role (adaptive default theme) restores the night-aware
     * `?smartbar_fgColor` / `?smartbar_button_fgColor` attr so the default path is unchanged.
     */
    private fun applyChromeForeground(candidateTextColor: Int?) {
        chromeForegroundTint = candidateTextColor
        val iconFg = candidateTextColor ?: getColorFromAttr(context, R.attr.smartbar_fgColor)
        val buttonFg = candidateTextColor ?: getColorFromAttr(context, R.attr.smartbar_button_fgColor)

        val iconTint = ColorStateList.valueOf(iconFg)
        toolbarToggleButton?.imageTintList = iconTint
        expandToggleButton?.imageTintList = iconTint

        val buttonTint = ColorStateList.valueOf(buttonFg)
        for (id in TOOLBAR_ICON_BUTTON_IDS) {
            findViewById<ImageButton>(id)?.imageTintList = buttonTint
        }
        val modeTextColors = modeButtonTextColors(buttonFg)
        for (id in TOOLBAR_MODE_BUTTON_IDS) {
            findViewById<Button>(id)?.setTextColor(modeTextColors)
        }
    }

    // Mirrors @color/mode_button_text: selected mode keeps white text (over the blue
    // accent fill); the unselected default takes the resolved chrome foreground.
    private fun modeButtonTextColors(defaultColor: Int): ColorStateList =
        ColorStateList(
            arrayOf(intArrayOf(android.R.attr.state_selected), intArrayOf()),
            intArrayOf(Color.WHITE, defaultColor),
        )

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
            // 根據鍵盤主題動態設定圖示顏色 (light-only theme role wins over the night attr)
            imageTintList = ColorStateList.valueOf(resolvedChromeIconTint())
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
        expandToggleButton?.imageTintList = ColorStateList.valueOf(resolvedChromeIconTint())
    }

    // Chrome icon tint: the active light-only theme's role, else the night-aware attr.
    private fun resolvedChromeIconTint(): Int =
        chromeForegroundTint ?: getColorFromAttr(context, R.attr.smartbar_fgColor)

    companion object {
        private val TOOLBAR_ICON_BUTTON_IDS =
            intArrayOf(
                R.id.toolbar_symbol_button,
                R.id.toolbar_layout_button,
                R.id.toolbar_globe_button,
                R.id.toolbar_dismiss_button,
                R.id.toolbar_settings_button,
            )
        private val TOOLBAR_MODE_BUTTON_IDS =
            intArrayOf(
                R.id.toolbar_mode_poj,
                R.id.toolbar_mode_tl,
                R.id.toolbar_mode_en,
                R.id.toolbar_mode_tps,
            )
    }
}
