// What the shipped user-data client and rules do over DesktopCoreBridge before and around the engine.

import Foundation
import os
@testable import TaigiInputMethodCore
import XCTest

/// The real client over `DesktopCoreBridge` with the engine's user data never
/// opened (the handle is process-wide — see `UserDataEndToEndTests` for
/// isolated-process SQLite tests), plus mock-transport contract tests for
/// `EngineUserDataClient` and `UserDataRules`.
final class UserDataClientTests: XCTestCase {
    /// A huge file is refused before it is read into memory — the engine
    /// never sees it — in desktop-core's words for the same refusal.
    func testAnImportOverTheSizeLimit_isRefusedBeforeItIsRead() throws {
        let file = try TestFixtures.scratchDirectory().appendingPathComponent("huge.csv")
        try Data(count: UserDataRules.maxImportFileBytes + 1).write(to: file)

        XCTAssertThrowsError(try EngineUserDataClient().importCSV(at: file)) {
            XCTAssertEqual($0 as? UserDataClientError, .refused(refusal: .fileTooLarge, detail: "file is larger than 5 MB"))
        }
    }

    /// A request the engine refuses — here, any page request before the
    /// open — is a failure the page can report, not an empty dictionary.
    func testARequestTheEngineRefuses_isAFailure() {
        XCTAssertThrowsError(try EngineUserDataClient().list(filter: "", limit: 10, offset: 0, inputMode: .tl)) {
            XCTAssertEqual($0 as? UserDataClientError, .engineUnavailable(op: "customDictionaryList"))
        }
    }

    /// The alert's diagnostic line: the engine's words as they came, and one
    /// line per store a reset could not empty.
    func testTheDiagnostic_isWhatTheEngineSaid() {
        XCTAssertEqual(
            UserDataClientError.refused(detail: "file holds more than 30000 entries").description,
            "file holds more than 30000 entries",
        )
        XCTAssertEqual(
            UserDataClientError.notEmptied(["user_frequency: disk I/O error", "learned_phrases: locked"])
                .description,
            "user_frequency: disk I/O error\nlearned_phrases: locked",
        )
    }

    /// A backup that cannot be read is refused before any engine round-trip,
    /// in desktop-core's words (`could not read the file: …`).
    func testAnUnreadableBackup_isRefusedBeforeTheEngine() throws {
        let missing = try TestFixtures.scratchDirectory().appendingPathComponent("absent.taigi")

        XCTAssertThrowsError(try EngineUserDataClient().importBackup(at: missing)) {
            guard case let .refused(refusal: .none, detail: detail) = $0 as? UserDataClientError else {
                return XCTFail("expected a refusal, got \($0)")
            }
            XCTAssertTrue(detail.hasPrefix("could not read the file: "), detail)
        }
    }

    /// A readable backup gets past the read: before the open the engine
    /// refuses the request, a failure rather than a refusal of the file.
    func testAReadableBackup_getsPastTheRead() throws {
        let file = try TestFixtures.scratchDirectory().appendingPathComponent("backup.taigi")
        try Data("{}".utf8).write(to: file)

        XCTAssertThrowsError(try EngineUserDataClient().importBackup(at: file)) {
            XCTAssertEqual($0 as? UserDataClientError, .engineUnavailable(op: "backupImport"))
        }
    }

    /// A CSV whose size cannot be read is refused in the same words.
    func testAMissingCSV_isRefusedBeforeTheEngine() throws {
        let missing = try TestFixtures.scratchDirectory().appendingPathComponent("absent.csv")

        XCTAssertThrowsError(try EngineUserDataClient().importCSV(at: missing)) {
            guard case let .refused(refusal: .none, detail: detail) = $0 as? UserDataClientError else {
                return XCTFail("expected a refusal, got \($0)")
            }
            XCTAssertTrue(detail.hasPrefix("could not read the file: "), detail)
        }
    }

