@testable import TaigiKeyboard
import XCTest

/// Every `UserDataClient` request fails the test: a fake subclasses this and
/// overrides only the requests the code under test should make. The test
/// process never opens the engine's user data (it shares the app's engine).
class UserDataClientStub: UserDataClient, @unchecked Sendable {
    func listCustomEntries(filter _: String, limit _: UInt32, offset _: UInt32) async throws -> CustomDictionaryPage {
        XCTFail("unused")
        return CustomDictionaryPage(entries: [], total: 0, matchingTotal: 0, offset: 0)
    }

    func save(_: CustomDictionaryEntry) async throws {
        XCTFail("unused")
    }

    func delete(id _: String) async throws {
        XCTFail("unused")
    }

    func deleteAll() async throws {
        XCTFail("unused")
    }

    func exportCSV() async throws -> Data {
        XCTFail("unused")
        return Data()
    }

    func importCSV(url _: URL) async throws -> (imported: Int, skipped: Int) {
        XCTFail("unused")
        return (0, 0)
    }

    func clearLearningRecords() async throws {
        XCTFail("unused")
    }

    func search(query _: String, mode _: InputMode, limit _: Int) async -> [CustomDictionaryEntry] {
        XCTFail("unused")
        return []
    }

    func exportBackup(appVersion _: String) async throws -> Data {
        XCTFail("unused")
        return Data()
    }

    func importBackup(url _: URL) async throws -> BackupImportResult {
        XCTFail("unused")
        throw CancellationError()
    }

    func listLearningRecords(
        kind _: Taigi_Engine_LearningRecordKind,
        order _: Taigi_Engine_LearningRecordOrder,
        filter _: String,
        limit _: UInt32,
        offset _: UInt32,
    ) async throws -> Taigi_Engine_LearningRecords {
        XCTFail("unused")
        return Taigi_Engine_LearningRecords()
    }

    func setLearningRecordCount(_: Taigi_Engine_LearningRecord, count _: Int64) async throws -> Bool {
        XCTFail("unused")
        return false
    }

    func deleteLearningRecord(_: Taigi_Engine_LearningRecord) async throws -> Bool {
        XCTFail("unused")
        return false
    }

    func addLearningRecordToCustomDictionary(_: Taigi_Engine_LearningRecord) async throws {
        XCTFail("unused")
    }
}
