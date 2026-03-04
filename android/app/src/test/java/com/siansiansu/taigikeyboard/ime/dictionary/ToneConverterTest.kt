package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * ToneConverter unit tests
 *
 * Ported from iOS ToneConverterTests.swift
 */
class ToneConverterTest {

    // MARK: - TL Mode (No Preprocessing)

    @Test
    fun testConvertToToneMarks_tlMode_multiSyllable() {
        val result = ToneConverter.convertToToneMarks("ho2-se3", mode = InputMode.TL)
        assertEquals("TL ho2-se3", "h\u00F3-s\u00E8", result)  // hó-sè
    }

    @Test
    fun testConvertToToneMarks_tlMode_noPreprocessing() {
        // In TL mode, "oo" stays "oo" (no o͘ conversion), tone 2 = acute
        val result = ToneConverter.convertToToneMarks("hoo2", mode = InputMode.TL)
        assertEquals("TL hoo2", "h\u00F3o", result)  // hóo
    }

    // MARK: - POJ Mode + OO

    @Test
    fun testConvertToToneMarks_pojMode_ooEnabled() {
        val result = ToneConverter.convertToToneMarks("hoo2", mode = InputMode.POJ)
        assertTrue(
            "POJ should produce o\u0358: got $result",
            result.any { it.code == 0x0358 }
        )
    }

    // MARK: - POJ Mode + NN

    @Test
    fun testConvertToToneMarks_pojMode_nnEnabled() {
        val result = ToneConverter.convertToToneMarks("ann2", mode = InputMode.POJ)
        assertTrue(
            "POJ should produce \u207F: got $result",
            result.contains("\u207F")
        )
    }

    // MARK: - POJ Mode: oo→o͘ Always Applies

    @Test
    fun testConvertToToneMarks_pojMode_ooAlwaysApplies() {
        // oo→o͘ always applies in POJ mode.
        // The enableDoubleTapOO setting only controls the keyboard shortcut, not output.
        val result = ToneConverter.convertToToneMarks("hoo2", mode = InputMode.POJ)
        assertTrue(
            "POJ should always produce o\u0358: got $result",
            result.any { it.code == 0x0358 }
        )
    }

    // MARK: - POJ Mode: nn→ⁿ Always Applies

    @Test
    fun testConvertToToneMarks_pojMode_nnAlwaysApplies() {
        // nn→ⁿ always applies in POJ mode.
        // The enableDoubleTapNN setting only controls the keyboard shortcut, not output.
        val result = ToneConverter.convertToToneMarks("ann2", mode = InputMode.POJ)
        assertTrue(
            "POJ should always produce \u207F: got $result",
            result.contains("\u207F")
        )
    }

    // MARK: - English Passthrough

    @Test
    fun testConvertToToneMarks_englishMode() {
        val result = ToneConverter.convertToToneMarks("hello2", mode = InputMode.ENGLISH)
        assertEquals("English mode should pass through without conversion", "hello2", result)
    }

    @Test
    fun testConvertToToneMarks_englishMode_multiSyllable() {
        val result = ToneConverter.convertToToneMarks("ka2-lang5", mode = InputMode.ENGLISH)
        assertEquals("ka2-lang5", result)
    }

    // MARK: - Nasal Marker Case

    @Test
    fun testConvertToToneMarks_pojMode_capsLockNN() {
        val result = ToneConverter.convertToToneMarks("ANN2", mode = InputMode.POJ)
        // Caps lock: uppercase vowel A → nasal should be ᴺ (U+1D3A)
        assertTrue(
            "Caps lock ANN2 should produce \u1D3A (U+1D3A): got $result",
            result.contains("\u1D3A")
        )
    }

    @Test
    fun testConvertToToneMarks_pojMode_singleShiftNN() {
        val result = ToneConverter.convertToToneMarks("Penn5", mode = InputMode.POJ)
        // Single shift: P uppercase but e lowercase → nasal follows e → ⁿ (U+207F)
        assertTrue(
            "Single shift Penn5 should produce \u207F (U+207F): got $result",
            result.contains("\u207F")
        )
        assertTrue(
            "Single shift Penn5 should NOT produce \u1D3A: got $result",
            !result.contains("\u1D3A")
        )
    }

    // MARK: - Edge Cases

    @Test
    fun testConvertToToneMarks_emptyString() {
        assertEquals("TL empty", "", ToneConverter.convertToToneMarks("", mode = InputMode.TL))
        assertEquals("POJ empty", "", ToneConverter.convertToToneMarks("", mode = InputMode.POJ))
    }

    @Test
    fun testConvertToToneMarks_pojMode_uppercaseOO() {
        val result1 = ToneConverter.convertToToneMarks("Oo2", mode = InputMode.POJ)
        assertTrue(
            "Uppercase Oo should also convert: got $result1",
            result1.any { it.code == 0x0358 }
        )
    }
}
