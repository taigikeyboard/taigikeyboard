import Foundation
import OSLog
import SQLite3

/// 使用者詞頻資料庫 Repository
/// 負責使用者詞頻資料的存取與管理
final class UserFrequencyRepository: @unchecked Sendable {

    // MARK: - Properties

    static let shared = UserFrequencyRepository()

    private let connectionManager: SQLiteConnectionManager
    private let logger = Logger(
        subsystem: LexiconConstants.Logging.subsystem,
        category: "UserFrequencyRepository"
    )

    private var isTablesCreated = false
    private var tableCreationTask: Task<Void, Error>?

    // MARK: - Test Data

    #if DEBUG
    static let testData = [
        ("word1", 1), ("word2", 1), ("góa", 1), ("我", 1), ("accumulate", 3),
        ("popular", 5), ("frequent", 4), ("common", 3), ("rare", 1),
    ]
    #endif

    // MARK: - Initialization

    init(connectionManager: SQLiteConnectionManager? = nil) {
        self.connectionManager = connectionManager ?? SQLiteConnectionManager(
            databasePath: Self.getDatabasePath,
            queueLabel: "com.siansiansu.taigikeyboard.userfrequency",
            loggerCategory: "UserFrequencyRepository"
        )
    }

    // MARK: - Database Path

    private static func getDatabasePath() throws -> String {
        guard let appSupportURL = FileManager.default.urls(
            for: .applicationSupportDirectory,
            in: .userDomainMask
        ).first else {
            throw DictionaryError.databaseNotFound
        }

        try FileManager.default.createDirectory(
            at: appSupportURL,
            withIntermediateDirectories: true
        )

        let databaseURL = appSupportURL.appendingPathComponent("user_frequency.db")
        return databaseURL.path
    }

    // MARK: - Initialization

    /// 確保資料庫已初始化並建立表格
    func ensureInitialized() async throws {
        // 使用 CREATE flag 初始化連接
        try await connectionManager.ensureInitialized(
            flags: SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE
        )

        // 確保表格已建立
        try await createTablesIfNeeded()
    }

    // MARK: - Table Management

    private func createTablesIfNeeded() async throws {
        // 快速檢查：如果已建立，直接返回
        if isTablesCreated {
            return
        }

        // 如果正在創建，等待完成
        if let existingTask = tableCreationTask {
            try await existingTask.value
            return
        }

        // 創建新任務
        let task = Task {
            try await connectionManager.execute { db in
                try self.createTables(db: db)
                try self.createIndexes(db: db)
                try self.createMetadataTable(db: db)
                try self.insertMetadata(db: db)
            }

            await MainActor.run {
                self.isTablesCreated = true
                self.tableCreationTask = nil
            }
        }

        tableCreationTask = task
        try await task.value
    }

