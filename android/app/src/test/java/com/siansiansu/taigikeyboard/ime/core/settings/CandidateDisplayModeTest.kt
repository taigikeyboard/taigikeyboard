package com.siansiansu.taigikeyboard.ime.core.settings

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins the 候選詞顯示 storage coercion + the ONE platform-side derivation
 * rule (`effective = stored && mode != ROMAN_ONLY`). `PrefHelper` itself
 * needs a real Android `Context` (DataStore), so the rule is tested at
 * the enum seam it delegates to — same rationale as
 * `EngineSettingsLiveReadTest`.
 */
class CandidateDisplayModeTest {
    @Test
    fun fromStorage_roundTripsEveryMode() {
        for (mode in CandidateDisplayMode.entries) {
            assertEquals(mode, CandidateDisplayMode.fromStorage(mode.storageValue))
        }
    }

    @Test
    fun fromStorage_unknownOrAbsentRaw_fallsBackToSideBySide() {
        assertEquals(CandidateDisplayMode.SIDE_BY_SIDE, CandidateDisplayMode.fromStorage("garbage"))
        assertEquals(CandidateDisplayMode.SIDE_BY_SIDE, CandidateDisplayMode.fromStorage(""))
        assertEquals(CandidateDisplayMode.SIDE_BY_SIDE, CandidateDisplayMode.fromStorage(null))
    }

    @Test
    fun storageValues_matchCrossPlatformRawStrings() {
        assertEquals("sideBySide", CandidateDisplayMode.SIDE_BY_SIDE.storageValue)
        assertEquals("romanOnly", CandidateDisplayMode.ROMAN_ONLY.storageValue)
    }

    /**
     * A stored `true` reads `false` under ROMAN_ONLY and comes back once
     * the mode returns to SIDE_BY_SIDE — storage is never rewritten.
     */
    @Test
    fun test_INVARIANT_roman_only_suppresses_stored_script_flags_without_clearing_them() {
        val storedIsTranslateSwapped = true
        val storedOutputBothScripts = true

        assertFalse(CandidateDisplayMode.ROMAN_ONLY.effectiveScriptFlag(storedIsTranslateSwapped))
        assertFalse(CandidateDisplayMode.ROMAN_ONLY.effectiveScriptFlag(storedOutputBothScripts))

        // Leaving roman-only restores the stored choice.
        assertTrue(CandidateDisplayMode.SIDE_BY_SIDE.effectiveScriptFlag(storedIsTranslateSwapped))
        assertTrue(CandidateDisplayMode.SIDE_BY_SIDE.effectiveScriptFlag(storedOutputBothScripts))

        // A stored `false` stays false in every mode.
        assertFalse(CandidateDisplayMode.SIDE_BY_SIDE.effectiveScriptFlag(false))
        assertFalse(CandidateDisplayMode.ROMAN_ONLY.effectiveScriptFlag(false))
    }
}
