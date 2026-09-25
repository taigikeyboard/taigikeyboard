import Foundation
import SQLite3

/// NextWord next-word prediction service (facade).
///
/// Owns `user_association.db`, the user-learned half of next-word prediction.
/// The bundled `association.bin` half, scoring, merging and shaping run in the
/// engine: the caller hands `userRows(word:roman:)` to
/// `RustEngineBridge.nextwordPredictNext`.
///
/// The facade owns the public API, concurrency state, capacity policy and wiring; `NextWordSchema`
/// owns the schema and `NextWordRepository` the CRUD.
final class NextWordService: @unchecked Sendable {
    // MARK: - Constants (capacity policy)

    private enum Constants {
        static let defaultLimit = 30

        /// User-association capacity
        static let maxUserAssociations = 50000
        static let pruneCheckInterval = 100
        static let pruneBatchSize = 5000
    }

    // MARK: - Public Types

    /// A user association row, the unit `BackupService` exports and imports.
    struct AssociationEntry {
        let prevWord: String
        let prevTl: String
        let nextWord: String
        let nextTl: String
        let count: Int

        init(prevWord: String, prevTl: String, nextWord: String, nextTl: String, count: Int) {
            self.prevWord = prevWord
            self.prevTl = prevTl
            self.nextWord = nextWord
            self.nextTl = nextTl
            self.count = count
        }

        init(row: NextWordRepository.AssociationRow) {
            self.init(
                prevWord: row.prevWord, prevTl: row.prevTl,
                nextWord: row.nextWord, nextTl: row.nextTl,
                count: row.count,
            )
        }
    }

    // MARK: - Properties

    private let userConnectionManager: SQLiteConnectionManager
    private let logger = DebugLogger(category: "NextWordService")

    /// Lock protecting mutable state (`_recordCounter`, `_tableCreationTask`).
    private let stateLock = NSLock()
    private var _recordCounter = 0
    /// Async-once gate for schema creation/migration — concurrent callers
    /// await the same `Task`; nil cache on failure allows retry.
    private var _tableCreationTask: Task<Void, Error>?
    /// Bumped every time `_tableCreationTask` is replaced. Used instead of
    /// identity comparison (Task is a struct, `===` unavailable) to ensure
    /// error handlers only clear the cache they created.
    private var _tableCreationGeneration: UInt64 = 0

    // MARK: - Initialization

    init() {
        userConnectionManager = SQLiteConnectionManager(
            databasePath: Self.getUserDatabasePath,
            queueLabel: "com.siansiansu.taigikeyboard.nextword.user",
            loggerCategory: "NextWordService.User",
        )
    }

    // MARK: - Public API: Prediction

    /// Learned rows following `word`, tagged `.user`, best evidence first —
    /// the order `nextwordPredictNext` must receive them in (§24). Over-fetches
    /// `limit * 2` so the engine's `(hanzi, tl)` merge keeps `limit` survivors.
    /// An empty `word` predicts nothing.
    func userRows(
        word: String,
        roman: String = "",
        limit: Int = Constants.defaultLimit,
    ) async -> [RustEngineBridge.NextWordRawRow] {
        guard !word.isEmpty else { return [] }
        do {
            try await ensureUserTablesCreated()
            let userRows = try await userConnectionManager.execute { db in
                NextWordRepository.fetchUserRows(db: db, word: word, roman: roman, limit: limit * 2)
            }
            logger.debug("[PREDICT][USER] rows=\(userRows.count) for word='\(word)'")
            return userRows.map { row in
                RustEngineBridge.NextWordRawRow(
                    hanzi: row.hanzi,
                    tl: row.tl,
                    count: Int64(row.count),
                    lastUsedMs: row.lastUsedMs,
                    source: .user,
                )
            }
        } catch {
            logger.error("[USER] Query failed: \(error.localizedDescription)")
            return []
        }
    }

    // MARK: - Public API: Recording

    func recordAssociation(
        prev: String,
        prevTl: String = "",
        nextHanzi: String,
        nextTl: String = "",
    ) async {
        guard !prev.isEmpty, !nextHanzi.isEmpty else { return }

        do {
            try await ensureUserTablesCreated()
            try await userConnectionManager.execute { db in
                try NextWordRepository.insertOrUpdate(
                    db: db, prev: prev, prevTl: prevTl,
                    nextHanzi: nextHanzi, nextTl: nextTl,
                )
            }
            logger.debug("[RECORD] '\(prev)' -> '\(nextHanzi)'")

            let shouldPrune: Bool = stateLock.withLock {
                _recordCounter += 1
                if _recordCounter >= Constants.pruneCheckInterval {
                    _recordCounter = 0
                    return true
                }
                return false
            }
            if shouldPrune {
                await pruneOldAssociations()
            }
        } catch {
            // Fire-and-forget: a failed association write must never block
            // typing. Single boundary — `insertOrUpdate` throws the real
            // error so this logs once. DebugLogger no-ops in release.
            logger.error("association.record.failed prev=\(prev) next=\(nextHanzi) error=\(error.localizedDescription)")
        }
    }

