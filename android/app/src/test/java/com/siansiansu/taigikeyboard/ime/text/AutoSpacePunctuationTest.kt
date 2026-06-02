package com.siansiansu.taigikeyboard.ime.text

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pure-JVM table tests for the attaching-punctuation set driving the
 * auto-space "smart punctuation" swap. INVARIANT_AUTO_SPACE_PUNCTUATION_SWAP —
 * mirrors iOS `AutoSpacePunctuationTests`; the two sets must stay identical.
 */
class AutoSpacePunctuationTest {
    @Test
    fun `INVARIANT attaching sentence-end`() {
        for (ch in listOf("。", "！", "？", ".", "!", "?")) {
            assertTrue("sentence-end '$ch' must attach", AutoSpacePunctuation.isAttaching(ch))
        }
    }

    @Test
    fun `INVARIANT attaching clause separators`() {
        for (ch in listOf("，", ",", "、", "；", ";", "：", ":")) {
            assertTrue("clause '$ch' must attach", AutoSpacePunctuation.isAttaching(ch))
        }
    }

    @Test
    fun `INVARIANT attaching closing brackets and quotes`() {
        for (ch in listOf(")", "）", "]", "】", "」", "』")) {
            assertTrue("closing '$ch' must attach", AutoSpacePunctuation.isAttaching(ch))
        }
    }

    @Test
    fun `INVARIANT opening brackets and quotes do not attach`() {
        for (ch in listOf("(", "（", "[", "【", "「", "『")) {
            assertFalse("opening '$ch' must NOT attach", AutoSpacePunctuation.isAttaching(ch))
        }
    }

    @Test
    fun `INVARIANT ascii straight quotes do not attach`() {
        for (ch in listOf("\"", "'")) {
            assertFalse("straight quote '$ch' must NOT attach", AutoSpacePunctuation.isAttaching(ch))
        }
    }

    @Test
    fun `INVARIANT letters and digits do not attach`() {
        for (ch in listOf("a", "A", "5", "-", "我", "â")) {
            assertFalse("'$ch' must NOT attach", AutoSpacePunctuation.isAttaching(ch))
        }
    }

    @Test
    fun `INVARIANT multi-char or empty does not attach`() {
        assertFalse(AutoSpacePunctuation.isAttaching(""))
        assertFalse(AutoSpacePunctuation.isAttaching("?!"))
        assertFalse(AutoSpacePunctuation.isAttaching("? "))
    }
}
