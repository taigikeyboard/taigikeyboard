// 中文: 共用的 SQLite 連線管理器 — 提供連線、PRAGMA 設定、async-once 延遲初始化、
// 中文: serialized queue 執行,以及 v3.4.8 的一次性 WAL → DELETE 遷移路徑。

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
            throw LexiconError.databaseConnectionFailed(errorMsg)
        }

        try configure()

        // configure() 的 PRAGMA 會逼 SQLite 真正開檔 → 檔案此時已落地，才標記排除
        // OS 備份（R7 隱私決策）。SQLite 採延遲開檔，open 後檔案尚未必存在。
        excludeFromOSBackup(path: path)
    }

    /// 一次性 WAL → DELETE 遷移
    /// 偵測舊 `.db-wal` 檔案，執行 checkpoint 後由 configure() 切換至 DELETE mode
    /// 跳過唯讀資料庫（如 dictionary.db），因為它們不需要遷移
    private func migrateFromWAL(path: String, flags: Int32) {
        // 唯讀資料庫不需要 WAL 遷移
        guard flags & SQLITE_OPEN_READWRITE != 0 else { return }

        let walPath = path + "-wal"
        guard FileManager.default.fileExists(atPath: walPath) else {
            logger.debug("[MIGRATE] No WAL file found, skipping migration")
            return
        }

        logger.debug("[MIGRATE] Found WAL file, performing checkpoint: \(walPath)")

        var db: OpaquePointer?
        guard sqlite3_open_v2(path, &db, flags, nil) == SQLITE_OK else {
            logger.warning("[MIGRATE] Failed to open database for WAL checkpoint: \(path)")
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

    /// 將資料庫檔案標記為排除 OS / iCloud 自動備份。
    ///
    /// 三個使用者資料庫（詞頻 / 詞關聯 / 自訂詞）都是裝置端學習或自建的打字資料，
    /// 不應隨 iCloud 自動上雲；跨裝置可攜僅靠手動 `.taigi` 匯出（R7 產品決策）。
    /// `isExcludedFromBackup` 是系統指示而非硬保證，且這是 best-effort 的檔案
    /// metadata 寫入 —— 失敗絕不可中斷 DB open（打字路徑），故吞錯只記 log。
    ///
    /// 須在 configure() 之後呼叫：SQLite 延遲開檔，`sqlite3_open_v2` 不會立即建檔，
    /// 要等首個語句；configure() 的 PRAGMA 以 O_CREAT 把主 `.db` 落地，此時才有檔可
    /// 標記。如此全新安裝「首次啟動」即標記成功，而非延到第二次。
    ///
    /// DELETE journal mode → 持久檔只有主 `.db`，無 `-wal`/`-shm` sidecar，標記主檔
    /// 即足夠。若日後改用 WAL，sidecar 需比照排除（或改目錄層級排除）。
    private func excludeFromOSBackup(path: String) {
        var url = URL(fileURLWithPath: path)
        var resourceValues = URLResourceValues()
        resourceValues.isExcludedFromBackup = true
        do {
            try url.setResourceValues(resourceValues)
        } catch {
            logger.warning("[BACKUP] Failed to set isExcludedFromBackup for \(path): \(error.localizedDescription)")
        }
    }

    /// 配置資料庫 PRAGMA 設定
    private func configure() throws {
        guard let db = connection else {
            throw LexiconError.databaseNotAvailable
        }

        // DELETE journal mode 為刻意選擇（App Group 跨進程穩定 + 無 WAL sidecar）。
        // 與 excludeFromOSBackup() 搭配：只標記主 `.db` 即可（無 -wal/-shm）。
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
                    continuation.resume(throwing: LexiconError.databaseNotAvailable)
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
    // 中文: SQLITE_TRANSIENT 等價物 — 告訴 SQLite 立即複製綁定值,呼叫端 buffer 不需保留。
    static let sqliteTransient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)

    // MARK: - Query Execution

    /// 在佇列中執行資料庫操作
    func execute<T>(_ operation: @escaping (OpaquePointer) throws -> T) async throws -> T {
        try await ensureInitialized()

        return try await withCheckedThrowingContinuation { continuation in
            queue.async { [weak self] in
                guard let self, let db = connection else {
                    continuation.resume(throwing: LexiconError.databaseNotAvailable)
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
            throw LexiconError.databaseNotAvailable
        }

        return try queue.sync { [weak self] in
            guard let self, let db = connection else {
                throw LexiconError.databaseNotAvailable
            }
            return try operation(db)
        }
    }
}
