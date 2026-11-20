// What the shipped user-data client does before and around the engine.

@testable import TaigiInputMethodCore
import XCTest

/// The real client over the real bridge, with the engine's user data never
/// opened (the handle is process-wide — `UserDataTestDoubles.swift`). What
/// the engine does once it is open, and the words each refusal carries, are
/// the engine's tests'; these cover the platform side.
final class UserDataClientTests: XCTestCase {
    /// A huge file is refused before it is read into memory — the engine
    /// never sees it — in the words the engine uses for the same refusal.
    func testAnImportOverTheSizeLimit_isRefusedBeforeItIsRead() throws {
        let file = try TestFixtures.scratchDirectory().appendingPathComponent("huge.csv")
        try Data(count: EngineUserDataClient.maxImportFileBytes + 1).write(to: file)

        XCTAssertThrowsError(try EngineUserDataClient().importCSV(at: file)) {
            XCTAssertEqual($0 as? UserDataClientError, .refused(detail: "file is larger than 5 MB"))
        }
    }

    /// A request the engine refuses — here, any page request before the
    /// open — is a failure the page can report, not an empty dictionary.
    func testARequestTheEngineRefuses_isAFailure() {
        XCTAssertThrowsError(try EngineUserDataClient().list(filter: "", limit: 10, offset: 0)) {
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
            guard case let .refused(detail) = $0 as? UserDataClientError else {
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
            guard case let .refused(detail) = $0 as? UserDataClientError else {
                return XCTFail("expected a refusal, got \($0)")
            }
            XCTAssertTrue(detail.hasPrefix("could not read the file: "), detail)
        }
    }

    /// trace: desktop-core `backup_refused` (`engine/user_data.rs`) —
    /// UnsupportedVersion → "unsupported backup version"; None / Unreadable →
    /// "not a .taigi backup".
    func testARefusedBackup_readsAsDesktopCoreSaysIt() {
        XCTAssertEqual(EngineUserDataClient.refusalDetail(.unsupportedVersion), "unsupported backup version")
        XCTAssertEqual(EngineUserDataClient.refusalDetail(.unreadable), "not a .taigi backup")
    }
}