    private func createTables(db: OpaquePointer) throws {
        let createTable = """
            CREATE TABLE IF NOT EXISTS user_frequency (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                word TEXT NOT NULL UNIQUE,
                count INTEGER DEFAULT 1,
                last_used TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, createTable, -1, &stmt, nil) == SQLITE_OK else {
            let errorMsg = String(cString: sqlite3_errmsg(db))
            throw DictionaryError.queryPreparationFailed("Create user_frequency table failed: \(errorMsg)")
        }

        guard sqlite3_step(stmt) == SQLITE_DONE else {
            let errorMsg = String(cString: sqlite3_errmsg(db))
            sqlite3_finalize(stmt)
            throw DictionaryError.queryExecutionFailed("Create user_frequency table failed: \(errorMsg)")
        }
        sqlite3_finalize(stmt)
    }

    private func createIndexes(db: OpaquePointer) throws {
        let indexes = [
            "CREATE INDEX IF NOT EXISTS idx_word ON user_frequency(word);",
            "CREATE INDEX IF NOT EXISTS idx_count ON user_frequency(count DESC);",
            "CREATE INDEX IF NOT EXISTS idx_last_used ON user_frequency(last_used DESC);",
        ]

        for indexSQL in indexes {
            var stmt: OpaquePointer?
            if sqlite3_prepare_v2(db, indexSQL, -1, &stmt, nil) == SQLITE_OK {
                sqlite3_step(stmt)
                sqlite3_finalize(stmt)
            }
        }
    }

    private func createMetadataTable(db: OpaquePointer) throws {
        let metadataTable = """
            CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT
            );
        """

        var stmt: OpaquePointer?
        if sqlite3_prepare_v2(db, metadataTable, -1, &stmt, nil) == SQLITE_OK {
            sqlite3_step(stmt)
            sqlite3_finalize(stmt)
        }
    }

    private func insertMetadata(db: OpaquePointer) throws {
        let sql = """
            INSERT OR IGNORE INTO metadata (key, value) VALUES
            ('app_version', ?),
            ('schema_version', '1.0'),
            ('created_date', datetime('now')),
            ('last_modified', datetime('now'));
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            return // Ignore metadata insertion errors
        }

        let version = getVersion()
        let TRANSIENT = unsafeBitCast(-1, to: sqlite3_destructor_type.self)
        sqlite3_bind_text(stmt, 1, version, -1, TRANSIENT)
        sqlite3_step(stmt)
        sqlite3_finalize(stmt)
    }

    private func getVersion() -> String {
        if let version = Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString") as? String {
            return version
        }
        return "1.0.0"
    }

    // MARK: - CRUD Operations

    /// 記錄詞彙使用
    func recordWord(_ word: String) async {
        do {
            try await ensureInitialized()
            try await connectionManager.execute { db in
                try self.insertOrUpdateWord(db: db, word: word)
            }
        } catch {
            logger.error("[RECORD] Failed to record usage for: \(word)")
        }
    }

    private func insertOrUpdateWord(db: OpaquePointer, word: String) throws {
        let sql = """
            INSERT INTO user_frequency (word, count, last_used)
            VALUES (?, 1, CURRENT_TIMESTAMP)
            ON CONFLICT(word) DO UPDATE SET
                count = count + 1,
                last_used = CURRENT_TIMESTAMP;
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            logger.error("[RECORD] Failed to prepare statement")
            return
        }

        defer { sqlite3_finalize(stmt) }

        let TRANSIENT = unsafeBitCast(-1, to: sqlite3_destructor_type.self)
        sqlite3_bind_text(stmt, 1, word, -1, TRANSIENT)

        if sqlite3_step(stmt) != SQLITE_DONE {
            logger.error("[RECORD] Failed to record usage for: \(word)")
        }
    }

    /// 取得詞彙使用次數
    func getCount(for word: String) -> Int {
        guard connectionManager.isConnected() else { return 0 }

        do {
            return try connectionManager.executeSync { db in
                try self.queryCount(db: db, word: word)
            }
        } catch {
            return 0
        }
    }

    private func queryCount(db: OpaquePointer, word: String) throws -> Int {
        let sql = "SELECT count FROM user_frequency WHERE word = ?;"
        var stmt: OpaquePointer?

        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            return 0
        }

        defer { sqlite3_finalize(stmt) }

        let TRANSIENT = unsafeBitCast(-1, to: sqlite3_destructor_type.self)
        sqlite3_bind_text(stmt, 1, word, -1, TRANSIENT)

        if sqlite3_step(stmt) == SQLITE_ROW {
            return Int(sqlite3_column_int(stmt, 0))
        }

        return 0
    }

    /// 取得使用次數最多的詞彙
    func getTopWords(limit: Int = 100) -> [(word: String, count: Int)] {
        guard connectionManager.isConnected() else { return [] }

        do {
            return try connectionManager.executeSync { db in
                try self.queryTopWords(db: db, limit: limit)
            }
        } catch {
            return []
        }
    }

    private func queryTopWords(db: OpaquePointer, limit: Int) throws -> [(word: String, count: Int)] {
        let sql = """
            SELECT word, count FROM user_frequency
            ORDER BY count DESC, last_used DESC
            LIMIT ?;
        """

        var stmt: OpaquePointer?
        guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
            return []
        }

        defer { sqlite3_finalize(stmt) }

        sqlite3_bind_int(stmt, 1, Int32(limit))

        var results: [(word: String, count: Int)] = []

        while sqlite3_step(stmt) == SQLITE_ROW {
            let word = sqlite3_column_text(stmt, 0).map(String.init(cString:)) ?? ""
            let count = Int(sqlite3_column_int(stmt, 1))
            results.append((word: word, count: count))
        }

        return results
    }

    // MARK: - Database Management

    /// 刪除使用者頻率資料庫
    func deleteDatabase() throws {
        // 先關閉資料庫連接
        connectionManager.close()

        // 重置表格建立狀態
        isTablesCreated = false

        // 獲取資料庫路徑並刪除
        let path = try Self.getDatabasePath()
        if FileManager.default.fileExists(atPath: path) {
            try FileManager.default.removeItem(atPath: path)
        }
    }

    // MARK: - Connection Status

    func isConnected() -> Bool {
        connectionManager.isConnected()
    }

    // MARK: - Debug Methods

    #if DEBUG
    func insertTestData() async throws {
        try await ensureInitialized()

        try await connectionManager.execute { db in
            if sqlite3_exec(db, "BEGIN TRANSACTION;", nil, nil, nil) != SQLITE_OK {
                self.logger.error("[TEST] Failed to begin transaction")
                throw DictionaryError.queryExecutionFailed("Failed to begin transaction")
            }

            let sql = """
                INSERT OR IGNORE INTO user_frequency (word, count, last_used, created_at)
                VALUES (?, ?, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP);
            """

            for (word, count) in UserFrequencyRepository.testData {
                var stmt: OpaquePointer?
                guard sqlite3_prepare_v2(db, sql, -1, &stmt, nil) == SQLITE_OK else {
                    self.logger.error("[TEST] Failed to prepare insert statement")
                    sqlite3_exec(db, "ROLLBACK;", nil, nil, nil)
                    throw DictionaryError.queryPreparationFailed("Failed to prepare test data insert")
                }

                defer { sqlite3_finalize(stmt) }

                let TRANSIENT = unsafeBitCast(-1, to: sqlite3_destructor_type.self)
                sqlite3_bind_text(stmt, 1, word, -1, TRANSIENT)
                sqlite3_bind_int(stmt, 2, Int32(count))

                if sqlite3_step(stmt) != SQLITE_DONE {
                    let errorMsg = String(cString: sqlite3_errmsg(db))
                    self.logger.error("[TEST] Failed to insert test data for '\(word)': \(errorMsg)")
                    sqlite3_exec(db, "ROLLBACK;", nil, nil, nil)
                    throw DictionaryError.queryExecutionFailed("Failed to insert test data")
                }
            }

            if sqlite3_exec(db, "COMMIT;", nil, nil, nil) != SQLITE_OK {
                self.logger.error("[TEST] Failed to commit transaction")
                sqlite3_exec(db, "ROLLBACK;", nil, nil, nil)
                throw DictionaryError.queryExecutionFailed("Failed to commit transaction")
            }
        }
    }
    #endif
}
