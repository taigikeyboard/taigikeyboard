package com.siansiansu.taigikeyboard.ime.text.composing

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord.MetadataKeys
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * v3.5.8 Phase 9 Item 4 — pins the Continuous-input candidate metadata
 * contract that
 * [com.siansiansu.taigikeyboard.ime.text.composing.TaigiAutocompleteService]
 * (producer) and
 * [com.siansiansu.taigikeyboard.ime.text.smartbar.CandidateClickHandler]
 * (consumer) share via
 * [com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord.additionalInfo].
 *
 * Mirrors iOS `AutocompleteServiceContinuousTests.swift`. If iOS / Android
 * disagree on these key strings, the platform tap path silently mis-aligns
 * `commitContinuous` consumed-byte offsets and corrupts the engine pending
 * buffer — invisible to the user until they hit a bad commit boundary.
 *
 * Per `docs/engine/continuous-input-ranking.md` §10.1.2 (supersedes legacy
 * slot-0 model) + §10.3 commit contract: Continuous mode has NO
 * composing-text cell at slot 0. `candidate[0]` is the engine ranker top;
 * Tap-0 commits `candidate[0].display_text` (clarification γ — canonical
 * dictionary string, NOT the roman-with-spaces visual form).
 *
 * Scope: pure JVM unit tests against the top-level
 * [buildContinuousSuggestionsForCandidates] helper (no Robolectric needed).
 * Full `ComposingManager` round-trip + `CandidateClickHandler` tap-decode
 * tests await Robolectric / mockk infra.
 */
class ContinuousSuggestionsContractTest {

    @Test
    fun `slot 0 is candidate top with isContinuous flag, no composing cell`() {
        // v3.5.8 Phase 9 Item 4: §10.1.2 supersedes notice — the Continuous
        // path no longer inserts a `createComposingTextCell` at index 0.
        // `candidate[0]` IS slot 0, carrying `IS_CONTINUOUS="true"` so the
        // click handler routes it through `handleContinuousCandidateClick`.
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
        val result = buildContinuousSuggestionsForCandidates(candidates)

        assertEquals("Continuous path emits exactly N cells (no slot-0 composing cell)", 1, result.size)
        val slot0 = result[0]
        assertEquals("Slot 0 == candidate[0] (id starts at 1, defense-in-depth)", 1, slot0.id)
        assertEquals("true", slot0.additionalInfo[MetadataKeys.IS_CONTINUOUS])
        assertNull("slot 0 must NOT carry isComposingText (legacy slot-0 model superseded)", slot0.additionalInfo[MetadataKeys.IS_COMPOSING_TEXT])
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
        val result = buildContinuousSuggestionsForCandidates(candidates)
        val first = result[0]
        val second = result[1]

        // First candidate (slot 0 — was slot 1 before Item 4)
        assertEquals("true", first.additionalInfo[MetadataKeys.IS_CONTINUOUS])
        assertEquals("4", first.additionalInfo[MetadataKeys.CONSUMED_BYTES])
        assertEquals("1", first.additionalInfo[MetadataKeys.SYLLABLE_COUNT])
        assertEquals("tsua", first.additionalInfo[MetadataKeys.DISPLAY_TEXT])

        // Second candidate (slot 1 — was slot 2 before Item 4)
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
        val result = buildContinuousSuggestionsForCandidates(candidates)
        assertEquals("7", result[0].additionalInfo[MetadataKeys.CONSUMED_BYTES])
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
        val result = buildContinuousSuggestionsForCandidates(candidates)
        assertEquals(
            "displayText sidechannel must equal engine's raw displayText",
            "tâi-uân",
            result[0].additionalInfo[MetadataKeys.DISPLAY_TEXT],
        )
    }

    @Test
    fun `gamma clarification — displayText sidechannel decouples from roman`() {
        // v3.5.8 Phase 9 Item 4 — pin §10.3 clarification γ at the producer
        // boundary. Today producer initializes both `roman` and `DISPLAY_TEXT`
        // from `candidate.displayText`, so they coincide. Once Item 5/6 add
        // proto `roman`/`hanji` fields and slot-0 renders the segmented
        // visual form, the producer will populate `roman` with the visual
        // form while `DISPLAY_TEXT` remains the canonical commit string.
        // The consumer (`CandidateClickHandler.handleContinuousCandidateClick`)
        // must already be reading `DISPLAY_TEXT` exclusively — no `?:` fallback
        // to `selectedWord.roman` — so γ holds regardless of which field
        // mutates first. This test pins the sidechannel emission as the
        // authoritative commit-string carrier so Item 6 cannot accidentally
        // route through `roman`.
        val candidates = listOf(
            RustEngineBridge.ContinuousCandidate(
                consumedSpanStart = 0,
                consumedSpanEnd = 6,
                syllableCount = 2,
                displayText = "tâi-gí",
                score = 1.0f,
                form = 1,
                mode = RustEngineBridge.CandidateMode.HANT,
            ),
        )
        val result = buildContinuousSuggestionsForCandidates(candidates)
        assertEquals(
            "DISPLAY_TEXT sidechannel is the canonical commit string (γ)",
            "tâi-gí",
            result[0].additionalInfo[MetadataKeys.DISPLAY_TEXT],
        )
        // Sanity: the producer's current shape (`roman = candidate.displayText`)
        // is exercised — Item 6 will diverge `roman` from `DISPLAY_TEXT` and
        // this assertion will need updating; the assertion below pins the
        // contract the consumer relies on (sidechannel, not roman).
        assertEquals(
            "Item 4 producer still couples roman to displayText (Item 6 will diverge)",
            "tâi-gí",
            result[0].roman,
        )
    }

    @Test
    fun `synthetic ids stay outside English and NextWord sentinel ranges`() {
        // English: id <= -100. NextWord: id < 0 && id > -100.
        // Lexicon-path slot-0 composing-text cell (non-Continuous mode only):
        // id == 0. Continuous: id >= 1. Routing decision keys off
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
        val result = buildContinuousSuggestionsForCandidates(candidates)
        result.forEach { word ->
            assertTrue("Continuous id must be >= 1, was ${word.id}", word.id >= 1)
            assertNotEquals("id == 0 reserved for lexicon-path composing cell", 0, word.id)
        }
    }

    @Test
    fun `empty candidate list emits empty list (caller handles fall-through)`() {
        // v3.5.8 Phase 9 Item 4: §10.7 edge case "Empty buffer" / partial
        // prefix — strip is empty when engine returns no candidates. Caller
        // (`TaigiAutocompleteService.autocomplete`) already guards
        // `if (continuousCandidates.isNotEmpty())` before invoking this
        // helper and falls through to the lexicon path when empty, which
        // re-inserts its own slot-0 composing-text cell per §10.5 mode
        // gating. The helper contract here is simply: zero candidates →
        // zero cells, no synthetic slot-0 fallback.
        val result = buildContinuousSuggestionsForCandidates(emptyList())
        assertTrue("Empty candidates → empty suggestions", result.isEmpty())
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
