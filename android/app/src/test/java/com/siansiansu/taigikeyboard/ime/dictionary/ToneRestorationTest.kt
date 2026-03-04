package com.siansiansu.taigikeyboard.ime.dictionary

import com.siansiansu.taigikeyboard.ime.dictionary.ToneConverterModels.InputMode
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * ToneRestoration unit tests
 *
 * Ported from iOS ToneRestorationTests.swift
 */
class ToneRestorationTest {

    // MARK: - Individual Combining Mark Removal

    @Test
    fun testRestore_acuteAccent() {
        val result = ToneRestoration.restore("k\u00E1", InputMode.TL)  // ká
        assertEquals("restore(\"ká\")", "ka", result)
    }

    @Test
    fun testRestore_graveAccent() {
        val result = ToneRestoration.restore("k\u00E0", InputMode.TL)  // kà
        assertEquals("restore(\"kà\")", "ka", result)
    }

    @Test
    fun testRestore_circumflex() {
        val result = ToneRestoration.restore("k\u00E2", InputMode.TL)  // kâ
        assertEquals("restore(\"kâ\")", "ka", result)
    }

    @Test
    fun testRestore_macron() {
        val result = ToneRestoration.restore("k\u0101", InputMode.TL)  // kā
        assertEquals("restore(\"kā\")", "ka", result)
    }

    @Test
    fun testRestore_verticalLine() {
        val result = ToneRestoration.restore("ka\u030D", InputMode.TL)  // ka̍
        assertEquals("restore(\"ka̍\")", "ka", result)
    }

    @Test
    fun testRestore_breve() {
        val result = ToneRestoration.restore("k\u0103", InputMode.POJ)  // kă
        assertEquals("restore(\"kă\")", "ka", result)
    }

    @Test
    fun testRestore_doubleAcute() {
        val result = ToneRestoration.restore("ka\u030B", InputMode.TL)
        assertEquals("restore(\"ka̋\")", "ka", result)
    }

    // MARK: - Multi-Syllable Behavior

    @Test
    fun testRestore_multiSyllable_removesLastMarkOnly() {
        // tâi-gí -> should remove mark from gí (last mark) -> tâi-gi
        val result = ToneRestoration.restore("t\u00E2i-g\u00ED", InputMode.TL)
        assertNotNull("restore(\"tâi-gí\") should not be null", result)
        // The last mark (acute on í) should be removed
        assertTrue("Last mark should be removed: got $result", result!!.contains("gi"))
        // The first mark (circumflex on â) should remain
        assertTrue(
            "First mark should be preserved: got $result",
            result.contains("\u00E2") || result.contains("a\u0302")
        )
    }

    // MARK: - No Mark / Empty

    @Test
    fun testRestore_noMark_returnsNull() {
        val result = ToneRestoration.restore("ka", InputMode.TL)
        assertNull("restore(\"ka\") should be null (no tone mark)", result)
    }

    @Test
    fun testRestore_empty_returnsNull() {
        val result = ToneRestoration.restore("", InputMode.TL)
        assertNull("restore(\"\") should be null", result)
    }

    @Test
    fun testRestore_plainDigit_returnsNull() {
        val result = ToneRestoration.restore("ka2", InputMode.TL)
        assertNull("restore(\"ka2\") should be null (digit is not a combining mark)", result)
    }

    // MARK: - Complex Syllables

    @Test
    fun testRestore_complexSyllable() {
        // tshiū -> remove macron -> tshiu
        val result = ToneRestoration.restore("tshi\u016B", InputMode.TL)
        assertEquals("restore(\"tshiū\")", "tshiu", result)
    }
}
