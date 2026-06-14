package com.siansiansu.taigikeyboard.ime.text.composing

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class EnglishWordMatcherTest {

    // Small deterministic fixture — NOT the bundled asset. Frequencies chosen so
    // prefix ranking (help > hello > held) is unambiguous.
    private fun fixtureMatcher(): EnglishWordMatcher = EnglishWordMatcher(
        listOf(
            EnglishWordMatcher.WordEntry("the", 1000),
            EnglishWordMatcher.WordEntry("help", 800),
            EnglishWordMatcher.WordEntry("hello", 500),
            EnglishWordMatcher.WordEntry("held", 300),
            EnglishWordMatcher.WordEntry("world", 700),
            EnglishWordMatcher.WordEntry("receive", 600),
            EnglishWordMatcher.WordEntry("test", 900),
            EnglishWordMatcher.WordEntry("testing", 200),
        ),
    )

    @Test
    fun suggest_prefix_returnsCompletionsRankedByFrequency() {
        val result = fixtureMatcher().suggest("hel", maxResults = 3)
        assertEquals(listOf("help", "hello", "held"), result)
    }

    @Test
    fun suggest_correction_transpositionCorrectsToTheTopHit() {
        // teh → the via a single adjacent transposition (OSA distance 1).
        val result = fixtureMatcher().suggest("teh", maxResults = 3)
        assertEquals("the", result.first())
    }

    @Test
    fun suggest_correction_recieveCorrectsToReceive() {
        val result = fixtureMatcher().suggest("recieve", maxResults = 3)
        assertEquals("receive", result.first())
    }

    @Test
    fun suggest_correction_wroldCorrectsToWorld() {
        val result = fixtureMatcher().suggest("wrold", maxResults = 3)
        assertTrue("expected 'world' in $result", "world" in result)
    }

    @Test
    fun suggest_casing_titleCaseInputProducesTitleCaseOutput() {
        val result = fixtureMatcher().suggest("Hel", maxResults = 3)
        assertEquals(listOf("Help", "Hello", "Held"), result)
    }

    @Test
    fun suggest_casing_allCapsInputProducesAllCapsOutput() {
        val result = fixtureMatcher().suggest("HEL", maxResults = 3)
        assertEquals(listOf("HELP", "HELLO", "HELD"), result)
    }

    @Test
    fun suggest_distance2_isIncludedButDistance3IsExcluded() {
        // woXYd → world is OSA distance 2 (two substitutions) → included.
        assertTrue("world" in fixtureMatcher().suggest("woXYd", maxResults = 3))
        // wXYZd → world is OSA distance 3 → excluded; nothing else matches.
        assertEquals(emptyList<String>(), fixtureMatcher().suggest("wXYZd", maxResults = 3))
    }

    @Test
    fun suggest_shortToken_skipsCorrection() {
        // "te" (length 2 < MIN_CORRECTION_LENGTH) → prefix only; "the" (a distance-1
        // correction) must NOT surface.
        val result = fixtureMatcher().suggest("te", maxResults = 3)
        assertEquals(listOf("test", "testing"), result)
        assertFalse("the" in result)
    }

    @Test
    fun suggest_exactWord_outranksHigherFrequencyCompletion() {
        val matcher = EnglishWordMatcher(
            listOf(
                EnglishWordMatcher.WordEntry("service", 100),
                EnglishWordMatcher.WordEntry("services", 999), // higher-frequency completion
            ),
        )
        // The exact typed word ranks first despite the completion's higher frequency.
        assertEquals(listOf("service", "services"), matcher.suggest("service", maxResults = 3))
    }

    @Test
    fun suggest_apostropheToken_producesNoNoiseCorrection() {
        val matcher = EnglishWordMatcher(
            listOf(
                EnglishWordMatcher.WordEntry("done", 900),
                EnglishWordMatcher.WordEntry("font", 800),
            ),
        )
        // "don't" carries an apostrophe → correction is skipped, so unrelated words
        // (done/font) are not offered; no word starts with "don't" → empty.
        assertEquals(emptyList<String>(), matcher.suggest("don't", maxResults = 3))
    }

    @Test
    fun suggest_returnsNoDuplicateWords() {
        // "help" is both an exact hit and reachable as a correction target of nearby
        // words; the prefix-dedup must keep each word at most once.
        val result = fixtureMatcher().suggest("help", maxResults = 3)
        assertEquals(result.distinct(), result)
    }

    @Test
    fun suggest_prefixOutranksCorrection() {
        // "help" is an exact/prefix hit; "held" is only a distance-1 correction.
        val result = fixtureMatcher().suggest("help", maxResults = 3)
        assertEquals("help", result.first())
    }

    @Test
    fun suggest_capsResultsAtMaxResults() {
        val result = fixtureMatcher().suggest("hel", maxResults = 2)
        assertEquals(listOf("help", "hello"), result)
    }

    @Test
    fun suggest_noMatch_returnsEmpty() {
        assertEquals(emptyList<String>(), fixtureMatcher().suggest("qzxwv", maxResults = 3))
    }

    @Test
    fun suggest_emptyToken_returnsEmpty() {
        assertEquals(emptyList<String>(), fixtureMatcher().suggest("", maxResults = 3))
    }

    @Test
    fun parseEntries_parsesTabSeparatedAndSkipsMalformed() {
        val entries = EnglishWordMatcher.parseEntries(
            sequenceOf(
                "the\t1000",
                "help\t800",
                "noTabHere",      // no tab → skipped
                "bad\tNaN",       // non-numeric frequency → skipped
                "\t500",          // empty word (tab at index 0) → skipped
            ),
        )
        assertEquals(2, entries.size)
        assertEquals(EnglishWordMatcher.WordEntry("the", 1000), entries[0])
        assertEquals(EnglishWordMatcher.WordEntry("help", 800), entries[1])
    }
}