    func clearAllAssociations() async {
        do {
            try await ensureUserTablesCreated()
            try await userConnectionManager.execute { db in
                NextWordRepository.deleteAll(db: db)
                // R6: reclaim freed pages after a full clear. Best-effort —
                // VACUUM needs exclusive access + ~2x temp; a failure leaves
                // the file larger but intact (sqliteExecSimple is silent). Runs
                // after the DELETE committed, outside any transaction.
                sqliteExecSimple(db: db, "VACUUM")
            }
            logger.info("[CLEAR] All user associations cleared")
        } catch {
            logger.error("[CLEAR] Failed: \(error.localizedDescription)")
        }
    }

    /// Delete a single user association entry.
    func deleteAssociation(_ entry: AssociationEntry) async {
        do {
            try await ensureUserTablesCreated()
            try await userConnectionManager.execute { db in
                NextWordRepository.deleteOne(
                    db: db,
                    prev: entry.prevWord, prevTl: entry.prevTl,
                    nextHanzi: entry.nextWord, nextTl: entry.nextTl,
                )
            }
        } catch {
            logger.error("[DELETE] Failed to delete association: \(error.localizedDescription)")
        }
    }

    /// Import association entries with merge-by-max strategy: keep the higher count.
    func batchImportAssociations(
        entries: [(prevWord: String, prevTl: String, nextWord: String, nextTl: String, count: Int)],
    ) async throws -> Int {
        try await ensureUserTablesCreated()
        return try await userConnectionManager.execute { db in
            NextWordRepository.batchImport(db: db, entries: entries)
        }
    }

    func associationCount() async -> Int {
        do {
            try await ensureUserTablesCreated()
            return try await userConnectionManager.execute { db in
                NextWordRepository.count(db: db)
            }
        } catch {
            return 0
        }
    }

    /// All user associations, for backup and UI listing.
    func allAssociations() async -> [AssociationEntry] {
        do {
            try await ensureUserTablesCreated()
            return try await userConnectionManager.execute { db in
                NextWordRepository.fetchAllRows(db: db).map(AssociationEntry.init(row:))
            }
        } catch {
            logger.error("[USER] allAssociations failed: \(error.localizedDescription)")
            return []
        }
    }

    // MARK: - Lifecycle

    func deleteUserDatabase() throws {
        // Cancel the in-flight init Task (if any) BEFORE closing the
        // connection so it bails out rather than racing against a fresh
        // Task installed by the next caller.
        let priorTask = stateLock.withLock { () -> Task<Void, Error>? in
            let task = _tableCreationTask
            _tableCreationTask = nil
            _tableCreationGeneration &+= 1
            return task
        }
        priorTask?.cancel()
        userConnectionManager.close()

        let path = try Self.getUserDatabasePath()
        if FileManager.default.fileExists(atPath: path) {
            try FileManager.default.removeItem(atPath: path)
        }
    }

    // MARK: - Schema (async-once gate)

    /// Single-flight schema initialization. Concurrent callers await the
    /// same `Task`; failures clear the cache so the next caller retries.
    private func ensureUserTablesCreated() async throws {
        let (task, generation) = stateLock.withLock { () -> (Task<Void, Error>, UInt64) in
            if let existing = _tableCreationTask {
                return (existing, _tableCreationGeneration)
            }
            _tableCreationGeneration &+= 1
            let gen = _tableCreationGeneration
            let logger = self.logger
            let connection = self.userConnectionManager
            let new = Task {
                try await connection.ensureInitialized(
                    flags: SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE,
                )
                try await connection.execute { db in
                    try NextWordSchema.ensureTables(db: db, logger: logger)
                }
            }
            _tableCreationTask = new
            return (new, gen)
        }
        do {
            try await task.value
        } catch {
            stateLock.withLock {
                // Only clear if we still own the cached task generation —
                // avoids wiping a newer task that a later caller installed.
                if _tableCreationGeneration == generation {
                    _tableCreationTask = nil
                }
            }
            throw error
        }
    }

    // MARK: - Pruning

    /// Prune the oldest/lowest-count entries when over capacity.
    /// Over-deletes by `pruneBatchSize` so the table sits below the cap
    /// between prune runs instead of oscillating around it.
    private func pruneOldAssociations() async {
        do {
            let currentCount = await associationCount()
            guard currentCount > Constants.maxUserAssociations else {
                logger.debug("[PRUNE] No pruning needed: \(currentCount) <= \(Constants.maxUserAssociations)")
                return
            }

            let deleteCount = min(
                Constants.pruneBatchSize,
                currentCount - Constants.maxUserAssociations + Constants.pruneBatchSize,
            )

            try await userConnectionManager.execute { db in
                NextWordRepository.pruneOldest(db: db, limit: deleteCount)
            }

            logger.info("[PRUNE] Deleted \(deleteCount) associations (was \(currentCount))")
        } catch {
            logger.error("[PRUNE] Failed: \(error.localizedDescription)")
        }
    }

    // MARK: - Database Path

    private static func getUserDatabasePath() throws -> String {
        try SharedDatabasePath.resolve(filename: "user_association.db")
    }
}
