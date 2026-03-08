import SwiftUI
import UniformTypeIdentifiers

/// Custom dictionary subpage
/// Lists all user-added entries with add/edit/delete and import/export
struct CustomDictionaryView: View {
    @StateObject private var languageManager = LanguageManager.shared

    @State private var entries: [CustomDictionaryEntry] = []
    @State private var isLoading = true
    @State private var showAddSheet = false
    @State private var editingEntry: CustomDictionaryEntry?
    @State private var showDeleteAllAlert = false

    // Import/Export
    @State private var showFileImporter = false
    @State private var showFileExporter = false
    @State private var csvDocument: CSVDocument?
    @State private var showImportResultAlert = false
    @State private var importResultMessage = ""
    @State private var showExportSuccessAlert = false
    @State private var showHelp = false
    @State private var showErrorAlert = false
    @State private var errorMessage = ""

    private let service = CustomDictionaryService.shared

    var body: some View {
        List {
            if isLoading {
                Section {
                    ProgressView()
                        .frame(maxWidth: .infinity)
                }
            } else {
                // Import/Export section
                Section {
                    Button {
                        showFileImporter = true
                    } label: {
                        Label(
                            languageManager.text(Tab3Texts.importCSV),
                            systemImage: "square.and.arrow.down"
                        )
                    }

                    Button {
                        exportCSV()
                    } label: {
                        Label(
                            languageManager.text(Tab3Texts.exportCSV),
                            systemImage: "square.and.arrow.up"
                        )
                    }
                } header: {
                    HStack {
                        Text(languageManager.text(Tab3Texts.importExportTitle))
                        Button {
                            showHelp = true
                        } label: {
                            Image(systemName: "questionmark.circle")
                        }
                    }
                }

                // Entry list section
                if entries.isEmpty {
                    Section {
                        VStack(spacing: 16) {
                            Image(systemName: "book.closed")
                                .font(KeyboardModels.Fonts.appFont(size: 48))
                                .foregroundColor(.secondary)
                            Text(languageManager.text(Tab3Texts.customDictEmpty))
                                .foregroundColor(.secondary)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(.vertical, 32)
                    }
                } else {
                    Section {
                        ForEach(entries) { entry in
                            Button {
                                editingEntry = entry
                            } label: {
                                HStack {
                                    Text(entry.roman)
                                        .font(KeyboardModels.Fonts.appFont(.body))
                                        .foregroundColor(.primary)
                                    Text(entry.hanzi)
                                        .font(KeyboardModels.Fonts.appFont(.body))
                                        .foregroundColor(.secondary)
                                    Spacer()
                                    Image(systemName: "chevron.right")
                                        .font(KeyboardModels.Fonts.appFont(.caption))
                                        .foregroundColor(.secondary)
                                }
                            }
                        }
                        .onDelete(perform: deleteEntries)
                    } header: {
                        Text("\(entries.count) \(languageManager.text(Tab3Texts.entriesCount))")
                    }

                    // Delete all section
                    Section {
                        Button(role: .destructive) {
                            showDeleteAllAlert = true
                        } label: {
                            Text(languageManager.text(Tab3Texts.deleteAll))
                        }
                    }
                }
            }
        }
        .navigationTitle(languageManager.text(Tab3Texts.customDictionary))
        .navigationBarTitleDisplayMode(.large)
        .toolbar {
            ToolbarItem(placement: .primaryAction) {
                Button {
                    showAddSheet = true
                } label: {
                    Image(systemName: "plus")
                }
            }
        }
        .sheet(isPresented: $showAddSheet) {
            CustomDictionaryEditView { newEntry in
                Task { await saveAndReload(newEntry) }
            }
        }
        .sheet(item: $editingEntry) { entry in
            CustomDictionaryEditView(entry: entry) { updatedEntry in
                Task { await saveAndReload(updatedEntry) }
            }
        }
        .fileImporter(
            isPresented: $showFileImporter,
            allowedContentTypes: [.commaSeparatedText, .plainText],
            allowsMultipleSelection: false
        ) { result in
            handleFileImport(result)
        }
        .fileExporter(
            isPresented: $showFileExporter,
            document: csvDocument,
            contentType: .commaSeparatedText,
            defaultFilename: "custom_dictionary.csv"
        ) { result in
            if case .success = result {
                showExportSuccessAlert = true
            }
        }
        .alert(languageManager.text(Tab3Texts.importExportHelpTitle), isPresented: $showHelp) {
            Button(languageManager.text(Tab3Texts.ok)) {}
        } message: {
            Text(languageManager.text(Tab3Texts.importExportHelp))
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

    private func deleteEntries(at offsets: IndexSet) {
        let idsToDelete = offsets.map { entries[$0].id }
        Task {
            for id in idsToDelete {
                try? await service.delete(id: id)
            }
            await loadEntries()
        }
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
        case .success(let urls):
            guard let url = urls.first else { return }
            Task {
                do {
                    let importResult = try await service.importFromFile(url: url)
                    await MainActor.run {
                        importResultMessage = String(
                            format: languageManager.text(Tab3Texts.importResult),
                            importResult.imported, importResult.skipped
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
        case .failure(let error):
            errorMessage = error.localizedDescription
            showErrorAlert = true
        }
    }
}
