// 中文: 使用者詞頻 DB 的 DDL — 主表 + 索引 + metadata 補種子,純結構。

import Foundation
import SQLite3

/// User-frequency DB schema (DDL only).
///
/// Owns the table + indexes + side metadata table for `user_frequency.db`.
/// No pruning, capacity, or scoring policy — those live in
/// `UserFrequencyPruner` / the repository.
/// Callers must serialize access (typically via `SQLiteConnectionManager.execute`).
// 中文: 使用者詞頻 schema — DDL only。pruning / capacity / scoring 政策都不在這裡。
enum UserFrequencySchema {
    static let tableName = "user_frequency"
    static let metadataTableName = "metadata"

    /// Create all tables + indexes and seed metadata. Idempotent.
    // 中文: 建立全部表 + 索引並補 metadata 種子。冪等。
    static func ensureTables(db: OpaquePointer) throws {
        try createFrequencyTable(db: db)
        createFrequencyIndexes(db: db)
        createMetadataTable(db: db)
        seedMetadata(db: db)
    }

    // MARK: - Private

    private static func createFrequencyTable(db: OpaquePointer) throws {
        let sql = """
            CREATE TABLE IF NOT EXISTS \(tableName) (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word TEXT NOT NULL UNIQUE,
                count INTEGER DEFAULT 1,
                last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
        """
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            throw LexiconError.queryPreparationFailed(
                "Create \(tableName) failed: \(String(cString: sqlite3_errmsg(db)))",
            )
        }
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_step(stmt) == SQLITE_DONE else {
            throw LexiconError.queryExecutionFailed(
                "Create \(tableName) failed: \(String(cString: sqlite3_errmsg(db)))",
            )
        }
    }

    private static func createFrequencyIndexes(db: OpaquePointer) {
        for sql in [
            "CREATE INDEX IF NOT EXISTS idx_word ON \(tableName)(word);",
            "CREATE INDEX IF NOT EXISTS idx_count ON \(tableName)(count DESC);",
            "CREATE INDEX IF NOT EXISTS idx_last_used ON \(tableName)(last_used DESC);",
        ] {
            sqliteExecSimple(db: db, sql)
        }
    }

    private static func createMetadataTable(db: OpaquePointer) {
        sqliteExecSimple(db: db, """
            CREATE TABLE IF NOT EXISTS \(metadataTableName) (
                key TEXT PRIMARY KEY,
                value TEXT
            );
        """)
    }

    private static func seedMetadata(db: OpaquePointer) {
        let sql = """
            INSERT OR IGNORE INTO \(metadataTableName) (key, value) VALUES
            ('app_version', ?),
            ('schema_version', '1.0'),
            ('created_date', datetime('now')),
            ('last_modified', datetime('now'));
        """
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return }
        defer { sqlite3_finalize(stmt) }

        stmt.bindText(1, appVersion())
        sqlite3_step(stmt)
    }

    private static func appVersion() -> String {
        Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String ?? "1.0.0"
    }
}
