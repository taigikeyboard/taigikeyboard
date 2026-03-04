package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.text.Spannable
import android.text.SpannableStringBuilder
import android.text.style.ForegroundColorSpan
import android.text.style.RelativeSizeSpan
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.Button
import androidx.recyclerview.widget.DiffUtil
import androidx.recyclerview.widget.ListAdapter
import androidx.recyclerview.widget.RecyclerView
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.util.FontUtils
import com.siansiansu.taigikeyboard.util.getColorFromAttr

/**
 * 候選詞 RecyclerView Adapter
 *
 * 使用 ListAdapter + DiffUtil 實現高效的差異更新
 * 只更新變化的項目，避免重建所有 View
 */
class CandidateAdapter(
    private val context: Context,
    private val isTranslateSwapped: () -> Boolean,
    private val fontType: () -> String,
    private val onCandidateClick: (TaigiWord, Int) -> Unit
) : ListAdapter<TaigiWord, CandidateAdapter.CandidateViewHolder>(CandidateDiffCallback()) {

    // 快取的資源值（避免重複取得）
    private val margin: Int by lazy {
        context.resources.getDimensionPixelSize(R.dimen.smartbar_button_margin)
    }
    private val padding: Int by lazy {
        context.resources.getDimensionPixelSize(R.dimen.smartbar_button_padding)
    }
    private val subtitleColor: Int by lazy {
        getColorFromAttr(context, R.attr.smartbar_candidate_subtitle_fgColor)
    }

    // 動態計算的文字大小
    private var candidateTextSizeSp: Float = 16f

    // Candidate text size scale factor (from appearance settings)
    private var textSizeScale: Float = 1.0f

    // Custom candidate text color override (null = use theme default)
    private var customTextColor: Int? = null

    /**
     * 設定候選詞文字大小（根據 Smartbar 高度計算）
     */
    fun setTextSize(smartbarHeight: Int) {
        val candidateTextSizePx = smartbarHeight * 0.46f * textSizeScale
        val scaledDensity = context.resources.displayMetrics.density * context.resources.configuration.fontScale
        candidateTextSizeSp = candidateTextSizePx / scaledDensity
    }

    /**
     * Set candidate text size scale factor from appearance settings.
     */
    fun setTextSizeScale(scale: Float) {
        textSizeScale = scale
    }

    /**
     * Set custom candidate text color from appearance settings (null = theme default).
     */
    fun setCustomTextColor(color: Int?) {
        customTextColor = color
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): CandidateViewHolder {
        val view = LayoutInflater.from(context).inflate(R.layout.item_candidate, parent, false)
        return CandidateViewHolder(view)
    }

    override fun onBindViewHolder(holder: CandidateViewHolder, position: Int) {
        val word = getItem(position)
        holder.bind(word, position)
    }

    inner class CandidateViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
        private val button: Button = itemView.findViewById(R.id.candidate_button)

        // 快取的 Typeface（避免重複載入）
        private var cachedTypeface: android.graphics.Typeface? = null
        private var cachedFontType: String? = null

        fun bind(word: TaigiWord, position: Int) {
            val isSwapped = isTranslateSwapped()
            val currentFontType = fontType()

            // 設定按鈕樣式
            button.apply {
                // 動態設定文字大小
                textSize = candidateTextSizeSp

                // 設定背景（第 0 個位置使用不同背景）
                val isNextWordCandidate = word.id < 0
                setBackgroundResource(
                    if (position == 0 && !isNextWordCandidate) R.drawable.candidate_composing_background
                    else R.drawable.candidate_button_background
                )

                // 設定 padding 和 margin
                val reducedVerticalPadding = padding / 2
                setPadding(padding, reducedVerticalPadding, padding, reducedVerticalPadding)

                // 設定 margin
                val lp = layoutParams as? RecyclerView.LayoutParams ?: RecyclerView.LayoutParams(
                    ViewGroup.LayoutParams.WRAP_CONTENT,
                    ViewGroup.LayoutParams.MATCH_PARENT
                )
                val horizontalSpacing = margin * 5
                val verticalSpacing = margin * 6
                lp.setMargins(horizontalSpacing, verticalSpacing, horizontalSpacing, verticalSpacing)
                layoutParams = lp

                // 設定文字（使用 SpannableString 顯示主副標題）
                text = buildDisplayText(word, isSwapped)

                // 設定字體（快取以避免重複載入）
                if (cachedFontType != currentFontType) {
                    cachedTypeface = FontUtils.getTypefaceByType(currentFontType, context)
                    cachedFontType = currentFontType
                }
                typeface = cachedTypeface

                // Apply custom text color if set
                customTextColor?.let { setTextColor(it) }

                // 設定點擊事件
                setOnClickListener {
                    onCandidateClick(word, position)
                }
            }
        }

        /**
         * 建立顯示文字（主標題 + 副標題）
         */
        private fun buildDisplayText(word: TaigiWord, isSwapped: Boolean): CharSequence {
            val effectiveSubtitleColor = customTextColor ?: subtitleColor
            return when {
                // 沒有漢字：只顯示羅馬字
                word.hanzi.isNullOrEmpty() -> word.roman

                // 翻譯交換模式：漢字為主，羅馬字為副
                isSwapped -> {
                    SpannableStringBuilder().apply {
                        append(word.hanzi)
                        append(" ")
                        val subtitleStart = length
                        append(word.roman)
                        setSpan(
                            RelativeSizeSpan(0.70f),
                            subtitleStart,
                            length,
                            Spannable.SPAN_EXCLUSIVE_EXCLUSIVE
                        )
                        setSpan(
                            ForegroundColorSpan(effectiveSubtitleColor),
                            subtitleStart,
                            length,
                            Spannable.SPAN_EXCLUSIVE_EXCLUSIVE
                        )
                    }
                }

                // 預設模式：羅馬字為主，漢字為副
                else -> {
                    SpannableStringBuilder().apply {
                        append(word.roman)
                        append(" ")
                        val subtitleStart = length
                        append(word.hanzi)
                        setSpan(
                            RelativeSizeSpan(0.70f),
                            subtitleStart,
                            length,
                            Spannable.SPAN_EXCLUSIVE_EXCLUSIVE
                        )
                        setSpan(
                            ForegroundColorSpan(effectiveSubtitleColor),
                            subtitleStart,
                            length,
                            Spannable.SPAN_EXCLUSIVE_EXCLUSIVE
                        )
                    }
                }
            }
        }
    }

    /**
     * DiffUtil Callback，用於計算列表差異
     */
    class CandidateDiffCallback : DiffUtil.ItemCallback<TaigiWord>() {
        override fun areItemsTheSame(oldItem: TaigiWord, newItem: TaigiWord): Boolean {
            // 使用 id 判斷是否為同一項目
            return oldItem.id == newItem.id
        }

        override fun areContentsTheSame(oldItem: TaigiWord, newItem: TaigiWord): Boolean {
            // 比較內容是否相同
            return oldItem.roman == newItem.roman &&
                   oldItem.hanzi == newItem.hanzi &&
                   oldItem.lengthScore == newItem.lengthScore
        }
    }
}
