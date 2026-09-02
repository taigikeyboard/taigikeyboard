package com.siansiansu.taigikeyboard.ime.text.smartbar

import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Pins the 漢羅濫 marked-cell commit contract (§42 second exception):
 * hanji cell → 漢字, or `漢字 (羅馬字)` under 括號標註 (the only marked
 * hanji commit that writes romanization); roman cell → the BARE roman,
 * brackets IGNORED (desktop `.alternate` parity), always romanization —
 * the `wroteRomanization` verdict drives the continuous final-commit
 * auto-space gate.
 */
class MarkedCellCommitResolverTest {
    @Test
    fun hanjiCell_bracketsOff_commitsHanjiAlone_noRomanizationWritten() {
        assertEquals(
            MarkedCellCommit("台語", wroteRomanization = false),
            resolveMarkedCellCommit(
                cellScript = TaigiWord.MetadataKeys.CELL_SCRIPT_HANJI,
                roman = "tâi-gí",
                hanzi = "台語",
                outputBothScripts = false,
            ),
        )
    }

    @Test
    fun hanjiCell_bracketsOn_commitsHanjiBracketRoman_romanizationWritten() {
        assertEquals(
            MarkedCellCommit("台語 (tâi-gí)", wroteRomanization = true),
            resolveMarkedCellCommit(
                cellScript = TaigiWord.MetadataKeys.CELL_SCRIPT_HANJI,
                roman = "tâi-gí",
                hanzi = "台語",
                outputBothScripts = true,
            ),
        )
    }

    /** 括號標註 ON must NOT bracket a roman-cell commit — bare roman, auto-space fires. */
    @Test
    fun test_INVARIANT_roman_cell_commits_bare_roman_even_with_brackets_on() {
        assertEquals(
            MarkedCellCommit("tâi-gí", wroteRomanization = true),
            resolveMarkedCellCommit(
                cellScript = TaigiWord.MetadataKeys.CELL_SCRIPT_ROMAN,
                roman = "tâi-gí",
                hanzi = "台語",
                outputBothScripts = true,
            ),
        )
        assertEquals(
            MarkedCellCommit("tâi-gí", wroteRomanization = true),
            resolveMarkedCellCommit(
                cellScript = TaigiWord.MetadataKeys.CELL_SCRIPT_ROMAN,
                roman = "tâi-gí",
                hanzi = "台語",
                outputBothScripts = false,
            ),
        )
    }

    @Test
    fun defectiveMarkers_failOpenToTheUnmarkedPath() {
        assertNull(
            "hanji marker without hanji is a wire defect — resolver declines",
            resolveMarkedCellCommit(TaigiWord.MetadataKeys.CELL_SCRIPT_HANJI, "tâi-gí", null, false),
        )
        assertNull(resolveMarkedCellCommit(TaigiWord.MetadataKeys.CELL_SCRIPT_HANJI, "tâi-gí", "", true))
        assertNull(
            "unknown marker value — resolver declines",
            resolveMarkedCellCommit("both", "tâi-gí", "台語", false),
        )
    }
}
