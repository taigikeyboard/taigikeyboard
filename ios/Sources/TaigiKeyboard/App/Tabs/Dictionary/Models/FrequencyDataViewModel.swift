// Talks straight to UserFrequencyRepository (SQLite-backed); there is no service layer.

import Foundation

/// One displayed frequency row: a `(word, tl)` reading + its count. R5
/// (#7): identity is the pair, so 一字多音 (重/tāng vs 重/tîng) are distinct
/// rows. `id` is the composite key — `word` alone is not unique.
struct FrequencyListItem: Identifiable {
    let word: String
    let tl: String
    let count: Int
    var id: String { "\(word)\t\(tl)" }
}

/// ViewModel for `FrequencyDataView`.
///
/// Owns frequency data loading, CSV import/export, deletion, and the
/// recording toggle. The view binds to `@Published` state and calls the
/// async actions.
@MainActor
final class FrequencyDataViewModel: ObservableObject {
    @Published var allData: [FrequencyListItem] = []
    @Published var isLoading = true
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

    func setRecordingEnabled(_ enabled: Bool) {
        isFrequencyRecordingEnabled = enabled
        settings.isFrequencyRecordingEnabled = enabled
    }

    func load() async {
        let rows = await repository.allFrequencyRowsAsync()
        allData = rows.map { FrequencyListItem(word: $0.word, tl: $0.tl, count: $0.count) }
        isLoading = false
    }

    func deleteWord(_ word: String, tl: String) async {
        try? await repository.deleteWord(word, tl: tl)
        allData.removeAll { $0.word == word && $0.tl == tl }
    }

    // Clears everything by dropping the underlying SQLite file, not by deleting rows.
    func clearAll() {
        do {
            try repository.deleteDatabase()
            allData = []
        } catch {
            logger.error("Failed to delete frequency database: \(error)")
        }
    }

    func exportCSV() async throws -> String {
        let data = await repository.allFrequencyRowsAsync()
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
        // 3-column rows carry the reading; legacy 2-column rows decode to
        // `tl == ""` (the tolerant fallback bucket, #7). Upsert is
        // `ON CONFLICT(word, tl)`, so each `(漢字, 羅馬字)` reading merges
        // into its own bucket.
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
