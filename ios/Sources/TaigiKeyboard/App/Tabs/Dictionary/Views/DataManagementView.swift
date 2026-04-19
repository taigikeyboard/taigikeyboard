import SwiftUI
import UIKit
import UniformTypeIdentifiers

/// Backup/Restore sub-page
/// Provides export and import of all user data
struct DataManagementView: View {
    @StateObject private var viewModel = DataManagementViewModel()

    @State private var showBackupExporter = false
    @State private var showBackupImporter = false
    @State private var backupDocument: BackupDocument?
    @State private var backupFilename = "taigi_backup.taigi"

    @State private var showBackupResultAlert = false
    @State private var backupResultMessage = ""
    @State private var showExportSuccessAlert = false
    @State private var showBackupErrorAlert = false
    @State private var backupErrorMessage = ""

    var body: some View {
        List {
            // Privacy warning
            Section {
                Text(DictionaryTexts.backupPrivacyWarning)
            }

            // Backup/Restore
            Section {
                Button {
                    exportBackup()
                } label: {
                    Label(
                        DictionaryTexts.exportBackup,
                        systemImage: "square.and.arrow.up",
                    )
                }
                .disabled(viewModel.isProcessing)
                if viewModel.isProcessing {
                    ProgressView()
                        .frame(maxWidth: .infinity)
                } else {
                    Button {
                        showBackupImporter = true
                    } label: {
                        Label(
                            DictionaryTexts.importBackup,
                            systemImage: "square.and.arrow.down",
                        )
                    }
                }
            }
        }
        .navigationTitle(DictionaryTexts.backupRestore)
        .navigationBarTitleDisplayMode(.large)
        .fileExporter(
            isPresented: $showBackupExporter,
            document: backupDocument,
            contentType: .taigiBackup,
            defaultFilename: backupFilename,
        ) { result in
            if case .success = result {
                showExportSuccessAlert = true
            }
        }
        .fileImporter(
            isPresented: $showBackupImporter,
            allowedContentTypes: [.taigiBackup, .json],
            allowsMultipleSelection: false,
        ) { result in
            handleBackupImport(result)
        }
        .alert(DictionaryTexts.exportBackup, isPresented: $showExportSuccessAlert) {
            Button(DictionaryTexts.ok) {}
        } message: {
            Text(DictionaryTexts.exportBackupSuccess)
        }
        .alert(DictionaryTexts.importBackup, isPresented: $showBackupResultAlert) {
            Button(DictionaryTexts.ok) {}
        } message: {
            Text(backupResultMessage)
        }
        .alert("Error", isPresented: $showBackupErrorAlert) {
            Button(DictionaryTexts.ok) {}
        } message: {
            Text(backupErrorMessage)
        }
    }

    // MARK: - Backup / Restore

    private func exportBackup() {
        Task {
            do {
                let payload = try await viewModel.exportBackup()
                backupFilename = payload.filename
                backupDocument = BackupDocument(payload.data)
                showBackupExporter = true
            } catch {
                backupErrorMessage = error.localizedDescription
                showBackupErrorAlert = true
            }
        }
    }

    private func handleBackupImport(_ result: Result<[URL], Error>) {
        switch result {
        case let .success(urls):
            guard let url = urls.first else { return }
            Task {
                do {
                    let importResult = try await viewModel.importBackup(url: url)
                    backupResultMessage = String(
                        format: DictionaryTexts.importBackupResult,
                        importResult.customDict,
                        importResult.frequency,
                        importResult.association,
                    )
                    showBackupResultAlert = true
                    UIImpactFeedbackGenerator(style: .medium).impactOccurred()
                } catch {
                    backupErrorMessage = error.localizedDescription
                    showBackupErrorAlert = true
                }
            }
        case let .failure(error):
            backupErrorMessage = error.localizedDescription
            showBackupErrorAlert = true
        }
    }
}
