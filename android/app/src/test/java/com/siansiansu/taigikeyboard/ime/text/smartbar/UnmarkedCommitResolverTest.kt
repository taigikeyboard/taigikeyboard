package com.siansiansu.taigikeyboard.ime.text.smartbar

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins the UNMARKED commit contract: the document string and the auto-space
 * verdict come from the SAME arm, so the gate can never disagree with what
 * went into the document (§23 + §34).
 */
class UnmarkedCommitResolverTest {
    private fun resolve(
        hanzi: String?,
        effectiveSwapped: Boolean,
        outputBothScripts: Boolean,
    ) = resolveUnmarkedCommit(
        roman = "tâi-gí",
        bracketRoman = "tâi-gí",
        hanzi = hanzi,
        effectiveSwapped = effectiveSwapped,
        outputBothScripts = outputBothScripts,
    )

    @Test
    fun romanLed_commitsTheRomanization_andEarnsTheSpace() {
        val resolved = resolve(hanzi = "台語", effectiveSwapped = false, outputBothScripts = false)
        assertEquals("tâi-gí", resolved.text)
        assertTrue(resolved.wroteRomanization)
    }

    @Test
    fun hanjiLed_commitsTheHanji_andEarnsNoSpace() {
        val resolved = resolve(hanzi = "台語", effectiveSwapped = true, outputBothScripts = false)
        assertEquals("台語", resolved.text)
        assertFalse(resolved.wroteRomanization)
    }

    @Test
    fun brackets_writeThePairEitherWayRound_soBothEarnTheSpace() {
        val hanjiLed = resolve(hanzi = "台語", effectiveSwapped = true, outputBothScripts = true)
        assertEquals("台語 (tâi-gí)", hanjiLed.text)
        assertTrue(hanjiLed.wroteRomanization)

        val romanLed = resolve(hanzi = "台語", effectiveSwapped = false, outputBothScripts = true)
        assertEquals("tâi-gí (台語)", romanLed.text)
        assertTrue(romanLed.wroteRomanization)
    }

    /**
     * trace: [resolveUnmarkedCommit] — the hanji-absent arm. Romanization
     * under EVERY mode, including the ones the old mode proxy called a hanji
     * commit.
     */
    @Test
    fun noHanji_commitsTheRomanizationUnderEveryMode() {
        for (hanzi in listOf(null, "")) {
            for (effectiveSwapped in listOf(false, true)) {
                for (outputBothScripts in listOf(false, true)) {
                    val resolved = resolve(hanzi, effectiveSwapped, outputBothScripts)
                    assertEquals("tâi-gí", resolved.text)
                    assertTrue(
                        "hanzi=$hanzi swapped=$effectiveSwapped both=$outputBothScripts",
                        resolved.wroteRomanization,
                    )
                }
            }
        }
    }

    /** Enter writes the composition as typed: romanization in TL/POJ, Bopomofo in TPS. */
    @Test
    fun rawCommit_writesRomanizationUnlessTheLayoutComposesBopomofo() {
        assertTrue(rawPreeditWritesRomanization(isTPSLayout = false))
        assertFalse(rawPreeditWritesRomanization(isTPSLayout = true))
    }
}
