// Pure row-packing for the expanded candidate overlay — wraps suggestions into rows by
// pre-measured pixel width. Mirrors iOS ExpandedCandidateRowLayout.arrangeRows.

package com.siansiansu.taigikeyboard.ime.text.smartbar

import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord

/**
 * Arranges candidate suggestions into wrapping rows based on pre-measured cell widths.
 *
 * Greedy pack: accumulate cells into the current row until the next cell would push the
 * row past [availableWidth], then start a new row. Measurement is injected so the packing
 * stays a pure, unit-testable function with no Android/Compose dependency.
 *
 * CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Overlays/ExpandedCandidateRowLayout.swift.
 * Drift causes silent divergence in candidate row breaks.
 */
object CandidateRowLayout {
    data class RowItem(
        val word: TaigiWord,
        val originalIndex: Int,
        val measuredWidth: Int,
    )

    fun arrangeRows(
        suggestions: List<TaigiWord>,
        availableWidth: Int,
        itemSpacing: Int,
        measureCellWidth: (TaigiWord) -> Int,
    ): List<List<RowItem>> {
        val rows = mutableListOf<List<RowItem>>()
        var currentRow = mutableListOf<RowItem>()
        var currentRowWidth = 0

        suggestions.forEachIndexed { index, word ->
            val cellWidth = measureCellWidth(word)
            val spacingNeeded = if (currentRow.isEmpty()) 0 else itemSpacing

            if (currentRow.isNotEmpty() && (currentRowWidth + spacingNeeded + cellWidth) > availableWidth) {
                rows.add(currentRow.toList())
                currentRow = mutableListOf()
                currentRowWidth = 0
            }

            currentRow.add(RowItem(word, index, cellWidth))
            currentRowWidth += (if (currentRow.size == 1) 0 else itemSpacing) + cellWidth
        }

        if (currentRow.isNotEmpty()) {
            rows.add(currentRow.toList())
        }

        return rows
    }
}