    /// `UserDataRules` surfaces desktop-core's non-key-path constants and
    /// enum metadata without requiring `Configure`.
    @MainActor
    func testUserDataRules_matchDesktopCoreValues() {
        XCTAssertEqual(UserDataRules.pageSize, 10)
        XCTAssertEqual(UserDataRules.maxImportFileBytes, 5 * 1024 * 1024)
        XCTAssertEqual(UserDataRules.learningRecordMaxCount, 1_000_000)
        XCTAssertEqual(UserDataRules.backupOpenExtensions, ["taigi", "json"])
        XCTAssertEqual(UserDataRules.learningRecordKindSpellings.map(\.value), [.frequency, .learnedPhrase])
        XCTAssertEqual(UserDataRules.learningRecordKindSpellings.map(\.spelling), ["frequency", "learnedPhrase"])
        XCTAssertEqual(UserDataRules.learningRecordOrderSpellings.map(\.value), [.mostUsed, .mostRecent])
        XCTAssertEqual(UserDataRules.learningRecordOrderSpellings.map(\.spelling), ["mostUsed", "mostRecent"])
        XCTAssertFalse(UserDataRules.offersAddToCustomDictionary(for: .frequency))
        XCTAssertTrue(UserDataRules.offersAddToCustomDictionary(for: .learnedPhrase))
        XCTAssertEqual(UserDataRules.countNoteKey(for: .frequency), .dictionaryLearningRecordsCountCapInfo)
        XCTAssertNil(UserDataRules.countNoteKey(for: .learnedPhrase))

        XCTAssertEqual(
            UserDataRules.exportFileName(kind: .customCsv, localDate: "2026-11-27"),
            "taigi_custom_dictionary_2026-11-27.csv",
        )
        XCTAssertEqual(
            UserDataRules.exportFileName(kind: .backup, localDate: "2026-11-27"),
            "taigi_backup_2026-11-27.taigi",
        )
        XCTAssertNotNil(
            UserDataFilePanels.exportFileName(kind: .customCsv)
                .wholeMatch(of: /taigi_custom_dictionary_\d{4}-\d{2}-\d{2}\.csv/),
        )
        XCTAssertNotNil(
            UserDataFilePanels.exportFileName(kind: .backup)
                .wholeMatch(of: /taigi_backup_\d{4}-\d{2}-\d{2}\.taigi/),
        )
    }

    func testClient_mapsUserDataErrorVariantsFromDesktopCore() {
        let csvURL = URL(fileURLWithPath: "/tmp/test.csv")

        let unavailableClient = EngineUserDataClient { _ in
            Self.userDataErrorResponse { $0.engineUnavailableOp = "customDictionaryList" }
        }
        XCTAssertThrowsError(try unavailableClient.list(filter: "", limit: 10, offset: 0, inputMode: .tl)) {
            XCTAssertEqual($0 as? UserDataClientError, .engineUnavailable(op: "customDictionaryList"))
        }

        let notUtf8Client = EngineUserDataClient { _ in
            Self.userDataErrorResponse {
                var refused = Taigi_DesktopShell_UserDataRefusal()
                refused.refusal = .notUtf8
                refused.detail = "file is not UTF-8"
                $0.refused = refused
            }
        }
        XCTAssertThrowsError(try notUtf8Client.importCSV(at: csvURL)) {
            XCTAssertEqual($0 as? UserDataClientError, .refused(refusal: .notUtf8, detail: "file is not UTF-8"))
        }

        let noRowsClient = EngineUserDataClient { _ in
            Self.userDataErrorResponse {
                var refused = Taigi_DesktopShell_UserDataRefusal()
                refused.refusal = .noUsableRows
                refused.detail = "no usable rows"
                $0.refused = refused
            }
        }
        XCTAssertThrowsError(try noRowsClient.importCSV(at: csvURL)) {
            XCTAssertEqual($0 as? UserDataClientError, .refused(refusal: .noUsableRows, detail: "no usable rows"))
        }

        let fullClient = EngineUserDataClient { _ in
            Self.userDataErrorResponse {
                var refused = Taigi_DesktopShell_UserDataRefusal()
                refused.refusal = .full
                refused.detail = "custom dictionary is full (max 30000 entries)"
                $0.refused = refused
            }
        }
        XCTAssertThrowsError(try fullClient.save(CustomDictionaryRow(roman: "tsit", hanji: "一"))) {
            XCTAssertEqual(
                $0 as? UserDataClientError,
                .refused(refusal: .full, detail: "custom dictionary is full (max 30000 entries)"),
            )
        }

        let notEmptiedClient = EngineUserDataClient { _ in
            Self.userDataErrorResponse {
                var notEmptied = Taigi_DesktopShell_UserDataNotEmptied()
                notEmptied.failures = ["user_frequency: locked"]
                $0.notEmptied = notEmptied
            }
        }
        XCTAssertThrowsError(try notEmptiedClient.clearLearningRecords()) {
            XCTAssertEqual($0 as? UserDataClientError, .notEmptied(["user_frequency: locked"]))
        }
    }

