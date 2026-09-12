package com.siansiansu.taigikeyboard.ime.text.smartbar

import com.siansiansu.taigikeyboard.engine.RustEngineBridge
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord
import com.siansiansu.taigikeyboard.ime.dictionary.TaigiWord.MetadataKeys
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins the NextWord prediction cell shape [buildPredictionWords] emits per
 * 候選詞顯示 mode (§42, USER 2026-09-12: 濫 must list both scripts of a
 * prediction, 羅馬字 the roman alone, 並排 today's dual-script cell).
 * Mirrors iOS `ActionHandler.setNextWordPredictions`.
 */
class PredictionWordsBuilderTest {
    private fun prediction(
        text: String,
        subtitle: String?,
        hanzi: String,
        tl: String = text,
        score: Double = 1.0,
    ) = RustEngineBridge.NextWordEnginePrediction(
        text = text,
        subtitle = subtitle,
        hanzi = hanzi,
        tl = tl,
        score = score,
    )

    private val predictions =
        listOf(
            prediction(text = "tsia̍h", subtitle = "食", hanzi = "食"),
            // 同音異字 — same roman, different hanji.
            prediction(text = "tsia̍h", subtitle = "𤆬", hanzi = "𤆬"),
            // 一字多音 — same hanji, different roman.
            prediction(text = "tîng", subtitle = "重", hanzi = "重"),
            prediction(text = "tāng", subtitle = "重", hanzi = "重"),
            // Hanji-only prediction (engine shaped no roman).
            prediction(text = "去", subtitle = null, hanzi = "去", tl = ""),
        )

    @Test
    fun unsplit_emits_one_dual_script_word_per_prediction_with_sentinel_ids() {
        val words = buildPredictionWords(predictions, splitCombinedCells = false)

        assertEquals(predictions.size, words.size)
        assertEquals(listOf(-1, -2, -3, -4, -5), words.map { it.id })
        assertEquals("tsia̍h", words[0].roman)
        assertEquals("食", words[0].hanzi)
        assertEquals("", words[4].roman)
        assertEquals("去", words[4].hanzi)
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
                // 𤆬's roman cell reads like 食's → collapsed; its 漢字 cell stays.
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
            assertEquals("重", it.hanzi)
            assertEquals("tāng", it.roman)
            assertEquals("tāng", it.additionalInfo[MetadataKeys.CANONICAL_TL])
            assertEquals("重", it.displayText)
        }
    }

    private fun TaigiWord.displayCellText(): String = if (additionalInfo[MetadataKeys.CELL_SCRIPT] == MetadataKeys.CELL_SCRIPT_ROMAN) roman else hanzi.orEmpty()
}
