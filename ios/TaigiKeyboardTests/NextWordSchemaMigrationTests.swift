@testable import TaigiKeyboard
import SQLite3
import XCTest

/// v3.6.1 R6 — `user_association.db` schema v4 → v5 hygiene migration.
///
/// v5 drops the redundant single-column `idx_user_prev_word`; the
/// `(prev_word, prev_tl)` composite's left prefix already serves every
/// `WHERE prev_word = ?` lookup. The drop is unconditional inside the
/// version guard so it reaches DBs jumping straight from v3 (which the old
/// `>= 4` step would have skipped), not just DBs that were exactly at v4.
///
/// Exercises the SQLite thin layer directly via an in-memory DB — the
/// Android counterpart cannot (no Robolectric / `.so` in JVM unit tests), so
/// Android pins the same behavior via the mirrored `DROP INDEX` migration
/// (`NextWordService.migrateV4ToV5`) + S-series dogfood.
final class NextWordSchemaMigrationTests: XCTestCase {
    private var db: OpaquePointer!
    private let logger = DebugLogger(category: "NextWordSchemaMigrationTests")

    override func setUpWithError() throws {
        try super.setUpWithError()
        guard sqlite3_open(":memory:", &db) == SQLITE_OK else {
            throw XCTSkip("could not open in-memory sqlite")
        }
    }

    override func tearDownWithError() throws {
        if db != nil {
            sqlite3_close(db)
            db = nil
        }
        try super.tearDownWithError()
    }

    /// A v4 DB (both indexes present) drops only the redundant single-column
    /// index, keeps the composite, lands at v5, and preserves all rows.
    func testEnsureTables_v4ToV5_dropsRedundantSingleIndex_preservesData() throws {
        seedV4WithBothIndexes()
        exec("INSERT INTO user_association (prev_word, prev_tl, next_word, next_tl, count) VALUES ('食','tsia̍h','飯','pn̄g',5)")
        exec("PRAGMA user_version = 4")
        XCTAssertTrue(indexExists("idx_user_prev_word"), "precondition: v4 has the single-column index")

        try NextWordSchema.ensureTables(db: db, logger: logger)

        XCTAssertFalse(indexExists("idx_user_prev_word"), "v5 drops the redundant single-column index")
        XCTAssertTrue(indexExists("idx_user_prev_word_tl"), "the (prev_word, prev_tl) composite is retained")
        XCTAssertEqual(sqliteReadUserVersion(db: db), 5, "user_version bumped to 5")
        XCTAssertEqual(rowCount(), 1, "no data loss across the index-drop migration")
    }

    /// A v3 DB (no `prev_tl` column, has the single-column index) upgrading
    /// straight to v5 must STILL drop the single index — the regression the
    /// unconditional-drop fix guards (the old `>= 4` gated step skipped it).
    func testEnsureTables_v3ToV5_addsPrevTlAndDropsSingleIndex() throws {
        // v3 layout: no prev_tl column, single-column index only.
        exec("""
            CREATE TABLE user_association (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                prev_word TEXT NOT NULL,
                next_word TEXT NOT NULL,
                next_tl TEXT DEFAULT '',
                count INTEGER DEFAULT 1,
                last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(prev_word, next_word, next_tl)
            );
        """)
        exec("CREATE INDEX idx_user_prev_word ON user_association(prev_word)")
        exec("INSERT INTO user_association (prev_word, next_word, next_tl, count) VALUES ('食','飯','pn̄g',5)")
        exec("PRAGMA user_version = 3")

        try NextWordSchema.ensureTables(db: db, logger: logger)

        XCTAssertFalse(indexExists("idx_user_prev_word"), "v3→v5 drops the single index (chaining-gap regression guard)")
        XCTAssertTrue(indexExists("idx_user_prev_word_tl"), "composite added by the v3→v4 step")
        XCTAssertTrue(columnExists("prev_tl"), "v3→v4 ALTER added prev_tl")
        XCTAssertEqual(sqliteReadUserVersion(db: db), 5)
        XCTAssertEqual(rowCount(), 1, "no data loss")
    }

    /// A fresh DB lands at v5 with only the composite index — the
    /// single-column index is never created.
    func testEnsureTables_freshDb_createsOnlyCompositeIndex() throws {
        try NextWordSchema.ensureTables(db: db, logger: logger)

        XCTAssertFalse(indexExists("idx_user_prev_word"), "fresh v5 never creates the single-column index")
        XCTAssertTrue(indexExists("idx_user_prev_word_tl"))
        XCTAssertEqual(sqliteReadUserVersion(db: db), 5)
    }

    // MARK: - Helpers

    private func seedV4WithBothIndexes() {
        exec("""
            CREATE TABLE user_association (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                prev_word TEXT NOT NULL,
                prev_tl TEXT DEFAULT '',
                next_word TEXT NOT NULL,
                next_tl TEXT DEFAULT '',
                count INTEGER DEFAULT 1,
                last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(prev_word, next_word, next_tl)
            );
        """)
        exec("CREATE INDEX idx_user_prev_word ON user_association(prev_word)")
        exec("CREATE INDEX idx_user_prev_word_tl ON user_association(prev_word, prev_tl)")
    }

    private func exec(_ sql: String) {
        XCTAssertEqual(sqlite3_exec(db, sql, nil, nil, nil), SQLITE_OK, "exec failed: \(sql)")
    }

    private func indexExists(_ name: String) -> Bool {
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, "SELECT 1 FROM sqlite_master WHERE type='index' AND name=?", -1, &stmt, nil) == SQLITE_OK else {
            return false
        }
        defer { sqlite3_finalize(stmt) }
        stmt.bindText(1, name)
        return sqlite3_step(stmt) == SQLITE_ROW
    }

    private func columnExists(_ column: String) -> Bool {
        sqliteColumnExists(db: db, table: "user_association", column: column)
    }

    private func rowCount() -> Int {
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, "SELECT COUNT(*) FROM user_association", -1, &stmt, nil) == SQLITE_OK else {
            return -1
        }
        defer { sqlite3_finalize(stmt) }
        return sqlite3_step(stmt) == SQLITE_ROW ? Int(sqlite3_column_int(stmt, 0)) : -1
    }
}