    func testClient_handlesSeamFailuresAndMalformedReplies() {
        let csvURL = URL(fileURLWithPath: "/tmp/test.csv")

        let failInvariant = EngineUserDataClient { _ in
            var response = Taigi_DesktopShell_DesktopResponse()
            response.error = .failInvariant
            return (try? response.serializedBytes()) ?? []
        }
        XCTAssertThrowsError(try failInvariant.exportCSV()) {
            XCTAssertEqual($0 as? UserDataClientError, .engineUnavailable(op: "customDictionaryExportCSV"))
        }

        let garbage = EngineUserDataClient { _ in [0xFF, 0xFF, 0xFF] }
        XCTAssertThrowsError(try garbage.exportBackup()) {
            XCTAssertEqual($0 as? UserDataClientError, .engineUnavailable(op: "backupExport"))
        }

        let wrongReply = EngineUserDataClient { _ in
            var response = Taigi_DesktopShell_DesktopResponse()
            response.error = .ok
            response.version = .init()
            return (try? response.serializedBytes()) ?? []
        }
        XCTAssertThrowsError(try wrongReply.delete(id: "1")) {
            XCTAssertEqual($0 as? UserDataClientError, .engineUnavailable(op: "customDictionaryDelete"))
        }

        let emptyUserDataReply = EngineUserDataClient { _ in
            var response = Taigi_DesktopShell_DesktopResponse()
            response.error = .ok
            response.userData = .init()
            return (try? response.serializedBytes()) ?? []
        }
        XCTAssertThrowsError(try emptyUserDataReply.importCSV(at: csvURL)) {
            XCTAssertEqual($0 as? UserDataClientError, .engineUnavailable(op: "customDictionaryImportCSV"))
        }
    }

    func testClient_decodesImportAndBackupCountsAndPercentDecodesFilePaths() throws {
        let sentCSVPath = OSAllocatedUnfairLock<String?>(initialState: nil)
        let csvClient = EngineUserDataClient { bytes in
            if let req = try? Taigi_DesktopShell_DesktopRequest(serializedBytes: Data(bytes)),
               case let .userData(userData) = req.request,
               case let .importCustomCsvFile(csvReq) = userData.method
            {
                sentCSVPath.withLock { $0 = csvReq.path }
            }
            var reply = Taigi_DesktopShell_UserDataReply()
            var imported = Taigi_DesktopShell_CustomCsvImported()
            imported.imported = 4
            imported.skipped = 1
            reply.customCsvImported = imported
            return Self.userDataSuccessResponse(reply)
        }
        let spacedURL = URL(fileURLWithPath: "/tmp/folder with spaces/words #1.csv")
        XCTAssertEqual(
            try csvClient.importCSV(at: spacedURL),
            CustomDictionaryImportResult(imported: 4, skipped: 1),
        )
        XCTAssertEqual(sentCSVPath.withLock { $0 }, "/tmp/folder with spaces/words #1.csv")

        let backupClient = EngineUserDataClient { _ in
            var reply = Taigi_DesktopShell_UserDataReply()
            var imported = Taigi_DesktopShell_BackupImported()
            imported.customDictionary = 2
            imported.frequency = 3
            imported.association = 5
            imported.learnedPhrases = 1
            reply.backupImported = imported
            return Self.userDataSuccessResponse(reply)
        }
        XCTAssertEqual(
            try backupClient.importBackup(at: URL(fileURLWithPath: "/tmp/b.taigi")),
            BackupImportResult(customDictionary: 2, frequency: 3, association: 5),
        )
    }

    private static func userDataSuccessResponse(_ reply: Taigi_DesktopShell_UserDataReply) -> [UInt8] {
        var response = Taigi_DesktopShell_DesktopResponse()
        response.error = .ok
        response.userData = reply
        return (try? response.serializedBytes()) ?? []
    }

    private static func userDataErrorResponse(
        _ configure: (inout Taigi_DesktopShell_UserDataError) -> Void,
    ) -> [UInt8] {
        var error = Taigi_DesktopShell_UserDataError()
        configure(&error)
        var reply = Taigi_DesktopShell_UserDataReply()
        reply.error = error
        return userDataSuccessResponse(reply)
    }
}
