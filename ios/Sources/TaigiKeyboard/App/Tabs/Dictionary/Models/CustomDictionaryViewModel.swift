// 中文: CustomDictionaryView 的 ViewModel — 自訂詞庫 CRUD + CSV 匯入匯出 + 啟用開關。
// 中文: 表單欄位狀態(輸入框、alert flag)仍由 View 自行持有。

import Foundation

/// ViewModel for `CustomDictionaryView`.
///
/// Owns custom dictionary entry loading, CRUD, CSV import/export, and the
/// enable toggle. The view binds to `@Published` state and calls the async
/// actions; form state (input fields, alert flags) stays in the view.
// 中文: 自訂詞庫畫面的 ViewModel,封裝 CustomDictionaryService 操作給 View 綁定。
@MainActor
final class CustomDictionaryViewModel: ObservableObject {
    // 中文: 全部自訂詞庫條目。
    @Published var entries: [CustomDictionaryEntry] = []
    // 中文: 載入中旗標,首次 load() 完成後切回 false。
    @Published var isLoading = true
    // 中文: 自訂詞庫啟用開關;切換時同步寫回 SharedSettings。
    @Published var isCustomDictEnabled: Bool

    private let service: CustomDictionaryService
    private let settings: SharedSettings

    init(
        service: CustomDictionaryService = CompositionRoot.customDictionaryService,
        settings: SharedSettings = .shared,
    ) {
        self.service = service
        self.settings = settings
        isCustomDictEnabled = settings.isCustomDictEnabled
    }

    // 中文: 切換自訂詞庫啟用狀態,同步寫回 SharedSettings。
    func setCustomDictEnabled(_ enabled: Bool) {
        isCustomDictEnabled = enabled
        settings.isCustomDictEnabled = enabled
    }

    // 中文: 從 service 載入全部自訂詞庫;失敗時清空 entries。
    func load() async {
        do {
            entries = try await service.fetchAll()
        } catch {
            entries = []
        }
        isLoading = false
    }

    // 中文: 新增或更新自訂詞庫條目,完成後重新載入。
    func save(_ entry: CustomDictionaryEntry) async {
        try? await service.save(entry)
        await load()
    }

    // 中文: 依 id 刪除單筆條目,完成後重新載入。
    func delete(id: String) async {
        try? await service.delete(id: id)
        await load()
    }

    // 中文: 清除全部自訂詞庫條目,完成後重新載入。
    func deleteAll() async {
        try? await service.deleteAll()
        await load()
    }

    // 中文: 匯出全部自訂詞庫為 CSV 字串(委派給 service)。
    func exportCSV() async throws -> String {
        try await service.exportCSV()
    }

    // 中文: 從 URL 匯入 CSV 檔案,回傳匯入/略過筆數。
    func importFile(url: URL) async throws -> (imported: Int, skipped: Int) {
        let result = try await service.importFromFile(url: url)
        return (imported: result.imported, skipped: result.skipped)
    }
}
