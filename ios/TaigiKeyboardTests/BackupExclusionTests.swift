import SQLite3
@testable import TaigiKeyboard
import XCTest

/// v3.6.1 R7 — user-data DBs are excluded from OS / iCloud automatic backup.
///
/// Pins `INVARIANT_USER_DATA_EXCLUDED_FROM_OS_BACKUP`
/// (`docs/architecture/behavioral-invariants.md` §29): every DB opened through
/// `SQLiteConnectionManager` (詞頻 / 詞關聯 / 自訂詞) is marked
/// `isExcludedFromBackup = true` right after open, so learned/authored typing
/// data is kept out of iCloud automatic backup. Cross-device portability is the
/// manual `.taigi` export only (R7 product decision).
///
/// `isExcludedFromBackup` is a system directive, not a hard guarantee; this test
/// pins that the attribute is *marked*, which is the observable contract. The
/// injection is centralized in `SQLiteConnectionManager.connect()`, so testing
/// the manager once covers all three user-data DBs.
final class BackupExclusionTests: XCTestCase {
    private var dbPath: String!

    override func setUpWithError() throws {
        try super.setUpWithError()
        dbPath = NSTemporaryDirectory()
            .appending("backup_exclusion_\(UUID().uuidString).db")
    }

    override func tearDownWithError() throws {
        for suffix in ["", "-wal", "-shm", "-journal"] where dbPath != nil {
            let path = dbPath! + suffix
            if FileManager.default.fileExists(atPath: path) {
                try? FileManager.default.removeItem(atPath: path)
            }
        }
        dbPath = nil
        try super.tearDownWithError()
    }

    private func makeManager() -> SQLiteConnectionManager {
        let path = dbPath!
        return SQLiteConnectionManager(
            databasePath: { path },
            queueLabel: "test.backup.exclusion.\(UUID().uuidString)",
            loggerCategory: "BackupExclusionTests",
        )
    }

    private func isExcludedFromBackup(at path: String) throws -> Bool? {
        let url = URL(fileURLWithPath: path)
        return try url.resourceValues(forKeys: [.isExcludedFromBackupKey]).isExcludedFromBackup
    }

    /// A freshly created DB file is marked excluded from backup immediately after
    /// the first successful open.
    func test_INVARIANT_USER_DATA_EXCLUDED_FROM_OS_BACKUP_freshDatabaseIsMarked() async throws {
        let manager = makeManager()
        try await manager.ensureInitialized(flags: SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE)

        XCTAssertTrue(
            FileManager.default.fileExists(atPath: dbPath!),
            "open with SQLITE_OPEN_CREATE must have created the DB file",
        )
        XCTAssertEqual(
            try isExcludedFromBackup(at: dbPath!), true,
            "a freshly opened user-data DB must be marked isExcludedFromBackup",
        )
    }

    /// An existing-install DB (file already on disk WITHOUT the attribute, e.g. a
    /// pre-R7 upgrade) gets the attribute set on the next open — the exclusion is
    /// applied on every connect, not only at file creation.
    func test_INVARIANT_USER_DATA_EXCLUDED_FROM_OS_BACKUP_existingDatabaseIsBackfilled() async throws {
        // Simulate a pre-R7 install: create the DB file via a bare open, with no
        // backup-exclusion attribute set.
        var bareConnection: OpaquePointer?
        XCTAssertEqual(
            sqlite3_open_v2(dbPath!, &bareConnection, SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE, nil),
            SQLITE_OK,
            "precondition: bare open creates the legacy DB file",
        )
        sqlite3_close(bareConnection)
        XCTAssertNotEqual(
            try isExcludedFromBackup(at: dbPath!), true,
            "precondition: legacy file is not yet excluded from backup",
        )

        let manager = makeManager()
        try await manager.ensureInitialized(flags: SQLITE_OPEN_READWRITE | SQLITE_OPEN_CREATE)

        XCTAssertEqual(
            try isExcludedFromBackup(at: dbPath!), true,
            "opening an existing legacy DB must backfill the exclusion attribute",
        )
    }
}
