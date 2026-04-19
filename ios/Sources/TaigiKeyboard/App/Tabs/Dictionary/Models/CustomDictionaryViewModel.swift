import Foundation

/// ViewModel for `CustomDictionaryView`.
///
/// Owns custom dictionary entry loading, CRUD, CSV import/export, and the
/// enable toggle. The view binds to `@Published` state and calls the async
/// actions; form state (input fields, alert flags) stays in the view.
@MainActor
final class CustomDictionaryViewModel: ObservableObject {
    @Published var entries: [CustomDictionaryEntry] = []
    @Published var isLoading = true
    @Published var isCustomDictEnabled: Bool

    private let service: CustomDictionaryService
    private let settings: SharedSettings

    init(
        service: CustomDictionaryService = .shared,
        settings: SharedSettings = .shared,
    ) {
        self.service = service
        self.settings = settings
        isCustomDictEnabled = settings.isCustomDictEnabled
    }

    func setCustomDictEnabled(_ enabled: Bool) {
        isCustomDictEnabled = enabled
        settings.isCustomDictEnabled = enabled
    }

    func load() async {
        do {
            entries = try await service.fetchAll()
        } catch {
            entries = []
        }
        isLoading = false
    }

    func save(_ entry: CustomDictionaryEntry) async {
        try? await service.save(entry)
        await load()
    }

    func delete(id: String) async {
        try? await service.delete(id: id)
        await load()
    }

    func deleteAll() async {
        try? await service.deleteAll()
        await load()
    }

    func exportCSV() async throws -> String {
        try await service.exportCSV()
    }

    func importFile(url: URL) async throws -> (imported: Int, skipped: Int) {
        let result = try await service.importFromFile(url: url)
        return (imported: result.imported, skipped: result.skipped)
    }
}
