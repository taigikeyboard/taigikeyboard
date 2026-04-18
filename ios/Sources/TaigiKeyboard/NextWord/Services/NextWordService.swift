import Foundation
import SQLite3

/// NextWord 下一詞預測服務 (facade)
///
/// 使用「相鄰字 Bigram」模型預測下一個字：
/// - 選擇「早安」→ 用「安」查詢 → 預測下一個字
///
/// 資料來源：
/// - `association.bin` (binary mmap): 字典關聯（冷啟動）
/// - `user_association.db`: 使用者學習（個人化）
///
/// Facade 責任：
/// - 持有 public API、concurrency state、capacity policy、wiring。
/// - Schema (`NextWordSchemaManager`)、CRUD (`NextWordRepository`)、
///   ranking (`NextWordScorer`) 各司其職。
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

    /// NextWord 預測結果
    struct Prediction {
        let hanzi: String
        let tl: String
        let score: Double
    }

    /// 使用者關聯資料。UI (DictionaryTab AssociationDataView) 依賴此公開型別；
    /// 保留為 facade-owned struct 以支援外部 `Identifiable` extension。
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

    static let shared = NextWordService()

    private let associationReader: AssociationBinaryReader?
    private let userConnectionManager: SQLiteConnectionManager
    private let settingsProvider: EngineSettingsProvider
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

    init(
        associationReader: AssociationBinaryReader? = nil,
        settingsProvider: EngineSettingsProvider = SharedSettings.shared,
    ) {
        self.associationReader = associationReader ?? AssociationBinaryReader()
        self.settingsProvider = settingsProvider

        userConnectionManager = SQLiteConnectionManager(
            databasePath: Self.getUserDatabasePath,
            queueLabel: "com.siansiansu.taigikeyboard.nextword.user",
            loggerCategory: "NextWordService.User",
        )
    }

    // MARK: - Public API: Prediction

    /// 預測下一個詞
    ///
    /// Mixed bigram model:
    /// - Dict layer: look up by last character → single-char predictions.
    /// - User layer: look up by full word → full-word predictions.
    func predict(word: String, roman: String = "", limit: Int = Constants.defaultLimit) async -> [Prediction] {
        guard !word.isEmpty else { return [] }
        guard let last = word.last else { return [] }
        let lastChar = String(last)

        logger.debug("[PREDICT][ENTRY] word='\(word)' lastChar='\(lastChar)'")

        var results: [String: Prediction] = [:]
        await queryDictAssociations(lastChar: lastChar, limit: limit, results: &results)
        let dictCount = results.count
        logger.debug("[PREDICT][DICT] dictResults.count=\(dictCount) for lastChar='\(lastChar)'")

        await queryUserAssociations(word: word, roman: roman, limit: limit, results: &results)
        let totalCount = results.count
        logger.debug("[PREDICT][USER] after user merge: totalResults.count=\(totalCount) (user added \(totalCount - dictCount) new entries) for word='\(word)'")

        let sorted = Array(results.values
            .sorted { $0.score > $1.score }
            .prefix(limit))
        logger.debug("[PREDICT] '\(word)' -> \(sorted.count) results")
        return sorted
    }

    // MARK: - Public API: Recording

    /// 記錄使用者選詞關聯
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
            logger.error("[RECORD] Failed: \(error.localizedDescription)")
        }
    }

    /// 清除所有使用者關聯資料
    func clearAllAssociations() async {
        do {
            try await ensureUserTablesCreated()
            try await userConnectionManager.execute { db in
                NextWordRepository.deleteAll(db: db)
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

    /// 取得使用者關聯數量
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

    /// 取得所有使用者關聯（for backup / UI listing）
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

    /// 刪除使用者關聯資料庫
    static func deleteUserDatabase() throws {
        shared.userConnectionManager.close()
        shared.stateLock.withLock {
            shared._tableCreationTask = nil
            shared._tableCreationGeneration &+= 1
        }

        let path = try getUserDatabasePath()
        if FileManager.default.fileExists(atPath: path) {
            try FileManager.default.removeItem(atPath: path)
        }
    }

    // MARK: - Prediction Pipeline

    /// Dict-side prediction — reads from `association.bin` mmap,
    /// applies the user's enabled-dictionary bitmask filter.
    private func queryDictAssociations(
        lastChar: String,
        limit: Int,
        results: inout [String: Prediction],
    ) async {
        guard let reader = associationReader else {
            logger.warning("[DICT] Association binary reader not available")
            return
        }

        // Over-fetch 2x to account for deduplication when merging dict + user results
        let entries = reader.lookup(prevWord: lastChar, limit: limit * 2)
        let enabledDicts = EnabledDictionaries(from: settingsProvider.current)

        for entry in entries {
            guard AssociationBinaryReader.passesFilter(
                entryBitmask: entry.bitmask,
                enabledDicts: enabledDicts,
            ) else { continue }

            let prediction = Prediction(
                hanzi: entry.nextWord,
                tl: entry.nextTl,
                score: NextWordScorer.scoreDict(count: entry.count),
            )
            results["\(prediction.hanzi)\t\(prediction.tl)"] = prediction
        }
    }

    /// User-side prediction — reads from `user_association.db` and merges
    /// (adding scores) with any dict-side predictions already in `results`.
    private func queryUserAssociations(
        word: String,
        roman: String,
        limit: Int,
        results: inout [String: Prediction],
    ) async {
        do {
            try await ensureUserTablesCreated()

            // Over-fetch 2x to account for deduplication when merging dict + user results
            let userRows = try await userConnectionManager.execute { db in
                NextWordRepository.fetchUserRows(db: db, word: word, roman: roman, limit: limit * 2)
            }

            let nowMs = Int64(Date().timeIntervalSince1970 * 1000)
            for row in userRows {
                let userScore = NextWordScorer.calculateUserScore(
                    count: row.count, lastUsedMs: row.lastUsedMs, nowMs: nowMs,
                )
                let key = "\(row.hanzi)\t\(row.tl)"

                if let existing = results[key] {
                    results[key] = Prediction(
                        hanzi: row.hanzi,
                        tl: row.tl.isEmpty ? existing.tl : row.tl,
                        score: existing.score + userScore,
                    )
                } else {
                    results[key] = Prediction(hanzi: row.hanzi, tl: row.tl, score: userScore)
                }
            }
        } catch {
            logger.error("[USER] Query failed: \(error.localizedDescription)")
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
                    try NextWordSchemaManager.ensureTables(db: db, logger: logger)
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
        guard let containerURL = SharedSettings.sharedContainerURL else {
            throw LexiconError.databaseNotFound
        }
        try FileManager.default.createDirectory(
            at: containerURL,
            withIntermediateDirectories: true,
        )
        return containerURL.appendingPathComponent("user_association.db").path
    }
}
