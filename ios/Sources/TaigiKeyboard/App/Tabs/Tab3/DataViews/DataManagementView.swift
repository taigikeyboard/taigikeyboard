import SwiftUI
import UIKit
import UniformTypeIdentifiers

/// Backup/Restore sub-page
/// Provides export and import of all user data
struct DataManagementView: View {
    @StateObject private var languageManager = LanguageManager.shared

    // Backup/Restore state
    @State private var showBackupExporter = false
    @State private var showBackupImporter = false
    @State private var backupDocument: BackupDocument?
    @State private var backupFilename = "taigi_backup.taigi"
    @State private var showBackupResultAlert = false
    @State private var backupResultMessage = ""
    @State private var showExportSuccessAlert = false
    @State private var showBackupErrorAlert = false
    @State private var backupErrorMessage = ""
    @State private var isProcessing = false

    var body: some View {
        List {
            // Privacy warning
            Section {
                Text(languageManager.text(Tab3Texts.backupPrivacyWarning))
            }

            // Backup/Restore
            Section {
                Button {
                    exportBackup()
                } label: {
                    Label(
                        languageManager.text(Tab3Texts.exportBackup),
                        systemImage: "square.and.arrow.up",
                    )
                }
                .disabled(isProcessing)
                if isProcessing {
                    ProgressView()
                        .frame(maxWidth: .infinity)
                } else {
                    Button {
                        showBackupImporter = true
                    } label: {
                        Label(
                            languageManager.text(Tab3Texts.importBackup),
                            systemImage: "square.and.arrow.down",
                        )
                    }
                }
            }
        }
        .navigationTitle(languageManager.text(Tab3Texts.backupRestore))
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
        .alert(languageManager.text(Tab3Texts.exportBackup), isPresented: $showExportSuccessAlert) {
            Button(languageManager.text(Tab3Texts.ok)) {}
        } message: {
            Text(languageManager.text(Tab3Texts.exportBackupSuccess))
        }
        .alert(languageManager.text(Tab3Texts.importBackup), isPresented: $showBackupResultAlert) {
            Button(languageManager.text(Tab3Texts.ok)) {}
        } message: {
            Text(backupResultMessage)
        }
        .alert("Error", isPresented: $showBackupErrorAlert) {
            Button(languageManager.text(Tab3Texts.ok)) {}
        } message: {
            Text(backupErrorMessage)
        }
    }

    // MARK: - Backup/Restore

    private func exportBackup() {
        isProcessing = true
        Task {
            defer { Task { @MainActor in isProcessing = false } }
            do {
                let data = try await BackupService.shared.exportAll()
                await MainActor.run {
                    let dateStr = {
                        let f = DateFormatter()
                        f.dateFormat = "yyyy-MM-dd"
                        return f.string(from: Date())
                    }()
                    backupFilename = "備份復原_\(dateStr).taigi"
                    backupDocument = BackupDocument(data)
                    showBackupExporter = true
                }
            } catch {
                await MainActor.run {
                    backupErrorMessage = error.localizedDescription
                    showBackupErrorAlert = true
                }
            }
        }
    }

    private func handleBackupImport(_ result: Result<[URL], Error>) {
        switch result {
        case let .success(urls):
            guard let url = urls.first else { return }
            isProcessing = true
            Task {
                defer { Task { @MainActor in isProcessing = false } }
                do {
                    let accessing = url.startAccessingSecurityScopedResource()
                    defer {
                        if accessing { url.stopAccessingSecurityScopedResource() }
                    }
                    let data = try Data(contentsOf: url)
                    let importResult = try await BackupService.shared.importAll(from: data)
                    await MainActor.run {
                        backupResultMessage = String(
                            format: languageManager.text(Tab3Texts.importBackupResult),
                            importResult.customDict,
                            importResult.frequency,
                            importResult.association,
                        )
                        showBackupResultAlert = true
                        let impactFeedback = UIImpactFeedbackGenerator(style: .medium)
                        impactFeedback.impactOccurred()
                    }
                } catch {
                    await MainActor.run {
                        backupErrorMessage = error.localizedDescription
                        showBackupErrorAlert = true
                    }
                }
            }
        case let .failure(error):
            backupErrorMessage = error.localizedDescription
            showBackupErrorAlert = true
        }
    }
}
