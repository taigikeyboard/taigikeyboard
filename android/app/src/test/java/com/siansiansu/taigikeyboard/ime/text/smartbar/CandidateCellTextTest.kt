package com.siansiansu.taigikeyboard.ime.text.smartbar

import com.siansiansu.taigikeyboard.ime.core.settings.CandidateDisplayMode
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Pins the candidate-cell arm order shared by the strip and the expanded
 * overlay: `hanji empty → TPS → ROMAN_ONLY → COMBINED → swapped → else`.
 * Mirrors iOS `CandidateCellHelperTests`.
 */
class CandidateCellTextTest {
    private fun cell(
        hanzi: String? = "台語",
        displayRoman: String = "tâi-gí",
        isTPSLayout: Boolean = false,
        mode: CandidateDisplayMode = CandidateDisplayMode.SIDE_BY_SIDE,
        isTranslateSwapped: Boolean = false,
    ) = candidateCellText(hanzi, displayRoman, isTPSLayout, mode, isTranslateSwapped)

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

    /** One label `漢字 羅馬字` (single ASCII space), no subtitle, whatever the swap flag says. */
    @Test
    fun test_INVARIANT_combined_cells_are_one_hanji_space_roman_label() {
        assertEquals(CandidateCellText("台語 tâi-gí", null), cell(mode = CandidateDisplayMode.COMBINED))
        // The derived swap flag is true under 漢羅合用; a stale `false` must render identically.
        assertEquals(
            CandidateCellText("台語 tâi-gí", null),
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
    }
}
