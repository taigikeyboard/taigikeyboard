import Foundation
import SQLite3

/// User-frequency DB schema (DDL + migration only).
///
/// Owns the table + indexes + side metadata table for `user_frequency.db`,
/// plus the v3.6.1 R5 `(word, tl)` pair-key migration. No pruning,
/// capacity, or scoring policy — those live in `UserFrequencyPruner` / the
/// repository. Callers must serialize access (typically via
/// `SQLiteConnectionManager.execute`).
///
/// **R5 identity = `(word, tl)` pair (Core Principle #7)**: a frequency
/// row is keyed by display text AND its canonical-TL reading, so 一字多音
/// (重/tîng vs 重/tāng) keep separate counts. Pre-R5 rows carry `tl = ''`
/// (the migration backfill); the engine treats `tl == ''` as a tolerant
/// fallback bucket all readings consult until each is re-learned.
enum UserFrequencySchema {
    static let tableName = "user_frequency"
    static let metadataTableName = "metadata"

    /// `PRAGMA user_version` for the pair-key schema. Bumped from the
    /// implicit 0/1 single-`word`-UNIQUE shape to 2 by R5. Used as the
    /// migration gate (integer PRAGMA over the string metadata row — Codex
    /// pre-impl Q3: robust, atomic with the rebuild transaction).
    static let pairKeySchemaVersion = 2

    /// Create all tables + indexes, migrate to pair-key if needed, and seed
    /// metadata. Idempotent.
    static func ensureTables(db: OpaquePointer) throws {
        try migrateToPairKeyIfNeeded(db: db)
        try createFrequencyTable(db: db)
        createFrequencyIndexes(db: db)
        createMetadataTable(db: db)
        seedMetadata(db: db)
        sqliteSetUserVersion(db: db, version: pairKeySchemaVersion)
    }

    // MARK: - Pair-key migration (R5)

    /// Rebuild a pre-R5 `user_frequency` table (inline `word UNIQUE`, no
    /// `tl` column) into the `(word, tl)` pair-key shape. SQLite cannot drop
    /// an inline column UNIQUE via `ALTER`, so this does the canonical
    /// create-new / copy / drop / rename inside a single `BEGIN IMMEDIATE`
    /// transaction (crash mid-migration rolls back). Existing rows backfill
    /// `tl = ''` (the legacy fallback bucket); `id` / `count` / `last_used`
    /// / `created_at` are preserved exactly. Fresh installs (no table) skip
    /// this entirely — `createFrequencyTable` builds the new shape directly.
    private static func migrateToPairKeyIfNeeded(db: OpaquePointer) throws {
        if sqliteReadUserVersion(db: db) >= pairKeySchemaVersion {
            return
        }
        // version < 2: fresh install (no table) OR pre-R5 v1 (table without
        // `tl`). Only the latter needs a rebuild. The `columnExists` re-check
        // (beyond the `user_version` gate) is an iOS-specific safety net —
        // iOS manages `user_version` manually so a pre-existing DB can sit at
        // 0 while already carrying the new shape; Android relies on
        // `SQLiteOpenHelper`'s framework-managed `oldVersion` and needs no
        // such re-check (intentional cross-platform divergence).
        guard sqliteTableExists(db: db, table: tableName),
              !sqliteColumnExists(db: db, table: tableName, column: "tl")
        else { return }

        let newTable = "\(tableName)_pairkey_migrate"
        let migrationSQL = """
            CREATE TABLE \(newTable) (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word TEXT NOT NULL,
                tl TEXT NOT NULL DEFAULT '',
                count INTEGER DEFAULT 1,
                last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(word, tl)
            );
            INSERT INTO \(newTable) (id, word, tl, count, last_used, created_at)
                SELECT id, word, '', count, last_used, created_at FROM \(tableName);
            DROP TABLE \(tableName);
            ALTER TABLE \(newTable) RENAME TO \(tableName);
        """

        guard sqlite3_exec(db, "BEGIN IMMEDIATE;", nil, nil, nil) == SQLITE_OK else {
            throw LexiconError.queryExecutionFailed(
                "user_frequency pair-key migration BEGIN failed: \(String(cString: sqlite3_errmsg(db)))",
            )
        }
        if sqlite3_exec(db, migrationSQL, nil, nil, nil) != SQLITE_OK {
            let message = String(cString: sqlite3_errmsg(db))
            sqlite3_exec(db, "ROLLBACK;", nil, nil, nil)
            throw LexiconError.queryExecutionFailed("user_frequency pair-key migration failed: \(message)")
        }
        guard sqlite3_exec(db, "COMMIT;", nil, nil, nil) == SQLITE_OK else {
            let message = String(cString: sqlite3_errmsg(db))
            sqlite3_exec(db, "ROLLBACK;", nil, nil, nil)
            throw LexiconError.queryExecutionFailed("user_frequency pair-key migration COMMIT failed: \(message)")
        }
    }

    // MARK: - Private (DDL)

    private static func createFrequencyTable(db: OpaquePointer) throws {
        let sql = """
            CREATE TABLE IF NOT EXISTS \(tableName) (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word TEXT NOT NULL,
                tl TEXT NOT NULL DEFAULT '',
                count INTEGER DEFAULT 1,
                last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(word, tl)
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
        // No standalone `idx_word`: the `UNIQUE(word, tl)` constraint's
        // autoindex has `word` as its leftmost column, so it already serves
        // `WHERE word = ?` / `WHERE word IN (...)` lookups (Codex Q3 — drop
        // the redundant index). `idx_count` / `idx_last_used` stay for
        // top-N + pruning order.
        for sql in [
            "CREATE INDEX IF NOT EXISTS idx_count ON \(tableName)(count DESC);",
            "CREATE INDEX IF NOT EXISTS idx_last_used ON \(tableName)(last_used DESC);",
        ] {
            sqliteExecSimple(db: db, sql)
        }
        // Pre-R5 installs created `idx_word`; drop it post-migration so the
        // schema converges to one shape across fresh + upgraded DBs.
        sqliteExecSimple(db: db, "DROP INDEX IF EXISTS idx_word;")
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
