import Foundation

/// ViewModel for `DataManagementView`.
///
/// Owns backup export/import orchestration. The view keeps the SwiftUI file
/// exporter/importer bindings (`fileExporter`/`fileImporter`) and drives them
/// with the `BackupExportPayload` returned from `exportBackup()`.
@MainActor
final class DataManagementViewModel: ObservableObject {
    /// Payload the view feeds into `.fileExporter`.
    struct BackupExportPayload {
        let data: Data
        let filename: String
    }

    @Published var isProcessing = false

    private let service: BackupService

    init(service: BackupService = CompositionRoot.backupService) {
        self.service = service
    }

    func exportBackup() async throws -> BackupExportPayload {
        isProcessing = true
        defer { isProcessing = false }
        let data = try await service.exportAll()
        return BackupExportPayload(
            data: data,
            filename: "taigi_backup_\(Self.formattedDate()).taigi",
        )
    }

    func importBackup(url: URL) async throws -> BackupService.ImportResult {
        isProcessing = true
        defer { isProcessing = false }
        let data = try await Self.readFileData(from: url)
        return try await service.importAll(from: data)
    }

    /// Read file off the main actor so a large import doesn't block UI.
    private static func readFileData(from url: URL) async throws -> Data {
        try await Task.detached(priority: .userInitiated) {
            let accessing = url.startAccessingSecurityScopedResource()
            defer {
                if accessing {
                    url.stopAccessingSecurityScopedResource()
                }
            }
            return try Data(contentsOf: url)
        }.value
    }

    private static func formattedDate() -> String {
        let formatter = DateFormatter()
        // Pin POSIX locale so the backup-filename date suffix is always Gregorian + ASCII
        // digits, matching Android's Locale.US — keeps the filename a stable ASCII artifact.
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd"
        return formatter.string(from: Date())
    }
}
