package com.siansiansu.taigikeyboard.ime.text.smartbar

import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Pins the greedy row-packing algorithm of [CandidateRowLayout.arrangeRows] — the pure
 * seam extracted from the legacy RecyclerView-based candidate overlay. Mirrors the iOS
 * exemplar `ExpandedCandidateRowLayout.arrangeRows`; a break here means Android candidate
 * row wrapping has diverged from the frozen behavior.
 */
class CandidateRowLayoutTest {
    private fun word(index: Int): TaigiWord =
        TaigiWord(id = index, roman = "r$index", hanzi = "h$index", lengthScore = 0)

    /** Each suggestion measures to a fixed width supplied by [widths] (px), keyed by id. */
    private fun pack(
        widths: List<Int>,
        availableWidth: Int,
        itemSpacing: Int,
    ): List<List<CandidateRowLayout.RowItem>> {
        val suggestions = widths.indices.map { word(it) }
        return CandidateRowLayout.arrangeRows(
            suggestions = suggestions,
            availableWidth = availableWidth,
            itemSpacing = itemSpacing,
            measureCellWidth = { widths[it.id] },
        )
    }

    @Test
    fun arrangeRows_emptyInput_returnsNoRows() {
        assertEquals(emptyList<List<CandidateRowLayout.RowItem>>(), pack(emptyList(), 100, 1))
    }

    @Test
    fun arrangeRows_cellExactlyFillingWidth_staysOnSameRow() {
        // Two 50px cells + 0 spacing == 100 available: the boundary is `>`, so equal does NOT wrap.
        val rows = pack(widths = listOf(50, 50), availableWidth = 100, itemSpacing = 0)
        assertEquals(1, rows.size)
        assertEquals(2, rows[0].size)
    }

    @Test
    fun arrangeRows_cellOnePastWidth_wrapsToNextRow() {
        val rows = pack(widths = listOf(50, 51), availableWidth = 100, itemSpacing = 0)
        assertEquals(2, rows.size)
        assertEquals(listOf(0), rows[0].map { it.originalIndex })
        assertEquals(listOf(1), rows[1].map { it.originalIndex })
    }

    @Test
    fun arrangeRows_spacingCountedFromSecondCellOnly() {
        // 40 + spacing(10) + 40 = 90 <= 90 → one row. With availableWidth 89 the spacing tips it over.
        assertEquals(1, pack(widths = listOf(40, 40), availableWidth = 90, itemSpacing = 10).size)
        assertEquals(2, pack(widths = listOf(40, 40), availableWidth = 89, itemSpacing = 10).size)
    }

    @Test
    fun arrangeRows_singleOversizedCell_formsItsOwnRow() {
        val rows = pack(widths = listOf(500), availableWidth = 100, itemSpacing = 1)
        assertEquals(1, rows.size)
        assertEquals(0, rows[0][0].originalIndex)
        assertEquals(500, rows[0][0].measuredWidth)
    }

    @Test
    fun arrangeRows_preservesOriginalIndexAcrossWraps() {
        // widths 60 each, available 130, spacing 10 → rows pack 2 per row (60+10+60=130).
        val rows = pack(widths = List(5) { 60 }, availableWidth = 130, itemSpacing = 10)
        assertEquals(3, rows.size)
        assertEquals(listOf(0, 1), rows[0].map { it.originalIndex })
        assertEquals(listOf(2, 3), rows[1].map { it.originalIndex })
        assertEquals(listOf(4), rows[2].map { it.originalIndex })
    }
}
