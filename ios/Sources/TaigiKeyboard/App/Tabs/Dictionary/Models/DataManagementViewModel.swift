// 中文: DataManagementView 的 ViewModel — 整體備份檔(.taigi)的匯出 / 匯入。
// 中文: SwiftUI fileExporter / fileImporter 綁定仍由 View 持有,本 VM 只提供 payload 與 service 呼叫。

import Foundation

/// ViewModel for `DataManagementView`.
///
/// Owns backup export/import orchestration. The view keeps the SwiftUI file
/// exporter/importer bindings (`fileExporter`/`fileImporter`) and drives them
/// with the `BackupExportPayload` returned from `exportBackup()`.
// 中文: 備份/復原畫面的 ViewModel,串接 BackupService。
@MainActor
final class DataManagementViewModel: ObservableObject {
    /// Payload the view feeds into `.fileExporter`.
    // 中文: 餵給 SwiftUI fileExporter 的資料 + 預設檔名。
    struct BackupExportPayload {
        let data: Data
        let filename: String
    }

    // 中文: 匯出/匯入進行中旗標,View 用來顯示 progress UI。
    @Published var isProcessing = false

    private let service: BackupService

    init(service: BackupService = CompositionRoot.backupService) {
        self.service = service
    }

    // 中文: 匯出全部使用者資料為備份 payload(預設檔名含日期)。
    func exportBackup() async throws -> BackupExportPayload {
        isProcessing = true
        defer { isProcessing = false }
        let data = try await service.exportAll()
        return BackupExportPayload(
            data: data,
            filename: "taigi_backup_\(Self.formattedDate()).taigi",
        )
    }

    // 中文: 從 URL 讀檔並交給 BackupService 匯入,回傳各分類匯入結果。
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
            defer { if accessing { url.stopAccessingSecurityScopedResource() } }
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
