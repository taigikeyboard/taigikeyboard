import Foundation
import SQLite3

/// User NextWord DB schema & forward migrations.
///
/// Owns the `PRAGMA user_version` contract for `user_association.db`.
/// DDL only — no pruning, capacity, or scoring policy.
/// Callers must serialize access (typically via `SQLiteConnectionManager.execute`).
enum NextWordSchema {
    /// Schema version for `user_association.db` (mirrors Android's DATABASE_VERSION).
    static let schemaVersion = 4

    /// Migrate forward then create tables + indexes.
    /// Safe to call repeatedly; CREATE and ALTER are guarded by version check + IF NOT EXISTS.
    static func ensureTables(db: OpaquePointer, logger: DebugLogger) throws {
        try migrate(db: db, logger: logger)
        try createTables(db: db)
        createIndexes(db: db)
    }

    // MARK: - Private

    private static func createTables(db: OpaquePointer) throws {
        let sql = """
            CREATE TABLE IF NOT EXISTS user_association (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                prev_word TEXT NOT NULL,
                prev_tl TEXT DEFAULT '',
                next_word TEXT NOT NULL,
                next_tl TEXT DEFAULT '',
                count INTEGER DEFAULT 1,
                last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(prev_word, next_word, next_tl)
            );
        """
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            throw LexiconError.queryPreparationFailed(String(cString: sqlite3_errmsg(db)))
        }
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_step(stmt) == SQLITE_DONE else {
            throw LexiconError.queryExecutionFailed(String(cString: sqlite3_errmsg(db)))
        }
    }

    private static func createIndexes(db: OpaquePointer) {
        for indexSQL in [
            "CREATE INDEX IF NOT EXISTS idx_user_prev_word ON user_association(prev_word);",
            "CREATE INDEX IF NOT EXISTS idx_user_prev_word_tl ON user_association(prev_word, prev_tl);",
        ] {
            sqliteExecSimple(db: db, indexSQL)
        }
    }

    /// Migrate forward using `PRAGMA user_version`.
    /// - v<3 → v3: DROP + CREATE (old schema incompatible).
    /// - v3 → v4: ALTER TABLE adds `prev_tl` (data preserved).
    private static func migrate(db: OpaquePointer, logger: DebugLogger) throws {
        var versionStmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, "PRAGMA user_version", -1, &versionStmt, nil) == SQLITE_OK else { return }
        let currentVersion = sqlite3_step(versionStmt) == SQLITE_ROW
            ? Int(sqlite3_column_int(versionStmt, 0))
            : 0
        sqlite3_finalize(versionStmt)

        guard currentVersion < schemaVersion else { return }
        logger.info("[MIGRATE] user_association.db v\(currentVersion) -> v\(schemaVersion)")

        if currentVersion < 3 {
            sqliteExecSimple(db: db, "DROP TABLE IF EXISTS user_association")
            sqliteExecSimple(db: db, "DROP INDEX IF EXISTS idx_user_prev_word")
        }
        if currentVersion >= 3, currentVersion < 4 {
            sqliteExecSimple(db: db, "ALTER TABLE user_association ADD COLUMN prev_tl TEXT DEFAULT ''")
            sqliteExecSimple(db: db, "CREATE INDEX IF NOT EXISTS idx_user_prev_word_tl ON user_association(prev_word, prev_tl)")
        }

        sqliteExecSimple(db: db, "PRAGMA user_version = \(schemaVersion)")
    }
}
