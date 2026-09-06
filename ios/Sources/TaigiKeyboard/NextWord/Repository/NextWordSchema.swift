import Foundation
import SQLite3

/// User NextWord DB schema & forward migrations.
///
/// Owns the `PRAGMA user_version` contract for `user_association.db`.
/// DDL only — no pruning, capacity, or scoring policy.
/// Callers must serialize access (typically via `SQLiteConnectionManager.execute`).
enum NextWordSchema {
    /// Schema version for `user_association.db`. CROSS-PLATFORM INVARIANT —
    /// mirrors Android `NextWordService.DATABASE_VERSION` and macOS
    /// `UserAssociationStore.schemaVersion`. Drift causes silent divergence.
    static let schemaVersion = 6

    private static let tableName = "user_association"

    /// Bring the database to `schemaVersion`, then guarantee the terminal
    /// table + indexes exist.
    ///
    /// An upgrade runs as ONE transaction covering the rebuild, the terminal
    /// DDL, and the version stamp together: a database that says v6 must
    /// actually have the v6 table and both v6 indexes, so a failure anywhere
    /// has to take the version stamp down with it.
    ///
    /// Safe to call repeatedly. At `schemaVersion` it only re-asserts the
    /// `IF NOT EXISTS` DDL, which is a no-op.
    static func ensureTables(db: OpaquePointer, logger: DebugLogger) throws {
        let currentVersion = try sqliteQueryScalarInt(db: db, "PRAGMA user_version")
        guard currentVersion < schemaVersion else {
            try createTables(db: db)
            try createIndexes(db: db)
            return
        }

        logger.info("[MIGRATE] user_association.db v\(currentVersion) -> v\(schemaVersion)")
        try sqliteExecChecked(db: db, "BEGIN IMMEDIATE;")
        do {
            try migrate(db: db, from: currentVersion)
            try createTables(db: db)
            try createIndexes(db: db)
            try sqliteExecChecked(db: db, "PRAGMA user_version = \(schemaVersion);")
            try sqliteExecChecked(db: db, "COMMIT;")
        } catch {
            // Harmless when no transaction is active; the alternative is asking
            // SQLite whether one is, which answers the same question twice.
            try? sqliteExecChecked(db: db, "ROLLBACK;")
            throw error
        }

        // Outside the transaction: refresh the query planner's stats once,
        // AFTER all DDL. Gated on an actual migration so it never runs on a
        // steady-state open, and best-effort because failing to optimize is
        // not a reason to fail the open.
        sqliteExecSimple(db: db, "PRAGMA optimize")
    }

    // MARK: - Private

    /// The v6 table. The UNIQUE key carries `prev_tl` because a Taiwanese word
    /// is the `(漢字, canonical TL)` pair (`CLAUDE.md` Core Principle #7) on the
    /// bigram's PREVIOUS side as well as its next: 重/tîng → 複 and 重/tāng → 複
    /// are two observations, not one. See `behavioral-invariants.md` §24.
    private static func createTables(db: OpaquePointer) throws {
        try sqliteExecChecked(db: db, tableDDL(named: tableName))
    }

    private static func tableDDL(named name: String) -> String {
        """
        CREATE TABLE IF NOT EXISTS \(name) (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            prev_word TEXT NOT NULL,
            prev_tl TEXT DEFAULT '',
            next_word TEXT NOT NULL,
            next_tl TEXT DEFAULT '',
            count INTEGER DEFAULT 1,
            last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
            UNIQUE(prev_word, prev_tl, next_word, next_tl)
        );
        """
    }

    /// One read index: `(prev_word, prev_tl)` serves the recall query's
    /// `WHERE prev_word = ?` from its left prefix, and its `prev_tl` tier
    /// ordering from the second column. The single-column `idx_user_prev_word`
    /// is a strict subset and was dropped in v5.
    private static func createIndexes(db: OpaquePointer) throws {
        try sqliteExecChecked(
            db: db,
            "CREATE INDEX IF NOT EXISTS idx_user_prev_word_tl ON \(tableName)(prev_word, prev_tl);",
        )
    }

