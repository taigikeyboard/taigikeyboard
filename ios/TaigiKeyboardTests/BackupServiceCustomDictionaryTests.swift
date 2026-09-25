@testable import TaigiKeyboard
import XCTest

/// The custom-dictionary section of the `.taigi` backup: an export is the
/// v2 shape (roman + hanzi, nothing else), a v2 file imports every row, and
/// the dev-only v3 rows tagged `origin = 1` (learned phrases, 2026-09-20)
/// are skipped — learned phrases never travel in a backup (USER 2026-09-21).
///
/// Only the custom-dictionary section is exercised — the frequency /
/// association arrays are empty, so those repositories are never opened.
final class BackupServiceCustomDictionaryTests: XCTestCase {
    private var dbPath: String!
    private var repository: CustomDictionaryRepository!
    private var backup: BackupService!

    override func setUpWithError() throws {
        try super.setUpWithError()
        dbPath = NSTemporaryDirectory()
            .appending("backup_custom_\(UUID().uuidString).db")
        let path = dbPath!
        let manager = SQLiteConnectionManager(
            databasePath: { path },
            queueLabel: "test.backup.custom.\(UUID().uuidString)",
            loggerCategory: "BackupServiceCustomDictionaryTests",
        )
        repository = CustomDictionaryRepository(connectionManager: manager)
        backup = BackupService(
            customDictionaryService: CustomDictionaryService(repository: repository),
            userFrequencyRepository: UserFrequencyRepository(),
            nextWordService: NextWordService(),
        )
    }

    override func tearDownWithError() throws {
        try? repository.deleteDatabase()
        repository = nil
        backup = nil
        for suffix in ["", "-wal", "-shm"] where dbPath != nil {
            let path = dbPath! + suffix
            if FileManager.default.fileExists(atPath: path) {
                try? FileManager.default.removeItem(atPath: path)
            }
        }
        dbPath = nil
        try super.tearDownWithError()
    }

    private func file(version: Int, rows: [[String: Any]]) throws -> Data {
        let json: [String: Any] = [
            "version": version,
            "exportedAt": "2026-09-21T00:00:00Z",
            "platform": "ios",
            "appVersion": "test",
            "customDictionary": rows,
            "userFrequency": [],
            "userAssociation": [],
        ]
        return try JSONSerialization.data(withJSONObject: json)
    }

    func test_v2FileImportsEveryRow() async throws {
        let data = try file(version: 2, rows: [["roman": "tâi-gí", "hanzi": "台語"]])
        let result = try await backup.importAll(from: data)
        XCTAssertEqual(result.customDict, 1)
        let all = try await repository.fetchAll()
        XCTAssertEqual(all.map(\.hanzi), ["台語"])
    }

    func test_pairRepeatedInsideTheFileLandsOnce() async throws {
        let data = try file(version: 2, rows: [
            ["roman": "tâi-gí", "hanzi": "台語"],
            ["roman": "tâi-gí", "hanzi": "台語"],
        ])
        let result = try await backup.importAll(from: data)
        XCTAssertEqual(result.customDict, 1)
        let all = try await repository.fetchAll()
        XCTAssertEqual(all.map(\.hanzi), ["台語"], "got \(all)")
    }

    func test_devV3FileSkipsLearnedRowsAndKeepsManualOnes() async throws {
        let data = try file(version: 3, rows: [
            ["roman": "kì--khí-lâi", "hanzi": "記起來", "origin": 1, "learnCount": 3],
            ["roman": "tâi-gí", "hanzi": "台語", "origin": 0, "learnCount": 0],
            ["roman": "gâu-tsá", "hanzi": "𠢕早"],
        ])
        let result = try await backup.importAll(from: data)
        XCTAssertEqual(result.customDict, 2, "the learned row is not counted")
        let all = try await repository.fetchAll()
        XCTAssertEqual(Set(all.map(\.hanzi)), ["台語", "𠢕早"], "a learned row never becomes a visible manual row")
    }

    func test_exportRowIsTheV2ShapeWithoutProvenance() throws {
        let data = try JSONEncoder().encode(BackupService.CustomDictEntry(roman: "tâi-gí", hanzi: "台語"))
        let row = try XCTUnwrap(JSONSerialization.jsonObject(with: data) as? [String: Any])
        XCTAssertEqual(Set(row.keys), ["roman", "hanzi"], "no origin / learnCount keys; got \(row)")
    }
}
