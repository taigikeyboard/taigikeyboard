import Foundation

/// ViewModel for `FrequencyDataView`.
///
/// Owns frequency data loading, CSV import/export, deletion, and the
/// recording toggle. The view binds to `@Published` state and calls the
/// async actions.
@MainActor
final class FrequencyDataViewModel: ObservableObject {
    @Published var allData: [(word: String, count: Int)] = []
    @Published var isLoading = true
    @Published var isFrequencyRecordingEnabled: Bool

    private let repository: UserFrequencyRepository
    private let settings: SharedSettings
    private let logger = DebugLogger(category: "FrequencyDataViewModel")

    init(
        repository: UserFrequencyRepository = .shared,
        settings: SharedSettings = .shared,
    ) {
        self.repository = repository
        self.settings = settings
        isFrequencyRecordingEnabled = settings.isFrequencyRecordingEnabled
    }

    func setRecordingEnabled(_ enabled: Bool) {
        isFrequencyRecordingEnabled = enabled
        settings.isFrequencyRecordingEnabled = enabled
    }

    func load() async {
        let freq = await repository.topWordsAsync(limit: Int.max)
        allData = freq
        isLoading = false
    }

    func deleteWord(_ word: String) async {
        try? await repository.deleteWord(word)
        allData.removeAll { $0.word == word }
    }

    func clearAll() {
        do {
            try UserFrequencyService.deleteUserDatabase()
            allData = []
        } catch {
            logger.error("Failed to delete frequency database: \(error)")
        }
    }

    func exportCSV() async throws -> String {
        let data = await repository.topWordsAsync(limit: Int.max)
        return CSVDocument.encodeFrequencyCSV(data)
    }

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
        let imported = try await repository.batchImportMerge(entries: entries)
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
