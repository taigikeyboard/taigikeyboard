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

    private let userData: any UserDataClient
    private let settings: SharedSettings

    init(
        userData: any UserDataClient = CompositionRoot.userData,
        settings: SharedSettings = .shared,
    ) {
        self.userData = userData
        self.settings = settings
        isCustomDictEnabled = settings.isCustomDictEnabled
    }

    func setCustomDictEnabled(_ enabled: Bool) {
        isCustomDictEnabled = enabled
        settings.isCustomDictEnabled = enabled
    }

    func load() async {
        do {
            entries = try await userData.listAll()
        } catch {
            entries = []
        }
        isLoading = false
    }

    func save(_ entry: CustomDictionaryEntry) async {
        try? await userData.save(entry)
        await load()
    }

    func delete(id: String) async {
        try? await userData.delete(id: id)
        await load()
    }

    func deleteAll() async {
        try? await userData.deleteAll()
        await load()
    }

    func exportCSV() async throws -> String {
        try await String(decoding: userData.exportCSV(), as: UTF8.self)
    }

    func importFile(url: URL) async throws -> (imported: Int, skipped: Int) {
        try await userData.importCSV(url: url)
    }
}
