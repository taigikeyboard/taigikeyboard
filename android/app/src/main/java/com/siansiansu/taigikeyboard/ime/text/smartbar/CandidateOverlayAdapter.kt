package com.siansiansu.taigikeyboard.ime.text.smartbar

import android.content.Context
import android.graphics.Typeface
import android.view.LayoutInflater
import android.view.View
import android.view.ViewGroup
import android.widget.LinearLayout
import android.widget.TextView
import androidx.recyclerview.widget.DiffUtil
import androidx.recyclerview.widget.ListAdapter
import androidx.recyclerview.widget.RecyclerView
import com.siansiansu.taigikeyboard.R
import com.siansiansu.taigikeyboard.ime.dictionary.TPSConverter
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.util.FontUtils

/**
 * RecyclerView adapter for candidate overlay grid.
 *
 * Each item represents one row of candidates (2-4 cells).
 * Cells are pre-inflated in onCreateViewHolder and shown/hidden on bind,
 * avoiding per-bind inflation.
 */
class CandidateOverlayAdapter(
    private val context: Context,
    private val isTranslateSwapped: () -> Boolean,
    private val fontType: () -> String,
    private val layoutType: () -> String = { "" },
    private val orMapsToER: () -> Boolean = { false },
    private val isClickEnabled: () -> Boolean,
    private val onCellClick: (TaigiWord, Int) -> Unit
) : ListAdapter<CandidateOverlayAdapter.CandidateRow, CandidateOverlayAdapter.RowViewHolder>(RowDiffCallback()) {

    companion object {
        // Pre-inflated pool size: covers most phones (44dp min cell + 1dp spacing).
        // If a row has more items, bind() inflates additional cells on demand.
        private const val PRE_INFLATED_CELLS = 8
    }

    // Cached resources
    private val spacing: Int = context.resources.getDimensionPixelSize(R.dimen.smartbar_button_margin)
    private val gridCellInflater: LayoutInflater = LayoutInflater.from(context)

    // Cached typeface (invalidated when fontType changes)
    private var cachedTypeface: Typeface? = null
    private var cachedFontType: String? = null

    private fun getTypeface(): Typeface {
        val currentFontType = fontType()
        if (cachedFontType != currentFontType) {
            cachedTypeface = FontUtils.getTypefaceByType(currentFontType, context)
            cachedFontType = currentFontType
        }
        return cachedTypeface ?: Typeface.DEFAULT
    }

    override fun onCreateViewHolder(parent: ViewGroup, viewType: Int): RowViewHolder {
        val rowView = gridCellInflater.inflate(R.layout.candidate_overlay_row, parent, false)
        return RowViewHolder(rowView)
    }

    override fun onBindViewHolder(holder: RowViewHolder, position: Int) {
        val row = getItem(position)
        val isLastRow = position == itemCount - 1
        holder.bind(row, isLastRow)
    }

    inner class RowViewHolder(itemView: View) : RecyclerView.ViewHolder(itemView) {
        private val itemsLayout: LinearLayout = itemView.findViewById(R.id.row_items_layout)
        private val divider: View = itemView.findViewById(R.id.row_divider)

        // Pre-inflated cell views (grid only — long cell removed)
        private val gridCells: Array<View>

        init {
            // Set row margins matching createRowLayout()
            itemsLayout.layoutParams = (itemsLayout.layoutParams as LinearLayout.LayoutParams).apply {
                setMargins(spacing * 2, spacing, spacing * 2, spacing)
            }

            // Set divider margins matching createRowLayout()
            divider.layoutParams = (divider.layoutParams as LinearLayout.LayoutParams).apply {
                setMargins(spacing * 2, 0, spacing * 2 + 60, 0)
            }

            // Pre-inflate common case; bind() inflates more if needed
            gridCells = Array(PRE_INFLATED_CELLS) {
                gridCellInflater.inflate(R.layout.candidate_grid_cell, itemsLayout, false)
            }
        }

        fun bind(row: CandidateRow, isLastRow: Boolean) {
            val typeface = getTypeface()
            val isSwapped = isTranslateSwapped()

            // Remove all cells from layout (they were added in a previous bind)
            itemsLayout.removeAllViews()

            row.items.forEachIndexed { cellIndex, item ->
                // Use pre-inflated cell if available, otherwise inflate on demand
                val cellView = if (cellIndex < gridCells.size) {
                    gridCells[cellIndex]
                } else {
                    gridCellInflater.inflate(R.layout.candidate_grid_cell, itemsLayout, false)
                }

                // Detach from any previous parent (safety: pre-inflated views may have been
                // added to a different row's itemsLayout if the ViewHolder was recycled)
                (cellView.parent as? ViewGroup)?.removeView(cellView)

                val primaryText = cellView.findViewById<TextView>(R.id.cell_primary_text)
                val subtitleText = cellView.findViewById<TextView>(R.id.cell_subtitle_text)

                // Bind content
                bindCellContent(item.word, primaryText, subtitleText, isSwapped)

                // Set typeface
                primaryText.typeface = typeface
                subtitleText.typeface = typeface

                // Pixel-based layout: measuredWidth as base, weight=1 for equal flex
                // Matches iOS .frame(minWidth: measuredWidth, maxWidth: .infinity)
                val lp = LinearLayout.LayoutParams(
                    item.measuredWidth,
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    1.0f
                ).apply {
                    if (cellIndex > 0) {
                        marginStart = spacing
                    }
                }
                cellView.layoutParams = lp

                // Composing cell (position 0): key_bgColor background, same as smartbar
                val isComposing = item.originalIndex == 0 && item.word.id >= 0
                if (isComposing) {
                    cellView.setBackgroundResource(R.drawable.candidate_grid_composing_background)
                } else {
                    cellView.setBackgroundResource(R.drawable.candidate_grid_cell_background)
                }

                // Set click listener
                cellView.setOnClickListener {
                    if (!isClickEnabled()) return@setOnClickListener
                    onCellClick(item.word, item.originalIndex)
                }

                itemsLayout.addView(cellView)
            }

            // Show divider for all rows except the last
            divider.visibility = if (isLastRow) View.GONE else View.VISIBLE
        }

        private fun bindCellContent(
            word: TaigiWord,
            primaryText: TextView,
            subtitleText: TextView,
            isSwapped: Boolean
        ) {
            val isTPSLayout = layoutType() == "tps"
            val displayRoman = TPSConverter.displayRoman(word.roman, layoutType(), orMapsToER())

            when {
                word.hanzi.isNullOrEmpty() -> {
                    primaryText.text = displayRoman
                    subtitleText.visibility = View.GONE
                }
                isTPSLayout -> {
                    // TPS mode: always show hanzi only
                    primaryText.text = word.hanzi
                    subtitleText.visibility = View.GONE
                }
                isSwapped -> {
                    primaryText.text = word.hanzi
                    subtitleText.text = displayRoman
                    subtitleText.visibility = View.VISIBLE
                }
                else -> {
                    primaryText.text = displayRoman
                    subtitleText.text = word.hanzi
                    subtitleText.visibility = View.VISIBLE
                }
            }
        }

    }

    // --- Data classes ---

    data class CandidateRow(
        val items: List<CandidateItem>
    )

    data class CandidateItem(
        val word: TaigiWord,
        val originalIndex: Int,
        val measuredWidth: Int
    )

    // --- DiffUtil ---

    class RowDiffCallback : DiffUtil.ItemCallback<CandidateRow>() {
        override fun areItemsTheSame(oldItem: CandidateRow, newItem: CandidateRow): Boolean {
            // Rows don't have stable IDs; compare by position (handled by ListAdapter)
            // Use content-based identity: same items in same order
            if (oldItem.items.size != newItem.items.size) return false
            return oldItem.items.zip(newItem.items).all { (a, b) ->
                a.word.id == b.word.id && a.originalIndex == b.originalIndex
            }
        }

        override fun areContentsTheSame(oldItem: CandidateRow, newItem: CandidateRow): Boolean {
            return oldItem == newItem
        }
    }
}
