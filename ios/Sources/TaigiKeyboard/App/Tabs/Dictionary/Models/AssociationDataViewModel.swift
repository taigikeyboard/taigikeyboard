// 中文: AssociationDataView 的 ViewModel — 聯想詞 CRUD + CSV 匯入匯出 + 錄製開關。
// 中文: 資料來源走 NextWordService(SQLite-backed),設定持久化交給 SharedSettings。

import Foundation

/// ViewModel for `AssociationDataView`.
///
/// Owns association data loading, CSV import/export, deletion, and the
/// recording toggle. The view binds to `@Published` state and calls the
/// async actions.
// 中文: 聯想詞管理畫面的 ViewModel,持有資料載入、CSV 匯入匯出、刪除與錄製開關。
@MainActor
final class AssociationDataViewModel: ObservableObject {
    // 中文: 全部聯想資料,View 透過 @Published 綁定即時更新。
    @Published var allData: [NextWordService.AssociationEntry] = []
    // 中文: 載入中旗標,首次 load() 完成後切回 false。
    @Published var isLoading = true
    // 中文: 聯想詞錄製開關;切換時同步寫回 SharedSettings。
    @Published var isAssociationRecordingEnabled: Bool

    private let service: NextWordService
    private let settings: SharedSettings

    init(
        service: NextWordService = CompositionRoot.nextWordService,
        settings: SharedSettings = .shared,
    ) {
        self.service = service
        self.settings = settings
        isAssociationRecordingEnabled = settings.isAssociationRecordingEnabled
    }

    // 中文: 切換聯想詞錄製開關,同步更新本地與 SharedSettings。
    func setRecordingEnabled(_ enabled: Bool) {
        isAssociationRecordingEnabled = enabled
        settings.isAssociationRecordingEnabled = enabled
    }

    // 中文: 從 NextWordService 載入全部聯想資料並關閉 loading 狀態。
    func load() async {
        let assoc = await service.allAssociations()
        allData = assoc
        isLoading = false
    }

    // 中文: 刪除單筆聯想詞並從 in-memory 清單移除。
    func delete(_ entry: NextWordService.AssociationEntry) async {
        await service.deleteAssociation(entry)
        allData.removeAll { $0.id == entry.id }
    }

    // 中文: 清除全部聯想詞(含 SQLite 與 in-memory)。
    func clearAll() async {
        await service.clearAllAssociations()
        allData = []
    }

    // 中文: 匯出聯想資料為 CSV 字串(交給 CSVDocument 編碼)。
    func exportCSV() async throws -> String {
        let data = await service.allAssociations()
        return CSVDocument.encodeAssociationCSV(data)
    }

    // 中文: 從 URL 讀取 CSV 檔案,解碼後批次匯入;回傳匯入/略過筆數。
    func importCSV(url: URL) async throws -> (imported: Int, skipped: Int) {
        let fileData = try await Self.readFileData(from: url)
        guard let csvString = String(data: fileData, encoding: .utf8) else {
            throw NSError(
                domain: "AssociationDataViewModel",
                code: -1,
                userInfo: [NSLocalizedDescriptionKey: "Cannot read file"],
            )
        }
        let entries = CSVDocument.decodeAssociationCSV(csvString)
        let imported = try await service.batchImportAssociations(entries: entries)
        return (imported: imported, skipped: entries.count - imported)
    }

    /// Read file off the main actor so a large import doesn't block UI.
    private static func readFileData(from url: URL) async throws -> Data {
        try await Task.detached(priority: .userInitiated) {
            let accessing = url.startAccessingSecurityScopedResource()
            defer { if accessing { url.stopAccessingSecurityScopedResource() } }
            return try Data(contentsOf: url)
        }.value
    }
}
