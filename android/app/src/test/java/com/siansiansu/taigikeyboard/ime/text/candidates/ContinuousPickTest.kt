package com.siansiansu.taigikeyboard.ime.text.candidates

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.engine.proto.CommitOutcome
import com.siansiansu.taigikeyboard.engine.proto.CommitResolution
import com.siansiansu.taigikeyboard.engine.proto.CommitScript
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Pins the Continuous tap request (R5): the script and roman the engine
 * resolves the document text from come off the candidate cell, and the
 * engine's answer decodes into the outcome the auto space is gated on. What
 * each request writes is the engine's (`engine/composing/src/commit_text.rs`
 * `truth_table` / `tps_truth_table`) — the JVM cannot load the engine.
 * Mirrors iOS `ActionHandlerContinuousPickTests`.
 */
class ContinuousPickTest {
    private fun candidate(
        roman: String,
        hanji: String?,
    ) = RustEngineBridge.ContinuousCandidate(
        consumedSpanEnd = 7,
        syllableCount = 2,
        displayText = hanji ?: roman,
        roman = roman,
        hanji = hanji,
        canonicalTl = "tâi-gí",
    )

    @Test
    fun unsplitCell_commitsTheLead_withTheCandidateMetadata() {
        val word = buildContinuousSuggestionsForCandidates(listOf(candidate("tâi-gí", "台語"))).single()
        assertEquals(
            RustEngineBridge.ContinuousPick(
                script = CommitScript.COMMIT_SCRIPT_LEAD,
                roman = "tâi-gí",
                canonicalText = "台語",
                associationTl = "tâi-gí",
                hanji = "台語",
                consumedBytes = 7,
                syllableCount = 2,
            ),
            continuousPick(word),
        )
    }

    /** A hanji-less pick sends no hanji (§50) — on the TPS layout the engine writes its Bopomofo. */
    @Test
    fun hanjilessCell_sendsItsRomanAndNoHanji() {
        val word = buildContinuousSuggestionsForCandidates(listOf(candidate("tâi-gí", null))).single()
        val pick = requireNotNull(continuousPick(word))
        assertEquals("tâi-gí", pick.roman)
        assertNull(pick.hanji)
        assertFalse(pick.toRequest().hasHanji())
    }

    /** §42 split cells commit the script their marker names; both carry the same identity and roman. */
    @Test
    fun splitCells_commitTheirOwnScript() {
        val cells =
            buildContinuousSuggestionsForCandidates(listOf(candidate("tâi-gí", "台語")), splitCombinedCells = true)
        val picks = cells.map { requireNotNull(continuousPick(it)) }
        assertEquals(
            listOf(CommitScript.COMMIT_SCRIPT_HANJI, CommitScript.COMMIT_SCRIPT_ROMAN),
            picks.map { it.script },
        )
        for (pick in picks) {
            assertEquals("tâi-gí", pick.roman)
            assertEquals("台語", pick.hanji)
            assertEquals("identity never moves", "台語", pick.canonicalText)
        }
    }

    @Test
    fun unknownMarker_commitsTheLead() {
        val word = buildContinuousSuggestionsForCandidates(listOf(candidate("tâi-gí", "台語"))).single()
        val defective = word.copy(additionalInfo = word.additionalInfo + (TaigiWord.MetadataKeys.CELL_SCRIPT to "both"))
        assertEquals(CommitScript.COMMIT_SCRIPT_LEAD, commitScript(defective))
    }

    /** Strict-required metadata: a word missing any of it is dropped, never committed by text. */
    @Test
    fun missingMetadata_dropsTheTap() {
        val word = buildContinuousSuggestionsForCandidates(listOf(candidate("tâi-gí", "台語"))).single()
        for (key in listOf(
            TaigiWord.MetadataKeys.DISPLAY_TEXT,
            TaigiWord.MetadataKeys.CONSUMED_BYTES,
            TaigiWord.MetadataKeys.SYLLABLE_COUNT,
        )) {
            assertNull("missing $key", continuousPick(word.copy(additionalInfo = word.additionalInfo - key)))
        }
    }

    @Test
    fun request_carriesScriptRomanAndIdentity_notADocumentString() {
        val request =
            RustEngineBridge
                .ContinuousPick(
                    script = CommitScript.COMMIT_SCRIPT_HANJI,
                    roman = "tâi-gí",
                    canonicalText = "台語",
                    associationTl = "tâi-gí",
                    hanji = "台語",
                    consumedBytes = 7,
                    syllableCount = 2,
                ).toRequest()
        assertEquals(CommitScript.COMMIT_SCRIPT_HANJI, request.script)
        assertEquals("tâi-gí", request.roman)
        assertEquals("台語", request.canonicalText)
        assertEquals("tâi-gí", request.associationTl)
        assertEquals("台語", request.hanji)
        assertEquals(7, request.consumedBytes)
        assertEquals(2, request.syllableCount)
    }

    @Test
    fun outcome_decodesTheEngineAnswer() {
        fun outcome(
            outcome: CommitOutcome,
            earnsAutoSpace: Boolean = false,
        ) = RustEngineBridge.ContinuousCommitOutcome.from(
            CommitResolution
                .newBuilder()
                .setOutcome(outcome)
                .setEarnsAutoSpace(earnsAutoSpace)
                .build(),
        )
        assertEquals(RustEngineBridge.ContinuousCommitOutcome.Nailed, outcome(CommitOutcome.COMMIT_OUTCOME_NAILED))
        assertEquals(
            RustEngineBridge.ContinuousCommitOutcome.Finalized(earnsAutoSpace = true),
            outcome(CommitOutcome.COMMIT_OUTCOME_FINALIZED, earnsAutoSpace = true),
        )
        assertEquals(RustEngineBridge.ContinuousCommitOutcome.Ignored, outcome(CommitOutcome.COMMIT_OUTCOME_IGNORED))
        assertEquals(
            "an answer without a resolution changed nothing this side can tell",
            RustEngineBridge.ContinuousCommitOutcome.Ignored,
            RustEngineBridge.ContinuousCommitOutcome.from(CommitResolution.getDefaultInstance()),
        )
    }
}
