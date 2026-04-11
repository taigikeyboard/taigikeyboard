import Foundation
import SQLite3

/// SQLite 連接管理器
/// 提供資料庫連接、配置、延遲初始化等共用功能
final class SQLiteConnectionManager: @unchecked Sendable {
    // MARK: - Properties

    private var connection: OpaquePointer?
    private let databasePath: () throws -> String
    private let queue: DispatchQueue
    private let logger: DebugLogger

    // 延遲初始化相關屬性
    private var isInitialized = false
    private var isInitializing = false
    private var initializationTask: Task<Void, Error>?
    private let initLock = NSLock()

    // MARK: - Initialization

    init(
        databasePath: @escaping () throws -> String,
        queueLabel: String,
        loggerCategory: String,
    ) {
        self.databasePath = databasePath
        queue = DispatchQueue(label: queueLabel, qos: .userInitiated)
        logger = DebugLogger(category: loggerCategory)
    }

    deinit {
        close()
    }

    // MARK: - Connection Management

    /// 連接資料庫
    private func connect(flags: Int32 = SQLITE_OPEN_READWRITE) throws {
        let path = try databasePath()

        // 一次性遷移：清理舊 WAL 檔（v3.4.8 升級用戶）
        migrateFromWAL(path: path, flags: flags)

        guard sqlite3_open_v2(path, &connection, flags, nil) == SQLITE_OK else {
            let errorMsg = connection != nil
                ? String(cString: sqlite3_errmsg(connection))
                : "Unknown error"
            logger.error("[INIT] Failed to open: \(errorMsg)")
            sqlite3_close(connection)
            connection = nil
            throw DictionaryError.databaseConnectionFailed(errorMsg)
        }

        try configure()
    }

    /// 一次性 WAL → DELETE 遷移
    /// 偵測舊 `.db-wal` 檔案，執行 checkpoint 後由 configure() 切換至 DELETE mode
    /// 跳過唯讀資料庫（如 dictionary.db），因為它們不需要遷移
    private func migrateFromWAL(path: String, flags: Int32) {
        // 唯讀資料庫不需要 WAL 遷移
        guard flags & SQLITE_OPEN_READWRITE != 0 else { return }

        let walPath = path + "-wal"
        guard FileManager.default.fileExists(atPath: walPath) else { return }

        logger.debug("[MIGRATE] Found WAL file, performing checkpoint: \(walPath)")

        var db: OpaquePointer?
        guard sqlite3_open_v2(path, &db, flags, nil) == SQLITE_OK else {
            sqlite3_close(db)
            return
        }

        // Checkpoint：將 WAL 內容寫回主資料庫並截斷 WAL 檔
        let rc = sqlite3_wal_checkpoint_v2(db, nil, SQLITE_CHECKPOINT_TRUNCATE, nil, nil)
        sqlite3_close(db)

        if rc == SQLITE_OK {
            logger.debug("[MIGRATE] WAL checkpoint completed")
        } else {
            logger.warning("[MIGRATE] WAL checkpoint returned code \(rc), WAL may not be fully cleared")
        }
    }

    /// 配置資料庫 PRAGMA 設定
    private func configure() throws {
        guard let db = connection else {
            throw DictionaryError.databaseNotAvailable
        }

        let configurations = [
            "PRAGMA journal_mode=DELETE;",
            "PRAGMA synchronous=NORMAL;",
            "PRAGMA cache_size=10000;",
            "PRAGMA temp_store=MEMORY;",
        ]

        for config in configurations {
            var stmt: OpaquePointer?
            if sqlite3_prepare_v2(db, config, -1, &stmt, nil) == SQLITE_OK {
                sqlite3_step(stmt)
                sqlite3_finalize(stmt)
            } else {
                let errorMsg = String(cString: sqlite3_errmsg(db))
                logger.warning("[CONFIG] Could not set pragma \(config): \(errorMsg)")
            }
        }
    }

    /// 關閉資料庫連接
    func close() {
        queue.sync {
            if let db = connection {
                sqlite3_close(db)
                connection = nil
                isInitialized = false
            }
        }
    }

    // MARK: - Lazy Initialization

    /// 確保資料庫已初始化（延遲初始化的核心方法）
    func ensureInitialized(flags: Int32 = SQLITE_OPEN_READWRITE) async throws {
        // 快速檢查：如果已初始化，直接返回
        if isInitialized {
            return
        }

        // 使用併發安全的方式避免重複初始化
        return try await withCheckedThrowingContinuation { continuation in
            initLock.lock()
            defer { initLock.unlock() }

            // 再次檢查（雙重檢查模式）
            if isInitialized {
                continuation.resume()
                return
            }

            // 如果正在初始化，等待完成
            if let existingTask = initializationTask {
                Task {
                    do {
                        try await existingTask.value
                        continuation.resume()
                    } catch {
                        continuation.resume(throwing: error)
                    }
                }
                return
            }

            isInitializing = true
            let task = Task {
                do {
                    try await performInitialization(flags: flags)
                    await MainActor.run {
                        self.isInitialized = true
                        self.isInitializing = false
                        self.initializationTask = nil
                    }
                } catch {
                    await MainActor.run {
                        self.isInitializing = false
                        self.initializationTask = nil
                    }
                    logger.error("[LAZY-INIT] Database initialization failed: \(error.localizedDescription)")
                    throw error
                }
            }

            initializationTask = task

            // 等待初始化完成
            Task {
                do {
                    try await task.value
                    continuation.resume()
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// 執行實際的資料庫初始化（在背景佇列中）
    private func performInitialization(flags: Int32) async throws {
        try await withCheckedThrowingContinuation { (continuation: CheckedContinuation<Void, Error>) in
            queue.async { [weak self] in
                guard let self else {
                    continuation.resume(throwing: DictionaryError.databaseNotAvailable)
                    return
                }

                do {
                    try connect(flags: flags)
                    continuation.resume()
                } catch {
                    connection = nil
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    // MARK: - Connection Status

    /// 檢查是否已連接
    func isConnected() -> Bool {
        guard isInitialized else { return false }
        return queue.sync { connection != nil }
    }

    // MARK: - Constants

    /// SQLITE_TRANSIENT equivalent — tells SQLite to copy the bound value immediately
    static let sqliteTransient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)

    // MARK: - Query Execution

    /// 在佇列中執行資料庫操作
    func execute<T>(_ operation: @escaping (OpaquePointer) throws -> T) async throws -> T {
        try await ensureInitialized()

        return try await withCheckedThrowingContinuation { continuation in
            queue.async { [weak self] in
                guard let self, let db = connection else {
                    continuation.resume(throwing: DictionaryError.databaseNotAvailable)
                    return
                }

                do {
                    let result = try operation(db)
                    continuation.resume(returning: result)
                } catch {
                    continuation.resume(throwing: error)
                }
            }
        }
    }

    /// 同步執行資料庫操作（用於已確保初始化的情況）
    func executeSync<T>(_ operation: @escaping (OpaquePointer) throws -> T) throws -> T {
        guard isInitialized else {
            throw DictionaryError.databaseNotAvailable
        }

        return try queue.sync { [weak self] in
            guard let self, let db = connection else {
                throw DictionaryError.databaseNotAvailable
            }
            return try operation(db)
        }
    }
}
