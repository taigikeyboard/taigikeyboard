// 中文: 自訂詞庫 DB 的 DDL 定義 — 表格、索引、column-existence 檢查。
// 中文: 純 DDL,沒有 derivation / migration / capacity 邏輯,那些在另外的檔案。

import Foundation
import SQLite3

/// Custom-dictionary DB schema (DDL only).
///
/// Owns the table + indexes for `custom_dictionary.db`. Data migrations
/// (adding missing columns on pre-existing databases, backfilling derived
/// search keys) live in `CustomDictionaryMigrator` so this type stays purely
/// DDL — no domain logic, no derivation, no capacity policy.
/// Callers must serialize access (typically via `SQLiteConnectionManager.execute`).
// 中文: 自訂詞庫 schema — 表格 + 索引 DDL 集中地。
enum CustomDictionarySchema {
    static let tableName = "custom_dictionary"

    /// Cross-mode search side table (v3.6.1 R3). One row per
    /// (entry, family, form) search key produced by
    /// `RustEngineBridge.deriveCustomSearchKeys`. The NEW query path joins
    /// here by the current input's family; the legacy `notone` / `abbrev` /
    /// `roman_num` columns on `custom_dictionary` stay write-only for
    /// backcompat / rollback.
    // 中文: 跨模式搜尋側表 — 一筆 (entry, family, form) 搜尋鍵。新查詢路徑走這張表,
    // 中文: 舊衍生欄位保留為 write-only(回滾用)。
    static let searchKeyTableName = "custom_search_key"
    static let searchKeyEntryIdColumn = "entry_id"
    static let searchKeyFamilyColumn = "family"
    static let searchKeyFormColumn = "form"
    static let searchKeyKeyColumn = "key"

    /// Bump when `CustomDictionaryDerivation` logic changes or new derived
    /// columns are added — `CustomDictionaryMigrator` re-runs ALTER + backfill
    /// against any DB whose `PRAGMA user_version` is below this value.
    // 中文: schema 版本號 — 衍生欄位邏輯改動時要 bump,
    // 中文: migrator 會對 PRAGMA user_version 低於此值的 DB 重跑 ALTER + backfill。
    static let schemaVersion = 2

    /// Derived column names backed by `CustomDictionaryDerivation`.
    /// Single source of truth for the `ALTER TABLE` migrator.
    // 中文: 衍生欄位名清單,作為 ALTER TABLE migrator 的單一資料來源。
    static let derivedColumns = ["notone", "abbrev", "roman_num"]

    /// Create the primary table + side table + all indexes. Idempotent via
    /// `IF NOT EXISTS`.
    // 中文: 建立主表 + 側表與全部索引,IF NOT EXISTS 保證冪等。
    static func ensureTables(db: OpaquePointer) throws {
        try createMainTable(db: db)
        createIndexes(db: db)
        try createSearchKeyTable(db: db)
        createSearchKeyIndexes(db: db)
    }

    /// Check whether a column exists on `custom_dictionary`.
    /// Public so `CustomDictionaryMigrator` can gate `ALTER TABLE` calls.
    // 中文: 檢查 custom_dictionary 是否已有指定欄位,給 migrator 判斷要不要 ALTER。
    static func columnExists(db: OpaquePointer, column: String) -> Bool {
        let sql = "SELECT COUNT(*) FROM pragma_table_info('\(tableName)') WHERE name = ?;"
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return false }
        defer { sqlite3_finalize(stmt) }
        stmt.bindText(1, column)
        return sqlite3_step(stmt) == SQLITE_ROW && sqlite3_column_int(stmt, 0) > 0
    }

    // MARK: - Private

    private static func createMainTable(db: OpaquePointer) throws {
        let sql = """
            CREATE TABLE IF NOT EXISTS \(tableName) (
                id TEXT PRIMARY KEY,
                roman TEXT NOT NULL,
                hanzi TEXT NOT NULL,
                notone TEXT DEFAULT '',
                abbrev TEXT DEFAULT '',
                roman_num TEXT DEFAULT '',
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
        """
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            throw LexiconError.queryPreparationFailed("Create \(tableName) failed: \(String(cString: sqlite3_errmsg(db)))")
        }
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_step(stmt) == SQLITE_DONE else {
            throw LexiconError.queryExecutionFailed("Create \(tableName) failed: \(String(cString: sqlite3_errmsg(db)))")
        }
    }

    private static func createIndexes(db: OpaquePointer) {
        for sql in [
            "CREATE INDEX IF NOT EXISTS idx_custom_roman ON \(tableName)(roman);",
            "CREATE INDEX IF NOT EXISTS idx_custom_notone ON \(tableName)(notone);",
            "CREATE INDEX IF NOT EXISTS idx_custom_abbrev ON \(tableName)(abbrev);",
            "CREATE INDEX IF NOT EXISTS idx_custom_roman_num ON \(tableName)(roman_num);",
        ] {
            sqliteExecSimple(db: db, sql)
        }
    }

    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/CustomDictionaryService.kt (custom_search_key). Drift causes silent divergence.
    private static func createSearchKeyTable(db: OpaquePointer) throws {
        let sql = """
            CREATE TABLE IF NOT EXISTS \(searchKeyTableName) (
                \(searchKeyEntryIdColumn) TEXT NOT NULL,
                \(searchKeyFamilyColumn) TEXT NOT NULL,
                \(searchKeyFormColumn) TEXT NOT NULL,
                \(searchKeyKeyColumn) TEXT NOT NULL
            );
        """
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            throw LexiconError.queryPreparationFailed("Create \(searchKeyTableName) failed: \(String(cString: sqlite3_errmsg(db)))")
        }
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_step(stmt) == SQLITE_DONE else {
            throw LexiconError.queryExecutionFailed("Create \(searchKeyTableName) failed: \(String(cString: sqlite3_errmsg(db)))")
        }
    }

    private static func createSearchKeyIndexes(db: OpaquePointer) {
        for sql in [
            "CREATE INDEX IF NOT EXISTS idx_csk_lookup ON \(searchKeyTableName)(\(searchKeyFamilyColumn), \(searchKeyFormColumn), \(searchKeyKeyColumn));",
            "CREATE INDEX IF NOT EXISTS idx_csk_entry ON \(searchKeyTableName)(\(searchKeyEntryIdColumn));",
        ] {
            sqliteExecSimple(db: db, sql)
        }
    }
}
