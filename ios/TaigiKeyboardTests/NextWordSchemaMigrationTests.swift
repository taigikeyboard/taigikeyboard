import SQLite3
@testable import TaigiKeyboard
import XCTest

/// `user_association.db` schema migrations, up to and including the v6 key
/// widening (`behavioral-invariants.md` §24).
///
/// v6 makes the UNIQUE key `(prev_word, prev_tl, next_word, next_tl)` so the
/// two readings of a 一字多音 previous word stay separate observations. SQLite
/// cannot ALTER a table-level UNIQUE, so every pre-v6 database with a table is
/// rebuilt in one convergent step rather than climbing the old ladder.
///
/// Exercises the SQLite thin layer directly via an in-memory DB — the Android
/// counterpart cannot (no Robolectric / `.so` in JVM unit tests), so Android
/// pins the same behavior via the mirrored `rebuildToV6` + S-series dogfood.
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

    // MARK: - Fresh

    func testEnsureTables_freshDb_createsV6KeyAndRecallIndex() throws {
        try NextWordSchema.ensureTables(db: db, logger: logger)

        XCTAssertEqual(sqliteReadUserVersion(db: db), 6)
        XCTAssertTrue(uniqueKeyCarriesPrevTl(), "fresh v6 key includes prev_tl")
        XCTAssertTrue(indexExists("idx_user_prev_word_tl"), "recall index")
        XCTAssertFalse(indexExists("idx_user_prev_word"), "the v4 single-column index is never created")
    }

    func testEnsureTables_atV6_isIdempotent() throws {
        try NextWordSchema.ensureTables(db: db, logger: logger)
        insertRow(prev: "食", prevTl: "tsia̍h", next: "飯", nextTl: "pn̄g", count: 5)

        try NextWordSchema.ensureTables(db: db, logger: logger)

        XCTAssertEqual(sqliteReadUserVersion(db: db), 6)
        XCTAssertEqual(rowCount(), 1, "re-running at v6 must not rebuild or drop anything")
    }

    // MARK: - v5 → v6

    /// The point of v6: two readings of the same previous Hanji are two rows.
    /// Under v5's `(prev_word, next_word, next_tl)` the second insert would
    /// have overwritten the first's `prev_tl` and summed their counts.
    func testEnsureTables_v6_keepsBothReadingsOfAPolyphonicPrevWord() throws {
        try NextWordSchema.ensureTables(db: db, logger: logger)

        insertRow(prev: "重", prevTl: "tîng", next: "複", nextTl: "ho̍k", count: 1)
        insertRow(prev: "重", prevTl: "tāng", next: "複", nextTl: "ho̍k", count: 1)

        XCTAssertEqual(rowCount(), 2, "重/tîng → 複 and 重/tāng → 複 are separate observations")
    }

    func testEnsureTables_v5ToV6_preservesEveryRowIncludingId() throws {
        seedV5()
        exec("INSERT INTO user_association (id, prev_word, prev_tl, next_word, next_tl, count, last_used) VALUES (7,'食','tsia̍h','飯','pn̄g',5,'2026-01-02 03:04:05')")
        exec("INSERT INTO user_association (id, prev_word, prev_tl, next_word, next_tl, count, last_used) VALUES (9,'台語','tâi-gí','齒盤','khí-puânn',2,'2026-01-03 03:04:05')")
        exec("PRAGMA user_version = 5")

        try NextWordSchema.ensureTables(db: db, logger: logger)

        XCTAssertEqual(sqliteReadUserVersion(db: db), 6)
        XCTAssertTrue(uniqueKeyCarriesPrevTl())
        XCTAssertEqual(rowCount(), 2, "widening a UNIQUE key cannot conflict — nothing is dropped")
        // `id` is the read query's final tiebreak, so renumbering would change
        // which row wins for rows that tie on everything else.
        XCTAssertEqual(ids(), [7, 9], "ids are copied, not reassigned")
        XCTAssertEqual(scalar("SELECT prev_tl FROM user_association WHERE id = 7"), "tsia̍h")
        XCTAssertEqual(scalar("SELECT last_used FROM user_association WHERE id = 7"), "2026-01-02 03:04:05")
        XCTAssertEqual(scalar("SELECT count FROM user_association WHERE id = 7"), "5")
    }

    /// A v3 DB has no `prev_tl` column at all. The rebuild reads the columns it
    /// finds rather than assuming, and backfills `''`.
    func testEnsureTables_v3ToV6_backfillsMissingPrevTlColumn() throws {
        seedWithoutPrevTl()
        exec("CREATE INDEX idx_user_prev_word ON user_association(prev_word)")
        exec("INSERT INTO user_association (prev_word, next_word, next_tl, count) VALUES ('食','飯','pn̄g',5)")
        exec("PRAGMA user_version = 3")

        try NextWordSchema.ensureTables(db: db, logger: logger)

        XCTAssertEqual(sqliteReadUserVersion(db: db), 6)
        XCTAssertTrue(columnExists("prev_tl"))
        XCTAssertTrue(uniqueKeyCarriesPrevTl())
        XCTAssertFalse(indexExists("idx_user_prev_word"), "the v4 single-column index goes with its table")
        XCTAssertEqual(rowCount(), 1, "no data loss")
        XCTAssertEqual(scalar("SELECT prev_tl FROM user_association"), "", "missing prev_tl backfills empty")
    }

    /// The Android ladder defect this round also repairs: a table stamped "v5"
    /// that never got its `prev_tl` column, because the old step gates tested
    /// the ORIGINAL version and so never fired for a v0/v1 database. iOS cannot
    /// reach that state itself; the rebuild is written to survive it either
    /// way, and this pins that.
    func testEnsureTables_stampedV5WithoutPrevTlColumn_isRepaired() throws {
        seedWithoutPrevTl()
        exec("INSERT INTO user_association (prev_word, next_word, next_tl, count) VALUES ('食','飯','pn̄g',5)")
        exec("PRAGMA user_version = 5")

        try NextWordSchema.ensureTables(db: db, logger: logger)

        XCTAssertEqual(sqliteReadUserVersion(db: db), 6)
        XCTAssertTrue(columnExists("prev_tl"), "the missing column is detected, not assumed")
        XCTAssertEqual(rowCount(), 1, "the rows survive the repair")
    }

    /// Pre-v3 shapes carried columns no later version can interpret. They were
    /// already dropped rather than migrated before v6, and still are — a fresh
    /// v6 table takes their place.
    func testEnsureTables_preV3_dropsIncompatibleTableAndLandsAtV6() throws {
        exec("""
            CREATE TABLE user_association (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                prev_word TEXT NOT NULL,
                next_word TEXT NOT NULL,
                next_poj TEXT,
                delimiter TEXT,
                count INTEGER DEFAULT 1,
                last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                UNIQUE(prev_word, next_word)
            );
        """)
        exec("INSERT INTO user_association (prev_word, next_word, count) VALUES ('食','飯',5)")
        exec("PRAGMA user_version = 2")

        try NextWordSchema.ensureTables(db: db, logger: logger)

        XCTAssertEqual(sqliteReadUserVersion(db: db), 6)
        XCTAssertTrue(uniqueKeyCarriesPrevTl())
        XCTAssertEqual(rowCount(), 0, "the incompatible pre-v3 shape is not migrated")
    }

    /// A failed rebuild must leave BOTH the old table and the old version stamp
    /// intact — a half-migrated database stamped v6 is worse than one still
    /// stamped v5. The blocking table makes the `CREATE TABLE …_new` fail.
    func testEnsureTables_rebuildFailure_rollsBackAndLeavesVersionAtV5() throws {
        seedV5()
        exec("INSERT INTO user_association (prev_word, prev_tl, next_word, next_tl, count) VALUES ('食','tsia̍h','飯','pn̄g',5)")
        exec("PRAGMA user_version = 5")
        exec("CREATE TABLE user_association_new (blocker INTEGER)")

        XCTAssertThrowsError(try NextWordSchema.ensureTables(db: db, logger: logger))

        XCTAssertEqual(sqliteReadUserVersion(db: db), 5, "a failed migration never stamps the new version")
        XCTAssertEqual(rowCount(), 1, "the old table is untouched")
        XCTAssertFalse(uniqueKeyCarriesPrevTl(), "still the v5 key")
    }

    // MARK: - Helpers

    private func seedV5() {
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
        exec("CREATE INDEX idx_user_prev_word_tl ON user_association(prev_word, prev_tl)")
    }

    /// The v3 layout, and equally the shape an Android v0/v1 ladder left behind.
    private func seedWithoutPrevTl() {
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
    }

    private func insertRow(prev: String, prevTl: String, next: String, nextTl: String, count: Int) {
        exec("INSERT INTO user_association (prev_word, prev_tl, next_word, next_tl, count) VALUES ('\(prev)','\(prevTl)','\(next)','\(nextTl)',\(count))")
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

    /// Reads the stored `CREATE TABLE` text — the only way to see a table-level
    /// UNIQUE constraint, which `pragma_table_info` does not expose.
    private func uniqueKeyCarriesPrevTl() -> Bool {
        let sql = scalar("SELECT sql FROM sqlite_master WHERE type='table' AND name='user_association'")
        return sql?.contains("UNIQUE(prev_word, prev_tl, next_word, next_tl)") ?? false
    }

    private func ids() -> [Int] {
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, "SELECT id FROM user_association ORDER BY id", -1, &stmt, nil) == SQLITE_OK else {
            return []
        }
        defer { sqlite3_finalize(stmt) }
        var result: [Int] = []
        while sqlite3_step(stmt) == SQLITE_ROW {
            result.append(Int(sqlite3_column_int(stmt, 0)))
        }
        return result
    }

    private func scalar(_ sql: String) -> String? {
        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else { return nil }
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_step(stmt) == SQLITE_ROW else { return nil }
        return sqlite3_column_text(stmt, 0).map { String(cString: $0) }
    }

    private func rowCount() -> Int {
        Int(scalar("SELECT COUNT(*) FROM user_association") ?? "-1") ?? -1
    }
}
