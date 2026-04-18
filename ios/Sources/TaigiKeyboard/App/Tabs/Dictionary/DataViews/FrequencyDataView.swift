import SwiftUI
import UniformTypeIdentifiers

/// Frequency data sub-page
/// Shows top word frequency list with toggle, import/export, and clear option
struct FrequencyDataView: View {

    @State private var isFrequencyRecordingEnabled: Bool
    @State private var allData: [(word: String, count: Int)] = []
    @State private var isLoading = true
    @State private var filterText = ""
    @State private var showClearAlert = false

    @StateObject private var importExport = ImportExportHandler()

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
        _isFrequencyRecordingEnabled = State(initialValue: SharedSettings.shared.isFrequencyRecordingEnabled)
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
                            Text(Tab3Texts.isFrequencyRecordingEnabled)
                            SettingInfoButton(description: Tab3Texts.isFrequencyRecordingEnabledInfo)
                        }
                    }
                    .onChange(of: isFrequencyRecordingEnabled) { _, newValue in
                        settings.isFrequencyRecordingEnabled = newValue
                    }
                }

                // Import/Export
                Section {
                    Text(Tab3Texts.frequencyDescription)
                        .font(AppStyle.bodyFont)
                    Button {
                        importExport.performExport { try await exportCSV() }
                    } label: {
                        Label(
                            Tab3Texts.frequencyExportCSV,
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
                                Tab3Texts.frequencyImportCSV,
                                systemImage: "square.and.arrow.down",
                            )
                        }
                    }
                } header: {
                    Text(Tab3Texts.importExportTitle)
                        .font(AppStyle.sectionHeaderFont)
                }

                // Clear
                Section {
                    Button(role: .destructive) {
                        showClearAlert = true
                    } label: {
                        Text(Tab3Texts.clearAllFrequency)
                    }
                }

                // Privacy warning
                Section {
                    Text(Tab3Texts.frequencyPrivacyWarning)
                }

                // Data list
                Section {
                    if allData.isEmpty {
                        Text(Tab3Texts.noData)
                            .foregroundColor(.secondary)
                    } else if !filterText.isEmpty, filteredData.isEmpty {
                        Text(Tab3Texts.noResults)
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
                        Text(Tab3Texts.frequencyManagement)
                            .font(AppStyle.sectionHeaderFont)
                        SettingInfoButton(description: Tab3Texts.filterHint)
                    }
                }
            }
        }
        .safeAreaInset(edge: .bottom) {
            SearchBar(text: $filterText, placeholder: Tab3Texts.searchPlaceholder)
        }
        .navigationTitle(Tab3Texts.frequencyManagement)
        .navigationBarTitleDisplayMode(.large)
        .alert(Tab3Texts.clearAllFrequency, isPresented: $showClearAlert) {
            Button(CommonTexts.cancel, role: .cancel) {}
            Button(Tab3Texts.clear, role: .destructive) {
                clearData()
            }
        } message: {
            Text(Tab3Texts.clearFrequencyMessage)
        }
        .importExportModifiers(
            handler: importExport,
            importAlertTitle: Tab3Texts.frequencyImportCSV,
            exportAlertTitle: Tab3Texts.frequencyExportCSV,
            exportFilename: { ImportExportHandler.exportFilename(prefix: "詞頻紀錄") },
            okText: Tab3Texts.ok,
            exportSuccessText: Tab3Texts.exportSuccess,
            onFileImport: { handleImport($0) },
        )
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
        } catch {
            DebugLogger(category: "FrequencyDataView").error("Failed to delete frequency database: \(error)")
        }
    }

    // MARK: - Export/Import

    private func exportCSV() async throws -> String {
        let data = await UserFrequencyRepository.shared.topWordsAsync(limit: Int.max)
        var csv = ""
        for item in data {
            csv += "\(CSVDocument.escape(item.word)),\(item.count)\n"
        }
        return csv
    }

    private func handleImport(_ result: Result<[URL], Error>) {
        importExport.handleFileImport(
            result,
            importAction: { url in
                let accessing = url.startAccessingSecurityScopedResource()
                defer { if accessing { url.stopAccessingSecurityScopedResource() } }
                let data = try Data(contentsOf: url)
                guard let csvString = String(data: data, encoding: .utf8) else {
                    throw NSError(domain: "", code: -1, userInfo: [NSLocalizedDescriptionKey: "Cannot read file"])
                }
                let entries = parseFrequencyCSV(csvString)
                try await UserFrequencyRepository.shared.ensureInitialized()
                let imported = try await UserFrequencyRepository.shared.batchImportMerge(entries: entries)
                return (imported: imported, skipped: entries.count - imported)
            },
            resultFormat: Tab3Texts.importResultFormat,
            onComplete: { await loadData() },
        )
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
