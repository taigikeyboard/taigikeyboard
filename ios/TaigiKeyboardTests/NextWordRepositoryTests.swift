import SQLite3
@testable import TaigiKeyboard
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
        // The real schema, so the tests exercise the shipped key + indexes.
        try NextWordSchema.ensureTables(
            db: db,
            logger: DebugLogger(category: "NextWordRepositoryTests"),
        )
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

    /// §24 — v6 stores the two readings of a 一字多音 previous word separately
    /// and the Hanji-only lookup returns BOTH. The engine keeps only the first
    /// user row per predicted `(hanzi, tl)`, so this query's job is to put the
    /// right one first: the reading actually being typed, ahead of the other
    /// reading even when that one has the higher count.
    ///
    /// **Row order is the contract.** If this ordering drifts, the engine
    /// silently scores the wrong reading's evidence.
    func testFetchUserRows_polyphonicPrevWord_ordersTypedReadingFirst() {
        _ = NextWordRepository.batchImport(db: db, entries: [
            (prevWord: "重", prevTl: "tîng", nextWord: "複", nextTl: "ho̍k", count: 2),
            (prevWord: "重", prevTl: "tāng", nextWord: "複", nextTl: "ho̍k", count: 9),
        ])

        let rows = NextWordRepository.fetchUserRows(db: db, word: "重", roman: "tîng", limit: 10)

        XCTAssertEqual(rows.count, 2, "both readings are recalled — the engine picks, not the SQL")
        XCTAssertEqual(
            rows.first?.count, 2,
            "the tîng row is first on tier despite the tāng row's higher count",
        )
    }

    /// The fallback tier: with no exact match, an empty (legacy) `prev_tl`
    /// still comes before another reading's.
    func testFetchUserRows_emptyPrevTlOrdersBeforeMismatch() {
        _ = NextWordRepository.batchImport(db: db, entries: [
            (prevWord: "重", prevTl: "", nextWord: "複", nextTl: "ho̍k", count: 1),
            (prevWord: "重", prevTl: "tāng", nextWord: "複", nextTl: "ho̍k", count: 9),
        ])

        let rows = NextWordRepository.fetchUserRows(db: db, word: "重", roman: "tîng", limit: 10)

        XCTAssertEqual(rows.first?.count, 1, "empty prev_tl outranks a mismatched reading")
    }

    /// The order must be TOTAL, not just tier-then-count: the engine keeps the
    /// first user row, so two rows tied on tier and count must still have a
    /// defined winner. `last_used DESC` decides, and `id ASC` after that.
    func testFetchUserRows_tiedTierAndCount_ordersByRecencyThenId() {
        exec("INSERT INTO user_association (id, prev_word, prev_tl, next_word, next_tl, count, last_used) VALUES (1,'重','tāng','複','ho̍k',5,'2026-01-01 00:00:00')")
        exec("INSERT INTO user_association (id, prev_word, prev_tl, next_word, next_tl, count, last_used) VALUES (2,'重','tshiòng','複','ho̍k',5,'2026-02-01 00:00:00')")

        let byRecency = NextWordRepository.fetchUserRows(db: db, word: "重", roman: "tîng", limit: 10)
        XCTAssertEqual(
            byRecency.first?.lastUsedMs,
            timestampMs("2026-02-01 00:00:00"),
            "tied on tier and count → the more recent row is first",
        )

        // Now tie on recency too; only `id ASC` can break it.
        exec("UPDATE user_association SET last_used = '2026-01-01 00:00:00' WHERE id = 2")
        let byId = NextWordRepository.fetchUserRows(db: db, word: "重", roman: "tîng", limit: 10)
        XCTAssertEqual(byId.count, 2, "precondition: both rows still recalled")
        XCTAssertEqual(
            byId.first?.lastUsedMs, byId.last?.lastUsedMs,
            "precondition: the rows are now tied on recency too",
        )
    }

    /// Two readings of the NEXT word are genuinely different predictions and
    /// both must reach the engine (Core Principle #7 on the next side).
    func testFetchUserRows_differentNextTl_bothPredictionsRecalled() {
        _ = NextWordRepository.batchImport(db: db, entries: [
            (prevWord: "真", prevTl: "tsin", nextWord: "重", nextTl: "tāng", count: 4),
            (prevWord: "真", prevTl: "tsin", nextWord: "重", nextTl: "tîng", count: 3),
        ])

        let rows = NextWordRepository.fetchUserRows(db: db, word: "真", roman: "tsin", limit: 10)

        XCTAssertEqual(rows.count, 2, "重/tāng and 重/tîng are different predictions")
        XCTAssertEqual(Set(rows.map(\.tl)), ["tāng", "tîng"])
    }

    // MARK: - Helpers

    private func exec(_ sql: String) {
        XCTAssertEqual(sqlite3_exec(db, sql, nil, nil, nil), SQLITE_OK, "exec failed: \(sql)")
    }

    /// The SELECT converts `last_used` via `strftime('%s', …) * 1000`, so the
    /// expected value has to come from SQLite, not from a hand-computed epoch.
    private func timestampMs(_ text: String) -> Int64 {
        var stmt: OpaquePointer?
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_prepare_v2(db, "SELECT strftime('%s', '\(text)') * 1000", -1, &stmt, nil) == SQLITE_OK,
              sqlite3_step(stmt) == SQLITE_ROW
        else {
            XCTFail("could not compute expected timestamp")
            return 0
        }
        return sqlite3_column_int64(stmt, 0)
    }
}
