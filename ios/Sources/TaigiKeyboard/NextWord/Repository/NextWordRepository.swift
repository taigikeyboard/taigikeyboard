import Foundation
import SQLite3

/// Stateless SQLite CRUD for `user_association.db`.
///
/// All methods take a live `OpaquePointer`. Callers are responsible for:
/// - serializing DB access (typically via `SQLiteConnectionManager.execute`),
/// - ensuring tables exist (via `NextWordSchema.ensureTables`).
enum NextWordRepository {
    /// Raw user-association row for prediction scoring.
    struct UserRow {
        let hanzi: String
        let tl: String
        let count: Int
        let lastUsedMs: Int64
    }

    /// Full user-association row (all columns) for listing/backup.
    struct AssociationRow {
        let prevWord: String
        let prevTl: String
        let nextWord: String
        let nextTl: String
        let count: Int
    }

    // MARK: - Writes

    /// Insert or increment count on UNIQUE(prev_word, prev_tl, next_word, next_tl) conflict.
    static func insertOrUpdate(
        db: OpaquePointer,
        prev: String,
        prevTl: String,
        nextHanzi: String,
        nextTl: String,
    ) throws {
        let sql = """
            INSERT INTO user_association (prev_word, prev_tl, next_word, next_tl, count, last_used)
            VALUES (?, ?, ?, ?, 1, CURRENT_TIMESTAMP)
            ON CONFLICT(prev_word, prev_tl, next_word, next_tl) DO UPDATE SET
                count = count + 1,
                last_used = CURRENT_TIMESTAMP
        """
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            throw LexiconError.queryPreparationFailed(String(cString: sqlite3_errmsg(db)))
        }
        defer { sqlite3_finalize(stmt) }
        stmt.bindText(1, prev)
        stmt.bindText(2, prevTl)
        stmt.bindText(3, nextHanzi)
        stmt.bindText(4, nextTl)
        if sqlite3_step(stmt) != SQLITE_DONE {
            throw LexiconError.queryExecutionFailed(String(cString: sqlite3_errmsg(db)))
        }
    }

    /// Batch-import with merge-by-max semantics (keep the higher count).
    /// Returns the number of entries imported successfully.
    static func batchImport(
        db: OpaquePointer,
        entries: [(prevWord: String, prevTl: String, nextWord: String, nextTl: String, count: Int)],
    ) -> Int {
        let sql = """
            INSERT INTO user_association (prev_word, prev_tl, next_word, next_tl, count, last_used)
            VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(prev_word, prev_tl, next_word, next_tl) DO UPDATE SET
                count = MAX(count, excluded.count),
                last_used = CURRENT_TIMESTAMP;
        """
        if sqlite3_exec(db, "BEGIN TRANSACTION;", nil, nil, nil) != SQLITE_OK { return 0 }

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            sqlite3_exec(db, "ROLLBACK;", nil, nil, nil)
            return 0
        }
        defer { sqlite3_finalize(stmt) }

        var imported = 0
        for entry in entries {
            sqlite3_reset(stmt)
            sqlite3_clear_bindings(stmt)
            stmt.bindText(1, entry.prevWord)
            stmt.bindText(2, entry.prevTl)
            stmt.bindText(3, entry.nextWord)
            stmt.bindText(4, entry.nextTl)
            sqlite3_bind_int(stmt, 5, Int32(entry.count))
            if sqlite3_step(stmt) == SQLITE_DONE {
                imported += 1
            }
        }

        sqlite3_exec(db, "COMMIT;", nil, nil, nil)
        return imported
    }

    /// Delete a single entry by (prev_word, prev_tl, next_word, next_tl).
    static func deleteOne(
        db: OpaquePointer,
        prev: String,
        prevTl: String,
        nextHanzi: String,
        nextTl: String,
    ) {
        let sql = "DELETE FROM user_association WHERE prev_word = ? AND prev_tl = ? AND next_word = ? AND next_tl = ?"
        var stmt: OpaquePointer?
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return }
        stmt.bindText(1, prev)
        stmt.bindText(2, prevTl)
        stmt.bindText(3, nextHanzi)
        stmt.bindText(4, nextTl)
        sqlite3_step(stmt)
    }

    /// Delete all rows.
    static func deleteAll(db: OpaquePointer) {
        sqliteExecSimple(db: db, "DELETE FROM user_association")
    }

    /// Delete the oldest/lowest-count rows up to `limit`.
    /// Selection order: count ASC, last_used ASC (low-usage + stale first).
    static func pruneOldest(db: OpaquePointer, limit: Int) {
        let sql = """
            DELETE FROM user_association
            WHERE id IN (
                SELECT id FROM user_association
                ORDER BY count ASC, last_used ASC
                LIMIT ?
            )
        """
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return }
        defer { sqlite3_finalize(stmt) }
        sqlite3_bind_int(stmt, 1, Int32(limit))
        sqlite3_step(stmt)
    }

    // MARK: - Reads

    /// Query next-word candidates by previous word, ranking `prev_tl`
    /// matches ahead of mismatches instead of hard-filtering on them, and
    /// returning at most one row per predicted `(next_word, next_tl)`.
    ///
    /// `prev_word` (Hanji) is the only lookup key — matching the bundled
    /// `association.bin` (Hanji-only prev key). A non-empty `prev_tl` that
    /// differs from the query `roman` (e.g. a bigram learned under the other
    /// reading of a 一字多音 Hanji, or a pre-v3.6.1 raw `taigi` where a
    /// normal commit stored canonical `tâi-gí`) is still recalled — it ranks
    /// after exact and empty matches but is never dropped.
    ///
    /// **Row order is load-bearing.** v6 stores 重/tîng → 複 and 重/tāng → 複
    /// separately (Core Principle #7 on the previous side), so a Hanji-only
    /// lookup can return several rows predicting the SAME word. The engine
    /// keeps only the FIRST user row per predicted `(hanzi, tl)`
    /// (`engine/nextword/src/filter.rs`) rather than summing their scores and
    /// handing one predicted word several learning bonuses — which makes the
    /// tier ordering below the thing that decides WHICH reading's evidence is
    /// used. Nothing between this cursor and the engine may reorder these rows.
    ///
    /// `ORDER BY` also ranks before the SQL `LIMIT` truncates, so exact-`prev_tl`
    /// rows survive the over-fetch window even when a hot `prev_word` has many
    /// rows. Callers over-fetch `limit * 2`; the Rust filter applies the final
    /// score sort + real limit.
    ///
    /// CROSS-PLATFORM INVARIANT — mirrors
    /// android/…/ime/dictionary/NextWordService.kt `predict`. Drift causes
    /// silent divergence. Pins `behavioral-invariants.md` §24.
    static func fetchUserRows(
        db: OpaquePointer,
        word: String,
        roman: String,
        limit: Int,
    ) -> [UserRow] {
        let sql = """
            SELECT next_word, next_tl, count,
                   strftime('%s', last_used) * 1000 AS last_used_ms
            FROM user_association
            WHERE prev_word = ?
            ORDER BY
                CASE WHEN prev_tl = ? THEN 0 WHEN prev_tl = '' THEN 1 ELSE 2 END,
                count DESC, last_used DESC, id ASC
            LIMIT ?
        """
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return [] }
        defer { sqlite3_finalize(stmt) }
        stmt.bindText(1, word)
        stmt.bindText(2, roman)
        sqlite3_bind_int(stmt, 3, Int32(limit))

        var results: [UserRow] = []
        while sqlite3_step(stmt) == SQLITE_ROW {
            let nextWord = sqlite3_column_text(stmt, 0).map(String.init(cString:)) ?? ""
            let nextTl = sqlite3_column_text(stmt, 1).map(String.init(cString:)) ?? ""
            let count = Int(sqlite3_column_int(stmt, 2))
            let lastUsedMs = sqlite3_column_int64(stmt, 3)
            results.append(UserRow(hanzi: nextWord, tl: nextTl, count: count, lastUsedMs: lastUsedMs))
        }
        return results
    }

    /// Fetch every row for backup/UI listing, sorted by count DESC, last_used DESC.
    static func fetchAllRows(db: OpaquePointer) -> [AssociationRow] {
        let sql = """
            SELECT prev_word, prev_tl, next_word, next_tl, count
            FROM user_association
            ORDER BY count DESC, last_used DESC
        """
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return [] }
        defer { sqlite3_finalize(stmt) }

        var results: [AssociationRow] = []
        while sqlite3_step(stmt) == SQLITE_ROW {
            let prevWord = sqlite3_column_text(stmt, 0).map(String.init(cString:)) ?? ""
            let prevTl = sqlite3_column_text(stmt, 1).map(String.init(cString:)) ?? ""
            let nextWord = sqlite3_column_text(stmt, 2).map(String.init(cString:)) ?? ""
            let nextTl = sqlite3_column_text(stmt, 3).map(String.init(cString:)) ?? ""
            let count = Int(sqlite3_column_int(stmt, 4))
            results.append(AssociationRow(
                prevWord: prevWord, prevTl: prevTl,
                nextWord: nextWord, nextTl: nextTl,
                count: count,
            ))
        }
        return results
    }

    /// Total row count.
    static func count(db: OpaquePointer) -> Int {
        var stmt: OpaquePointer?
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_prepare_v2(db, "SELECT COUNT(*) FROM user_association", -1, &stmt, nil) == SQLITE_OK else {
            return 0
        }
        return sqlite3_step(stmt) == SQLITE_ROW ? Int(sqlite3_column_int(stmt, 0)) : 0
    }
}
