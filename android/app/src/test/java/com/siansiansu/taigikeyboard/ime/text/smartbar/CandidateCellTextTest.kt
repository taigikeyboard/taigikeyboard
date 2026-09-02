package com.siansiansu.taigikeyboard.ime.text.smartbar

import com.siansiansu.taigikeyboard.ime.core.settings.CandidateDisplayMode
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Pins the candidate-cell arm order shared by the strip and the expanded
 * overlay: `hanji empty → TPS → ROMAN_ONLY → COMBINED → swapped → else`.
 * COMBINED cells are one-script (§42 second exception): the CELL_SCRIPT
 * marker picks the script; an unmarked COMBINED row (a NextWord
 * prediction — not split) renders hanji-led. Mirrors iOS
 * `CandidateCellHelperTests`.
 */
class CandidateCellTextTest {
    private fun cell(
        hanzi: String? = "台語",
        displayRoman: String = "tâi-gí",
        isTPSLayout: Boolean = false,
        mode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
        isTranslateSwapped: Boolean = false,
        cellScript: String? = null,
    ) = candidateCellText(hanzi, displayRoman, isTPSLayout, mode, isTranslateSwapped, cellScript)

    @Test
    fun sideBySide_romanLeads_hanjiSubtitle() {
        assertEquals(CandidateCellText("tâi-gí", "台語"), cell())
    }

    @Test
    fun sideBySide_swapped_hanjiLeads_romanSubtitle() {
        assertEquals(CandidateCellText("台語", "tâi-gí"), cell(isTranslateSwapped = true))
    }

    @Test
    fun test_INVARIANT_roman_only_cells_have_no_subtitle() {
        assertEquals(CandidateCellText("tâi-gí", null), cell(mode = CandidateDisplayMode.ROMAN_ONLY))
        // The derived swap flag is false under roman-only, but even a stale `true` must not leak hanji.
        assertEquals(
            CandidateCellText("tâi-gí", null),
            cell(mode = CandidateDisplayMode.ROMAN_ONLY, isTranslateSwapped = true),
        )
    }

    /**
     * §42 second exception: a marked 濫 cell renders ONE script, no
     * subtitle — the split already happened at the builder. The roman cell
     * keeps its `hanzi` field (identity) but renders roman alone.
     */
    @Test
    fun test_INVARIANT_combined_marked_cells_are_single_script() {
        assertEquals(
            CandidateCellText("台語", null),
            cell(mode = CandidateDisplayMode.COMBINED, cellScript = TaigiWord.MetadataKeys.CELL_SCRIPT_HANJI),
        )
        assertEquals(
            CandidateCellText("tâi-gí", null),
            cell(mode = CandidateDisplayMode.COMBINED, cellScript = TaigiWord.MetadataKeys.CELL_SCRIPT_ROMAN),
        )
        // The swap flag (projected true under 濫) must not change either render.
        assertEquals(
            CandidateCellText("tâi-gí", null),
            cell(
                mode = CandidateDisplayMode.COMBINED,
                cellScript = TaigiWord.MetadataKeys.CELL_SCRIPT_ROMAN,
                isTranslateSwapped = true,
            ),
        )
        assertEquals(
            CandidateCellText("台語", null),
            cell(
                mode = CandidateDisplayMode.COMBINED,
                cellScript = TaigiWord.MetadataKeys.CELL_SCRIPT_HANJI,
                isTranslateSwapped = true,
            ),
        )
    }

    /** NextWord prediction rows are not split — unmarked 濫 rows render hanji-led single-script. */
    @Test
    fun combined_unmarkedRow_rendersHanjiLedSingleScript() {
        assertEquals(CandidateCellText("台語", null), cell(mode = CandidateDisplayMode.COMBINED))
        assertEquals(
            CandidateCellText("台語", null),
            cell(mode = CandidateDisplayMode.COMBINED, isTranslateSwapped = true),
        )
    }

    @Test
    fun tps_ignoresDisplayMode_hanjiOnly() {
        assertEquals(CandidateCellText("台語", null), cell(isTPSLayout = true))
        assertEquals(
            CandidateCellText("台語", null),
            cell(isTPSLayout = true, mode = CandidateDisplayMode.ROMAN_ONLY),
        )
        assertEquals(
            CandidateCellText("台語", null),
            cell(isTPSLayout = true, mode = CandidateDisplayMode.COMBINED),
        )
    }

    @Test
    fun hanjiLessRow_isRomanInEveryMode() {
        for (mode in CandidateDisplayMode.entries) {
            assertEquals(CandidateCellText("tâi-gí", null), cell(hanzi = null, mode = mode))
            assertEquals(CandidateCellText("tâi-gí", null), cell(hanzi = "", mode = mode, isTranslateSwapped = true))
        }
        // A hanji-less roman cell (§34 literal) resolves through the same first arm.
        assertEquals(
            CandidateCellText("tâi-gí", null),
            cell(hanzi = null, mode = CandidateDisplayMode.COMBINED, cellScript = TaigiWord.MetadataKeys.CELL_SCRIPT_ROMAN),
        )
    }
}
