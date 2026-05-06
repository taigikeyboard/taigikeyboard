// 中文: CustomDictionary / Frequency / Association 三個資料管理畫面共用的匯入匯出 handler。
// 中文: 集中管理 fileImporter / fileExporter / progress / 各式 alert 的狀態,View 透過
// 中文: importExportModifiers() 一次掛上 SwiftUI modifiers。

import SwiftUI
import UniformTypeIdentifiers

/// Manages import/export state and flow for data management views.
/// Each DictionaryTab data view (CustomDictionary, Frequency, Association) creates
/// its own instance to handle file import/export, progress tracking, and alerts.
// 中文: 匯入匯出流程的 ObservableObject;含進度旗標、CSV document 暫存、各式 alert 訊息。
@MainActor
final class ImportExportHandler: ObservableObject {
    // 中文: 匯入進行中旗標,View 顯示 progress UI 用。
    @Published var isImporting = false
    // 中文: 觸發 SwiftUI fileImporter sheet。
    @Published var showFileImporter = false
    // 中文: 觸發 SwiftUI fileExporter sheet。
    @Published var showFileExporter = false
    // 中文: 待匯出的 CSV 文件實體;在 performExport 完成後填入。
    @Published var csvDocument: CSVDocument?

    // 中文: 匯入結果 alert 開關 + 顯示文字(由 resultFormat 格式化)。
    @Published var showImportResultAlert = false
    @Published var importResultMessage = ""
    // 中文: 匯出成功 alert 開關。
    @Published var showExportSuccessAlert = false
    // 中文: 錯誤 alert 開關 + 顯示文字。
    @Published var showErrorAlert = false
    @Published var errorMessage = ""

    nonisolated init() {}

    /// Exports data as CSV and triggers the file exporter sheet.
    // 中文: 把 csvGenerator() 產生的字串包成 CSVDocument 並觸發 fileExporter sheet。
    func performExport(_ csvGenerator: @escaping () async throws -> String) {
        Task {
            do {
                let csv = try await csvGenerator()
                csvDocument = CSVDocument(csv)
                showFileExporter = true
            } catch {
                showError(error)
            }
        }
    }

    /// Handles the `.fileImporter` result with shared boilerplate:
    /// progress tracking, error handling, and result alert display.
    // 中文: 統一處理 fileImporter 結果 — 設定 isImporting、跑 importAction、組訊息、彈 alert。
    func handleFileImport(
        _ result: Result<[URL], Error>,
        importAction: @escaping (URL) async throws -> (imported: Int, skipped: Int),
        resultFormat: String,
        onComplete: (() async -> Void)? = nil,
    ) {
        switch result {
        case let .success(urls):
            guard let url = urls.first else { return }
            isImporting = true
            Task {
                defer { isImporting = false }
                do {
                    let importResult = try await importAction(url)
                    importResultMessage = String(
                        format: resultFormat,
                        importResult.imported, importResult.skipped,
                    )
                    showImportResultAlert = true
                    await onComplete?()
                } catch {
                    showError(error)
                }
            }
        case let .failure(error):
            showError(error)
        }
    }

    /// Generates an export filename with date suffix.
    // 中文: 產生帶日期後綴的匯出檔名,格式 "<prefix>_yyyy-MM-dd.csv"。
    static func exportFilename(prefix: String) -> String {
        let formatter = DateFormatter()
        formatter.dateFormat = "yyyy-MM-dd"
        return "\(prefix)_\(formatter.string(from: Date())).csv"
    }

    // 中文: 設定錯誤訊息並彈出 error alert。
    private func showError(_ error: Error) {
        errorMessage = error.localizedDescription
        showErrorAlert = true
    }
}

// MARK: - View Modifier

/// Attaches file importer, file exporter, and alert modifiers for import/export flow.
// 中文: 私有 ViewModifier — 一次掛上 fileImporter / fileExporter / 三類 alert。
private struct ImportExportModifiers: ViewModifier {
    @ObservedObject var handler: ImportExportHandler
    let importAlertTitle: String
    let exportAlertTitle: String
    let exportFilename: () -> String
    let okText: String
    let exportSuccessText: String
    let onFileImport: (Result<[URL], Error>) -> Void

    func body(content: Content) -> some View {
        content
            .fileImporter(
                isPresented: $handler.showFileImporter,
                allowedContentTypes: [.commaSeparatedText, .plainText],
                allowsMultipleSelection: false,
            ) { result in
                onFileImport(result)
            }
            .fileExporter(
                isPresented: $handler.showFileExporter,
                document: handler.csvDocument,
                contentType: .commaSeparatedText,
                defaultFilename: exportFilename(),
            ) { result in
                if case .success = result {
                    handler.showExportSuccessAlert = true
                }
            }
            .alert(importAlertTitle, isPresented: $handler.showImportResultAlert) {
                Button(okText) {}
            } message: {
                Text(handler.importResultMessage)
            }
            .alert(exportAlertTitle, isPresented: $handler.showExportSuccessAlert) {
                Button(okText) {}
            } message: {
                Text(exportSuccessText)
            }
            .alert("Error", isPresented: $handler.showErrorAlert) {
                Button(okText) {}
            } message: {
                Text(handler.errorMessage)
            }
    }
}

extension View {
    // 中文: 便利 API — 在 View 上一次掛載匯入匯出相關的所有 SwiftUI modifiers。
    func importExportModifiers(
        handler: ImportExportHandler,
        importAlertTitle: String,
        exportAlertTitle: String,
        exportFilename: @escaping () -> String,
        okText: String,
        exportSuccessText: String,
        onFileImport: @escaping (Result<[URL], Error>) -> Void,
    ) -> some View {
        modifier(ImportExportModifiers(
            handler: handler,
            importAlertTitle: importAlertTitle,
            exportAlertTitle: exportAlertTitle,
            exportFilename: exportFilename,
            okText: okText,
            exportSuccessText: exportSuccessText,
            onFileImport: onFileImport,
        ))
    }
}
