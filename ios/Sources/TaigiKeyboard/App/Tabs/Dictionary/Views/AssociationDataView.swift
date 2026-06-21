// 中文: 詞關聯(NextWord 聯想)資料管理子頁。
// 中文: 含錄製開關、CSV 匯入匯出、全部清除、隱私警語、列表瀏覽 + 過濾搜尋。

import SwiftUI
import UniformTypeIdentifiers

/// Association data sub-page
/// Shows word association list with toggle, import/export, and clear option
// 中文: 詞關聯資料子頁的根 View。資料層由 AssociationDataViewModel 提供;
// 中文: 匯入匯出由 ImportExportHandler 負責。
struct AssociationDataView: View {
    @Environment(DisplayLanguageStore.self) private var lang
    @StateObject private var viewModel = AssociationDataViewModel()
    @StateObject private var importExport = ImportExportHandler()

    @State private var filterText = ""
    @State private var showClearAlert = false

    private let displayLimit = 100

    // 中文: 依 filterText 對 prevWord/prevTl/nextWord/nextTl 做大小寫不敏感子字串比對;
    // 中文: 無關鍵字時走 displayLimit 上限以避免大量列表卡頓。
    private var filteredData: [NextWordService.AssociationEntry] {
        if filterText.isEmpty {
            return Array(viewModel.allData.prefix(displayLimit))
        }
        let query = filterText.lowercased()
        return viewModel.allData.filter {
            $0.prevWord.lowercased().contains(query) ||
                $0.prevTl.lowercased().contains(query) ||
                $0.nextWord.lowercased().contains(query) ||
                $0.nextTl.lowercased().contains(query)
        }
    }

    var body: some View {
        List {
            if viewModel.isLoading {
                Section {
                    ProgressView()
                        .frame(maxWidth: .infinity)
                }
            } else {
                // Toggle
                Section {
                    Toggle(
                        isOn: Binding(
                            get: { viewModel.isAssociationRecordingEnabled },
                            set: { viewModel.setRecordingEnabled($0) },
                        ),
                    ) {
                        HStack {
                            Text(lang.string(.dictionaryAssociationRecordingEnabled))
                            SettingInfoButton(description: lang.string(.dictionaryAssociationRecordingEnabledInfo))
                        }
                    }
                }

                // Import/Export
                Section {
                    Text(lang.string(.dictionaryAssociationDescription))
                        .font(AppStyle.bodyFont)
                    Button {
                        importExport.performExport { try await viewModel.exportCSV() }
                    } label: {
                        Label(
                            lang.string(.dictionaryAssociationExportCSV),
                            systemImage: "square.and.arrow.up",
                        )
                    }
                    .disabled(importExport.isImporting)
                    if importExport.isImporting {
                        ProgressView()
                            .frame(maxWidth: .infinity)
                    } else {
                        Button {
                            importExport.showFileImporter = true
                        } label: {
                            Label(
                                lang.string(.dictionaryAssociationImportCSV),
                                systemImage: "square.and.arrow.down",
                            )
                        }
                    }
                } header: {
                    Text(lang.string(.dictionaryImportExportTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // Clear
                Section {
                    Button(role: .destructive) {
                        showClearAlert = true
                    } label: {
                        Text(lang.string(.dictionaryClearAllAssociation))
                    }
                }

                // Privacy warning
                Section {
                    Text(lang.string(.dictionaryAssociationPrivacyWarning))
                }

                // Data list
                Section {
                    if viewModel.allData.isEmpty {
                        Text(lang.string(.dictionaryNoData))
                            .foregroundColor(.secondary)
                    } else if !filterText.isEmpty, filteredData.isEmpty {
                        Text(lang.string(.dictionaryNoResults))
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(filteredData, id: \.id) { item in
                            HStack {
                                Text(associationDisplayText(item))
                                Spacer()
                                Text("\(item.count)")
                                    .foregroundColor(.secondary)
                                    .font(AppStyle.captionFont)
                            }
                            .swipeActions(edge: .trailing) {
                                Button(role: .destructive) {
                                    Task { await viewModel.delete(item) }
                                } label: {
                                    Image(latinSystemName: "trash")
                                }
                            }
                        }
                    }
                } header: {
                    HStack {
                        Text(lang.string(.dictionaryAssociationManagement))
                            .font(AppStyle.sectionHeaderFont)
                        SettingInfoButton(description: lang.string(.dictionaryFilterHint))
                    }
                }
            }
        }
        .safeAreaInset(edge: .bottom) {
            SearchBar(text: $filterText, placeholder: lang.string(.dictionarySearchPlaceholder))
        }
        .navigationTitle(lang.string(.dictionaryAssociationManagement))
        .navigationBarTitleDisplayMode(.large)
        .alert(lang.string(.dictionaryClearAllAssociation), isPresented: $showClearAlert) {
            Button(lang.string(.commonCancel), role: .cancel) {}
            Button(lang.string(.dictionaryClear), role: .destructive) {
                Task { await viewModel.clearAll() }
            }
        } message: {
            Text(lang.string(.dictionaryClearAssociationMessage))
        }
        .importExportModifiers(
            handler: importExport,
            importAlertTitle: lang.string(.dictionaryAssociationImportCSV),
            exportAlertTitle: lang.string(.dictionaryAssociationExportCSV),
            exportFilename: { ImportExportHandler.exportFilename(prefix: "詞關聯紀錄") },
            okText: lang.string(.commonOk),
            exportSuccessText: lang.string(.dictionaryExportSuccess),
            onFileImport: { handleImport($0) },
        )
        .task {
            await viewModel.load()
        }
    }

    // MARK: - Display

    // 中文: 把單筆聯想紀錄組成 "prev → next" 顯示字串;tl 為空時退化為純漢字顯示。
    private func associationDisplayText(_ item: NextWordService.AssociationEntry) -> String {
        let prev = item.prevTl.isEmpty ? item.prevWord : "(\(item.prevTl), \(item.prevWord))"
        let next = item.nextTl.isEmpty ? item.nextWord : "(\(item.nextTl), \(item.nextWord))"
        return "\(prev) → \(next)"
    }

    // MARK: - Import

    // 中文: 把 fileImporter 結果轉交 ImportExportHandler;完成後觸發 viewModel.load() 重新載入。
    private func handleImport(_ result: Result<[URL], Error>) {
        importExport.handleFileImport(
            result,
            importAction: { url in try await viewModel.importCSV(url: url) },
            formatResult: { lang.resolver.dictionaryImportResult(imported: $0, skipped: $1) },
            onComplete: { await viewModel.load() },
        )
    }
}
