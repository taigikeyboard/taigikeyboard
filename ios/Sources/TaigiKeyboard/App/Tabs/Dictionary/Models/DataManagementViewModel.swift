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

    private let userData: any UserDataClient

    init(userData: any UserDataClient = CompositionRoot.userData) {
        self.userData = userData
    }

    func exportBackup() async throws -> BackupExportPayload {
        isProcessing = true
        defer { isProcessing = false }
        let data = try await userData.exportBackup(appVersion: Bundle.main.shortVersion)
        return BackupExportPayload(
            data: data,
            filename: "taigi_backup_\(Self.formattedDate()).taigi",
        )
    }

    func importBackup(url: URL) async throws -> BackupImportResult {
        isProcessing = true
        defer { isProcessing = false }
        return try await userData.importBackup(url: url)
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
