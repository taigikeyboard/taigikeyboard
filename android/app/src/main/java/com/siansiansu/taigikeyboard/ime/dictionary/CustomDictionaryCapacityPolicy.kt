package com.siansiansu.taigikeyboard.ime.dictionary

import android.database.sqlite.SQLiteDatabase

/**
 * Capacity policy for the custom-dictionary table. Owns the row-count cap
 * ([MAX_ENTRIES]) and the helpers needed to enforce it inside the same
 * transaction as the write (TOCTOU-safe). Stateless — every method operates on
 * a caller-provided [SQLiteDatabase], mirroring iOS `CustomDictionaryCapacityPolicy`
 * (which takes an `OpaquePointer`). Platform DB-policy code, NOT shared-core.
 *
 * The pure decision helpers ([wouldExceedCap], [remainingCapacity]) are the
 * JVM-testable seam — they take a row count instead of a DB handle, so the
 * boundary can be exercised without inserting 30000 rows.
 */
internal object CustomDictionaryCapacityPolicy {
    // CROSS-PLATFORM INVARIANT — mirrors ios/Sources/TaigiKeyboard/Lexicon/Database/CustomDictionaryCapacityPolicy.swift:17 (maxEntries).
    // Drift causes silent divergence.
    const val MAX_ENTRIES = 30_000

    private const val TABLE_NAME = "custom_dictionary"

    /** Current row count. Returns 0 on query failure (treated as "not full"). */
    fun currentEntryCount(db: SQLiteDatabase): Int =
        db.rawQuery("SELECT COUNT(*) FROM $TABLE_NAME", null).use {
            if (it.moveToFirst()) it.getInt(0) else 0
        }

    /** True when a row with the given [id] already exists. */
    fun entryExists(
        db: SQLiteDatabase,
        id: String,
    ): Boolean =
        db.rawQuery("SELECT 1 FROM $TABLE_NAME WHERE id = ? LIMIT 1", arrayOf(id)).use {
            it.moveToFirst()
        }

    /**
     * Pure decision: would inserting push past the cap? Updating an existing row
     * ([isExistingRow] = true) is not an insert and never exceeds.
     */
    fun wouldExceedCap(
        currentCount: Int,
        isExistingRow: Boolean,
    ): Boolean = !isExistingRow && currentCount >= MAX_ENTRIES

    /** Remaining capacity headroom. Negative values clamp to 0. */
    fun remainingCapacity(currentCount: Int): Int = (MAX_ENTRIES - currentCount).coerceAtLeast(0)

    /**
     * Throw [CustomDictionaryFullException] if inserting a NEW [id] would exceed
     * [MAX_ENTRIES]. Updating an existing row bypasses the check. Must run in the
     * same transaction as the write to avoid a TOCTOU race with concurrent writers.
     */
    fun guardInsertCapacity(
        db: SQLiteDatabase,
        id: String,
    ) {
        if (wouldExceedCap(currentEntryCount(db), entryExists(db, id))) {
            throw CustomDictionaryFullException(MAX_ENTRIES)
        }
    }
}

/** Thrown when a custom-dictionary insert would exceed the row cap. */
class CustomDictionaryFullException(
    maxEntries: Int,
) : Exception("Custom dictionary is full (max $maxEntries entries)")
