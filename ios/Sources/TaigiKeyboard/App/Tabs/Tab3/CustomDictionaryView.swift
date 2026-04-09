import SwiftUI
import UniformTypeIdentifiers

/// Custom dictionary subpage
/// Lists all user-added entries with add/edit/delete and import/export
struct CustomDictionaryView: View {
    @StateObject private var languageManager = LanguageManager.shared
    @State private var isCustomDictEnabled: Bool

    @State private var entries: [CustomDictionaryEntry] = []
    @State private var isLoading = true
    @State private var filterText = ""
    @State private var showEntryAlert = false
    @State private var editingEntry: CustomDictionaryEntry?
    @State private var romanInput = ""
    @State private var hanziInput = ""
    @State private var showDeleteAllAlert = false

    // Import/Export
    @State private var isImporting = false
    @State private var showFileImporter = false
    @State private var showFileExporter = false
    @State private var csvDocument: CSVDocument?
    @State private var showImportResultAlert = false
    @State private var importResultMessage = ""
    @State private var showExportSuccessAlert = false
    @State private var showErrorAlert = false
    @State private var errorMessage = ""

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
        _isCustomDictEnabled = State(initialValue: SharedSettings.shared.customDictEnabled)
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
                            Text(languageManager.text(Tab3Texts.customDictEnabled))
                            SettingInfoButton(description: languageManager.text(Tab3Texts.customDictEnabledInfo))
                        }
                    }
                    .onChange(of: isCustomDictEnabled) { _, newValue in
                        settings.customDictEnabled = newValue
                    }
                }

                // Import/Export
                Section {
                    Image("csv_example")
                        .resizable()
                        .aspectRatio(contentMode: .fit)
                        .clipShape(RoundedRectangle(cornerRadius: AppStyle.smallCornerRadius))
                        .listRowSeparator(.hidden)
                    Text(languageManager.text(Tab3Texts.customDictDescription))
                        .font(AppStyle.bodyFont)
                    Button {
                        exportCSV()
                    } label: {
                        Label(
                            languageManager.text(Tab3Texts.exportCSV),
                            systemImage: "square.and.arrow.up",
                        )
                    }
                    .disabled(isImporting)
                    if isImporting {
                        ProgressView()
                            .frame(maxWidth: .infinity)
                    } else {
                        Button {
                            showFileImporter = true
                        } label: {
                            Label(
                                languageManager.text(Tab3Texts.importCSV),
                                systemImage: "square.and.arrow.down",
                            )
                        }
                    }
                } header: {
                    Text(languageManager.text(Tab3Texts.importExportTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // Delete all
                Section {
                    Button(role: .destructive) {
                        showDeleteAllAlert = true
                    } label: {
                        Text(languageManager.text(Tab3Texts.deleteAll))
                    }
                    .disabled(isImporting)
                }

                // Privacy warning
                Section {
                    Text(languageManager.text(Tab3Texts.customDictPrivacyWarning))
                }

                // Entry list
                Section {
                    if entries.isEmpty {
                        VStack(spacing: 16) {
                            Image(systemName: "book.closed")
                                .font(AppStyle.appFont(size: 48))
                                .foregroundColor(.secondary)
                            Text(languageManager.text(Tab3Texts.customDictEmpty))
                                .foregroundColor(.secondary)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 32)
                    } else if !filterText.isEmpty, filteredEntries.isEmpty {
                        Text(languageManager.text(Tab3Texts.noResults))
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
                        Text(languageManager.text(Tab3Texts.customDictionary))
                            .font(AppStyle.sectionHeaderFont)
                        SettingInfoButton(description: languageManager.text(Tab3Texts.filterHint))
                    }
                }
            }
        }
        .safeAreaInset(edge: .bottom) {
            SearchBar(text: $filterText, placeholder: languageManager.text(Tab3Texts.searchPlaceholder))
        }
        .navigationTitle(languageManager.text(Tab3Texts.customDictionary))
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
            languageManager.text(editingEntry != nil ? Tab3Texts.editEntry : Tab3Texts.addEntry),
            isPresented: $showEntryAlert,
        ) {
            TextField(languageManager.text(Tab3Texts.romanPlaceholder), text: $romanInput)
                .autocorrectionDisabled()
                .textInputAutocapitalization(.never)
            TextField(languageManager.text(Tab3Texts.hanziPlaceholder), text: $hanziInput)
            Button(languageManager.text(Tab3Texts.cancel), role: .cancel) {
                editingEntry = nil
            }
            Button(languageManager.text(Tab3Texts.save)) {
                saveEntryFromAlert()
            }
        }
        .fileImporter(
            isPresented: $showFileImporter,
            allowedContentTypes: [.commaSeparatedText, .plainText],
            allowsMultipleSelection: false,
        ) { result in
            handleFileImport(result)
        }
        .fileExporter(
            isPresented: $showFileExporter,
            document: csvDocument,
            contentType: .commaSeparatedText,
            defaultFilename: customDictExportFilename(),
        ) { result in
            if case .success = result {
                showExportSuccessAlert = true
            }
        }
        .alert(languageManager.text(Tab3Texts.importCSV), isPresented: $showImportResultAlert) {
            Button(languageManager.text(Tab3Texts.ok)) {}
        } message: {
            Text(importResultMessage)
        }
        .alert(languageManager.text(Tab3Texts.exportCSV), isPresented: $showExportSuccessAlert) {
            Button(languageManager.text(Tab3Texts.ok)) {}
        } message: {
            Text(languageManager.text(Tab3Texts.exportSuccess))
        }
        .alert("Error", isPresented: $showErrorAlert) {
            Button(languageManager.text(Tab3Texts.ok)) {}
        } message: {
            Text(errorMessage)
        }
        .alert(languageManager.text(Tab3Texts.deleteAll), isPresented: $showDeleteAllAlert) {
            Button(languageManager.text(Tab3Texts.cancel), role: .cancel) {}
            Button(languageManager.text(Tab3Texts.clear), role: .destructive) {
                Task {
                    try? await service.deleteAll()
                    await loadEntries()
                }
            }
        } message: {
            Text(languageManager.text(Tab3Texts.deleteAllMessage))
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

    private func customDictExportFilename() -> String {
        let f = DateFormatter()
        f.dateFormat = "yyyy-MM-dd"
        return "自訂詞庫_\(f.string(from: Date())).csv"
    }

    private func exportCSV() {
        Task {
            do {
                let csv = try await service.exportCSV()
                await MainActor.run {
                    csvDocument = CSVDocument(csv)
                    showFileExporter = true
                }
            } catch {
                await MainActor.run {
                    errorMessage = error.localizedDescription
                    showErrorAlert = true
                }
            }
        }
    }

    private func handleFileImport(_ result: Result<[URL], Error>) {
        switch result {
        case let .success(urls):
            guard let url = urls.first else { return }
            isImporting = true
            Task {
                defer {
                    Task { @MainActor in isImporting = false }
                }
                do {
                    let importResult = try await service.importFromFile(url: url)
                    await MainActor.run {
                        importResultMessage = String(
                            format: languageManager.text(Tab3Texts.importResult),
                            importResult.imported, importResult.skipped,
                        )
                        showImportResultAlert = true
                    }
                    await loadEntries()
                } catch {
                    await MainActor.run {
                        errorMessage = error.localizedDescription
                        showErrorAlert = true
                    }
                }
            }
        case let .failure(error):
            errorMessage = error.localizedDescription
            showErrorAlert = true
        }
    }
}
