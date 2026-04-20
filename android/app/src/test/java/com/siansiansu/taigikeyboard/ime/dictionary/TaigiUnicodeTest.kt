package com.siansiansu.taigikeyboard.ime.dictionary

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Test

/**
 * Pure-engine tests for [TaigiUnicode]. Mirrors the fixtures exercised by
 * iOS `TaigiUnicodeTests.swift`. Labels match `behavioral-invariants.md` §2.
 *
 * Parity with iOS is asserted inline by matching the same inputs iOS
 * checks against the same expected byte sequences. A cross-platform
 * fixture export (shared file under `docs/architecture/fixtures/`) is
 * tracked separately as FU-A2 — A9 asserts parity by re-stating the
 * iOS fixture values verbatim in this file.
 *
 * NOTE on `o͘` semantics: `nfdPreprocessed` maps the U+0358 combining
 * scalar to the literal character `"o"` (not to the empty string) so a
 * base `o` followed by U+0358 becomes `"oo"` (the TL spelling). The test
 * labels below assert that normalized Android output matches the
 * documented TL form.
 */
class TaigiUnicodeTest {
    @Test
    fun test_INVARIANT_nfd_preprocessed_platform_parity() {
        // (input, expected) pairs mirroring iOS `TaigiUnicodeTests` fixtures.
        val fixtures =
            listOf(
                // ASCII + numeric tone — identity.
                "kua2" to "kua2",
                // POJ acute accent decomposes to base + combining scalar.
                "\u00E1" to "a\u0301",
                // `o + U+0358` maps to `oo` (TL spelling — the combining
                // mark becomes the literal "o" per `nfdPreprocessed` step 3).
                "ho\u0358" to "hoo",
                // POJ nasal ⁿ (U+207F) → "nn" BEFORE NFD hop.
                "sa\u207F" to "sann",
                // POJ modifier nasal ᴺ (U+1D3A) → "nn".
                "sa\u1D3A" to "sann",
                // Tone mark + U+0358 both present. NFD canonical-orders
                // combining marks: U+0301 (class 230) before U+0358 (class
                // 233). Step 3 then maps U+0358 to literal "o".
                "ho\u0358\u0301" to "ho\u0301o",
                // Already-NFD input is preserved across the hop.
                "a\u0301" to "a\u0301",
                // Empty input is a fixed point.
                "" to "",
            )
        for ((input, expected) in fixtures) {
            assertEquals(
                "nfdPreprocessed must match iOS byte-for-byte on '$input'",
                expected,
                TaigiUnicode.nfdPreprocessed(input),
            )
        }
    }

    /**
     * POJ nasal marker variants `ⁿ` (U+207F) and `ᴺ` (U+1D3A) both
     * substitute to the literal digraph `"nn"` before any NFD step.
     */
    @Test
    fun test_INVARIANT_poj_nasal_to_nn_substitution() {
        assertEquals("sann", TaigiUnicode.nfdPreprocessed("sa\u207F"))
        assertEquals("sann", TaigiUnicode.nfdPreprocessed("sa\u1D3A"))
        // Multiple nasals in one string are all substituted.
        assertEquals("annbnn", TaigiUnicode.nfdPreprocessed("a\u207Fb\u1D3A"))
        // A syllable with no nasal is unaffected by the substitution step.
        assertEquals("kua", TaigiUnicode.nfdPreprocessed("kua"))
    }

    /**
     * The `o͘` combining scalar (U+0358 on top of base `o`) maps to the
     * literal character `"o"` during the preprocessing hop, producing TL
     * `"oo"`. The output MUST NOT contain U+0358 verbatim in any case.
     */
    @Test
    fun test_INVARIANT_o_dot_combining_collapse() {
        // Base `o` plus combining dot → TL `oo`.
        val single = TaigiUnicode.nfdPreprocessed("ho\u0358")
        assertEquals("hoo", single)
        assertFalse("U+0358 must be absent from output", single.contains('\u0358'))

        // Multiple occurrences in a word — each U+0358 maps to a literal "o",
        // so `hoˍloˍo` (where ˍ marks U+0358 for clarity) becomes `hoolooo`.
        val multi = TaigiUnicode.nfdPreprocessed("ho\u0358lo\u0358o")
        assertEquals("hoolooo", multi)
        assertFalse("U+0358 must be absent from multi-occurrence output", multi.contains('\u0358'))

        // Combining tone mark preserved; U+0358 still maps to literal "o"
        // after NFD canonical reorder moves U+0301 ahead of U+0358.
        val withTone = TaigiUnicode.nfdPreprocessed("ho\u0358\u0301")
        assertEquals("ho\u0301o", withTone)
    }
}