    /// Bring a pre-v6 database to the v6 shape. Runs inside the caller's
    /// transaction.
    ///
    /// - v<3 → drop, unchanged. iOS has always dropped rather than migrated
    ///   these, and this round keeps that: the pre-v3 iOS shape is not
    ///   documented anywhere, so a rebuild that guessed at it could fail the
    ///   open outright — strictly worse than the drop those installs already
    ///   got. `createTables` then builds v6 fresh.
    ///
    ///   NAMED CROSS-PLATFORM DIVERGENCE (`cross-platform-alignment.md` §3,
    ///   pre-existing, legacy-only): Android drops at exactly v2 and rebuilds
    ///   v0/v1, because ITS v0/v1 shape is known — `migrateV0ToV2` copied the
    ///   rows and dropped only the unused `next_poj` / `delimiter` columns.
    ///   The divergence is confined to databases last written before v3.4.x;
    ///   every version a user can still be on converges.
    /// - v3…v5 → ONE convergent rebuild, not a step ladder. SQLite cannot ALTER
    ///   a table-level UNIQUE, so widening the key means rebuilding the table
    ///   anyway, and a rebuild that reads the columns it finds subsumes every
    ///   intermediate step (v3's missing `prev_tl` included).
    private static func migrate(db: OpaquePointer, from currentVersion: Int) throws {
        if currentVersion < 3 {
            try sqliteExecChecked(db: db, "DROP TABLE IF EXISTS \(tableName);")
        } else if try sqliteTableExistsChecked(db: db, table: tableName) {
            try rebuildToV6(db: db)
        }
        // The v4 single-column index: dropped with its table on either branch
        // above, so this only catches a DB whose table was already absent.
        try sqliteExecChecked(db: db, "DROP INDEX IF EXISTS idx_user_prev_word;")
    }

    /// Rebuild the table under the v6 key, preserving every row.
    ///
    /// Widening a UNIQUE key can never conflict — the old key
    /// `(prev_word, next_word, next_tl)` is a strict subset of the new one, so
    /// rows already unique under the old key stay unique under the new. The
    /// copy therefore needs no dedupe or merge step.
    ///
    /// `id` is copied rather than reassigned: it is the final tiebreak of the
    /// read query's per-prediction pick, so renumbering could silently change
    /// which row wins for rows that tie on everything else.
    ///
    /// Order is SQLite's documented one — create new, copy, drop old, rename —
    /// rather than rename-first, which can rewrite references inside triggers
    /// and views.
    ///
    /// CROSS-PLATFORM INVARIANT — mirrors
    /// android/…/ime/dictionary/NextWordService.kt `rebuildToV6`.
    /// Drift causes silent divergence.
    private static func rebuildToV6(db: OpaquePointer) throws {
        // A v3 table predates the `prev_tl` column, and an Android DB that came
        // up the v0/v1 ladder can be stamped v5 without it. Read what is there
        // rather than assuming — and let an introspection FAILURE throw, since
        // mistaking it for "the column is absent" would blank every stored
        // romanization.
        let prevTl = try sqliteColumnExistsChecked(db: db, table: tableName, column: "prev_tl") ? "COALESCE(prev_tl, '')" : "''"
        let nextTl = try sqliteColumnExistsChecked(db: db, table: tableName, column: "next_tl") ? "COALESCE(next_tl, '')" : "''"

        try sqliteExecChecked(db: db, tableDDL(named: "\(tableName)_new"))
        try sqliteExecChecked(db: db, """
            INSERT INTO \(tableName)_new
                (id, prev_word, prev_tl, next_word, next_tl, count, last_used)
            SELECT id, prev_word, \(prevTl), next_word, \(nextTl), count, last_used
            FROM \(tableName);
        """)
        try sqliteExecChecked(db: db, "DROP TABLE \(tableName);")
        try sqliteExecChecked(db: db, "ALTER TABLE \(tableName)_new RENAME TO \(tableName);")
    }
}
