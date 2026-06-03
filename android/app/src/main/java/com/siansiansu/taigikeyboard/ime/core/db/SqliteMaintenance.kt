// Best-effort SQLite maintenance shared across the user-data services.

package com.siansiansu.taigikeyboard.ime.core.db

import android.database.sqlite.SQLiteDatabase
import com.siansiansu.taigikeyboard.ime.core.logging.LoggerBackend

/**
 * Reclaim freed pages after a user-initiated bulk clear (R6).
 *
 * Best-effort by design: VACUUM needs exclusive DB access and ~2x the file
 * size in temp space, so a failure (DB locked, low disk) leaves the file
 * larger but fully intact — we just log and move on. MUST be called outside
 * any open transaction and after the clearing DELETE has committed; VACUUM
 * cannot run inside a transaction. Not for the prune hot path — bounded LRU
 * tables reuse their freelist, so VACUUM there would only add latency.
 */
fun vacuumBestEffort(
    db: SQLiteDatabase,
    logger: LoggerBackend,
    tag: String,
) {
    try {
        db.execSQL("VACUUM")
    } catch (e: Exception) {
        logger.w(tag, "vacuum.skipped", e)
    }
}
