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
/// - Schema (`NextWordSchema`)、CRUD (`NextWordRepository`) 各司其職。
/// - 排序 / 合併 / 截斷 移交 Rust 端 `engine/nextword/` filter 步驟
///   (post-v3.5.5)：`predict()` 回傳未排序的 `[NextWordRawRow]`，呼叫端
///   走 `RustEngineBridge.nextwordFilter` 完成 score + merge + sort + limit。
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

    /// Bundled-bigram lookups go through `RustEngineBridge.lexiconAssocLookup`
    /// (Rust shared-core lexicon engine). The previous `AssociationBinaryReader`
    /// platform mirror was deleted in v3.5.6 commit 13.
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
        settingsProvider: EngineSettingsProvider = SharedSettings.shared,
    ) {
        self.settingsProvider = settingsProvider

        userConnectionManager = SQLiteConnectionManager(
            databasePath: Self.getUserDatabasePath,
            queueLabel: "com.siansiansu.taigikeyboard.nextword.user",
            loggerCategory: "NextWordService.User",
        )
    }

    // MARK: - Public API: Prediction

    /// Predict raw rows for the next-word bigram bridge.
    ///
    /// Post-v3.5.5: returns un-merged un-scored rows tagged by source. The
    /// caller passes them to `RustEngineBridge.nextwordFilter` which scores
    /// (dict via DICT_WEIGHT; user via decay+learning math), merges by
    /// `(hanzi, tl)`, sorts desc by score, applies limit, and shapes per
    /// display rules.
    ///
    /// Mixed bigram model:
    /// - Dict layer: look up by last character → single-char predictions.
    /// - User layer: look up by full word → full-word predictions.
    func predict(
        word: String,
        roman: String = "",
        limit: Int = Constants.defaultLimit,
    ) async -> [RustEngineBridge.NextWordRawRow] {
        guard !word.isEmpty else { return [] }
        guard let last = word.last else { return [] }
        let lastChar = String(last)

        logger.debug("[PREDICT][ENTRY] word='\(word)' lastChar='\(lastChar)'")

        var rows: [RustEngineBridge.NextWordRawRow] = []
        await collectDictAssociations(lastChar: lastChar, limit: limit, rows: &rows)
        logger.debug("[PREDICT][DICT] dictRows.count=\(rows.count) for lastChar='\(lastChar)'")

        let dictCount = rows.count
        await collectUserAssociations(word: word, roman: roman, limit: limit, rows: &rows)
        logger.debug("[PREDICT][USER] userRows added=\(rows.count - dictCount) total=\(rows.count) for word='\(word)'")

        return rows
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

    // MARK: - Prediction Pipeline

    /// Dict-side raw rows — reads from `association.bin` mmap, applies the
    /// user's enabled-dictionary bitmask filter, returns un-scored rows
    /// tagged `.dict`. `lastUsedMs = 0` since dict entries have no
    /// last-used timestamp; the Rust filter ignores it for `.dict` source.
    ///
    /// Over-fetches `limit * 2` so the Rust filter has slack to merge
    /// `(hanzi, tl)` collisions across dict + user without dropping below
    /// the caller's requested limit (Codex post-impl P2-1).
    private func collectDictAssociations(
        lastChar: String,
        limit: Int,
        rows: inout [RustEngineBridge.NextWordRawRow],
    ) async {
        // Bridge call into engine/lexicon — engine applies the 1-layer source
        // filter (low 9 bits of bitmask) internally per audit §4. Over-fetch
        // limit*2 for merge-slack (Codex post-impl P2-1 carry-over from
        // v3.5.5 NextWord slice). Bitmask plumbed end-to-end since
        // r3173013233 (prior to that, api.rs hardcoded u32::MAX).
        let enabled = EnabledDictionaries(from: settingsProvider.current)
        let bitmask: UInt32 = enabled.allAssociationSourcesEnabled
            ? UInt32.max
            : UInt32(enabled.associationBitmask())
        let entries = RustEngineBridge.lexiconAssocLookup(
            previousWord: lastChar,
            limit: UInt32(limit * 2),
            enabledSourcesBitmask: bitmask,
        )

        for entry in entries {
            // Engine's `assoc_lookup` carries both candidate_word (hanzi) +
            // candidate_tl (TL) per bundled bigram; the platform NextWord
            // pipeline needs both to reconstruct the prediction row.
            rows.append(RustEngineBridge.NextWordRawRow(
                hanzi: entry.candidateWord,
                tl: entry.candidateTl,
                count: Int64(entry.count),
                lastUsedMs: 0,
                source: .dict,
            ))
        }
    }

    /// User-side raw rows — reads from `user_association.db`, returns
    /// un-scored rows tagged `.user` so the Rust filter can apply
    /// `calculateUserScore` (decay + learning bonus) at filter time.
    ///
    /// Over-fetches `limit * 2` for the same merge-slack reason as
    /// `collectDictAssociations` (Codex post-impl P2-1).
    private func collectUserAssociations(
        word: String,
        roman: String,
        limit: Int,
        rows: inout [RustEngineBridge.NextWordRawRow],
    ) async {
        do {
            try await ensureUserTablesCreated()

            let userRows = try await userConnectionManager.execute { db in
                NextWordRepository.fetchUserRows(db: db, word: word, roman: roman, limit: limit * 2)
            }

            for row in userRows {
                rows.append(RustEngineBridge.NextWordRawRow(
                    hanzi: row.hanzi,
                    tl: row.tl,
                    count: Int64(row.count),
                    lastUsedMs: row.lastUsedMs,
                    source: .user,
                ))
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
