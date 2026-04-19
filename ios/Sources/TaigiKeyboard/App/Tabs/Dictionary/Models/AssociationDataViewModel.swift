import Foundation

/// ViewModel for `AssociationDataView`.
///
/// Owns association data loading, CSV import/export, deletion, and the
/// recording toggle. The view binds to `@Published` state and calls the
/// async actions.
@MainActor
final class AssociationDataViewModel: ObservableObject {
    @Published var allData: [NextWordService.AssociationEntry] = []
    @Published var isLoading = true
    @Published var isAssociationRecordingEnabled: Bool

    private let service: NextWordService
    private let settings: SharedSettings

    init(
        service: NextWordService = .shared,
        settings: SharedSettings = .shared,
    ) {
        self.service = service
        self.settings = settings
        isAssociationRecordingEnabled = settings.isAssociationRecordingEnabled
    }

    func setRecordingEnabled(_ enabled: Bool) {
        isAssociationRecordingEnabled = enabled
        settings.isAssociationRecordingEnabled = enabled
    }

    func load() async {
        let assoc = await service.allAssociations()
        allData = assoc
        isLoading = false
    }

    func delete(_ entry: NextWordService.AssociationEntry) async {
        await service.deleteAssociation(entry)
        allData.removeAll { $0.id == entry.id }
    }

    func clearAll() async {
        await service.clearAllAssociations()
        allData = []
    }

    func exportCSV() async throws -> String {
        let data = await service.allAssociations()
        return CSVDocument.encodeAssociationCSV(data)
    }

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
