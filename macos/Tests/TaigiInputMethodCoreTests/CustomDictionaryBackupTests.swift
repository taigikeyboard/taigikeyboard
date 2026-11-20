// The `.taigi` backup pair on the Custom Dictionary page (#1032), as desktop-core's
// `export_backup_job` / `import_backup_job` behave on Windows and Linux.

import AppKit
@testable import TaigiInputMethodCore
import XCTest

@MainActor
final class CustomDictionaryBackupTests: XCTestCase {
    private var client = FakeUserDataClient()
    private var panels = FakeFilePanels()
    private let window = NSWindow()

    private func makeModel(rows: Int = 1) async -> CustomDictionaryPageModel {
        client = FakeUserDataClient()
        panels = FakeFilePanels()
        let model = CustomDictionaryPageModel(client: client, panels: panels)
        for index in 0 ..< rows {
            await model.save(CustomDictionaryRow(roman: "row\(index)", hanji: "字\(index)"))
        }
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
        XCTAssertTrue(model.beginWork(.desktopProgressWorking))

        await model.exportBackup(in: window)

        XCTAssertNil(panels.written)
        XCTAssertEqual(model.activity, .working(.desktopProgressWorking))
    }

    // MARK: - Import

    /// The open panel offers the backup type and plain JSON (an Android
    /// export can be named `.taigi.json`); the counts come back as the
    /// receipt, and the list shows the restored words.
    func testImport_reportsEachStoresCountAndReloadsTheList() async throws {
        let model = await makeModel()
        client.backupRowsToImport = [CustomDictionaryRow(roman: "tsit", hanji: "一")]
        panels.fileToOpen = try TestFixtures.scratchDirectory().appendingPathComponent("taigi_backup.taigi.json")

        await model.importBackup(in: window)

        XCTAssertEqual(panels.openedContentTypes, [UserDataFilePanels.backupContentType, .json])
        XCTAssertEqual(client.lastBackupImportURL, panels.fileToOpen)
        XCTAssertEqual(
            model.message,
            .backupImported(BackupImportResult(customDictionary: 1, frequency: 1, association: 2)),
        )
        XCTAssertEqual(model.list.rows.map(\.roman), ["tsit", "row0"])
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
        XCTAssertEqual(model.list.rows.map(\.roman), ["row0"])
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
        XCTAssertTrue(model.beginWork(.desktopProgressWorking))

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
