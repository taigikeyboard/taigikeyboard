// 中文: user_association.db 的 schema 與向前遷移。只負責 DDL,
// 中文: 不處理 pruning / capacity / scoring。

import Foundation
import SQLite3

/// User NextWord DB schema & forward migrations.
///
/// Owns the `PRAGMA user_version` contract for `user_association.db`.
/// DDL only — no pruning, capacity, or scoring policy.
/// Callers must serialize access (typically via `SQLiteConnectionManager.execute`).
// 中文: NextWord 使用者資料庫的建表與遷移工具。透過 PRAGMA user_version 控管版本。
enum NextWordSchema {
    /// Schema version for `user_association.db` (mirrors Android's DATABASE_VERSION).
    // 中文: schema 版本號,需與 Android DATABASE_VERSION 對齊。
    static let schemaVersion = 4

    /// Migrate forward then create tables + indexes.
    /// Safe to call repeatedly; CREATE and ALTER are guarded by version check + IF NOT EXISTS.
    // 中文: 先做向前遷移,再 CREATE TABLE / INDEX。可重複呼叫,內部以版本與 IF NOT EXISTS 防衛。
    static func ensureTables(db: OpaquePointer, logger: DebugLogger) throws {
        try migrate(db: db, logger: logger)
        try createTables(db: db)
        createIndexes(db: db)
    }

    // MARK: - Private

    // 中文: 建立 user_association 主表 — 含 UNIQUE(prev_word, next_word, next_tl) 防止重複關聯。
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

    // 中文: 建立查詢用 index — 走 prev_word 主索引與 (prev_word, prev_tl) 複合索引。
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
    // 中文: 用 PRAGMA user_version 做向前遷移。v<3 直接重建,v3→v4 ALTER TABLE 加欄位保留資料。
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
