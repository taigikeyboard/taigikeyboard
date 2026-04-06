import SQLite3
import XCTest

/// Direct SQL verification of dictionary.db content for 教育部臺灣台語常用詞辭典 (kautian=1).
///
/// Guards against dictionary rebuild regressions that silently drop entries.
/// Layers:
/// 1. Total count — entry count must not decrease
/// 2. (tl, hanzi) uniqueness — no accidental duplicates
/// 3. Known tone variant cases — specific entries that previously caused issues
/// 4. Data integrity — no empty tl, no orphan entries
/// 5. Data quality baselines — empty hanzi, zero frequency, syllable-hanzi mismatch
final class DictionaryContentTests: XCTestCase {
    private var db: OpaquePointer?

    override func setUpWithError() throws {
        try super.setUpWithError()

        // Prefer test bundle resource (works in simulator sandbox on macOS 26+)
        // Fallback to #filePath-based resolution for local development
        let dbPath: String
        if let bundlePath = Bundle(for: type(of: self))
            .path(forResource: "dictionary", ofType: "db")
        {
            dbPath = bundlePath
        } else {
            let testFileURL = URL(fileURLWithPath: #filePath)
            let iosDir = testFileURL
                .deletingLastPathComponent() // TaigiKeyboardTests/
                .deletingLastPathComponent() // ios/
            dbPath = iosDir
                .appendingPathComponent("Resources/Dictionaries/dictionary.db")
                .path
        }

        let rc = sqlite3_open_v2(dbPath, &db, SQLITE_OPEN_READONLY, nil)
        guard rc == SQLITE_OK else {
            XCTFail("Failed to open dictionary.db at \(dbPath): \(rc)")
            return
        }
    }

    override func tearDown() {
        if let db {
            sqlite3_close(db)
        }
        db = nil
        super.tearDown()
    }

    // MARK: - Layer 1: Total Count

    /// Verifies kautian=1 entry count has not decreased from the known baseline.
    /// If a dictionary rebuild legitimately adds entries, update the baseline.
    func testKautian_totalCountNotDecreased() throws {
        let db = try XCTUnwrap(db, "database not opened")
        let count = querySingleInt(db: db, sql: "SELECT COUNT(*) FROM dictionary WHERE kautian = 1")
        XCTAssertGreaterThanOrEqual(count, 49612,
                                    "kautian=1 entry count dropped: got \(count), baseline is 49612")
    }

    // MARK: - Layer 2: (tl, hanzi) Uniqueness

    /// Verifies no duplicate (tl, hanzi) pairs exist within kautian=1 entries.
    func testKautian_noDuplicateTlHanziPairs() throws {
        let db = try XCTUnwrap(db, "database not opened")
        let dupes = querySingleInt(db: db, sql: """
        SELECT COUNT(*) FROM (
            SELECT tl, hanzi FROM dictionary WHERE kautian = 1
            GROUP BY tl, hanzi HAVING COUNT(*) > 1
        )
        """)
        XCTAssertEqual(dupes, 0, "Found \(dupes) duplicate (tl, hanzi) pairs in kautian=1")
    }

    // MARK: - Layer 3: Known Tone Variant Cases

    /// Verifies that known multi-tone words all exist in dictionary.db.
    /// Each case represents a past regression or a known tone-variant pair.
    func testKautian_toneVariantsExist() throws {
        let db = try XCTUnwrap(db, "database not opened")

        // (description, expected tl values, expected hanzi)
        let cases: [(label: String, tlValues: [String], hanzi: String)] = [
            // 伴手: tone 2 vs 7
            ("伴手 tone variants",
             ["phuǎnn-tshiú", "phuānn-tshiú"], "伴手"),
            // 抱歉: 3 tone variants on second syllable
            ("抱歉 tone variants",
             ["phō-khiàm", "phō-khiam", "phō-khiám"], "抱歉"),
            // 病院: 3 tone variants across both syllables
            ("病院 tone variants",
             ["pīnn-ìnn", "pīnn-īnn", "pǐnn-ǐnn"], "病院"),
            // 貧惰: 3 variants, tone differs on both syllables
            ("貧惰 tone variants",
             ["pân-tuānn", "pân-tuǎnn", "pān-tuānn"], "貧惰"),
            // 下昏: tone 1 vs 7
            ("下昏 tone variants",
             ["e-hng", "ē-hng"], "下昏"),
            // 假期: tone 3 vs 2
            ("假期 tone variants",
             ["kà-kî", "ká-kî"], "假期"),
        ]

        for tc in cases {
            for tl in tc.tlValues {
                let count = queryCountWhere(db: db, tl: tl, hanzi: tc.hanzi, kautian: true)
                XCTAssertGreaterThan(count, 0,
                                     "\(tc.label): tl=\"\(tl)\" hanzi=\"\(tc.hanzi)\" not found in kautian=1")
            }
        }
    }

    // MARK: - Layer 4: Data Integrity

    /// Every kautian=1 entry must have a non-empty tl field, otherwise it is unreachable by search.
    func testKautian_noEmptyTl() throws {
        let db = try XCTUnwrap(db, "database not opened")
        let count = querySingleInt(db: db, sql:
            "SELECT COUNT(*) FROM dictionary WHERE kautian = 1 AND (tl IS NULL OR tl = '')")
        XCTAssertEqual(count, 0, "Found \(count) kautian=1 entries with empty tl — these are unsearchable")
    }

    /// Every entry must belong to at least one dictionary source, otherwise no filter combination can find it.
    func testNoOrphanEntries() throws {
        let db = try XCTUnwrap(db, "database not opened")
        let count = querySingleInt(db: db, sql: """
        SELECT COUNT(*) FROM dictionary WHERE
          kautian = 0 AND taigitv = 0 AND itaigi = 0 AND sitbut = 0 AND
          taihoa = 0 AND taijit = 0 AND kungge = 0 AND stti = 0 AND
          khpoo = 0 AND khiin = 0 AND dev = 0 AND lkk = 0
        """)
        XCTAssertEqual(count, 0, "Found \(count) orphan entries with all source flags = 0")
    }

    // MARK: - Layer 5: Data Quality Baselines

    /// kautian=1 entries with empty hanzi (romanization-only words like loanwords).
    /// Not a bug, but count should not grow — growth indicates data corruption.
    func testKautian_emptyHanziCountStable() throws {
        let db = try XCTUnwrap(db, "database not opened")
        let count = querySingleInt(db: db, sql:
            "SELECT COUNT(*) FROM dictionary WHERE kautian = 1 AND (hanzi IS NULL OR hanzi = '')")
        XCTAssertLessThanOrEqual(count, 198,
                                 "kautian=1 empty-hanzi count grew to \(count), baseline is 198 — check for data corruption")
    }

    /// kautian=1 entries with frequency <= 0 (rare/literary characters).
    /// Not a bug, but count should not grow unexpectedly.
    func testKautian_zeroFrequencyCountStable() throws {
        let db = try XCTUnwrap(db, "database not opened")
        let count = querySingleInt(db: db, sql:
            "SELECT COUNT(*) FROM dictionary WHERE kautian = 1 AND (frequency IS NULL OR frequency <= 0)")
        XCTAssertLessThanOrEqual(count, 785,
                                 "kautian=1 zero-frequency count grew to \(count), baseline is 785 — check for data corruption")
    }

    /// Entries where syllable count (hyphens+1) differs from hanzi character count by more than 1.
    /// Includes legitimate mixed-script entries, but growth indicates data corruption.
    func testKautian_syllableHanziMismatchCountStable() throws {
        let db = try XCTUnwrap(db, "database not opened")
        let count = querySingleInt(db: db, sql: """
        SELECT COUNT(*) FROM (
            SELECT tl, hanzi,
                LENGTH(REPLACE(tl, '--', '-')) - LENGTH(REPLACE(REPLACE(tl, '--', '-'), '-', '')) + 1 AS syllables
            FROM dictionary
            WHERE kautian = 1 AND hanzi IS NOT NULL AND hanzi != '' AND tl IS NOT NULL AND tl != ''
        ) WHERE ABS(syllables - LENGTH(hanzi)) > 1
        """)
        XCTAssertLessThanOrEqual(count, 69,
                                 "kautian=1 syllable-hanzi mismatch count grew to \(count), baseline is 69 — check for data corruption")
    }

    // MARK: - Helpers

    private func querySingleInt(db: OpaquePointer, sql: String) -> Int {
        var stmt: OpaquePointer?
        defer { sqlite3_finalize(stmt) }

        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            XCTFail("Failed to prepare: \(String(cString: sqlite3_errmsg(db)))")
            return 0
        }
        guard sqlite3_step(stmt) == SQLITE_ROW else {
            XCTFail("No result from query: \(sql)")
            return 0
        }
        return Int(sqlite3_column_int(stmt, 0))
    }

    private func queryCountWhere(db: OpaquePointer, tl: String, hanzi: String, kautian: Bool) -> Int {
        let sql = "SELECT COUNT(*) FROM dictionary WHERE tl = ? AND hanzi = ? AND kautian = ?"
        var stmt: OpaquePointer?
        defer { sqlite3_finalize(stmt) }

        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            XCTFail("Failed to prepare: \(String(cString: sqlite3_errmsg(db)))")
            return 0
        }
        sqlite3_bind_text(stmt, 1, (tl as NSString).utf8String, -1, nil)
        sqlite3_bind_text(stmt, 2, (hanzi as NSString).utf8String, -1, nil)
        sqlite3_bind_int(stmt, 3, kautian ? 1 : 0)

        guard sqlite3_step(stmt) == SQLITE_ROW else {
            XCTFail("No result from COUNT query for tl=\"\(tl)\" hanzi=\"\(hanzi)\"")
            return 0
        }
        return Int(sqlite3_column_int(stmt, 0))
    }
}
