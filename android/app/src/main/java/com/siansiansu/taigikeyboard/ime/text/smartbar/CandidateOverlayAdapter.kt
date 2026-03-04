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
    private val isClickEnabled: () -> Boolean,
    private val onCellClick: (TaigiWord, Int) -> Unit
) : ListAdapter<CandidateOverlayAdapter.CandidateRow, CandidateOverlayAdapter.RowViewHolder>(RowDiffCallback()) {

    companion object {
        private const val MAX_ITEMS_PER_ROW = 4
        private const val LONG_WORD_THRESHOLD = 12
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

        // Pre-inflated cell views (grid + long variants)
        private val gridCells: Array<View>
        private val longCells: Array<View>

        init {
            // Set row margins matching createRowLayout()
            itemsLayout.layoutParams = (itemsLayout.layoutParams as LinearLayout.LayoutParams).apply {
                setMargins(spacing * 2, spacing, spacing * 2, spacing)
            }

            // Set divider margins matching createRowLayout()
            divider.layoutParams = (divider.layoutParams as LinearLayout.LayoutParams).apply {
                setMargins(spacing * 2, 0, spacing * 2 + 60, 0)
            }

            // Pre-inflate MAX_ITEMS_PER_ROW grid cells and long cells
            gridCells = Array(MAX_ITEMS_PER_ROW) {
                gridCellInflater.inflate(R.layout.candidate_grid_cell, itemsLayout, false)
            }
            longCells = Array(MAX_ITEMS_PER_ROW) {
                gridCellInflater.inflate(R.layout.candidate_long_cell, itemsLayout, false)
            }
        }

        fun bind(row: CandidateRow, isLastRow: Boolean) {
            val typeface = getTypeface()
            val isSwapped = isTranslateSwapped()

            // Remove all cells from layout (they were added in a previous bind)
            itemsLayout.removeAllViews()

            row.items.forEachIndexed { cellIndex, item ->
                val charCount = getCharacterCount(item.word)
                val isLong = charCount >= LONG_WORD_THRESHOLD

                val cellView = if (isLong) longCells[cellIndex] else gridCells[cellIndex]

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

                // Set weight-based layout params
                val lp = LinearLayout.LayoutParams(
                    0,
                    LinearLayout.LayoutParams.WRAP_CONTENT,
                    item.weight.toFloat()
                ).apply {
                    if (cellIndex > 0) {
                        marginStart = spacing
                    }
                }
                cellView.layoutParams = lp

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
            when {
                word.hanzi.isNullOrEmpty() -> {
                    primaryText.text = word.roman
                    subtitleText.visibility = View.GONE
                }
                isSwapped -> {
                    primaryText.text = word.hanzi
                    subtitleText.text = word.roman
                    subtitleText.visibility = View.VISIBLE
                }
                else -> {
                    primaryText.text = word.roman
                    subtitleText.text = word.hanzi
                    subtitleText.visibility = View.VISIBLE
                }
            }
        }

        private fun getCharacterCount(word: TaigiWord): Int {
            val romanLength = word.roman.length
            val hanziLength = word.hanzi?.length ?: 0
            return maxOf(romanLength, hanziLength)
        }
    }

    // --- Data classes ---

    data class CandidateRow(
        val items: List<CandidateItem>,
        val totalWeight: Double
    )

    data class CandidateItem(
        val word: TaigiWord,
        val originalIndex: Int,
        val weight: Double
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
