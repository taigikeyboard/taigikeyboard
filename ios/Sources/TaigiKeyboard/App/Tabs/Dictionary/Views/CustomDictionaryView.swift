import SwiftUI
import UniformTypeIdentifiers

/// Custom dictionary subpage
/// Lists all user-added entries with add/edit/delete and import/export
struct CustomDictionaryView: View {
    @StateObject private var viewModel = CustomDictionaryViewModel()
    @StateObject private var importExport = ImportExportHandler()

    @State private var filterText = ""
    @State private var showEntryAlert = false
    @State private var editingEntry: CustomDictionaryEntry?
    @State private var romanInput = ""
    @State private var hanziInput = ""
    @State private var showDeleteAllAlert = false

    private var filteredEntries: [CustomDictionaryEntry] {
        if filterText.isEmpty {
            return Array(viewModel.entries.prefix(100))
        }
        let query = filterText.lowercased()
        return viewModel.entries.filter {
            $0.roman.lowercased().contains(query) ||
                $0.hanzi.lowercased().contains(query)
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
                // Enable/Disable toggle
                Section {
                    Toggle(
                        isOn: Binding(
                            get: { viewModel.isCustomDictEnabled },
                            set: { viewModel.setCustomDictEnabled($0) },
                        ),
                    ) {
                        HStack {
                            Text(DictionaryTexts.isCustomDictEnabled)
                            SettingInfoButton(description: DictionaryTexts.isCustomDictEnabledInfo)
                        }
                    }
                }

                // Import/Export
                Section {
                    Image("csv_example")
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .clipShape(RoundedRectangle(cornerRadius: AppStyle.smallCornerRadius))
                        .listRowSeparator(.hidden)
                    Text(DictionaryTexts.customDictDescription)
                        .font(AppStyle.bodyFont)
                    Button {
                        importExport.performExport { try await viewModel.exportCSV() }
                    } label: {
                        Label(
                            DictionaryTexts.exportCSV,
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
                                DictionaryTexts.importCSV,
                                systemImage: "square.and.arrow.down",
                            )
                        }
                    }
                } header: {
                    Text(DictionaryTexts.importExportTitle)
                        .font(AppStyle.sectionHeaderFont)
                }

                // Delete all
                Section {
                    Button(role: .destructive) {
                        showDeleteAllAlert = true
                    } label: {
                        Text(DictionaryTexts.deleteAll)
                    }
                    .disabled(importExport.isImporting)
                }

                // Privacy warning
                Section {
                    Text(DictionaryTexts.customDictPrivacyWarning)
                }

                // Entry list
                Section {
                    if viewModel.entries.isEmpty {
                        VStack(spacing: 16) {
                            Image(latinSystemName: "book.closed")
                                .font(AppStyle.appFont(size: 48))
                                .foregroundColor(.secondary)
                            Text(DictionaryTexts.customDictEmpty)
                                .foregroundColor(.secondary)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 32)
                    } else if !filterText.isEmpty, filteredEntries.isEmpty {
                        Text(DictionaryTexts.noResults)
                            .foregroundColor(.secondary)
                    } else {
                        ForEach(filteredEntries) { entry in
                            Button {
                                editingEntry = entry
                                romanInput = entry.roman
                                hanziInput = entry.hanzi
                                showEntryAlert = true
                            } label: {
                                HStack {
                                    Text("\(entry.roman) → \(entry.hanzi)")
                                        .font(AppStyle.bodyFont)
                                        .foregroundColor(.primary)
                                    Spacer()
                                    Image(latinSystemName: "chevron.right")
                                        .font(AppStyle.captionFont)
                                        .foregroundColor(.secondary)
                                }
                            }
                            .swipeActions(edge: .trailing) {
                                Button(role: .destructive) {
                                    Task { await viewModel.delete(id: entry.id) }
                                } label: {
                                    Image(latinSystemName: "trash")
                                }
                            }
                        }
                    }
                } header: {
                    HStack {
                        Text(DictionaryTexts.customDictionary)
                            .font(AppStyle.sectionHeaderFont)
                        SettingInfoButton(description: DictionaryTexts.filterHint)
                    }
                }
            }
        }
        .safeAreaInset(edge: .bottom) {
            SearchBar(text: $filterText, placeholder: DictionaryTexts.searchPlaceholder)
        }
        .navigationTitle(DictionaryTexts.customDictionary)
        .navigationBarTitleDisplayMode(.large)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button {
                    editingEntry = nil
                    romanInput = ""
                    hanziInput = ""
                    showEntryAlert = true
                } label: {
                    Image(latinSystemName: "plus")
                }
            }
        }
        .alert(
            editingEntry != nil ? DictionaryTexts.editEntry : DictionaryTexts.addEntry,
            isPresented: $showEntryAlert,
        ) {
            TextField(DictionaryTexts.romanPlaceholder, text: $romanInput)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.never)
            TextField(DictionaryTexts.hanziPlaceholder, text: $hanziInput)
            Button(CommonTexts.cancel, role: .cancel) {
                editingEntry = nil
            }
            Button(DictionaryTexts.save) {
                saveEntryFromAlert()
            }
        }
        .importExportModifiers(
            handler: importExport,
            importAlertTitle: DictionaryTexts.importCSV,
            exportAlertTitle: DictionaryTexts.exportCSV,
            exportFilename: { ImportExportHandler.exportFilename(prefix: "自訂詞庫") },
            okText: DictionaryTexts.ok,
            exportSuccessText: DictionaryTexts.exportSuccess,
            onFileImport: { handleImport($0) },
        )
        .alert(DictionaryTexts.deleteAll, isPresented: $showDeleteAllAlert) {
            Button(CommonTexts.cancel, role: .cancel) {}
            Button(DictionaryTexts.clear, role: .destructive) {
                Task { await viewModel.deleteAll() }
            }
        } message: {
            Text(DictionaryTexts.deleteAllMessage)
        }
        .task {
            await viewModel.load()
        }
    }

    // MARK: - Actions

    private func saveEntryFromAlert() {
        let trimmedRoman = romanInput.trimmingCharacters(in: .whitespacesAndNewlines)
        let trimmedHanzi = hanziInput.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedRoman.isEmpty, !trimmedHanzi.isEmpty else { return }

        let entry = if let existing = editingEntry {
            CustomDictionaryEntry(
                id: existing.id,
                roman: trimmedRoman,
                hanzi: trimmedHanzi,
                createdAt: existing.createdAt,
                updatedAt: Date(),
            )
        } else {
            CustomDictionaryEntry(roman: trimmedRoman, hanzi: trimmedHanzi)
        }
        editingEntry = nil
        Task { await viewModel.save(entry) }
    }

    private func handleImport(_ result: Result<[URL], Error>) {
        importExport.handleFileImport(
            result,
            importAction: { url in try await viewModel.importFile(url: url) },
            resultFormat: DictionaryTexts.importResultFormat,
            onComplete: { await viewModel.load() },
        )
    }
}
