import Foundation
import SQLite3

/// One learned phrase (§50): the `(Hanji, canonical-TL)` pair the user once
/// composed segment by segment, and how often it was composed or picked.
struct LearnedPhrase: Equatable {
    let hanzi: String
    let canonicalTl: String
    let learnCount: Int
}

/// Repository for `learned_phrases.db` (`LearnedPhraseSchema`, §50).
///
/// Learning data, not the user's dictionary: the keyboard writes it from
/// `Effect.PhraseLearned`, reads it back per keystroke as
/// `FetchAtPos.learned_entries`, and the only user-facing operation is the
/// learning-data wipe (`deleteAll`). Stored in the App Group container so
/// the extension and the host app share it; writes are serialized behind
/// `SQLiteConnectionManager`, the hot-path read is synchronous.
final class LearnedPhraseRepository: @unchecked Sendable {
    // MARK: - Properties

    private let connectionManager: SQLiteConnectionManager

    private let stateLock = NSLock()
    private var _tableCreationTask: Task<Void, Error>?
    private var _tableCreationGeneration: UInt64 = 0

    /// Row quota; the production default is the cross-platform constant,
    /// tests lower it to exercise eviction cheaply.
    private let maxEntries: Int

    /// Learned rows kept; past it the fewest-composed, then least recently
    /// touched, row goes (ChiaKey's policy) so a learn never fails.
    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/LearnedPhraseService.kt (MAX_ENTRIES).
    // Drift causes silent divergence.
    static let maxEntries = 2000

    /// Largest `learn_count` a row can carry — the column is bound as `Int32`.
    static let maxLearnCount = 1_000_000

    // MARK: - Initialization

    init(
        connectionManager: SQLiteConnectionManager? = nil,
        maxEntries: Int = LearnedPhraseRepository.maxEntries,
    ) {
        self.connectionManager = connectionManager ?? SQLiteConnectionManager(
            databasePath: { try SharedDatabasePath.resolve(filename: "learned_phrases.db") },
            queueLabel: "com.siansiansu.taigikeyboard.learnedphrases",
            loggerCategory: "LearnedPhraseRepository",
        )
        self.maxEntries = maxEntries
    }

    func ensureInitialized() async throws {
        try await connectionManager.ensureInitialized(
            flags: SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
        )
        try await createTablesIfNeeded()
    }

    // MARK: - Learning

    /// Record one `Effect.PhraseLearned`: insert the pair or bump its
    /// `learn_count`, in one statement on the `(hanzi, roman)` unique
    /// constraint. A fresh row gets its search keys and may evict past the
    /// cap; all of it in ONE transaction so a crash cannot leave a row
    /// without keys and the host app's wipe cannot interleave.
    func learnPhrase(hanzi: String, canonicalTl: String) async throws {
        guard !hanzi.isEmpty, !canonicalTl.isEmpty else { return }
        let cap = maxEntries
        try await ensureInitialized()
        try await connectionManager.execute { db in
            try sqliteTransaction(db: db) {
                var stmt: OpaquePointer?
                defer { sqlite3_finalize(stmt) }
                guard sqlite3_prepare_v2(db, Self.learnSQL, -1, &stmt, nil) == SQLITE_OK else {
                    throw LexiconError.queryPreparationFailed("Learn phrase: \(String(cString: sqlite3_errmsg(db)))")
                }
                stmt.bindText(1, canonicalTl)
                stmt.bindText(2, hanzi)
                guard sqlite3_step(stmt) == SQLITE_ROW else {
                    throw LexiconError.queryExecutionFailed("Learn phrase: \(String(cString: sqlite3_errmsg(db)))")
                }
                let rowId = sqlite3_column_int64(stmt, 0)
                // `learn_count` reads 1 only for the row this statement just
                // inserted (a bump lands at 2 or more) — the one case that
                // needs keys and can push the table past its cap.
                guard sqlite3_column_int(stmt, 1) == 1 else { return }
                try Self.writeSearchKeys(db: db, phraseId: rowId, roman: canonicalTl)
                try Self.evictPastCap(db: db, cap: cap, keeping: rowId)
            }
        }
    }

