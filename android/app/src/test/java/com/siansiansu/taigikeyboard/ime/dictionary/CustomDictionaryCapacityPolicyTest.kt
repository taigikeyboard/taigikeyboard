// Verifies the v3.6.1 R4 custom-dictionary capacity policy. The pure decision
// helpers take a row count instead of a SQLiteDatabase, so the cap boundary is
// exercised directly here without inserting 30000 rows (and without Robolectric).
// The DB-bound helpers (currentEntryCount / entryExists / guardInsertCapacity)
// + grandfather-on-import behavior are dogfood-pinned (S14) — same JVM-can't-load
// constraint as the R3 cross-mode round.

package com.siansiansu.taigikeyboard.ime.dictionary

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class CustomDictionaryCapacityPolicyTest {
    /**
     * INVARIANT_CUSTOM_DICT_CAPACITY — the row cap is 30000, pinned on both
     * platforms (iOS `CustomDictionaryCapacityPolicy.maxEntries`).
     */
    @Test
    fun INVARIANT_CUSTOM_DICT_CAPACITY_maxEntriesPinned() {
        assertEquals(30_000, CustomDictionaryCapacityPolicy.MAX_ENTRIES)
    }

    /**
     * INVARIANT_CUSTOM_DICT_CAPACITY — a NEW insert is blocked only once the
     * count reaches the cap; an existing-row update never exceeds (bypass).
     */
    @Test
    fun INVARIANT_CUSTOM_DICT_CAPACITY_wouldExceedCapBoundary() {
        val max = CustomDictionaryCapacityPolicy.MAX_ENTRIES

        // New insert: allowed below cap, blocked at/over cap.
        assertFalse(CustomDictionaryCapacityPolicy.wouldExceedCap(currentCount = 0, isExistingRow = false))
        assertFalse(CustomDictionaryCapacityPolicy.wouldExceedCap(currentCount = max - 1, isExistingRow = false))
        assertTrue(CustomDictionaryCapacityPolicy.wouldExceedCap(currentCount = max, isExistingRow = false))
        assertTrue(CustomDictionaryCapacityPolicy.wouldExceedCap(currentCount = max + 5, isExistingRow = false))

        // Existing-row update: never an insert, never exceeds — even at/over cap.
        assertFalse(CustomDictionaryCapacityPolicy.wouldExceedCap(currentCount = max, isExistingRow = true))
        assertFalse(CustomDictionaryCapacityPolicy.wouldExceedCap(currentCount = max + 5, isExistingRow = true))
    }

    /**
     * INVARIANT_CUSTOM_DICT_CAPACITY — remaining headroom is `max - count`,
     * clamped to 0 so an over-cap (grandfathered) DB imports nothing new.
     */
    @Test
    fun INVARIANT_CUSTOM_DICT_CAPACITY_remainingCapacityClamps() {
        val max = CustomDictionaryCapacityPolicy.MAX_ENTRIES

        assertEquals(max, CustomDictionaryCapacityPolicy.remainingCapacity(currentCount = 0))
        assertEquals(1, CustomDictionaryCapacityPolicy.remainingCapacity(currentCount = max - 1))
        assertEquals(0, CustomDictionaryCapacityPolicy.remainingCapacity(currentCount = max))
        // Grandfathered over-cap DB: no negative headroom, import adds nothing.
        assertEquals(0, CustomDictionaryCapacityPolicy.remainingCapacity(currentCount = max + 500))
    }
}
