
package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.content.res.ColorStateList
import android.util.AttributeSet
import android.util.Log
import android.util.TypedValue
import android.view.View
import android.widget.Button
import android.widget.ImageButton
import android.widget.LinearLayout
import androidx.recyclerview.widget.RecyclerView
import com.siansiansu.taigikeyboard.BuildConfig
import com.siansiansu.taigikeyboard.R

/**
 * Smartbar 視圖
 *
 * 提供候選詞顯示、數字列、快捷動作等功能
 * 候選詞支援左右滑動，可動態顯示最多 100 個候選詞
 */
class SmartbarView : LinearLayout {

    private val smartbarManager = SmartbarManager.getInstance()

    // 候選詞相關視圖
    var candidatesContainer: LinearLayout? = null
        private set
    var candidatesRecyclerView: RecyclerView? = null
        private set

    // 展開收合相關視圖
    var expandToggleButton: ImageButton? = null
        private set
    var dividerView: View? = null
        private set

    // 其他容器
    var numberRowView: LinearLayout? = null
        private set
    var quickActionsView: LinearLayout? = null
        private set

    // 英文三欄式候選詞容器
    var englishCandidatesContainer: LinearLayout? = null
        private set
    var englishCandidate1: Button? = null
        private set
    var englishCandidate2: Button? = null
        private set
    var englishCandidate3: Button? = null
        private set

    // 模式切換按鈕
    var buttonModePoj: Button? = null
        private set
    var buttonModeTl: Button? = null
        private set
    var buttonModeEn: Button? = null
        private set

    constructor(context: Context) : this(context, null)
    constructor(context: Context, attrs: AttributeSet?) : this(context, attrs, 0)
    constructor(context: Context, attrs: AttributeSet?, defStyleAttr: Int) : super(context, attrs, defStyleAttr)

    override fun onAttachedToWindow() {
        if (BuildConfig.DEBUG) Log.i(this::class.simpleName, "onAttachedToWindow()")

        super.onAttachedToWindow()

        // 候選詞視圖
        candidatesContainer = findViewById(R.id.candidates_container)
        candidatesRecyclerView = findViewById(R.id.candidates_recycler_view)

        // 展開收合按鈕與分隔線
        expandToggleButton = findViewById(R.id.expand_toggle_button)
        dividerView = findViewById(R.id.candidate_divider)

        // 初始化展開按鈕 tint
        applyExpandButtonTint()

        // 其他視圖
        numberRowView = findViewById(R.id.number_row)
        quickActionsView = findViewById(R.id.quick_actions)

        // 英文三欄式候選詞
        englishCandidatesContainer = findViewById(R.id.english_candidates_container)
        englishCandidate1 = findViewById(R.id.english_candidate_1)
        englishCandidate2 = findViewById(R.id.english_candidate_2)
        englishCandidate3 = findViewById(R.id.english_candidate_3)

        // 模式切換按鈕
        buttonModePoj = findViewById(R.id.button_mode_poj)
        buttonModeTl = findViewById(R.id.button_mode_tl)
        buttonModeEn = findViewById(R.id.button_mode_en)

        smartbarManager.registerSmartbarView(this)
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
                if (isExpanded) R.drawable.ic_keyboard_arrow_up
                else R.drawable.ic_keyboard_arrow_down
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
     * 重置候選詞列滑動位置至起點
     * 確保新候選詞總是從起點開始顯示，改善使用者體驗
     */
    fun resetCandidateScrollPosition() {
        candidatesRecyclerView?.scrollToPosition(0)
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
