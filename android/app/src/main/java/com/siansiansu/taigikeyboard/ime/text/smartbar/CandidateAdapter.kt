package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.graphics.drawable.InsetDrawable
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.TextView
import androidx.core.content.ContextCompat
import androidx.recyclerview.widget.DiffUtil
import androidx.recyclerview.widget.ListAdapter
import androidx.recyclerview.widget.RecyclerView
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.engine.RustEngineBridge
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
    private val layoutType: () -> String = { "" },
    private val orMapsToER: () -> Boolean = { false },
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
    private val primaryColor: Int by lazy {
        getColorFromAttr(context, R.attr.smartbar_candidate_fgColor)
    }

    // 動態計算的文字大小
    private var titleTextSizeSp: Float = 16f
    private var subtitleTextSizeSp: Float = 11f

    // Candidate text size scale factor (from appearance settings)
    private var textSizeScale: Float = 1.0f

    // Custom candidate text color override (null = use theme default)
    private var customTextColor: Int? = null

    /**
     * 設定候選詞文字大小（根據 Smartbar 高度計算）
     */
    fun setTextSize(smartbarHeight: Int) {
        val density = context.resources.displayMetrics.density
        val fontScale = context.resources.configuration.fontScale
        val scaledDensity = density * fontScale

        // Available height = smartbar minus vertical padding and margins
        // Padding: (padding/3) top + (padding/3) bottom ≈ padding*2/3
        // Margin: margin*6 top + margin*6 bottom = margin*12
        val verticalPaddingPx = padding * 2 / 3
        val verticalMarginPx = margin * 4
        val subtitleGapPx = 2 * density  // ~2dp gap between title and subtitle
        val availablePx = (smartbarHeight - verticalPaddingPx - verticalMarginPx - subtitleGapPx)
            .coerceAtLeast(20f * density)

        // Line height factor: with includeFontPadding=false, actual rendered
        // height is ~1.15x font size (vs ~1.3x with default font padding)
        val lineHeightFactor = 1.15f

        // Partition: title 58%, subtitle 42%, then shrink by line height factor
        val titlePx = availablePx * 0.58f * textSizeScale / lineHeightFactor
        val subtitlePx = availablePx * 0.42f * textSizeScale / lineHeightFactor

        titleTextSizeSp = (titlePx / scaledDensity).coerceIn(10f, 21f)
        subtitleTextSizeSp = (subtitlePx / scaledDensity).coerceIn(8f, 16f)
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
        private val container: LinearLayout = itemView.findViewById(R.id.candidate_container)
        private val titleView: TextView = itemView.findViewById(R.id.candidate_title)
        private val subtitleView: TextView = itemView.findViewById(R.id.candidate_subtitle)

        // 快取的 Typeface（避免重複載入）
        private var cachedTypeface: android.graphics.Typeface? = null
        private var cachedFontType: String? = null

        fun bind(word: TaigiWord, position: Int) {
            val isSwapped = isTranslateSwapped()
            val currentFontType = fontType()

            // 設定字體（快取以避免重複載入）
            if (cachedFontType != currentFontType) {
                cachedTypeface = FontUtils.getTypefaceByType(currentFontType, context)
                cachedFontType = currentFontType
            }

            // 設定背景（composing: key_bgColor shrunk to wrap text, others: transparent — match iOS）
            val isNextWordCandidate = word.id < 0
            if (position == 0 && !isNextWordCandidate) {
                val bg = ContextCompat.getDrawable(context, R.drawable.candidate_composing_background)
                val verticalInset = padding / 2   // shrink height to wrap text
                container.background = InsetDrawable(bg, 0, verticalInset, 0, verticalInset)
            } else {
                container.setBackgroundResource(R.drawable.candidate_button_background)
            }

            // 設定 padding 和 margin
            val reducedVerticalPadding = padding / 3
            container.setPadding(padding, reducedVerticalPadding, padding, reducedVerticalPadding)

            val lp = container.layoutParams as? RecyclerView.LayoutParams ?: RecyclerView.LayoutParams(
                ViewGroup.LayoutParams.WRAP_CONTENT,
                ViewGroup.LayoutParams.MATCH_PARENT
            )
            val horizontalSpacing = margin * 3
            val verticalSpacing = margin * 2
            lp.setMargins(horizontalSpacing, verticalSpacing, horizontalSpacing, verticalSpacing)
            container.layoutParams = lp

            // Determine title and subtitle content
            val titleText: String
            val subtitleText: String?
            val isTPSLayout = layoutType() == "tps"
            val displayRoman = if (isTPSLayout) {
                RustEngineBridge.tlDisplayToTps(word.roman, orMapsToER())
            } else {
                word.roman
            }

            when {
                word.hanzi.isNullOrEmpty() -> {
                    titleText = displayRoman
                    subtitleText = null
                }
                isTPSLayout -> {
                    // TPS mode: always show hanzi only
                    titleText = word.hanzi
                    subtitleText = null
                }
                isSwapped -> {
                    titleText = word.hanzi
                    subtitleText = displayRoman
                }
                else -> {
                    titleText = displayRoman
                    subtitleText = word.hanzi
                }
            }

            // Title
            titleView.apply {
                text = titleText
                textSize = titleTextSizeSp
                typeface = cachedTypeface
                setTextColor(customTextColor ?: primaryColor)
            }

            // Subtitle
            if (subtitleText != null && subtitleText != titleText) {
                subtitleView.apply {
                    visibility = View.VISIBLE
                    text = subtitleText
                    textSize = subtitleTextSizeSp
                    typeface = cachedTypeface
                    setTextColor(customTextColor ?: subtitleColor)
                    (layoutParams as? LinearLayout.LayoutParams)?.let {
                        it.topMargin = margin * 2
                        layoutParams = it
                    }
                }
            } else {
                subtitleView.apply {
                    visibility = View.GONE
                    (layoutParams as? LinearLayout.LayoutParams)?.let {
                        it.topMargin = 0
                        layoutParams = it
                    }
                }
            }

            // 設定點擊事件
            container.setOnClickListener {
                onCandidateClick(word, position)
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