    /// Bump a phrase the user just committed as one candidate, so a phrase
    /// that is used stays ahead of the eviction line. No-op for an unknown pair.
    func touchPhrase(hanzi: String, canonicalTl: String) async throws {
        guard !hanzi.isEmpty, !canonicalTl.isEmpty else { return }
        try await ensureInitialized()
        try await connectionManager.execute { db in
            var stmt: OpaquePointer?
            defer { sqlite3_finalize(stmt) }
            guard sqlite3_prepare_v2(db, Self.touchSQL, -1, &stmt, nil) == SQLITE_OK else {
                throw LexiconError.queryPreparationFailed("Touch phrase: \(String(cString: sqlite3_errmsg(db)))")
            }
            stmt.bindText(1, hanzi)
            stmt.bindText(2, canonicalTl)
            // `SQLITE_DONE` whether or not a row matched — an unknown pair is
            // a successful no-op, only a SQLite failure throws.
            guard sqlite3_step(stmt) == SQLITE_DONE else {
                throw LexiconError.queryExecutionFailed("Touch phrase: \(String(cString: sqlite3_errmsg(db)))")
            }
        }
    }

    // MARK: - Reads

    /// Phrases whose derived key EQUALS the query key — the whole typed
    /// buffer, not a prefix — for `FetchAtPos.learned_entries`. Sync for the
    /// keyboard hot path; `[]` before the connection is open (the first
    /// keystroke after launch must not block on a cold connect).
    func matchesSync(family: String, form: String, key: String, limit: Int = 5) -> [LearnedPhrase] {
        guard connectionManager.isConnected() else { return [] }
        do {
            return try connectionManager.executeSync { db in
                var stmt: OpaquePointer?
                guard sqlite3_prepare_v2(db, Self.exactMatchSQL, -1, &stmt, nil) == SQLITE_OK else { return [] }
                defer { sqlite3_finalize(stmt) }
                stmt.bindText(1, family)
                stmt.bindText(2, form)
                stmt.bindText(3, key)
                sqlite3_bind_int(stmt, 4, Int32(limit))
                return Self.readPhrases(from: stmt)
            }
        } catch {
            return []
        }
    }

    /// Every phrase, most composed first. No product surface lists learned
    /// phrases (USER 2026-09-21) — this is the test seam.
    func allPhrases() async throws -> [LearnedPhrase] {
        try await ensureInitialized()
        return try await connectionManager.execute { db in
            var stmt: OpaquePointer?
            guard sqlite3_prepare_v2(db, Self.allPhrasesSQL, -1, &stmt, nil) == SQLITE_OK else { return [] }
            defer { sqlite3_finalize(stmt) }
            return Self.readPhrases(from: stmt)
        }
    }

    // MARK: - Wipe

    /// The learning-data wipe: every phrase and its keys, by DELETE inside
    /// the open connection — never by unlinking a file the extension may
    /// hold open (`sqlite.org/howtocorrupt.html`).
    func deleteAll() async throws {
        try await ensureInitialized()
        try await connectionManager.execute { db in
            try sqliteTransaction(db: db) {
                try sqliteExecChecked(db: db, "DELETE FROM \(LearnedPhraseSchema.searchKeyTableName);")
                try sqliteExecChecked(db: db, "DELETE FROM \(LearnedPhraseSchema.tableName);")
            }
        }
    }

    // MARK: - Schema (async-once gate)

    /// Single-flight schema initialization; failures clear the cache for retry.
    private func createTablesIfNeeded() async throws {
        let (task, generation) = stateLock.withLock { () -> (Task<Void, Error>, UInt64) in
            if let existing = _tableCreationTask {
                return (existing, _tableCreationGeneration)
            }
            _tableCreationGeneration &+= 1
            let gen = _tableCreationGeneration
            let connection = connectionManager
            let new = Task {
                try await connection.execute { db in
                    try LearnedPhraseSchema.ensureTables(db: db)
                }
            }
            _tableCreationTask = new
            return (new, gen)
        }
        do {
            try await task.value
        } catch {
            stateLock.withLock {
                if _tableCreationGeneration == generation {
                    _tableCreationTask = nil
                }
            }
            throw error
        }
    }

    // MARK: - SQL

    /// `CURRENT_TIMESTAMP` (SQLite's `YYYY-MM-DD HH:MM:SS` UTC text) for the
    /// timestamps, as `user_frequency.db` does — they are only ever ordered.
    private static let learnSQL = """
        INSERT INTO \(LearnedPhraseSchema.tableName) (roman, hanzi, learn_count, updated_at)
        VALUES (?, ?, 1, CURRENT_TIMESTAMP)
        ON CONFLICT(hanzi, roman) DO UPDATE SET
            learn_count = MIN(learn_count + 1, \(maxLearnCount)),
            updated_at = CURRENT_TIMESTAMP
        RETURNING id, learn_count;
    """

    private static let touchSQL = """
        UPDATE \(LearnedPhraseSchema.tableName)
        SET learn_count = MIN(learn_count + 1, \(maxLearnCount)), updated_at = CURRENT_TIMESTAMP
        WHERE hanzi = ? AND roman = ?;
    """

