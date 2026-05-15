package com.siansiansu.taigikeyboard.ime.text.composing

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.core.logging.NullLoggerBackend
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord.MetadataKeys
import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertEquals
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
 * Mirrors iOS `AutocompleteServiceContinuousTests`
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
