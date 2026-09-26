@testable import TaigiKeyboard
import XCTest

/// v3.6.1 R7 — user-data DBs are excluded from OS / iCloud automatic backup.
///
/// Pins `INVARIANT_USER_DATA_EXCLUDED_FROM_OS_BACKUP`
/// (`docs/architecture/behavioral-invariants.md` §29): every file the engine
/// keeps for the user — the four stores and the `<file>.pre-engine` copies its
/// takeover leaves (user-data-engine-roadmap P7b) — is marked
/// `isExcludedFromBackup = true`, so learned/authored typing data is kept out
/// of iCloud automatic backup. Cross-device portability is the manual `.taigi`
/// export only (R7 product decision).
///
/// `isExcludedFromBackup` is a system directive, not a hard guarantee; this
/// test pins that the attribute is *marked*, which is the observable contract.
/// Files on disk stand in for the engine's: this process never opens the
/// user data (the engine opens once per process).
final class BackupExclusionTests: XCTestCase {
    private var directory: URL!

    override func setUpWithError() throws {
        try super.setUpWithError()
        directory = FileManager.default.temporaryDirectory
            .appendingPathComponent("backup_exclusion_\(UUID().uuidString)")
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: directory)
        directory = nil
        try super.tearDownWithError()
    }

    private func isExcludedFromBackup(_ name: String) throws -> Bool? {
        try directory.appendingPathComponent(name)
            .resourceValues(forKeys: [.isExcludedFromBackupKey]).isExcludedFromBackup
    }

    /// Every store file and every pre-takeover copy on disk is marked; a legacy
    /// file without the attribute (a pre-R7 install) is backfilled.
    func test_INVARIANT_USER_DATA_EXCLUDED_FROM_OS_BACKUP_everyStoreAndCopyIsMarked() throws {
        let names = RustEngineBridge.userDataFileNames.flatMap { [$0, "\($0).pre-engine"] }
        XCTAssertEqual(names.count, 8, "four stores, each with its takeover copy")
        for name in names {
            FileManager.default.createFile(atPath: directory.appendingPathComponent(name).path, contents: Data())
            XCTAssertNotEqual(try isExcludedFromBackup(name), true, "precondition: \(name) not yet excluded")
        }

        UserDataOpening.excludeFromBackup(in: directory)

        for name in names {
            XCTAssertEqual(try isExcludedFromBackup(name), true, "\(name) must be excluded from backup")
        }
    }

    /// A file the engine has not written yet is skipped, not created.
    func test_aMissingFileIsNotCreated() {
        UserDataOpening.excludeFromBackup(in: directory)

        XCTAssertEqual(try FileManager.default.contentsOfDirectory(atPath: directory.path), [])
    }
}
