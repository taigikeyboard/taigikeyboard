package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Cross-component engine integration tests
 *
 * Ported from iOS EngineIntegrationTests.swift
 * Tests the interaction between InputNormalizer, TaigiPhonetics,
 * ToneRestoration, and SyllableSegmenter.
 */
class EngineIntegrationTest {

    // MARK: - A. Normalization Pipeline

    @Test
    fun testNormalizationPipeline_tlDiacriticInput() {
        // TL diacritic input -> TL numeric key
        val result = InputNormalizer.normalize("h\u00F3-b\u00F4", InputMode.TL)
        assertEquals("TL diacritic hó-bô", "ho2bo5", result)
    }

    @Test
    fun testNormalizationPipeline_pojDiacriticInput() {
        // POJ diacritic input -> POJ numeric (stays in POJ spelling)
        val result = InputNormalizer.normalize("ch\u00FA", InputMode.POJ)
        assertEquals("POJ diacritic chú -> POJ chu2", "chu2", result)
    }

    @Test
    fun testNormalizationPipeline_tlNumericInput() {
        // Already numeric TL -> passes through
        val result = InputNormalizer.normalize("ka2", InputMode.TL)
        assertEquals("TL numeric ka2", "ka2", result)
    }

    @Test
    fun testNormalizationPipeline_pojNumericInput() {
        // POJ numeric stays in POJ form
        val result = InputNormalizer.normalize("koa1", InputMode.POJ)
        assertEquals("POJ numeric koa1 -> POJ koa1", "koa1", result)
    }

    @Test
    fun testNormalizationPipeline_multiSyllable() {
        val result = InputNormalizer.normalize("t\u00E2i-g\u00ED", InputMode.TL)
        assertEquals("TL multi-syllable tâi-gí", "tai5gi2", result)
    }

    // MARK: - B. Tone Mark Round-Trip (POJ mode cross-component)

    @Test
    fun testToneMarkRoundTrip_pojMode() {
        val marked = TaigiPhonetics.convertSyllable("ka2", InputMode.POJ)
        assertEquals("convertSyllable(ka2, POJ)", "k\u00E1", marked)  // ká

        val restored = ToneRestoration.restore(marked, InputMode.POJ)
        assertEquals("restore(ká, POJ)", "ka", restored)
    }

    // MARK: - C. Segmenter + Normalizer Pipeline

    @Test
    fun testSegmenterThenNormalizer_continuousInput() {
        val segments = SyllableSegmenter.segment("gua2si7")
        assertEquals("segment(gua2si7)", listOf("gua2", "si7"), segments)

        val normalized = InputNormalizer.normalize(
            segments.joinToString("-"), InputMode.TL
        )
        assertEquals("normalize(gua2-si7, TL)", "gua2si7", normalized)
    }

    @Test
    fun testSegmenterThenNormalizer_pojInput() {
        val segments = SyllableSegmenter.segment("chhi2ka1")
        assertEquals("segment(chhi2ka1)", listOf("chhi2", "ka1"), segments)

        val normalized = InputNormalizer.normalize(
            segments.joinToString("-"), InputMode.POJ
        )
        // POJ stays in POJ form
        assertEquals("normalize(chhi2-ka1, POJ)", "chhi2ka1", normalized)
    }

    @Test
    fun testSegmenterThenNormalizer_singleSyllable() {
        val segments = SyllableSegmenter.segment("lang5")
        assertEquals("segment(lang5)", listOf("lang5"), segments)

        val normalized = InputNormalizer.normalize("lang5", InputMode.TL)
        assertEquals("normalize(lang5, TL)", "lang5", normalized)
    }
}
