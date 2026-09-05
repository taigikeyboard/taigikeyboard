import SwiftUI
import UniformTypeIdentifiers

/// Custom dictionary subpage
/// Lists all user-added entries with add/edit/delete and import/export
struct CustomDictionaryView: View {
    @Environment(DisplayLanguageStore.self) private var lang
    @StateObject private var viewModel = CustomDictionaryViewModel()
    @StateObject private var importExport = ImportExportHandler()

    @State private var filterText = ""
    @State private var showEntryAlert = false
    @State private var editingEntry: CustomDictionaryEntry?
    @State private var romanInput = ""
    @State private var hanziInput = ""
    @State private var showDeleteAllAlert = false

    // Case-insensitive substring match on roman/hanzi; with no query, cap at 100 rows so the list stays smooth.
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
                            Text(lang.string(.dictionaryCustomDictEnabled))
                            SettingInfoButton(description: lang.string(.dictionaryCustomDictEnabledInfo))
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
                    Text(lang.string(.dictionaryCustomDictDescription))
                        .font(AppStyle.bodyFont)
                    Button {
                        importExport.performExport { try await viewModel.exportCSV() }
                    } label: {
                        Label(
                            lang.string(.dictionaryExportCSV),
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
                                lang.string(.dictionaryImportCSV),
                                systemImage: "square.and.arrow.down",
                            )
                        }
                    }
                } header: {
                    Text(lang.string(.dictionaryImportExportTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // Delete all
                Section {
                    Button(role: .destructive) {
                        showDeleteAllAlert = true
                    } label: {
                        Text(lang.string(.dictionaryDeleteAll))
                    }
                    .disabled(importExport.isImporting)
                }

                // Privacy warning
                Section {
                    Text(lang.string(.dictionaryCustomDictPrivacyWarning))
                }

                // Entry list
                Section {
                    if viewModel.entries.isEmpty {
                        VStack(spacing: 16) {
                            Image(latinSystemName: "book.closed")
                                .font(AppStyle.appFont(size: 48))
                                .foregroundColor(.secondary)
                            Text(lang.string(.dictionaryCustomDictEmpty))
                                .foregroundColor(.secondary)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 32)
                    } else if !filterText.isEmpty, filteredEntries.isEmpty {
                        Text(lang.string(.dictionaryNoResults))
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
                        Text(lang.string(.dictionaryCustomDictionary))
                            .font(AppStyle.sectionHeaderFont)
                        SettingInfoButton(description: lang.string(.dictionaryFilterHint))
                    }
                }
            }
        }
        .safeAreaInset(edge: .bottom) {
            SearchBar(text: $filterText, placeholder: lang.string(.dictionarySearchPlaceholder))
        }
        .navigationTitle(lang.string(.dictionaryCustomDictionary))
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
            editingEntry != nil ? lang.string(.dictionaryEditEntry) : lang.string(.dictionaryAddEntry),
            isPresented: $showEntryAlert,
        ) {
            TextField(lang.string(.dictionaryRomanPlaceholder), text: $romanInput)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.never)
            TextField(lang.string(.dictionaryHanziPlaceholder), text: $hanziInput)
            Button(lang.string(.commonCancel), role: .cancel) {
                editingEntry = nil
            }
            Button(lang.string(.dictionarySave)) {
                saveEntryFromAlert()
            }
        }
        .importExportModifiers(
            handler: importExport,
            importAlertTitle: lang.string(.dictionaryImportCSV),
            exportAlertTitle: lang.string(.dictionaryExportCSV),
            errorTitle: lang.string(.commonError),
            exportFilename: { ImportExportHandler.exportFilename(prefix: "taigi_custom_dictionary") },
            okText: lang.string(.commonOk),
            exportSuccessText: lang.string(.dictionaryExportSuccess),
            onFileImport: { handleImport($0) },
        )
        .alert(lang.string(.dictionaryDeleteAll), isPresented: $showDeleteAllAlert) {
            Button(lang.string(.commonCancel), role: .cancel) {}
            Button(lang.string(.dictionaryClear), role: .destructive) {
                Task { await viewModel.deleteAll() }
            }
        } message: {
            Text(lang.string(.dictionaryDeleteAllMessage))
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
            formatResult: { lang.resolver.dictionaryImportResult(imported: $0, skipped: $1) },
            formatError: localizedImportMessage,
            onComplete: { await viewModel.load() },
        )
    }

    // Maps the engine's typed import error to a localized message at the display boundary — keeps the
    // `Lexicon/` service unaware of `StringKey` (presentation layer). Mirrors Android's
    // `CustomDictionaryScreen` error → `StringKey` resolution.
    private func localizedImportMessage(_ error: Error) -> String {
        guard let key = (error as? CustomDictionaryError)?.messageKey else {
            return error.localizedDescription
        }
        return lang.string(key)
    }
}

private extension CustomDictionaryError {
    var messageKey: StringKey? {
        switch self {
        case .invalidCSVFormat: .dictionaryInvalidCSVFormat
        case .fileTooLarge: .dictionaryFileTooLarge
        case .tooManyEntries: .dictionaryTooManyEntries
        case .invalidCSVData: nil
        }
    }
}
