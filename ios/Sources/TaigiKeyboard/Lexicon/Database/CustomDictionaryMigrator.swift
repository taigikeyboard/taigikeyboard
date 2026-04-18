import Foundation
import SQLite3

/// Custom-dictionary data migrations.
///
/// Forward data migrations that are not DDL:
/// 1. Add derived columns (`notone` / `abbrev` / `roman_num`) to databases
///    created before those columns existed.
/// 2. Re-derive every row's derived columns so values always match the
///    current `CustomDictionaryDerivation` logic (cheap: a single prepared
///    UPDATE reused across rows).
///
/// Runs after `CustomDictionarySchema.ensureTables`; callers must serialize
/// access (typically via `SQLiteConnectionManager.execute`).
enum CustomDictionaryMigrator {
    /// Apply all forward data migrations. Safe to call repeatedly.
    static func run(db: OpaquePointer) {
        addMissingDerivedColumns(db: db)
        backfillDerivedColumns(db: db)
    }

    // MARK: - Private

    /// Add derived columns via `ALTER TABLE` when missing. The whitelist is
    /// required because SQLite DDL cannot parameterize column identifiers.
    private static func addMissingDerivedColumns(db: OpaquePointer) {
        let allowedColumns: Set = ["notone", "abbrev", "roman_num"]
        for column in allowedColumns where !CustomDictionarySchema.columnExists(db: db, column: column) {
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
