import SwiftUI
import UniformTypeIdentifiers

/// Custom dictionary subpage
/// Lists all user-added entries with add/edit/delete and import/export
struct CustomDictionaryView: View {
    @State private var isCustomDictEnabled: Bool

    @State private var entries: [CustomDictionaryEntry] = []
    @State private var isLoading = true
    @State private var filterText = ""
    @State private var showEntryAlert = false
    @State private var editingEntry: CustomDictionaryEntry?
    @State private var romanInput = ""
    @State private var hanziInput = ""
    @State private var showDeleteAllAlert = false

    @StateObject private var importExport = ImportExportHandler()

    private let settings = SharedSettings.shared
    private let service = CustomDictionaryService.shared

    private var filteredEntries: [CustomDictionaryEntry] {
        if filterText.isEmpty {
            return Array(entries.prefix(100))
        }
        let query = filterText.lowercased()
        return entries.filter {
            $0.roman.lowercased().contains(query) ||
                $0.hanzi.lowercased().contains(query)
        }
    }

    init() {
        _isCustomDictEnabled = State(initialValue: SharedSettings.shared.isCustomDictEnabled)
    }

    var body: some View {
        List {
            if isLoading {
                Section {
                    ProgressView()
                        .frame(maxWidth: .infinity)
                }
            } else {
                // Enable/Disable toggle
                Section {
                    Toggle(isOn: $isCustomDictEnabled) {
                        HStack {
                            Text(Tab3Texts.isCustomDictEnabled)
                            SettingInfoButton(description: Tab3Texts.isCustomDictEnabledInfo)
                        }
                    }
                    .onChange(of: isCustomDictEnabled) { _, newValue in
                        settings.isCustomDictEnabled = newValue
                    }
                }

                // Import/Export
                Section {
                    Image("csv_example")
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .clipShape(RoundedRectangle(cornerRadius: AppStyle.smallCornerRadius))
                        .listRowSeparator(.hidden)
                    Text(Tab3Texts.customDictDescription)
                        .font(AppStyle.bodyFont)
                    Button {
                        importExport.performExport { try await service.exportCSV() }
                    } label: {
                        Label(
                            Tab3Texts.exportCSV,
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
                                Tab3Texts.importCSV,
                                systemImage: "square.and.arrow.down",
                            )
                        }
                    }
                } header: {
                    Text(Tab3Texts.importExportTitle)
                        .font(AppStyle.sectionHeaderFont)
                }

                // Delete all
                Section {
                    Button(role: .destructive) {
                        showDeleteAllAlert = true
                    } label: {
                        Text(Tab3Texts.deleteAll)
                    }
                    .disabled(importExport.isImporting)
                }

                // Privacy warning
                Section {
                    Text(Tab3Texts.customDictPrivacyWarning)
                }

                // Entry list
                Section {
                    if entries.isEmpty {
                        VStack(spacing: 16) {
                            Image(systemName: "book.closed")
                                .font(AppStyle.appFont(size: 48))
                                .foregroundColor(.secondary)
                            Text(Tab3Texts.customDictEmpty)
                                .foregroundColor(.secondary)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 32)
                    } else if !filterText.isEmpty, filteredEntries.isEmpty {
                        Text(Tab3Texts.noResults)
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
                                    Image(systemName: "chevron.right")
                                        .font(AppStyle.captionFont)
                                        .foregroundColor(.secondary)
                                }
                            }
                            .swipeActions(edge: .trailing) {
                                Button(role: .destructive) {
                                    Task {
                                        try? await service.delete(id: entry.id)
                                        await loadEntries()
                                    }
                                } label: {
                                    Image(systemName: "trash")
                                }
                            }
                        }
                    }
                } header: {
                    HStack {
                        Text(Tab3Texts.customDictionary)
                            .font(AppStyle.sectionHeaderFont)
                        SettingInfoButton(description: Tab3Texts.filterHint)
                    }
                }
            }
        }
        .safeAreaInset(edge: .bottom) {
            SearchBar(text: $filterText, placeholder: Tab3Texts.searchPlaceholder)
        }
        .navigationTitle(Tab3Texts.customDictionary)
        .navigationBarTitleDisplayMode(.large)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button {
                    editingEntry = nil
                    romanInput = ""
                    hanziInput = ""
                    showEntryAlert = true
                } label: {
                    Image(systemName: "plus")
                }
            }
        }
        .alert(
            editingEntry != nil ? Tab3Texts.editEntry : Tab3Texts.addEntry,
            isPresented: $showEntryAlert,
        ) {
            TextField(Tab3Texts.romanPlaceholder, text: $romanInput)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.never)
            TextField(Tab3Texts.hanziPlaceholder, text: $hanziInput)
            Button(CommonTexts.cancel, role: .cancel) {
                editingEntry = nil
            }
            Button(Tab3Texts.save) {
                saveEntryFromAlert()
            }
        }
        .importExportModifiers(
            handler: importExport,
            importAlertTitle: Tab3Texts.importCSV,
            exportAlertTitle: Tab3Texts.exportCSV,
            exportFilename: { ImportExportHandler.exportFilename(prefix: "自訂詞庫") },
            okText: Tab3Texts.ok,
            exportSuccessText: Tab3Texts.exportSuccess,
            onFileImport: { handleImport($0) },
        )
        .alert(Tab3Texts.deleteAll, isPresented: $showDeleteAllAlert) {
            Button(CommonTexts.cancel, role: .cancel) {}
            Button(Tab3Texts.clear, role: .destructive) {
                Task {
                    try? await service.deleteAll()
                    await loadEntries()
                }
            }
        } message: {
            Text(Tab3Texts.deleteAllMessage)
        }
        .task {
            await loadEntries()
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
        Task { await saveAndReload(entry) }
    }

    private func loadEntries() async {
        do {
            entries = try await service.fetchAll()
        } catch {
            entries = []
        }
        isLoading = false
    }

    private func saveAndReload(_ entry: CustomDictionaryEntry) async {
        try? await service.save(entry)
        await loadEntries()
    }

    private func handleImport(_ result: Result<[URL], Error>) {
        importExport.handleFileImport(
            result,
            importAction: { url in
                let result = try await service.importFromFile(url: url)
                return (imported: result.imported, skipped: result.skipped)
            },
            resultFormat: Tab3Texts.importResultFormat,
            onComplete: { await loadEntries() },
        )
    }
}
