package com.siansiansu.taigikeyboard.ime.text.keyboard

import android.text.InputType
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pure-JVM table tests for the `isComposingCharacter` classifier that
 * gates [TextInputKeyHandler.handleTaigiInput] composing-mode entry.
 *
 * Mocking the full handler reproduces little real behaviour (5 mutable
 * collaborators + InputConnection); composing-character classification
 * is the one pure-function carve-out worth unit-locking.
 */
class TextInputKeyHandlerTest {
    @Test
    fun `isComposingCharacter accepts a-z lowercase`() {
        for (ch in 'a'..'z') {
            assertTrue("lowercase '$ch' must enter composing", isComposingCharacter(ch.toString()))
        }
    }

    @Test
    fun `isComposingCharacter accepts A-Z uppercase`() {
        for (ch in 'A'..'Z') {
            assertTrue("uppercase '$ch' must enter composing", isComposingCharacter(ch.toString()))
        }
    }

    @Test
    fun `isComposingCharacter accepts TPS bopomofo Lo block`() {
        // Spot-check across ㄅ (U+3105) … ㆷ (U+31B7) Lo range.
        val samples = listOf("ㄅ", "ㄆ", "ㄇ", "ㄉ", "ㄊ", "ㄋ", "ㄌ", "ㄍ", "ㆡ", "ㆢ", "ㆷ")
        for (s in samples) {
            assertTrue("bopomofo '$s' must enter composing", isComposingCharacter(s))
        }
    }

    @Test
    fun `isComposingCharacter accepts TPS tone marks via isLetter Lm`() {
        // ˋ (U+02CB), ˊ (U+02CA), ˇ (U+02C7), ˆ (U+02C6) are Lm — covered by isLetter.
        val samples = listOf("ˋ", "ˊ", "ˇ", "ˆ")
        for (s in samples) {
            assertTrue("tone mark '$s' must enter composing", isComposingCharacter(s))
        }
    }

    @Test
    fun `isComposingCharacter accepts TPS Sk tone marks via explicit allowlist`() {
        // ˪ (U+02EA tone 3), ˫ (U+02EB tone 7), ˙ (U+02D9 tone 8) are Sk —
        // NOT covered by isLetter; classifier whitelists them explicitly.
        assertTrue("U+02EA must enter composing", isComposingCharacter("˪"))
        assertTrue("U+02EB must enter composing", isComposingCharacter("˫"))
        assertTrue("U+02D9 must enter composing", isComposingCharacter("˙"))
    }

    @Test
    fun `isComposingCharacter accepts hyphen`() {
        assertTrue("hyphen syllable boundary must enter composing", isComposingCharacter("-"))
    }

    @Test
    fun `isComposingCharacter rejects digits`() {
        for (ch in '0'..'9') {
            assertFalse("digit '$ch' must not enter composing", isComposingCharacter(ch.toString()))
        }
    }

    @Test
    fun `isComposingCharacter rejects whitespace`() {
        assertFalse("space must not enter composing", isComposingCharacter(" "))
        assertFalse("tab must not enter composing", isComposingCharacter("\t"))
        assertFalse("newline must not enter composing", isComposingCharacter("\n"))
    }

    @Test
    fun `isComposingCharacter rejects punctuation and symbols`() {
        val samples = listOf(".", ",", "!", "?", ";", ":", "/", "@", "#", "$", "%", "&", "*", "+", "=", "(", ")")
        for (s in samples) {
            assertFalse("punctuation '$s' must not enter composing", isComposingCharacter(s))
        }
    }

    @Test
    fun `isComposingCharacter rejects empty string`() {
        assertFalse("empty string must not enter composing", isComposingCharacter(""))
    }

    @Test
    fun `isComposingCharacter inspects only the first character`() {
        // Only `firstOrNull()` is consulted — multi-char strings classify
        // by their leading code point.
        assertTrue("first letter 'a' wins", isComposingCharacter("a123"))
        assertFalse("first digit '1' wins", isComposingCharacter("1abc"))
        assertTrue("first hyphen wins", isComposingCharacter("-x"))
    }

    // lastGraphemeLength backs the rich-editor backspace (deleteSurroundingText
    // deletes one user-perceived character). JVM java.text.BreakIterator covers
    // ASCII / surrogate pairs / combining marks reliably; full ZWJ-sequence
    // grapheme behaviour is ICU-backed on-device and dogfood-verified.

    @Test
    fun `lastGraphemeLength empty string is zero`() {
        assertEquals(0, lastGraphemeLength(""))
    }

    @Test
    fun `lastGraphemeLength single character is one unit`() {
        assertEquals(1, lastGraphemeLength("a"))
        assertEquals(1, lastGraphemeLength("台"))
    }

    @Test
    fun `lastGraphemeLength returns only the trailing cluster`() {
        assertEquals(1, lastGraphemeLength("abc"))
    }

    @Test
    fun `lastGraphemeLength keeps a surrogate-pair emoji whole`() {
        // 😀 U+1F600 = 2 UTF-16 code units; backspace must remove both.
        assertEquals(2, lastGraphemeLength("😀"))
        assertEquals(2, lastGraphemeLength("hi😀"))
    }

    @Test
    fun `lastGraphemeLength keeps a base-plus-combining sequence whole`() {
        // "e" + U+0301 combining acute = one grapheme (é), 2 UTF-16 units.
        assertEquals(2, lastGraphemeLength("é"))
    }

    // resolveBackspaceDeletion is the editor-capability dispatch table. InputType
    // constants inline as primitives, so this runs on plain JVM without an
    // InputConnection mock; the IC calls that apply each action are
    // dogfood-verified.

    @Test
    fun `resolveBackspaceDeletion picks raw key event for a TYPE_NULL editor`() {
        // TYPE_NULL wins ahead of selection / context.
        assertEquals(
            BackspaceDeletion.RawKeyEvent,
            resolveBackspaceDeletion(InputType.TYPE_NULL, hasSelection = false, textBefore = null),
        )
        assertEquals(
            BackspaceDeletion.RawKeyEvent,
            resolveBackspaceDeletion(InputType.TYPE_NULL, hasSelection = true, textBefore = "abc"),
        )
    }

    @Test
    fun `resolveBackspaceDeletion deletes the selection in a rich editor`() {
        assertEquals(
            BackspaceDeletion.Selection,
            resolveBackspaceDeletion(InputType.TYPE_CLASS_TEXT, hasSelection = true, textBefore = null),
        )
    }

    @Test
    fun `resolveBackspaceDeletion falls back to code point when context is null`() {
        assertEquals(
            BackspaceDeletion.CodePoint,
            resolveBackspaceDeletion(InputType.TYPE_CLASS_TEXT, hasSelection = false, textBefore = null),
        )
    }

    @Test
    fun `resolveBackspaceDeletion is a no-op at the field start`() {
        assertEquals(
            BackspaceDeletion.NoOp,
            resolveBackspaceDeletion(InputType.TYPE_CLASS_TEXT, hasSelection = false, textBefore = ""),
        )
    }

    @Test
    fun `resolveBackspaceDeletion deletes one grapheme of context`() {
        assertEquals(
            BackspaceDeletion.Grapheme(1),
            resolveBackspaceDeletion(InputType.TYPE_CLASS_TEXT, hasSelection = false, textBefore = "abc"),
        )
        // Trailing surrogate-pair emoji = one grapheme = 2 UTF-16 units.
        assertEquals(
            BackspaceDeletion.Grapheme(2),
            resolveBackspaceDeletion(InputType.TYPE_CLASS_TEXT, hasSelection = false, textBefore = "hi😀"),
        )
    }
}
