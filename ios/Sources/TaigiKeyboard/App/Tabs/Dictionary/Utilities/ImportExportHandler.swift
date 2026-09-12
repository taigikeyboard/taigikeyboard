import SwiftUI
import UniformTypeIdentifiers

/// Manages import/export state and flow for data management views.
/// Each DictionaryTab data view (CustomDictionary, Frequency, Association) creates
/// its own instance to handle file import/export, progress tracking, and alerts.
@MainActor
final class ImportExportHandler: ObservableObject {
    @Published var isImporting = false
    @Published var showFileImporter = false
    @Published var showFileExporter = false
    @Published var csvDocument: CSVDocument?

    @Published var showImportResultAlert = false
    @Published var importResultMessage = ""
    @Published var showExportSuccessAlert = false
    @Published var showErrorAlert = false
    @Published var errorMessage = ""

    nonisolated init() {}

    // nonisolated: the stored value is referenced as a default-argument from a nonisolated context; the
    // closure itself stays @MainActor-typed and is only ever invoked on the main actor.
    private nonisolated static let defaultErrorFormatter: @MainActor (Error) -> String = { $0.localizedDescription }

    /// Exports data as CSV and triggers the file exporter sheet.
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
    ///
    /// `formatResult` turns the imported/skipped counts into the result message — counts are computed
    /// inside this handler, so the caller passes a formatter (backed by the generated typed i18n
    /// accessor) rather than a pre-built string. `formatError` localizes a thrown error at the display
    /// boundary; the default surfaces `localizedDescription`, and a caller whose service raises a typed
    /// error (e.g. `CustomDictionaryError`) maps it to a localized string via the App-layer resolver.
    func handleFileImport(
        _ result: Result<[URL], Error>,
        importAction: @escaping (URL) async throws -> (imported: Int, skipped: Int),
        formatResult: @MainActor @escaping (_ imported: Int, _ skipped: Int) -> String,
        formatError: @MainActor @escaping (Error) -> String = ImportExportHandler.defaultErrorFormatter,
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
                    importResultMessage = formatResult(importResult.imported, importResult.skipped)
                    showImportResultAlert = true
                    await onComplete?()
                } catch {
                    showError(error, formatError)
                }
            }
        case let .failure(error):
            showError(error, formatError)
        }
    }

    /// Generates an export filename with date suffix.
    static func exportFilename(prefix: String) -> String {
        let formatter = DateFormatter()
        // Pin POSIX locale so the date suffix is always Gregorian + ASCII digits,
        // matching Android's Locale.US — keeps the filename a stable ASCII artifact.
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.dateFormat = "yyyy-MM-dd"
        return "\(prefix)_\(formatter.string(from: Date())).csv"
    }

    private func showError(_ error: Error, _ formatError: @MainActor (Error) -> String = ImportExportHandler.defaultErrorFormatter) {
        errorMessage = formatError(error)
        showErrorAlert = true
    }
}

// MARK: - View Modifier

/// Attaches file importer, file exporter, and alert modifiers for import/export flow.
private struct ImportExportModifiers: ViewModifier {
    @ObservedObject var handler: ImportExportHandler
    let importAlertTitle: String
    let exportAlertTitle: String
    let errorTitle: String
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
            .alert(errorTitle, isPresented: $handler.showErrorAlert) {
                Button(okText) {}
            } message: {
                Text(handler.errorMessage)
            }
    }
}

extension View {
    func importExportModifiers(
        handler: ImportExportHandler,
        importAlertTitle: String,
        exportAlertTitle: String,
        errorTitle: String,
        exportFilename: @escaping () -> String,
        okText: String,
        exportSuccessText: String,
        onFileImport: @escaping (Result<[URL], Error>) -> Void,
    ) -> some View {
        modifier(ImportExportModifiers(
            handler: handler,
            importAlertTitle: importAlertTitle,
            exportAlertTitle: exportAlertTitle,
            errorTitle: errorTitle,
            exportFilename: exportFilename,
            okText: okText,
            exportSuccessText: exportSuccessText,
            onFileImport: onFileImport,
        ))
    }
}
