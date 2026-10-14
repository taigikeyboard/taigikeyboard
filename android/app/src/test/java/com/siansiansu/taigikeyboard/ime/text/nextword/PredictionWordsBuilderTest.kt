package com.siansiansu.taigikeyboard.ime.text.nextword

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.engine.proto.CommitScript
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord.MetadataKeys
import com.siansiansu.taigikeyboard.ime.text.candidates.predictionPick
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins the NextWord prediction cell shape [buildPredictionWords] emits per
 * Candidate Display mode (§42, USER 2026-09-12: mixed mode must list both scripts of a
 * prediction, Romanization Only the roman alone, Pairing today's dual-script cell).
 * Mirrors iOS `ActionHandler.setNextWordPredictions`.
 */
class PredictionWordsBuilderTest {
    private fun prediction(
        text: String,
        subtitle: String?,
        hanji: String,
        tl: String = text,
    ) = RustEngineBridge.NextWordEnginePrediction(
        text = text,
        subtitle = subtitle,
        hanji = hanji,
        tl = tl,
    )

    private val predictions =
        listOf(
            prediction(text = "tsia̍h", subtitle = "食", hanji = "食"),
            // Homophones — same roman, different hanji.
            prediction(text = "tsia̍h", subtitle = "𤆬", hanji = "𤆬"),
            // Multi-reading Hanji — same hanji, different roman.
            prediction(text = "tîng", subtitle = "重", hanji = "重"),
            prediction(text = "tāng", subtitle = "重", hanji = "重"),
            // Hanji-only prediction (engine shaped no roman).
            prediction(text = "去", subtitle = null, hanji = "去", tl = ""),
        )

    @Test
    fun unsplit_emits_one_dual_script_word_per_prediction_with_sentinel_ids() {
        val words = buildPredictionWords(predictions, splitCombinedCells = false)

        assertEquals(predictions.size, words.size)
        assertEquals(listOf(-1, -2, -3, -4, -5), words.map { it.id })
        assertEquals("tsia̍h", words[0].roman)
        assertEquals("食", words[0].hanji)
        assertEquals("", words[4].roman)
        assertEquals("去", words[4].hanji)
        words.forEach { assertNull(it.additionalInfo[MetadataKeys.CELL_SCRIPT]) }
        assertEquals("tāng", words[3].additionalInfo[MetadataKeys.CANONICAL_TL])
    }

    @Test
    fun split_emits_hanji_then_roman_cells_deduped_per_script() {
        val words = buildPredictionWords(predictions, splitCombinedCells = true)

        val shown = words.map { it.additionalInfo[MetadataKeys.CELL_SCRIPT] to it.displayCellText() }
        assertEquals(
            listOf(
                MetadataKeys.CELL_SCRIPT_HANJI to "食",
                MetadataKeys.CELL_SCRIPT_ROMAN to "tsia̍h",
                // 𤆬's roman cell reads like 食's → collapsed; its Hanji cell stays.
                MetadataKeys.CELL_SCRIPT_HANJI to "𤆬",
                MetadataKeys.CELL_SCRIPT_HANJI to "重",
                MetadataKeys.CELL_SCRIPT_ROMAN to "tîng",
                // 重/tāng: 重 already drawn, its own roman cell stays reachable.
                MetadataKeys.CELL_SCRIPT_ROMAN to "tāng",
                MetadataKeys.CELL_SCRIPT_HANJI to "去",
            ),
            shown,
        )
        // Ids stay contiguous inside the NextWord sentinel range.
        assertEquals((1..words.size).map { -it }, words.map { it.id })
        assertTrue(words.all { it.id > -100 })
    }

    @Test
    fun split_cells_share_the_prediction_identity() {
        val words = buildPredictionWords(listOf(predictions[3]), splitCombinedCells = true)

        assertEquals(2, words.size)
        words.forEach {
            assertEquals("重", it.hanji)
            assertEquals("tāng", it.roman)
            assertEquals("tāng", it.additionalInfo[MetadataKeys.CANONICAL_TL])
            assertEquals("重", it.displayText)
        }
    }

    private fun TaigiWord.displayCellText(): String = if (additionalInfo[MetadataKeys.CELL_SCRIPT] == MetadataKeys.CELL_SCRIPT_ROMAN) roman else hanji.orEmpty()

    /**
     * The tap request (D1) is read off the word itself — the roman the engine
     * resolves from (empty for a Hanji-only prediction), the Hanji, the
     * canonical TL and a split cell's own script — never the cell text. What
     * each request writes is the engine's
     * (`engine/composing/tests/prediction_commit_resolution.rs`); the JVM
     * cannot load it. Mirrors iOS `testPredictionPick_readsTheMetadataNotTheCellText`.
     */
    @Test
    fun prediction_pick_reads_the_word_not_the_cell_text() {
        val unsplit = buildPredictionWords(predictions, splitCombinedCells = false)
        assertEquals(
            RustEngineBridge.PredictionPick(CommitScript.COMMIT_SCRIPT_LEAD, "tāng", "重", "tāng"),
            predictionPick(unsplit[3]),
        )
        assertEquals(
            RustEngineBridge.PredictionPick(CommitScript.COMMIT_SCRIPT_LEAD, "", "去", ""),
            predictionPick(unsplit[4]),
        )

        val split = buildPredictionWords(listOf(predictions[3]), splitCombinedCells = true).map(::predictionPick)
        assertEquals(
            listOf(CommitScript.COMMIT_SCRIPT_HANJI, CommitScript.COMMIT_SCRIPT_ROMAN),
            split.map { it.script },
        )
        // The Hanji cell carries the roman its Annotate in Brackets commit appends.
        split.forEach { assertEquals("tāng", it.roman) }
    }
}
