import Foundation
import SQLite3

/// Custom-dictionary forward data migrations, gated by `PRAGMA user_version`.
///
/// Owns version-gated forward migrations that are not DDL:
/// 1. Add derived columns (`notone` / `abbrev` / `roman_num`) to databases
///    created before those columns existed.
/// 2. Re-derive every row's derived columns so values match the current
///    `CustomDictionaryDerivation` logic (cheap: a single prepared UPDATE
///    reused across rows).
///
/// Runs after `CustomDictionarySchema.ensureTables`; callers must serialize
/// access (typically via `SQLiteConnectionManager.execute`).
/// No-op once `PRAGMA user_version` matches `CustomDictionarySchema.schemaVersion`,
/// so the O(N) backfill happens at most once per derivation-logic change
/// instead of on every cold start.
enum CustomDictionaryMigrator {
    /// Apply pending forward migrations. Safe to call repeatedly — fast-path
    /// returns immediately when `PRAGMA user_version` is already current.
    static func runIfNeeded(db: OpaquePointer, logger: DebugLogger) {
        let currentVersion = readUserVersion(db: db)
        guard currentVersion < CustomDictionarySchema.schemaVersion else { return }

        logger.info("[MIGRATE] custom_dictionary.db v\(currentVersion) -> v\(CustomDictionarySchema.schemaVersion)")

        if currentVersion < 1 {
            addMissingDerivedColumns(db: db)
            backfillDerivedColumns(db: db)
        }

        sqliteExecSimple(db: db, "PRAGMA user_version = \(CustomDictionarySchema.schemaVersion)")
    }

    // MARK: - Private

    private static func readUserVersion(db: OpaquePointer) -> Int {
        var stmt: OpaquePointer?
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_prepare_v2(db, "PRAGMA user_version", -1, &stmt, nil) == SQLITE_OK else { return 0 }
        return sqlite3_step(stmt) == SQLITE_ROW ? Int(sqlite3_column_int(stmt, 0)) : 0
    }

    /// Add derived columns via `ALTER TABLE` when missing. The whitelist is
    /// required because SQLite DDL cannot parameterize column identifiers.
    private static func addMissingDerivedColumns(db: OpaquePointer) {
        for column in CustomDictionarySchema.derivedColumns
            where !CustomDictionarySchema.columnExists(db: db, column: column)
        {
            sqliteExecSimple(
                db: db,
                "ALTER TABLE \(CustomDictionarySchema.tableName) ADD COLUMN \(column) TEXT DEFAULT '';",
            )
        }
    }

    /// Re-derive `notone` / `abbrev` / `roman_num` for every row. Reuses a
    /// single prepared UPDATE (reset + clear bindings per iteration) so the
    /// cost scales O(N) in rows rather than O(N) in prepared statements.
    private static func backfillDerivedColumns(db: OpaquePointer) {
        var selectStmt: OpaquePointer?
        guard sqlite3_prepare_v2(
            db,
            "SELECT id, roman FROM \(CustomDictionarySchema.tableName);",
            -1, &selectStmt, nil,
        ) == SQLITE_OK else { return }
        defer { sqlite3_finalize(selectStmt) }

        var updateStmt: OpaquePointer?
        let updateSQL = """
            UPDATE \(CustomDictionarySchema.tableName)
            SET notone = ?, abbrev = ?, roman_num = ?
            WHERE id = ?;
        """
        guard sqlite3_prepare_v2(db, updateSQL, -1, &updateStmt, nil) == SQLITE_OK else { return }
        defer { sqlite3_finalize(updateStmt) }

        while sqlite3_step(selectStmt) == SQLITE_ROW {
            let id = String(cString: sqlite3_column_text(selectStmt, 0))
            let roman = String(cString: sqlite3_column_text(selectStmt, 1))

            sqlite3_reset(updateStmt)
            sqlite3_clear_bindings(updateStmt)

            updateStmt.bindText(1, CustomDictionaryDerivation.generateNotone(roman))
            updateStmt.bindText(2, CustomDictionaryDerivation.generateAbbrev(roman))
            updateStmt.bindText(3, CustomDictionaryDerivation.generateRomanNum(roman))
            updateStmt.bindText(4, id)
            sqlite3_step(updateStmt)
        }
    }
}
