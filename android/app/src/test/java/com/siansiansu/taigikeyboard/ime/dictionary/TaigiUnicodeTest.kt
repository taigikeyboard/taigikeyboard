package com.siansiansu.taigikeyboard.ime.dictionary

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Pure-engine tests for [TaigiUnicode]. Mirrors the fixtures exercised by
 * iOS `TaigiUnicodeTests.swift`. Labels match `behavioral-invariants.md` §2.
 */
class TaigiUnicodeTest {
    @Test
    fun `INVARIANT_nfd_preprocessed_idempotent_on_ascii`() {
        val cases =
            listOf(
                "" to "",
                "a" to "a",
                "abc" to "abc",
                "ka2" to "ka2",
                "tshiu7" to "tshiu7",
                "hello world" to "hello world",
            )
        for ((input, expected) in cases) {
            assertEquals(
                "nfdPreprocessed($input)",
                expected,
                TaigiUnicode.nfdPreprocessed(input),
            )
        }
    }

    @Test
    fun `INVARIANT_nfd_preprocessed_nasal_marker_substitution`() {
        assertEquals("sann", TaigiUnicode.nfdPreprocessed("saⁿ"))
        assertEquals("sann", TaigiUnicode.nfdPreprocessed("saᴺ"))
        assertEquals("annbnn", TaigiUnicode.nfdPreprocessed("aⁿbᴺ"))
        assertEquals("kua", TaigiUnicode.nfdPreprocessed("kua"))
    }

    @Test
    fun `INVARIANT_nfd_preprocessed_o_combining_dot_collapses`() {
        // POJ `o͘` decomposed = `o + ͘`; preprocess replaces the combining
        // dot with `o`, yielding `oo`.
        assertEquals("hoo", TaigiUnicode.nfdPreprocessed("ho͘"))
    }

    @Test
    fun `INVARIANT_nfd_preprocessed_repeated_o_combining_dot`() {
        assertEquals("hoolooo", TaigiUnicode.nfdPreprocessed("ho͘lo͘o"))
    }

    @Test
    fun `INVARIANT_nfd_preprocessed_keeps_tone_combining_marks`() {
        // `͘` (POJ dot) is rewritten to `o`; tone diacritics like
        // combining acute (`́`) stay decomposed for callers to walk.
        assertEquals("ho\u0301o", TaigiUnicode.nfdPreprocessed("ho\u0358\u0301"))
    }
}
