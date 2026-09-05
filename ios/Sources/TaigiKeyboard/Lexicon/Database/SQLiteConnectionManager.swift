// Shared SQLite connection owner: open + PRAGMA config, async-once lazy init, serialized-queue
// execution, plus the one-time v3.4.8 WAL → DELETE migration.

import Foundation
import SQLite3

final class SQLiteConnectionManager: @unchecked Sendable {
    // MARK: - Properties

    private var connection: OpaquePointer?
    private let databasePath: () throws -> String
    private let queue: DispatchQueue
    private let logger: DebugLogger

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

    private func connect(flags: Int32 = SQLITE_OPEN_READWRITE) throws {
        let path = try databasePath()

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

        // configure()'s PRAGMA forces the file into existence (SQLite opens lazily), so only
        // now is there a file to mark excluded from OS backup (R7 privacy decision).
        excludeFromOSBackup(path: path)
    }

    /// One-time WAL → DELETE migration: checkpoints a leftover `.db-wal` so configure() can switch
    /// journal mode. Read-only databases such as dictionary.db are skipped.
    private func migrateFromWAL(path: String, flags: Int32) {
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

        let rc = sqlite3_wal_checkpoint_v2(db, nil, SQLITE_CHECKPOINT_TRUNCATE, nil, nil)
        sqlite3_close(db)

        if rc == SQLITE_OK {
            logger.debug("[MIGRATE] WAL checkpoint completed")
        } else {
            logger.warning("[MIGRATE] WAL checkpoint returned code \(rc), WAL may not be fully cleared")
        }
    }

    /// Marks the database file as excluded from OS / iCloud backup. The three user databases
    /// (frequency / association / custom dictionary) hold device-local typing data; cross-device
    /// portability is manual `.taigi` export only (R7 product decision). Best-effort — a failed
    /// metadata write must never break DB open, so it is logged and swallowed.
    ///
    /// Must run AFTER configure(): SQLite opens lazily, so the file exists only once configure()'s
    /// PRAGMA creates it. DELETE journal mode leaves the main `.db` as the only persistent file, so
    /// marking it is enough; switching to WAL would mean excluding the `-wal`/`-shm` sidecars too.
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

    private func configure() throws {
        guard let db = connection else {
            throw LexiconError.databaseNotAvailable
        }

        // DELETE journal mode is deliberate: stable across App Group processes and no WAL
        // sidecar, so excludeFromOSBackup() only has to mark the main `.db`.
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

    func ensureInitialized(flags: Int32 = SQLITE_OPEN_READWRITE) async throws {
        if isInitialized {
            return
        }

        return try await withCheckedThrowingContinuation { continuation in
            initLock.lock()
            defer { initLock.unlock() }

            // Double-check under the lock.
            if isInitialized {
                continuation.resume()
                return
            }

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

    /// Runs the actual connect on the serialized queue.
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

    func isConnected() -> Bool {
        guard isInitialized else { return false }
        return queue.sync { connection != nil }
    }

    // MARK: - Constants

    /// SQLITE_TRANSIENT equivalent — tells SQLite to copy the bound value immediately
    static let sqliteTransient = unsafeBitCast(-1, to: sqlite3_destructor_type.self)

    // MARK: - Query Execution

    /// Runs a database operation on the serialized queue, initializing first if needed.
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

    /// Synchronous variant for callers that already ensured initialization.
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
