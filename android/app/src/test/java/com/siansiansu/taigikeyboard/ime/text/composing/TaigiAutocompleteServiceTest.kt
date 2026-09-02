package com.siansiansu.taigikeyboard.ime.text.composing

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.logging.NullLoggerBackend
import com.siansiansu.taigikeyboard.ime.core.settings.CandidateDisplayMode
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord.MetadataKeys
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * v3.5.8 Phase 9 Item 13 — pins `platform_autocomplete_no_lexicon_branch`
 * (`docs/engine/continuous-candidate-display.md` §15.6). After the
 * fallback retire the Continuous-input engine is the single candidate
 * source: an engine that returns no candidates yields an empty strip —
 * there is NO platform lexicon path and NO slot-0 composing-text cell.
 *
 * Mirrors iOS `TaigiAutocompleteServiceContinuousTests`
 * `testAutocomplete_EmptyEngine_NoLexiconBranch_EmptyResult`.
 *
 * Pure JVM unit test — the slimmed [TaigiAutocompleteService] ctor only
 * needs a [com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend]
 * and the `continuousFetcher` lambda (no lexicon / NextWord collaborators
 * after Item 13), so no Robolectric / mockk infra is required.
 */
class TaigiAutocompleteServiceTest {

    private fun cand(
        consumedSpanEnd: Int,
        displayText: String,
        syllableCount: Int = 1,
    ): RustEngineBridge.ContinuousCandidate = RustEngineBridge.ContinuousCandidate(
        consumedSpanStart = 0,
        consumedSpanEnd = consumedSpanEnd,
        syllableCount = syllableCount,
        displayText = displayText,
        score = 1.0f,
        form = 1,
        mode = RustEngineBridge.CandidateMode.HANT,
        roman = displayText,
        hanji = null,
        canonicalTl = displayText,
    )

    @Test
    fun `empty engine yields empty strip - no lexicon branch, no slot-0 cell`() = runTest {
        val service = TaigiAutocompleteService(
            logger = NullLoggerBackend,
            continuousFetcher = { emptyList() },
        )
        val result = service.autocomplete(rawInput = "gua", displayText = "gua")
        assertTrue(
            "empty engine -> empty strip (no lexicon fallback, no slot-0 cell)",
            result.isEmpty(),
        )
    }

    @Test
    fun `engine candidates pass through single-source with no composing cell`() = runTest {
        val service = TaigiAutocompleteService(
            logger = NullLoggerBackend,
            continuousFetcher = { listOf(cand(consumedSpanEnd = 3, displayText = "guá")) },
        )
        val result = service.autocomplete(rawInput = "gua", displayText = "gua")
        assertEquals("engine candidates pass through 1:1", 1, result.size)
        assertEquals("true", result[0].additionalInfo[MetadataKeys.IS_CONTINUOUS])
        assertNull(
            "no slot-0 composing-text cell on the single-source path",
            result[0].additionalInfo[MetadataKeys.IS_COMPOSING_TEXT],
        )
    }

    /**
     * §42 濫 split gate: 漢羅濫 + non-TPS splits, everything else does not.
     * Pins the polarity of the TPS clause (an inverted condition would split
     * under TPS and stop splitting under 漢羅濫).
     */
    @Test
    fun `split gate is combined mode outside TPS only`() {
        assertTrue(shouldSplitCombinedCells(CandidateDisplayMode.COMBINED, isTpsLayout = false))
        assertFalse(
            "TPS ignores the picker — hanji-first by construction",
            shouldSplitCombinedCells(CandidateDisplayMode.COMBINED, isTpsLayout = true),
        )
        for (mode in listOf(CandidateDisplayMode.SIDE_BY_SIDE, CandidateDisplayMode.ROMAN_ONLY)) {
            assertFalse("$mode never splits", shouldSplitCombinedCells(mode, isTpsLayout = false))
            assertFalse("$mode never splits under TPS", shouldSplitCombinedCells(mode, isTpsLayout = true))
        }
    }

    /**
     * The split flag is LIVE-READ per fetch, never snapshotted at construction:
     * flipping 候選詞顯示 takes effect on the very next keystroke.
     */
    @Test
    fun `split provider is re-read on every fetch`() = runTest {
        var splitCombinedCells = false
        val service = TaigiAutocompleteService(
            logger = NullLoggerBackend,
            continuousFetcher = {
                listOf(cand(consumedSpanEnd = 5, displayText = "tâi-gí").copy(hanji = "台語"))
            },
            splitCombinedCellsProvider = { splitCombinedCells },
        )

        val unsplit = service.autocomplete(rawInput = "taigi", displayText = "taigi")
        assertEquals("split OFF → one un-split cell", 1, unsplit.size)
        assertNull(unsplit[0].additionalInfo[MetadataKeys.CELL_SCRIPT])

        splitCombinedCells = true
        val split = service.autocomplete(rawInput = "taigi", displayText = "taigi")
        assertEquals("split ON on the NEXT fetch → 漢字 cell + 羅馬字 cell", 2, split.size)
        assertEquals(MetadataKeys.CELL_SCRIPT_HANJI, split[0].additionalInfo[MetadataKeys.CELL_SCRIPT])
        assertEquals(MetadataKeys.CELL_SCRIPT_ROMAN, split[1].additionalInfo[MetadataKeys.CELL_SCRIPT])

        splitCombinedCells = false
        assertEquals("switching back un-splits immediately", 1, service.autocomplete("taigi", "taigi").size)
    }

    @Test
    fun `empty rawInput or displayText short-circuits to empty`() = runTest {
        val service = TaigiAutocompleteService(
            logger = NullLoggerBackend,
            continuousFetcher = { error("fetcher must not be called when guard trips") },
        )
        assertTrue(service.autocomplete(rawInput = "", displayText = "gua").isEmpty())
        assertTrue(service.autocomplete(rawInput = "gua", displayText = "").isEmpty())
    }
}
