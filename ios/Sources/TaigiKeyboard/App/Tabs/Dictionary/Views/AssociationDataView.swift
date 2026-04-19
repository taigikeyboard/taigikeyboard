import SwiftUI
import UniformTypeIdentifiers

/// Association data sub-page
/// Shows word association list with toggle, import/export, and clear option
struct AssociationDataView: View {
    @StateObject private var viewModel = AssociationDataViewModel()
    @StateObject private var importExport = ImportExportHandler()

    @State private var filterText = ""
    @State private var showClearAlert = false

    private let displayLimit = 100

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
                            Text(DictionaryTexts.isAssociationRecordingEnabled)
                            SettingInfoButton(description: DictionaryTexts.isAssociationRecordingEnabledInfo)
                        }
                    }
                }

                // Import/Export
                Section {
                    Text(DictionaryTexts.associationDescription)
                        .font(AppStyle.bodyFont)
                    Button {
                        importExport.performExport { try await viewModel.exportCSV() }
                    } label: {
                        Label(
                            DictionaryTexts.associationExportCSV,
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
                                DictionaryTexts.associationImportCSV,
                                systemImage: "square.and.arrow.down",
                            )
                        }
                    }
                } header: {
                    Text(DictionaryTexts.importExportTitle)
                        .font(AppStyle.sectionHeaderFont)
                }

                // Clear
                Section {
                    Button(role: .destructive) {
                        showClearAlert = true
                    } label: {
                        Text(DictionaryTexts.clearAllAssociation)
                    }
                }

                // Privacy warning
                Section {
                    Text(DictionaryTexts.associationPrivacyWarning)
                }

                // Data list
                Section {
                    if viewModel.allData.isEmpty {
                        Text(DictionaryTexts.noData)
                            .foregroundColor(.secondary)
                    } else if !filterText.isEmpty, filteredData.isEmpty {
                        Text(DictionaryTexts.noResults)
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
                                    Image(systemName: "trash")
                                }
                            }
                        }
                    }
                } header: {
                    HStack {
                        Text(DictionaryTexts.associationManagement)
                            .font(AppStyle.sectionHeaderFont)
                        SettingInfoButton(description: DictionaryTexts.filterHint)
                    }
                }
            }
        }
        .safeAreaInset(edge: .bottom) {
            SearchBar(text: $filterText, placeholder: DictionaryTexts.searchPlaceholder)
        }
        .navigationTitle(DictionaryTexts.associationManagement)
        .navigationBarTitleDisplayMode(.large)
        .alert(DictionaryTexts.clearAllAssociation, isPresented: $showClearAlert) {
            Button(CommonTexts.cancel, role: .cancel) {}
            Button(DictionaryTexts.clear, role: .destructive) {
                Task { await viewModel.clearAll() }
            }
        } message: {
            Text(DictionaryTexts.clearAssociationMessage)
        }
        .importExportModifiers(
            handler: importExport,
            importAlertTitle: DictionaryTexts.associationImportCSV,
            exportAlertTitle: DictionaryTexts.associationExportCSV,
            exportFilename: { ImportExportHandler.exportFilename(prefix: "詞關聯紀錄") },
            okText: DictionaryTexts.ok,
            exportSuccessText: DictionaryTexts.exportSuccess,
            onFileImport: { handleImport($0) },
        )
        .task {
            await viewModel.load()
        }
    }

    // MARK: - Display

    private func associationDisplayText(_ item: NextWordService.AssociationEntry) -> String {
        let prev = item.prevTl.isEmpty ? item.prevWord : "(\(item.prevTl), \(item.prevWord))"
        let next = item.nextTl.isEmpty ? item.nextWord : "(\(item.nextTl), \(item.nextWord))"
        return "\(prev) → \(next)"
    }

    // MARK: - Import

    private func handleImport(_ result: Result<[URL], Error>) {
        importExport.handleFileImport(
            result,
            importAction: { url in try await viewModel.importCSV(url: url) },
            resultFormat: DictionaryTexts.importResultFormat,
            onComplete: { await viewModel.load() },
        )
    }
}
