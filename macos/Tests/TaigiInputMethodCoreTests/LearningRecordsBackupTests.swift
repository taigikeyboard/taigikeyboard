// The `.taigi` backup pair on the Learning Records page (#1032), as desktop-core's
// `export_backup_job` / `import_backup_job` behave on Windows and Linux.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

@MainActor
final class LearningRecordsBackupTests: XCTestCase {
    private var client = FakeUserDataClient()
    private var panels = FakeFilePanels()
    private let window = NSWindow()
    /// Each case's own defaults domain, as `LearningRecordsPageTests` keeps.
    private var suiteName = ""
    private var settings: SettingsStore!

    override func setUp() async throws {
        try await super.setUp()
        suiteName = "LearningRecordsBackupTests.\(UUID().uuidString)"
        settings = try SettingsStore(userDefaults: XCTUnwrap(UserDefaults(suiteName: suiteName)))
    }

    override func tearDown() async throws {
        settings.userDefaults.removePersistentDomain(forName: suiteName)
        try await super.tearDown()
    }

    /// A frequency record, as the page opens on word frequency.
    private func record(id: Int64) -> Taigi_DesktopShell_LearningRecord {
        var record = Taigi_DesktopShell_LearningRecord()
        record.kind = .frequency
        record.id = id
        record.text = "字\(id)"
        record.tl = "ji\(id)"
        record.count = id
        return record
    }

    private func makeModel() async -> LearningRecordsPageModel {
        client = FakeUserDataClient()
        client.seedLearningRecords([record(id: 1)])
        panels = FakeFilePanels()
        let model = LearningRecordsPageModel(client: client, settings: settings, panels: panels)
        await model.load()
        return model
    }

    // MARK: - Export

    /// Silent on success, as the CSV export; the file is named as the phones
    /// and the other desktops name it.
    func testExport_writesTheEnginesBytesUnderTheDatedNameAndSaysNothing() async throws {
        let model = await makeModel()

        await model.exportBackup(in: window)

        let written = try XCTUnwrap(panels.written)
        XCTAssertEqual(written.data, client.backupToExport)
        // desktop-core `backup_file_name`: taigi_backup_<yyyy-MM-dd>.taigi.
        XCTAssertNotNil(
            written.suggestedName.wholeMatch(of: /taigi_backup_\d{4}-\d{2}-\d{2}\.taigi/),
            written.suggestedName,
        )
        XCTAssertEqual(written.contentTypes, [UserDataFilePanels.backupContentType])
        XCTAssertNil(model.message)
        XCTAssertEqual(model.activity, .idle)
    }

    func testExport_thatTheEngineCannotAnswer_isExportFailed() async {
        let model = await makeModel()
        client.backupError = .engineUnavailable(op: "backupExport")

        await model.exportBackup(in: window)

        XCTAssertNil(panels.written, "no panel for bytes that do not exist")
        XCTAssertEqual(
            model.message,
            .failure(.commonExportFailed, diagnostic: "the engine did not answer backupExport"),
        )
        XCTAssertEqual(model.activity, .idle)
    }

    func testExport_whoseWriteFails_isExportFailed() async {
        let model = await makeModel()
        panels.writeError = CocoaError(.fileWriteOutOfSpace)

        await model.exportBackup(in: window)

        guard case .failure(.commonExportFailed, _) = model.message else {
            return XCTFail("expected the export failure, got \(String(describing: model.message))")
        }
    }

    func testExport_doesNothingWhileAnotherHoldsTheSlot() async {
        let model = await makeModel()
        XCTAssertTrue(model.beginWork())

        await model.exportBackup(in: window)

        XCTAssertNil(panels.written)
        XCTAssertEqual(model.activity, .working(.desktopProgressWorking))
    }

    // MARK: - Import

    /// The open panel offers the backup type and plain JSON (an Android
    /// export can be named `.taigi.json`); the counts come back as the
    /// receipt, and the list shows the restored records.
    func testImport_reportsEachStoresCountAndReloadsTheList() async throws {
        let model = await makeModel()
        client.backupRecordsToImport = [record(id: 2)]
        panels.fileToOpen = try TestFixtures.scratchDirectory().appendingPathComponent("taigi_backup.taigi.json")

        await model.importBackup(in: window)

        XCTAssertEqual(panels.openedContentTypes, [UserDataFilePanels.backupContentType, .json])
        XCTAssertEqual(client.lastBackupImportURL, panels.fileToOpen)
        XCTAssertEqual(
            model.message,
            .backupImported(BackupImportResult(customDictionary: 0, frequency: 1, association: 2)),
        )
        XCTAssertEqual(model.list.rows.map(\.tl), ["ji2", "ji1"])
        XCTAssertEqual(model.activity, .idle)
    }

    /// A refused file is Import Failed in the engine's words, and the list
    /// still reloads, as `import_backup_job` asks.
    func testImport_thatTheEngineRefuses_isImportFailedInItsWords() async throws {
        let model = await makeModel()
        client.backupError = .refused(detail: "unsupported backup version")

        try await model.importBackup(at: TestFixtures.scratchDirectory().appendingPathComponent("old.taigi"))

        XCTAssertEqual(
            model.message,
            .failure(.commonImportFailed, diagnostic: "unsupported backup version"),
        )
        XCTAssertEqual(model.list.rows.map(\.tl), ["ji1"])
    }

    /// A cancelled panel imports nothing, says nothing, and frees the slot.
    func testImport_cancelled_doesNothing() async {
        let model = await makeModel()

        await model.importBackup(in: window)

        XCTAssertNotNil(panels.openedContentTypes)
        XCTAssertNil(client.lastBackupImportURL)
        XCTAssertNil(model.message)
        XCTAssertEqual(model.activity, .idle)
    }

    func testImport_doesNothingWhileAnotherHoldsTheSlot() async throws {
        let model = await makeModel()
        panels.fileToOpen = try TestFixtures.scratchDirectory().appendingPathComponent("taigi_backup.taigi")
        XCTAssertTrue(model.beginWork())

        await model.importBackup(in: window)

        XCTAssertNil(panels.openedContentTypes, "the slot is taken before the panel opens")
        XCTAssertNil(client.lastBackupImportURL)
        XCTAssertEqual(model.activity, .working(.desktopProgressWorking))
    }

    // MARK: - The receipt

    func testTheReceipt_isImportCompleteWithEachCountInItsOwnPlural() {
        let english = StringResolver(.english)
        // trace: dictionary.importBackupResult en — CLDR `one` iff n == 1.
        let singular = UserDataPageMessage.backupImported(
            BackupImportResult(customDictionary: 1, frequency: 1, association: 1),
        )
        XCTAssertEqual(singular.title(english), english.resolve(.desktopImportComplete))
        XCTAssertEqual(
            singular.detail(english),
            "Imported 1 custom entry, 1 frequency record, 1 association record",
        )
        let plural = UserDataPageMessage.backupImported(
            BackupImportResult(customDictionary: 0, frequency: 2, association: 17677),
        )
        XCTAssertEqual(
            plural.detail(english),
            "Imported 0 custom entries, 2 frequency records, 17677 association records",
        )
        // trace: hanji has no plural arms — one template, the counts in order.
        XCTAssertEqual(
            plural.detail(StringResolver(.hanji)),
            "匯入 0 項自訂詞、2 項詞頻、17677 項詞關聯",
        )
    }
}
