// 中文: 整體備份 / 復原子頁。
// 中文: 透過 .taigi 備份檔一次匯入匯出全部使用者資料(自訂詞庫、詞頻、聯想資料)。

import SwiftUI
import UIKit
import UniformTypeIdentifiers

/// Backup/Restore sub-page
/// Provides export and import of all user data
// 中文: 備份 / 復原子頁的根 View。資料層由 DataManagementViewModel + BackupService 提供。
struct DataManagementView: View {
    @Environment(DisplayLanguageStore.self) private var lang
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
                Text(lang.string(.dictionaryBackupPrivacyWarning))
            }

            // Backup/Restore
            Section {
                Button {
                    exportBackup()
                } label: {
                    Label(
                        lang.string(.dictionaryExportBackup),
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
                            lang.string(.dictionaryImportBackup),
                            systemImage: "square.and.arrow.down",
                        )
                    }
                }
            }
        }
        .navigationTitle(lang.string(.dictionaryBackupRestore))
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
        .alert(lang.string(.dictionaryExportBackup), isPresented: $showExportSuccessAlert) {
            Button(lang.string(.commonOk)) {}
        } message: {
            Text(lang.string(.dictionaryExportBackupSuccess))
        }
        .alert(lang.string(.dictionaryImportBackup), isPresented: $showBackupResultAlert) {
            Button(lang.string(.commonOk)) {}
        } message: {
            Text(backupResultMessage)
        }
        .alert("Error", isPresented: $showBackupErrorAlert) {
            Button(lang.string(.commonOk)) {}
        } message: {
            Text(backupErrorMessage)
        }
    }

    // MARK: - Backup / Restore

    // 中文: 觸發匯出流程 — 從 ViewModel 取 BackupExportPayload,設定檔名與 BackupDocument,彈出 fileExporter。
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

    // 中文: 處理 fileImporter 結果 — 呼叫 ViewModel.importBackup,組三類資料筆數訊息,彈 alert + haptic。
    private func handleBackupImport(_ result: Result<[URL], Error>) {
        switch result {
        case let .success(urls):
            guard let url = urls.first else { return }
            Task {
                do {
                    let importResult = try await viewModel.importBackup(url: url)
                    backupResultMessage = lang.resolver.dictionaryImportBackupResult(
                        customDict: importResult.customDict,
                        frequency: importResult.frequency,
                        association: importResult.association,
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
