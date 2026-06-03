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
    static let schemaVersion = 5

    /// Migrate forward then create tables + indexes.
    /// Safe to call repeatedly; CREATE and ALTER are guarded by version check + IF NOT EXISTS.
    // 中文: 先做向前遷移,再 CREATE TABLE / INDEX。可重複呼叫,內部以版本與 IF NOT EXISTS 防衛。
    static func ensureTables(db: OpaquePointer, logger: DebugLogger) throws {
        let didMigrate = try migrate(db: db, logger: logger)
        try createTables(db: db)
        createIndexes(db: db)
        if didMigrate {
            // R6: refresh the query planner's stats once, AFTER all DDL
            // (CREATE INDEX included). Gated on an actual migration so it never
            // runs on a steady-state open. Cheap no-op when nothing changed.
            sqliteExecSimple(db: db, "PRAGMA optimize")
        }
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

    // 中文: 建立查詢用 index — 僅 (prev_word, prev_tl) 複合索引。
    // 中文: 單欄 idx_user_prev_word 是它的左前綴子集,SQLite 可用複合索引服務
    // 中文: `WHERE prev_word = ?` 查詢,故 R6 移除,改由 v4→v5 migration 清舊 DB。
    private static func createIndexes(db: OpaquePointer) {
        sqliteExecSimple(
            db: db,
            "CREATE INDEX IF NOT EXISTS idx_user_prev_word_tl ON user_association(prev_word, prev_tl);",
        )
    }

    /// Migrate forward using `PRAGMA user_version`.
    /// - v<3 → v3: DROP + CREATE (old schema incompatible).
    /// - v3 → v4: ALTER TABLE adds `prev_tl` (data preserved).
    /// - v4 → v5: DROP redundant `idx_user_prev_word` (left-prefix of the
    ///   `idx_user_prev_word_tl` composite). Data preserved.
    // 中文: 用 PRAGMA user_version 做向前遷移。v<3 直接重建,v3→v4 ALTER 加欄位,v4→v5 刪冗餘單欄索引。
    // 中文: 回傳是否實際執行了遷移(供呼叫端決定要不要跑 PRAGMA optimize)。
    private static func migrate(db: OpaquePointer, logger: DebugLogger) throws -> Bool {
        var versionStmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, "PRAGMA user_version", -1, &versionStmt, nil) == SQLITE_OK else { return false }
        let currentVersion = sqlite3_step(versionStmt) == SQLITE_ROW
            ? Int(sqlite3_column_int(versionStmt, 0))
            : 0
        sqlite3_finalize(versionStmt)

        guard currentVersion < schemaVersion else { return false }
        logger.info("[MIGRATE] user_association.db v\(currentVersion) -> v\(schemaVersion)")

        if currentVersion < 3 {
            sqliteExecSimple(db: db, "DROP TABLE IF EXISTS user_association")
            sqliteExecSimple(db: db, "DROP INDEX IF EXISTS idx_user_prev_word")
        }
        if currentVersion >= 3, currentVersion < 4 {
            sqliteExecSimple(db: db, "ALTER TABLE user_association ADD COLUMN prev_tl TEXT DEFAULT ''")
            sqliteExecSimple(db: db, "CREATE INDEX IF NOT EXISTS idx_user_prev_word_tl ON user_association(prev_word, prev_tl)")
        }
        // v4 → v5 (R6): drop the redundant single-column idx_user_prev_word —
        // it is the left-prefix subset of the (prev_word, prev_tl) composite.
        // Unconditional inside the `currentVersion < schemaVersion` guard so it
        // reaches every pre-v5 DB (a v3 DB jumping straight to v5 would skip a
        // `>= 4` step), not just DBs that were exactly at v4. Idempotent.
        sqliteExecSimple(db: db, "DROP INDEX IF EXISTS idx_user_prev_word")

        sqliteExecSimple(db: db, "PRAGMA user_version = \(schemaVersion)")
        return true
    }
}
