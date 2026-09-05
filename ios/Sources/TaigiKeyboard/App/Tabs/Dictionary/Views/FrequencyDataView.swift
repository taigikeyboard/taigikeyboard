import SwiftUI
import UniformTypeIdentifiers

/// Frequency data sub-page
/// Shows top word frequency list with toggle, import/export, and clear option
struct FrequencyDataView: View {
    @Environment(DisplayLanguageStore.self) private var lang
    @StateObject private var viewModel = FrequencyDataViewModel()
    @StateObject private var importExport = ImportExportHandler()

    @State private var filterText = ""
    @State private var showClearAlert = false

    private let displayLimit = 100

    // Unfiltered list caps at displayLimit to stay responsive; filtering matches word or tl case-insensitively.
    private var filteredData: [FrequencyListItem] {
        if filterText.isEmpty {
            return Array(viewModel.allData.prefix(displayLimit))
        }
        let query = filterText.lowercased()
        return viewModel.allData.filter {
            $0.word.lowercased().contains(query) || $0.tl.lowercased().contains(query)
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
                            get: { viewModel.isFrequencyRecordingEnabled },
                            set: { viewModel.setRecordingEnabled($0) },
                        ),
                    ) {
                        HStack {
                            Text(lang.string(.dictionaryFrequencyRecordingEnabled))
                            SettingInfoButton(description: lang.string(.dictionaryFrequencyRecordingEnabledInfo))
                        }
                    }
                }

                // Import/Export
                Section {
                    Text(lang.string(.dictionaryFrequencyDescription))
                        .font(AppStyle.bodyFont)
                    Button {
                        importExport.performExport { try await viewModel.exportCSV() }
                    } label: {
                        Label(
                            lang.string(.dictionaryFrequencyExportCSV),
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
                                lang.string(.dictionaryFrequencyImportCSV),
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
                        Text(lang.string(.dictionaryClearAllFrequency))
                    }
                }

                // Privacy warning
                Section {
                    Text(lang.string(.dictionaryFrequencyPrivacyWarning))
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
                        ForEach(filteredData) { item in
                            HStack(spacing: 8) {
                                // Roman (caption) + hanji (body) on one row; tl == "" or tl == word skips the prefix.
                                if !item.tl.isEmpty, item.tl != item.word {
                                    Text(item.tl)
                                        .foregroundColor(.secondary)
                                        .font(AppStyle.captionFont)
                                }
                                Text(item.word)
                                Spacer()
                                Text("\(item.count)")
                                    .foregroundColor(.secondary)
                                    .font(AppStyle.captionFont)
                            }
                            .swipeActions(edge: .trailing) {
                                Button(role: .destructive) {
                                    Task { await viewModel.deleteWord(item.word, tl: item.tl) }
                                } label: {
                                    Image(latinSystemName: "trash")
                                }
                            }
                        }
                    }
                } header: {
                    HStack {
                        Text(lang.string(.dictionaryFrequencyManagement))
                            .font(AppStyle.sectionHeaderFont)
                        SettingInfoButton(description: lang.string(.dictionaryFilterHint))
                    }
                }
            }
        }
        .safeAreaInset(edge: .bottom) {
            SearchBar(text: $filterText, placeholder: lang.string(.dictionarySearchPlaceholder))
        }
        .navigationTitle(lang.string(.dictionaryFrequencyManagement))
        .navigationBarTitleDisplayMode(.large)
        .alert(lang.string(.dictionaryClearAllFrequency), isPresented: $showClearAlert) {
            Button(lang.string(.commonCancel), role: .cancel) {}
            Button(lang.string(.dictionaryClear), role: .destructive) {
                viewModel.clearAll()
            }
        } message: {
            Text(lang.string(.dictionaryClearFrequencyMessage))
        }
        .importExportModifiers(
            handler: importExport,
            importAlertTitle: lang.string(.dictionaryFrequencyImportCSV),
            exportAlertTitle: lang.string(.dictionaryFrequencyExportCSV),
            errorTitle: lang.string(.commonError),
            exportFilename: { ImportExportHandler.exportFilename(prefix: "taigi_frequency") },
            okText: lang.string(.commonOk),
            exportSuccessText: lang.string(.dictionaryExportSuccess),
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
            formatResult: { lang.resolver.dictionaryImportResult(imported: $0, skipped: $1) },
            formatError: { _ in lang.string(.commonImportFailed) },
            onComplete: { await viewModel.load() },
        )
    }
}
