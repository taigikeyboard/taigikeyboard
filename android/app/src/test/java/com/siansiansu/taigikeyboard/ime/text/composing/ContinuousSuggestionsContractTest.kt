package com.siansiansu.taigikeyboard.ime.text.composing

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord.MetadataKeys
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * v3.5.8 Phase 8 — pins the Continuous-input candidate metadata contract that
 * [com.siansiansu.taigikeyboard.ime.text.composing.TaigiAutocompleteService]
 * (producer) and [com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateClickHandler]
 * (consumer) share via [com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord.additionalInfo].
 *
 * Mirrors iOS `AutocompleteServiceContinuousTests.swift`. If iOS / Android
 * disagree on these key strings, the platform tap path silently mis-aligns
 * `commitContinuous` consumed-byte offsets and corrupts the engine pending
 * buffer — invisible to the user until they hit a bad commit boundary.
 *
 * Scope: pure JVM unit tests against the top-level
 * [buildContinuousSuggestionsForCandidates] helper (no Robolectric needed).
 * Full ComposingManager round-trip tests await Robolectric / mockk infra.
 */
class ContinuousSuggestionsContractTest {

    @Test
    fun `slot 0 is composing cell with isComposingText flag`() {
        val candidates = listOf(
            RustEngineBridge.ContinuousCandidate(
                consumedSpanStart = 0,
                consumedSpanEnd = 4,
                syllableCount = 1,
                displayText = "tsua",
                score = 1.0f,
                form = 1,
                mode = RustEngineBridge.CandidateMode.HANT,
            ),
        )
        val result = buildContinuousSuggestionsForCandidates(candidates, "tsua")

        assertEquals("slot 0 + 1 candidate = 2 cells", 2, result.size)
        val slot0 = result[0]
        assertEquals(0, slot0.id)
        assertEquals("tsua", slot0.roman)
        assertEquals("true", slot0.additionalInfo[MetadataKeys.IS_COMPOSING_TEXT])
        assertNull("slot 0 should NOT carry isContinuous", slot0.additionalInfo[MetadataKeys.IS_CONTINUOUS])
    }

    @Test
    fun `continuous candidates carry exact metadata key strings`() {
        val candidates = listOf(
            RustEngineBridge.ContinuousCandidate(
                consumedSpanStart = 0,
                consumedSpanEnd = 4,
                syllableCount = 1,
                displayText = "tsua",
                score = 1.0f,
                form = 1,
                mode = RustEngineBridge.CandidateMode.HANT,
            ),
            RustEngineBridge.ContinuousCandidate(
                consumedSpanStart = 0,
                consumedSpanEnd = 7,
                syllableCount = 2,
                displayText = "珠仔",
                score = 0.5f,
                form = 1,
                mode = RustEngineBridge.CandidateMode.HANT,
            ),
        )
        val result = buildContinuousSuggestionsForCandidates(candidates, "tsua")
        val first = result[1]
        val second = result[2]

        // First candidate
        assertEquals("true", first.additionalInfo[MetadataKeys.IS_CONTINUOUS])
        assertEquals("4", first.additionalInfo[MetadataKeys.CONSUMED_BYTES])
        assertEquals("1", first.additionalInfo[MetadataKeys.SYLLABLE_COUNT])
        assertEquals("tsua", first.additionalInfo[MetadataKeys.DISPLAY_TEXT])

        // Second candidate (multi-syllable / hanzi)
        assertEquals("true", second.additionalInfo[MetadataKeys.IS_CONTINUOUS])
        assertEquals("7", second.additionalInfo[MetadataKeys.CONSUMED_BYTES])
        assertEquals("2", second.additionalInfo[MetadataKeys.SYLLABLE_COUNT])
        assertEquals("珠仔", second.additionalInfo[MetadataKeys.DISPLAY_TEXT])
    }

    @Test
    fun `consumedBytes uses consumedSpanEnd not consumedSpanStart`() {
        // Engine spans are byte-relative-to-pending-buffer; commit consumes
        // bytes [start, end). The consumer needs `end` to know how many
        // bytes to drop from pending. Mid-commit candidate.
        val candidates = listOf(
            RustEngineBridge.ContinuousCandidate(
                consumedSpanStart = 3,
                consumedSpanEnd = 7,
                syllableCount = 1,
                displayText = "uan",
                score = 1.0f,
                form = 1,
                mode = RustEngineBridge.CandidateMode.HANT,
            ),
        )
        val result = buildContinuousSuggestionsForCandidates(candidates, "taiuan")
        assertEquals("7", result[1].additionalInfo[MetadataKeys.CONSUMED_BYTES])
    }

