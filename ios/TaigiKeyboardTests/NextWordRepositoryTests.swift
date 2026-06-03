@testable import TaigiKeyboard
import SQLite3
import XCTest

/// v3.6.1 R1 — `user_association.db` next-word recall.
///
/// Pins `docs/architecture/behavioral-invariants.md` §24
/// `INVARIANT_NEXTWORD_PREV_HANJI_LOOKUP`: `fetchUserRows` keys on
/// `prev_word` (Hanji) only; `prev_tl` is a ranking signal (exact > empty
/// > mismatch), NOT a hard filter, so a mismatched non-empty `prev_tl`
/// (continuous-input raw `taigi` vs normal-commit canonical `tâi-gí` for
/// the same word) is still recalled.
///
/// Exercises the SQLite thin layer directly via an in-memory DB — the
/// Android counterpart cannot (no Robolectric / `.so` in JVM unit tests),
/// so Android pins this behavior via the shared SQL string + S-series
/// dogfood.
final class NextWordRepositoryTests: XCTestCase {
    private var db: OpaquePointer!

    override func setUpWithError() throws {
        try super.setUpWithError()
        guard sqlite3_open(":memory:", &db) == SQLITE_OK else {
            throw XCTSkip("could not open in-memory sqlite")
        }
        // Mirrors NextWordSchema.createTables (v4 layout).
        let ddl = """
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
        """
        XCTAssertEqual(sqlite3_exec(db, ddl, nil, nil, nil), SQLITE_OK)
    }

    override func tearDownWithError() throws {
        if db != nil {
            sqlite3_close(db)
            db = nil
        }
        try super.tearDownWithError()
    }

    /// INVARIANT_NEXTWORD_PREV_HANJI_LOOKUP — a mismatched non-empty
    /// `prev_tl` is recalled (was dropped pre-fix), and an exact `prev_tl`
    /// match outranks it even when the mismatch row has a higher count.
    func testFetchUserRows_recallsMismatchedPrevTl_rankedAfterExact() {
        let imported = NextWordRepository.batchImport(db: db, entries: [
            (prevWord: "食", prevTl: "tsia̍h", nextWord: "飯", nextTl: "pn̄g", count: 5), // exact
            (prevWord: "食", prevTl: "", nextWord: "麭", nextTl: "pháng", count: 3), // empty/legacy
            (prevWord: "食", prevTl: "chiah", nextWord: "水", nextTl: "tsuí", count: 10), // mismatch, higher count
        ])
        XCTAssertEqual(imported, 3)

        let rows = NextWordRepository.fetchUserRows(db: db, word: "食", roman: "tsia̍h", limit: 10)
        let hanzi = rows.map(\.hanzi)

        XCTAssertEqual(hanzi.count, 3, "mismatched prev_tl row is recalled, not filtered out")
        XCTAssertTrue(hanzi.contains("水"), "mismatched prev_tl row 水 must be recalled")
        XCTAssertEqual(rows.first?.hanzi, "飯", "exact prev_tl ranks first despite mismatch's higher count")
        XCTAssertEqual(rows.last?.hanzi, "水", "mismatch (CASE 2) ranks last")
    }

    /// The relaxed `WHERE prev_word = ?` stays scoped to its Hanji key —
    /// a different prev_word does not leak in.
    func testFetchUserRows_scopedToPrevWord() {
        _ = NextWordRepository.batchImport(db: db, entries: [
            (prevWord: "食", prevTl: "tsia̍h", nextWord: "飯", nextTl: "pn̄g", count: 5),
            (prevWord: "啉", prevTl: "lim", nextWord: "茶", nextTl: "tê", count: 5),
        ])
        let rows = NextWordRepository.fetchUserRows(db: db, word: "食", roman: "tsia̍h", limit: 10)
        XCTAssertEqual(rows.map(\.hanzi), ["飯"])
    }
}
