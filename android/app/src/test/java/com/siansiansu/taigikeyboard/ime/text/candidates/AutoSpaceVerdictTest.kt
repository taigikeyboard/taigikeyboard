package com.siansiansu.taigikeyboard.ime.text.candidates

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins the platform auto-space predicate the commits the engine does not
 * resolve read (an English word, the raw preedit); a Continuous or
 * prediction tap takes the engine's `earns_auto_space`. Mirrors iOS
 * `ActionHandlerCommitRulesTests`.
 */
class AutoSpaceVerdictTest {
    /**
     * The auto-space predicate itself: setting ON + romanization written +
     * the committed DOCUMENT string not ending in a hyphen continuation.
     */
    @Test
    fun shouldAppendAutoSpace_requiresSettingRomanizationAndNoHyphenTail() {
        assertTrue(shouldAppendAutoSpace(isAutoSpaceEnabled = true, wroteRomanization = true, committedText = "tâi-gí"))
        assertFalse(
            "setting off",
            shouldAppendAutoSpace(isAutoSpaceEnabled = false, wroteRomanization = true, committedText = "tâi-gí"),
        )
        assertFalse(
            "pure hanji commit wrote no romanization",
            shouldAppendAutoSpace(isAutoSpaceEnabled = true, wroteRomanization = false, committedText = "台語"),
        )
        assertFalse(
            "trailing hyphen = 連字 continuation, keep composing",
            shouldAppendAutoSpace(isAutoSpaceEnabled = true, wroteRomanization = true, committedText = "tâi-"),
        )
    }

    /** Enter writes the composition as typed: romanization in TL/POJ, Bopomofo in TPS. */
    @Test
    fun rawCommit_writesRomanizationUnlessTheLayoutComposesBopomofo() {
        assertTrue(rawPreeditWritesRomanization(isTPSLayout = false))
        assertFalse(rawPreeditWritesRomanization(isTPSLayout = true))
    }
}