    @Test
    fun `displayText sidechannel is engine-supplied raw, not roman field rewrite`() {
        // Codex P1 fix `f01559cf` on iOS — TPS layout would view-rewrite the
        // roman field, breaking commitContinuous alignment. Sidechannel is
        // the contract.
        val candidates = listOf(
            RustEngineBridge.ContinuousCandidate(
                consumedSpanStart = 0,
                consumedSpanEnd = 6,
                syllableCount = 2,
                displayText = "tâi-uân",
                score = 1.0f,
                form = 1,
                mode = RustEngineBridge.CandidateMode.HANT,
            ),
        )
        val result = buildContinuousSuggestionsForCandidates(candidates, "taiuan")
        assertEquals(
            "displayText sidechannel must equal engine's raw displayText",
            "tâi-uân",
            result[1].additionalInfo[MetadataKeys.DISPLAY_TEXT],
        )
    }

    @Test
    fun `synthetic ids stay outside English and NextWord sentinel ranges`() {
        // English: id <= -100. NextWord: id < 0 && id > -100.
        // Continuous: id >= 1. Slot 0: id == 0. Routing decision keys off
        // additionalInfo NOT id, but the sentinel ranges must not be
        // shadowed lest the click handler picks the wrong branch on a
        // metadata-decode failure fallthrough.
        val candidates = (0 until 5).map { i ->
            RustEngineBridge.ContinuousCandidate(
                consumedSpanStart = 0,
                consumedSpanEnd = 3,
                syllableCount = 1,
                displayText = "c$i",
                score = 1.0f,
                form = 1,
                mode = RustEngineBridge.CandidateMode.HANT,
            )
        }
        val result = buildContinuousSuggestionsForCandidates(candidates, "raw")
        assertEquals(0, result[0].id)
        result.drop(1).forEach { word ->
            assertTrue("Continuous id must be >= 1, was ${word.id}", word.id >= 1)
        }
    }

    @Test
    fun `empty candidate list still emits slot 0 composing cell`() {
        val result = buildContinuousSuggestionsForCandidates(emptyList(), "abc")
        // NB: the autocomplete() wrapper short-circuits on empty list so this
        // path is currently unused at runtime — but the helper contract is
        // "always include slot 0", which protects against future call sites.
        assertEquals(1, result.size)
        assertEquals(0, result[0].id)
        assertNotNull(result[0].additionalInfo[MetadataKeys.IS_COMPOSING_TEXT])
    }

    // --- v3.5.8 Phase 9.2 — CandidateMode wire decode ---

    @Test
    fun `CandidateMode decode maps all four wire values`() {
        // Pins UNSPECIFIED=0, HANT=1, TAILO=2, MIXED=3 from
        // `engine/protos/proto/composing.proto::CandidateMode`. Mirrors
        // iOS RustEngineBridgeContinuousTests.testCandidateModeDecode_AllWireValues.
        assertEquals(RustEngineBridge.CandidateMode.UNSPECIFIED, RustEngineBridge.CandidateMode.decode(0))
        assertEquals(RustEngineBridge.CandidateMode.HANT, RustEngineBridge.CandidateMode.decode(1))
        assertEquals(RustEngineBridge.CandidateMode.TAILO, RustEngineBridge.CandidateMode.decode(2))
        assertEquals(RustEngineBridge.CandidateMode.MIXED, RustEngineBridge.CandidateMode.decode(3))
    }

    @Test
    fun `CandidateMode decode falls back to UNSPECIFIED for unknown wire values`() {
        // Forward-compat: a wire value the platform binding doesn't recognize
        // (e.g. a newer engine added a fourth variant) must collapse to
        // UNSPECIFIED rather than crash or randomly map. Mirrors iOS Codex F8.
        assertEquals(RustEngineBridge.CandidateMode.UNSPECIFIED, RustEngineBridge.CandidateMode.decode(99))
        assertEquals(RustEngineBridge.CandidateMode.UNSPECIFIED, RustEngineBridge.CandidateMode.decode(-1))
    }
}
