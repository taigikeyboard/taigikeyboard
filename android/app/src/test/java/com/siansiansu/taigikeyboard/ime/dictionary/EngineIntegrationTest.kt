package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Cross-component engine integration tests
 *
 * Ported from iOS EngineIntegrationTests.swift
 * Tests the interaction between InputNormalizer, TaigiPhonetics,
 * and ToneRestoration.
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

}
