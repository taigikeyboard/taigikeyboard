// 中文: FrequencyDataView 的 ViewModel — 詞頻資料 CRUD + CSV 匯入匯出 + 錄製開關。
// 中文: 直接走 UserFrequencyRepository(SQLite-backed),沒走 Service 層。

import Foundation

/// ViewModel for `FrequencyDataView`.
///
/// Owns frequency data loading, CSV import/export, deletion, and the
/// recording toggle. The view binds to `@Published` state and calls the
/// async actions.
// 中文: 詞頻管理畫面的 ViewModel,封裝 UserFrequencyRepository 操作。
@MainActor
final class FrequencyDataViewModel: ObservableObject {
    // 中文: 全部詞頻資料 (word, count) 排序後清單。
    @Published var allData: [(word: String, count: Int)] = []
    // 中文: 載入中旗標,首次 load() 完成後切回 false。
    @Published var isLoading = true
    // 中文: 詞頻錄製開關;切換時同步寫回 SharedSettings。
    @Published var isFrequencyRecordingEnabled: Bool

    private let repository: UserFrequencyRepository
    private let settings: SharedSettings
    private let logger = DebugLogger(category: "FrequencyDataViewModel")

    init(
        repository: UserFrequencyRepository = CompositionRoot.userFrequencyRepository,
        settings: SharedSettings = .shared,
    ) {
        self.repository = repository
        self.settings = settings
        isFrequencyRecordingEnabled = settings.isFrequencyRecordingEnabled
    }

    // 中文: 切換詞頻錄製開關,同步更新本地與 SharedSettings。
    func setRecordingEnabled(_ enabled: Bool) {
        isFrequencyRecordingEnabled = enabled
        settings.isFrequencyRecordingEnabled = enabled
    }

    // 中文: 從 repository 載入全部詞頻資料(無上限)。
    func load() async {
        let freq = await repository.topWordsAsync(limit: Int.max)
        allData = freq
        isLoading = false
    }

    // 中文: 刪除單一詞的詞頻紀錄並從 in-memory 清單移除。
    func deleteWord(_ word: String) async {
        try? await repository.deleteWord(word)
        allData.removeAll { $0.word == word }
    }

    // 中文: 清除全部詞頻資料 — 直接刪除底層 SQLite 檔。
    func clearAll() {
        do {
            try repository.deleteDatabase()
            allData = []
        } catch {
            logger.error("Failed to delete frequency database: \(error)")
        }
    }

    // 中文: 匯出全部詞頻資料為 CSV 字串(交給 CSVDocument 編碼)。
    func exportCSV() async throws -> String {
        let data = await repository.topWordsAsync(limit: Int.max)
        return CSVDocument.encodeFrequencyCSV(data)
    }

    // 中文: 從 URL 讀取 CSV,解碼後 batchImportMerge;回傳匯入/略過筆數。
    func importCSV(url: URL) async throws -> (imported: Int, skipped: Int) {
        let data = try await Self.readFileData(from: url)
        guard let csvString = String(data: data, encoding: .utf8) else {
            throw NSError(
                domain: "FrequencyDataViewModel",
                code: -1,
                userInfo: [NSLocalizedDescriptionKey: "Cannot read file"],
            )
        }
        let entries = CSVDocument.decodeFrequencyCSV(csvString)
        try await repository.ensureInitialized()
        // R5: the user-facing frequency CSV stays `(word, count)` — a hand
        // editable format with no reading column. Imported rows land in the
        // legacy `tl == ""` fallback bucket (#7 tolerant). Full per-reading
        // fidelity lives in the `.taigi` backup, not the CSV.
        let imported = try await repository.batchImportMerge(
            entries: entries.map { (word: $0.word, tl: "", count: $0.count) },
        )
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
