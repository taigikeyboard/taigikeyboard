import SwiftUI
import UniformTypeIdentifiers

/// Frequency data sub-page
/// Shows top word frequency list with toggle, import/export, and clear option
struct FrequencyDataView: View {
    @StateObject private var viewModel = FrequencyDataViewModel()
    @StateObject private var importExport = ImportExportHandler()

    @State private var filterText = ""
    @State private var showClearAlert = false

    private let displayLimit = 100

    private var filteredData: [(word: String, count: Int)] {
        if filterText.isEmpty {
            return Array(viewModel.allData.prefix(displayLimit))
        }
        let query = filterText.lowercased()
        return viewModel.allData.filter { $0.word.lowercased().contains(query) }
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
                            get: { viewModel.isFrequencyRecordingEnabled },
                            set: { viewModel.setRecordingEnabled($0) },
                        ),
                    ) {
                        HStack {
                            Text(DictionaryTexts.isFrequencyRecordingEnabled)
                            SettingInfoButton(description: DictionaryTexts.isFrequencyRecordingEnabledInfo)
                        }
                    }
                }

                // Import/Export
                Section {
                    Text(DictionaryTexts.frequencyDescription)
                        .font(AppStyle.bodyFont)
                    Button {
                        importExport.performExport { try await viewModel.exportCSV() }
                    } label: {
                        Label(
                            DictionaryTexts.frequencyExportCSV,
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
                                DictionaryTexts.frequencyImportCSV,
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
                        Text(DictionaryTexts.clearAllFrequency)
                    }
                }

                // Privacy warning
                Section {
                    Text(DictionaryTexts.frequencyPrivacyWarning)
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
                        ForEach(filteredData, id: \.word) { item in
                            HStack {
                                Text(item.word)
                                Spacer()
                                Text("\(item.count)")
                                    .foregroundColor(.secondary)
                                    .font(AppStyle.captionFont)
                            }
                            .swipeActions(edge: .trailing) {
                                Button(role: .destructive) {
                                    Task { await viewModel.deleteWord(item.word) }
                                } label: {
                                    Image(latinSystemName: "trash")
                                }
                            }
                        }
                    }
                } header: {
                    HStack {
                        Text(DictionaryTexts.frequencyManagement)
                            .font(AppStyle.sectionHeaderFont)
                        SettingInfoButton(description: DictionaryTexts.filterHint)
                    }
                }
            }
        }
        .safeAreaInset(edge: .bottom) {
            SearchBar(text: $filterText, placeholder: DictionaryTexts.searchPlaceholder)
        }
        .navigationTitle(DictionaryTexts.frequencyManagement)
        .navigationBarTitleDisplayMode(.large)
        .alert(DictionaryTexts.clearAllFrequency, isPresented: $showClearAlert) {
            Button(CommonTexts.cancel, role: .cancel) {}
            Button(DictionaryTexts.clear, role: .destructive) {
                viewModel.clearAll()
            }
        } message: {
            Text(DictionaryTexts.clearFrequencyMessage)
        }
        .importExportModifiers(
            handler: importExport,
            importAlertTitle: DictionaryTexts.frequencyImportCSV,
            exportAlertTitle: DictionaryTexts.frequencyExportCSV,
            exportFilename: { ImportExportHandler.exportFilename(prefix: "詞頻紀錄") },
            okText: DictionaryTexts.ok,
            exportSuccessText: DictionaryTexts.exportSuccess,
            onFileImport: { handleImport($0) },
        )
        .task {
            await viewModel.load()
        }
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