    private static let allPhrasesSQL = """
        SELECT hanzi, roman, learn_count FROM \(LearnedPhraseSchema.tableName)
        ORDER BY learn_count DESC, updated_at DESC, id;
    """

    // CROSS-PLATFORM INVARIANT — mirrors android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/LearnedPhraseService.kt (EXACT_MATCH_SQL). Drift causes silent divergence.
    private static let exactMatchSQL = """
        SELECT p.hanzi, p.roman, p.learn_count
        FROM \(LearnedPhraseSchema.tableName) p
        JOIN \(LearnedPhraseSchema.searchKeyTableName) k ON k.phrase_id = p.id
        WHERE k.family = ? AND k.form = ? AND k.key = ?
        ORDER BY p.learn_count DESC, p.updated_at DESC
        LIMIT ?;
    """

    /// The full cross-mode key bundle for a fresh row, from the same engine
    /// derivation the custom dictionary uses (`CustomDictionaryDerivation`).
    private static func writeSearchKeys(db: OpaquePointer, phraseId: Int64, roman: String) throws {
        let sql = """
            INSERT OR IGNORE INTO \(LearnedPhraseSchema.searchKeyTableName) (phrase_id, family, form, key)
            VALUES (?, ?, ?, ?);
        """
        var stmt: OpaquePointer?
        defer { sqlite3_finalize(stmt) }
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            throw LexiconError.queryPreparationFailed("Learned keys: \(String(cString: sqlite3_errmsg(db)))")
        }
        for searchKey in CustomDictionaryDerivation.searchKeys(for: roman) {
            sqlite3_reset(stmt)
            sqlite3_clear_bindings(stmt)
            sqlite3_bind_int64(stmt, 1, phraseId)
            stmt.bindText(2, searchKey.family)
            stmt.bindText(3, searchKey.form)
            stmt.bindText(4, searchKey.key)
            guard sqlite3_step(stmt) == SQLITE_DONE else {
                throw LexiconError.queryExecutionFailed("Learned keys: \(String(cString: sqlite3_errmsg(db)))")
            }
        }
    }

    /// Drop rows past `cap`, never `keeping` (the row just inserted survives
    /// whatever its timestamp ties with). A cheap `COUNT(*)` first — under
    /// the cap (nearly every user, forever) the ordered walk never runs on
    /// the queue the keystroke read waits on. Keys first, so the subquery
    /// still resolves against the intact main table; `OFFSET cap - 1`
    /// selects exactly the rows past the cap once the kept row is set aside.
    private static func evictPastCap(db: OpaquePointer, cap: Int, keeping keptId: Int64) throws {
        guard try sqliteQueryScalarInt(db: db, "SELECT COUNT(*) FROM \(LearnedPhraseSchema.tableName);") > cap else {
            return
        }
        let pastCap = """
            SELECT id FROM \(LearnedPhraseSchema.tableName)
            WHERE id <> ?
            ORDER BY learn_count DESC, updated_at DESC, id
            LIMIT -1 OFFSET ?
        """
        for table in [
            "\(LearnedPhraseSchema.searchKeyTableName) WHERE phrase_id IN (\(pastCap))",
            "\(LearnedPhraseSchema.tableName) WHERE id IN (\(pastCap))",
        ] {
            var stmt: OpaquePointer?
            defer { sqlite3_finalize(stmt) }
            guard sqlite3_prepare_v2(db, "DELETE FROM \(table);", -1, &stmt, nil) == SQLITE_OK else {
                throw LexiconError.queryPreparationFailed("Learned eviction: \(String(cString: sqlite3_errmsg(db)))")
            }
            sqlite3_bind_int64(stmt, 1, keptId)
            sqlite3_bind_int(stmt, 2, Int32(max(cap - 1, 0)))
            guard sqlite3_step(stmt) == SQLITE_DONE else {
                throw LexiconError.queryExecutionFailed("Learned eviction: \(String(cString: sqlite3_errmsg(db)))")
            }
        }
    }

    // MARK: - Row Reader

    /// Every remaining row of a `hanzi, roman, learn_count` cursor.
    private static func readPhrases(from stmt: OpaquePointer?) -> [LearnedPhrase] {
        var results: [LearnedPhrase] = []
        while sqlite3_step(stmt) == SQLITE_ROW {
            results.append(LearnedPhrase(
                hanzi: sqlite3_column_text(stmt, 0).map(String.init(cString:)) ?? "",
                canonicalTl: sqlite3_column_text(stmt, 1).map(String.init(cString:)) ?? "",
                learnCount: Int(sqlite3_column_int(stmt, 2)),
            ))
        }
        return results
    }
}
