package com.siansiansu.taigikeyboard.ime.core.settings

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins the 候選詞顯示 storage coercion + the two platform-side derivation
 * rules (`effectiveTranslateSwapped` / `effectiveOutputBothScripts`).
 * `PrefHelper` itself needs a real Android `Context` (DataStore), so the
 * rules are tested at the enum seam it delegates to — same rationale as
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
        assertEquals("combined", CandidateDisplayMode.COMBINED.storageValue)
        assertEquals(CandidateDisplayMode.COMBINED, CandidateDisplayMode.fromStorage("combined"))
    }

    /**
     * A stored `true` reads `false` under ROMAN_ONLY and comes back once
     * the mode returns to SIDE_BY_SIDE — storage is never rewritten.
     */
    @Test
    fun test_INVARIANT_roman_only_suppresses_stored_script_flags_without_clearing_them() {
        val storedIsTranslateSwapped = true
        val storedOutputBothScripts = true

        assertFalse(CandidateDisplayMode.ROMAN_ONLY.effectiveTranslateSwapped(storedIsTranslateSwapped))
        assertFalse(CandidateDisplayMode.ROMAN_ONLY.effectiveOutputBothScripts(storedOutputBothScripts))

        // Leaving roman-only restores the stored choice.
        assertTrue(CandidateDisplayMode.SIDE_BY_SIDE.effectiveTranslateSwapped(storedIsTranslateSwapped))
        assertTrue(CandidateDisplayMode.SIDE_BY_SIDE.effectiveOutputBothScripts(storedOutputBothScripts))

        // A stored `false` stays false in every mode but COMBINED's swap (pinned below).
        assertFalse(CandidateDisplayMode.SIDE_BY_SIDE.effectiveTranslateSwapped(false))
        assertFalse(CandidateDisplayMode.SIDE_BY_SIDE.effectiveOutputBothScripts(false))
        assertFalse(CandidateDisplayMode.ROMAN_ONLY.effectiveTranslateSwapped(false))
        assertFalse(CandidateDisplayMode.ROMAN_ONLY.effectiveOutputBothScripts(false))
    }

    /**
     * COMBINED projects to "cell leads with hanji, commit writes hanji":
     * swap reads `true` whatever is stored, 括號標註 keeps the stored value.
     * Storage is untouched, so SIDE_BY_SIDE restores the user's choice.
     */
    @Test
    fun test_INVARIANT_combined_forces_swap_true_and_passes_output_both_through() {
        // stored (swap=false, both=false) → effective (true, false)
        assertTrue(CandidateDisplayMode.COMBINED.effectiveTranslateSwapped(false))
        assertFalse(CandidateDisplayMode.COMBINED.effectiveOutputBothScripts(false))

        // stored (swap=false, both=true) → effective (true, true)
        assertTrue(CandidateDisplayMode.COMBINED.effectiveTranslateSwapped(false))
        assertTrue(CandidateDisplayMode.COMBINED.effectiveOutputBothScripts(true))

        // stored swap=true is also true (idempotent projection).
        assertTrue(CandidateDisplayMode.COMBINED.effectiveTranslateSwapped(true))

        // Back to SIDE_BY_SIDE: the stored `false` swap is what the user sees again.
        assertFalse(CandidateDisplayMode.SIDE_BY_SIDE.effectiveTranslateSwapped(false))
        assertTrue(CandidateDisplayMode.SIDE_BY_SIDE.effectiveOutputBothScripts(true))
    }
}
