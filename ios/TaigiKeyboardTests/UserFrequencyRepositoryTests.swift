import SQLite3
@testable import TaigiKeyboard
import XCTest

/// v3.6.1 R5 — `user_frequency.db` `(漢字, canonical-TL)` pair-key identity.
///
/// Pins `INVARIANT_USER_FREQ_PAIR_KEY` (`docs/architecture/behavioral-invariants.md`
/// §28): frequency is keyed by the `(word, tl)` PAIR (Core Principle #7), so
/// 一字多音 (重/tîng vs 重/tāng) keep separate counts. Pre-R5 rows carry
/// `tl == ""` and the migration backfills them as the legacy fallback bucket.
///
/// Exercises the real repository write→batch-read + the schema rebuild
/// migration against a temp-file `SQLiteConnectionManager`. The engine's
/// tolerant `(display, tl) → (display, "")` fallback math is unit-tested in
/// `engine/ranking/src/score.rs`; Android pins this via the shared SQL string
/// + S-series dogfood (no JVM SQLite/`.so` test).
final class UserFrequencyRepositoryTests: XCTestCase {
    private var dbPath: String!
    private var repository: UserFrequencyRepository!

    override func setUpWithError() throws {
        try super.setUpWithError()
        dbPath = NSTemporaryDirectory()
            .appending("user_freq_pairkey_\(UUID().uuidString).db")
        repository = makeRepository()
    }

    override func tearDownWithError() throws {
        try? repository.deleteDatabase()
        repository = nil
        for suffix in ["", "-wal", "-shm"] where dbPath != nil {
            let path = dbPath! + suffix
            if FileManager.default.fileExists(atPath: path) {
                try? FileManager.default.removeItem(atPath: path)
            }
        }
        dbPath = nil
        try super.tearDownWithError()
    }

    private func makeRepository() -> UserFrequencyRepository {
        let path = dbPath!
        let manager = SQLiteConnectionManager(
            databasePath: { path },
            queueLabel: "test.userfreq.pairkey.\(UUID().uuidString)",
            loggerCategory: "UserFrequencyRepositoryTests",
        )
        return UserFrequencyRepository(connectionManager: manager)
    }

    // MARK: - Pair-key identity

    /// Two readings of 重 (tîng / tāng) accumulate SEPARATE counts — the
    /// pre-R5 hanji-only key merged them into one bucket.
    func test_INVARIANT_USER_FREQ_PAIR_KEY_separatesHomographReadings() async throws {
        try await repository.ensureInitialized()
        await repository.recordWord("重", tl: "tîng")
        await repository.recordWord("重", tl: "tîng")
        await repository.recordWord("重", tl: "tîng")
        await repository.recordWord("重", tl: "tāng")

        let rows = repository.frequencyDataBatch(for: ["重"])
        let byTl = Dictionary(uniqueKeysWithValues: rows.map { ($0.tl, $0.data.count) })
        XCTAssertEqual(byTl["tîng"], 3, "重/tîng count is independent")
        XCTAssertEqual(byTl["tāng"], 1, "重/tāng count is independent")
        XCTAssertEqual(rows.count, 2, "exactly two reading buckets for 重")
    }

    /// A legacy `tl == ""` bucket and a re-learned exact reading COEXIST as
    /// distinct rows — the batch read returns both so the engine can resolve
    /// exact-then-legacy. The legacy row is never overwritten by a pair write.
    func test_INVARIANT_USER_FREQ_PAIR_KEY_legacyAndExactCoexist() async throws {
        try await repository.ensureInitialized()
        await repository.recordWord("重", tl: "") // legacy fallback bucket
        await repository.recordWord("重", tl: "tîng") // re-learned reading

        let rows = repository.frequencyDataBatch(for: ["重"])
        let tls = Set(rows.map(\.tl))
        XCTAssertTrue(tls.contains(""), "legacy tl=\"\" bucket retained")
        XCTAssertTrue(tls.contains("tîng"), "exact reading bucket present")
        XCTAssertEqual(rows.count, 2, "legacy + exact are distinct rows, never merged")
    }

    /// The single-word aggregate accessor sums across readings (UI / compat).
    func test_userFrequency_singleWordAccessor_aggregatesReadings() async throws {
        try await repository.ensureInitialized()
        await repository.recordWord("重", tl: "tîng")
        await repository.recordWord("重", tl: "tîng")
        await repository.recordWord("重", tl: "tāng")

        XCTAssertEqual(repository.count(for: "重"), 3, "aggregate count = tîng(2) + tāng(1)")
    }

    // MARK: - Migration

    /// A pre-R5 table (inline `word UNIQUE`, no `tl`) is rebuilt into the
    /// `(word, tl)` pair-key shape on first init; existing rows backfill
    /// `tl == ""` with count / id preserved.
    func test_INVARIANT_USER_FREQ_PAIR_KEY_migrationBackfillsEmptyTl() async throws {
        seedPreR5Schema(word: "重", count: 5)

        // Fresh repository over the same path triggers the migration.
        repository = makeRepository()
        try await repository.ensureInitialized()

        let rows = repository.frequencyDataBatch(for: ["重"])
        XCTAssertEqual(rows.count, 1, "one migrated row for 重")
        XCTAssertEqual(rows.first?.tl, "", "pre-R5 row backfilled to the legacy tl=\"\" bucket")
        XCTAssertEqual(rows.first?.data.count, 5, "count preserved through the rebuild")

        // The migrated row still increments in place on a matching write.
        await repository.recordWord("重", tl: "")
        XCTAssertEqual(repository.count(for: "重"), 6, "legacy bucket increments, not duplicates")
    }

    /// Directly create the pre-R5 `user_frequency` schema + seed one row, then
    /// close the connection so the repository under test reopens + migrates it.
    private func seedPreR5Schema(word: String, count: Int) {
        var db: OpaquePointer?
        XCTAssertEqual(sqlite3_open(dbPath, &db), SQLITE_OK)
        defer { sqlite3_close(db) }
        let ddl = """
            CREATE TABLE user_frequency (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word TEXT NOT NULL UNIQUE,
                count INTEGER DEFAULT 1,
                last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX idx_word ON user_frequency(word);
            INSERT INTO user_frequency (word, count) VALUES ('\(word)', \(count));
        """
        XCTAssertEqual(sqlite3_exec(db, ddl, nil, nil, nil), SQLITE_OK)
    }
}
