import SwiftUI
import UniformTypeIdentifiers

/// Frequency data sub-page
/// Shows top word frequency list with toggle, import/export, and clear option
struct FrequencyDataView: View {
    @StateObject private var languageManager = LanguageManager.shared

    @State private var isFrequencyRecordingEnabled: Bool
    @State private var allData: [(word: String, count: Int)] = []
    @State private var isLoading = true
    @State private var filterText = ""
    @State private var showClearAlert = false

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
    private let displayLimit = 100

    private var filteredData: [(word: String, count: Int)] {
        if filterText.isEmpty {
            return Array(allData.prefix(displayLimit))
        }
        let query = filterText.lowercased()
        return allData.filter { $0.word.lowercased().contains(query) }
    }

    init() {
        _isFrequencyRecordingEnabled = State(initialValue: SharedSettings.shared.frequencyRecordingEnabled)
    }

    var body: some View {
        List {
            if isLoading {
                Section {
                    ProgressView()
                        .frame(maxWidth: .infinity)
                }
            } else {
                // Toggle
                Section {
                    Toggle(isOn: $isFrequencyRecordingEnabled) {
                        HStack {
                            Text(languageManager.text(Tab3Texts.frequencyRecordingEnabled))
                            SettingInfoButton(description: languageManager.text(Tab3Texts.frequencyRecordingEnabledInfo))
                        }
                    }
                    .onChange(of: isFrequencyRecordingEnabled) { _, newValue in
                        settings.frequencyRecordingEnabled = newValue
                    }
                }

                // Import/Export
                Section {
                    Text(languageManager.text(Tab3Texts.frequencyDescription))
                        .font(AppStyle.bodyFont)
                        .foregroundColor(.primary)
                    Button {
                        exportFrequencyCSV()
                    } label: {
                        Label(
                            languageManager.text(Tab3Texts.frequencyExportCSV),
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
                                languageManager.text(Tab3Texts.frequencyImportCSV),
                                systemImage: "square.and.arrow.down",
                            )
                        }
                    }
                } header: {
                    Text(languageManager.text(Tab3Texts.importExportTitle))
                        .font(AppStyle.sectionHeaderFont)
                }

                // Clear
                Section {
                    Button(role: .destructive) {
                        showClearAlert = true
                    } label: {
                        Text(languageManager.text(Tab3Texts.clearAllFrequency))
                    }
                }

                // Privacy warning
                Section {
                    Text(languageManager.text(Tab3Texts.frequencyPrivacyWarning))
                }

                // Data list
                Section {
                    if allData.isEmpty {
                        Text(languageManager.text(Tab3Texts.noData))
                            .foregroundColor(.secondary)
                    } else if !filterText.isEmpty, filteredData.isEmpty {
                        Text(languageManager.text(Tab3Texts.noResults))
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
                                    Task {
                                        try? await UserFrequencyRepository.shared.deleteWord(item.word)
                                        allData.removeAll { $0.word == item.word }
                                    }
                                } label: {
                                    Image(systemName: "trash")
                                }
                            }
                        }
                    }
                } header: {
                    HStack {
                        Text(languageManager.text(Tab3Texts.frequencyManagement))
                            .font(AppStyle.sectionHeaderFont)
                        SettingInfoButton(description: languageManager.text(Tab3Texts.filterHint))
                    }
                }
            }
        }
        .safeAreaInset(edge: .bottom) {
            SearchBar(text: $filterText, placeholder: languageManager.text(Tab3Texts.searchPlaceholder))
        }
        .navigationTitle(languageManager.text(Tab3Texts.frequencyManagement))
        .navigationBarTitleDisplayMode(.large)
        .alert(languageManager.text(Tab3Texts.clearAllFrequency), isPresented: $showClearAlert) {
            Button(languageManager.text(Tab3Texts.cancel), role: .cancel) {}
            Button(languageManager.text(Tab3Texts.clear), role: .destructive) {
                clearData()
            }
        } message: {
            Text(languageManager.text(Tab3Texts.clearFrequencyMessage))
        }
        .fileImporter(
            isPresented: $showFileImporter,
            allowedContentTypes: [.commaSeparatedText, .plainText],
            allowsMultipleSelection: false,
        ) { result in
            handleFrequencyImport(result)
        }
        .fileExporter(
            isPresented: $showFileExporter,
            document: csvDocument,
            contentType: .commaSeparatedText,
            defaultFilename: exportFilename(),
        ) { result in
            if case .success = result {
                showExportSuccessAlert = true
            }
        }
        .alert(languageManager.text(Tab3Texts.frequencyImportCSV), isPresented: $showImportResultAlert) {
            Button(languageManager.text(Tab3Texts.ok)) {}
        } message: {
            Text(importResultMessage)
        }
        .alert(languageManager.text(Tab3Texts.frequencyExportCSV), isPresented: $showExportSuccessAlert) {
            Button(languageManager.text(Tab3Texts.ok)) {}
        } message: {
            Text(languageManager.text(Tab3Texts.exportSuccess))
        }
        .alert("Error", isPresented: $showErrorAlert) {
            Button(languageManager.text(Tab3Texts.ok)) {}
        } message: {
            Text(errorMessage)
        }
        .task {
            await loadData()
        }
    }

    // MARK: - Data

    private func loadData() async {
        let freq = await UserFrequencyRepository.shared.topWordsAsync(limit: Int.max)
        await MainActor.run {
            allData = freq
            isLoading = false
        }
    }

    private func clearData() {
        do {
            try UserFrequencyService.deleteUserDatabase()
            allData = []
        } catch {}
    }

    // MARK: - Export/Import

    private func exportFilename() -> String {
        let f = DateFormatter()
        f.dateFormat = "yyyy-MM-dd"
        return "詞頻紀錄_\(f.string(from: Date())).csv"
    }

    private func exportFrequencyCSV() {
        Task {
            let allData = await UserFrequencyRepository.shared.topWordsAsync(limit: Int.max)
            var csv = ""
            for item in allData {
                csv += "\(CSVDocument.escape(item.word)),\(item.count)\n"
            }
            await MainActor.run {
                csvDocument = CSVDocument(csv)
                showFileExporter = true
            }
        }
    }

    private func handleFrequencyImport(_ result: Result<[URL], Error>) {
        switch result {
        case let .success(urls):
            guard let url = urls.first else { return }
            isImporting = true
            Task {
                defer { Task { @MainActor in isImporting = false } }
                do {
                    let accessing = url.startAccessingSecurityScopedResource()
                    defer { if accessing { url.stopAccessingSecurityScopedResource() } }
                    let data = try Data(contentsOf: url)
                    guard let csvString = String(data: data, encoding: .utf8) else {
                        throw NSError(domain: "", code: -1, userInfo: [NSLocalizedDescriptionKey: "Cannot read file"])
                    }
                    let entries = parseFrequencyCSV(csvString)
                    try await UserFrequencyRepository.shared.ensureInitialized()
                    let imported = try await UserFrequencyRepository.shared.batchImportMerge(entries: entries)
                    let skipped = entries.count - imported
                    await MainActor.run {
                        importResultMessage = String(
                            format: languageManager.text(Tab3Texts.frequencyImportResult),
                            imported, skipped,
                        )
                        showImportResultAlert = true
                    }
                    await loadData()
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

    // MARK: - CSV Helpers

    private func parseFrequencyCSV(_ csv: String) -> [(word: String, count: Int)] {
        let lines = csv.components(separatedBy: .newlines)
        var entries: [(word: String, count: Int)] = []
        for line in lines {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty else { continue }
            let columns = CSVDocument.parseLine(trimmed)
            guard columns.count >= 2 else { continue }
            let word = columns[0].trimmingCharacters(in: .whitespacesAndNewlines)
            guard !word.isEmpty,
                  let count = Int(columns[1].trimmingCharacters(in: .whitespacesAndNewlines)),
                  count > 0 else { continue }
            entries.append((word: word, count: count))
        }
        return entries
    }
}
