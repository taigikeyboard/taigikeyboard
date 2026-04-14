import SwiftUI
import UniformTypeIdentifiers

/// Association data sub-page
/// Shows word association list with toggle, import/export, and clear option
struct AssociationDataView: View {

    @State private var isAssociationRecordingEnabled: Bool
    @State private var allData: [NextWordService.AssociationEntry] = []
    @State private var isLoading = true
    @State private var filterText = ""
    @State private var showClearAlert = false

    @StateObject private var importExport = ImportExportHandler()

    private let settings = SharedSettings.shared
    private let displayLimit = 100

    private var filteredData: [NextWordService.AssociationEntry] {
        if filterText.isEmpty {
            return Array(allData.prefix(displayLimit))
        }
        let query = filterText.lowercased()
        return allData.filter {
            $0.prevWord.lowercased().contains(query) ||
                $0.prevTl.lowercased().contains(query) ||
                $0.nextWord.lowercased().contains(query) ||
                $0.nextTl.lowercased().contains(query)
        }
    }

    init() {
        _isAssociationRecordingEnabled = State(initialValue: SharedSettings.shared.isAssociationRecordingEnabled)
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
                    Toggle(isOn: $isAssociationRecordingEnabled) {
                        HStack {
                            Text(Tab3Texts.isAssociationRecordingEnabled)
                            SettingInfoButton(description: Tab3Texts.isAssociationRecordingEnabledInfo)
                        }
                    }
                    .onChange(of: isAssociationRecordingEnabled) { _, newValue in
                        settings.isAssociationRecordingEnabled = newValue
                    }
                }

                // Import/Export
                Section {
                    Text(Tab3Texts.associationDescription)
                        .font(AppStyle.bodyFont)
                    Button {
                        importExport.performExport { try await exportCSV() }
                    } label: {
                        Label(
                            Tab3Texts.associationExportCSV,
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
                                Tab3Texts.associationImportCSV,
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
                        Text(Tab3Texts.clearAllAssociation)
                    }
                }

                // Privacy warning
                Section {
                    Text(Tab3Texts.associationPrivacyWarning)
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
                                    Task {
                                        await NextWordService.shared.deleteAssociation(item)
                                        allData.removeAll { $0.id == item.id }
                                    }
                                } label: {
                                    Image(systemName: "trash")
                                }
                            }
                        }
                    }
                } header: {
                    HStack {
                        Text(Tab3Texts.associationManagement)
                            .font(AppStyle.sectionHeaderFont)
                        SettingInfoButton(description: Tab3Texts.filterHint)
                    }
                }
            }
        }
        .safeAreaInset(edge: .bottom) {
            SearchBar(text: $filterText, placeholder: Tab3Texts.searchPlaceholder)
        }
        .navigationTitle(Tab3Texts.associationManagement)
        .navigationBarTitleDisplayMode(.large)
        .alert(Tab3Texts.clearAllAssociation, isPresented: $showClearAlert) {
            Button(CommonTexts.cancel, role: .cancel) {}
            Button(Tab3Texts.clear, role: .destructive) {
                clearData()
            }
        } message: {
            Text(Tab3Texts.clearAssociationMessage)
        }
        .importExportModifiers(
            handler: importExport,
            importAlertTitle: Tab3Texts.associationImportCSV,
            exportAlertTitle: Tab3Texts.associationExportCSV,
            exportFilename: { ImportExportHandler.exportFilename(prefix: "詞關聯紀錄") },
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
        let assoc = await NextWordService.shared.allAssociations()
        await MainActor.run {
            allData = assoc
            isLoading = false
        }
    }

    private func associationDisplayText(_ item: NextWordService.AssociationEntry) -> String {
        let prev = item.prevTl.isEmpty ? item.prevWord : "(\(item.prevTl), \(item.prevWord))"
        let next = item.nextTl.isEmpty ? item.nextWord : "(\(item.nextTl), \(item.nextWord))"
        return "\(prev) → \(next)"
    }

    private func clearData() {
        Task {
            await NextWordService.shared.clearAllAssociations()
            await MainActor.run {
                allData = []
            }
        }
    }

    // MARK: - Export/Import

    private func exportCSV() async throws -> String {
        let data = await NextWordService.shared.allAssociations()
        var csv = ""
        for item in data {
            csv += "\(CSVDocument.escape(item.prevWord)),\(CSVDocument.escape(item.prevTl)),\(CSVDocument.escape(item.nextWord)),\(CSVDocument.escape(item.nextTl)),\(item.count)\n"
        }
        return csv
    }

    private func handleImport(_ result: Result<[URL], Error>) {
        importExport.handleFileImport(
            result,
            importAction: { url in
                let accessing = url.startAccessingSecurityScopedResource()
                defer { if accessing { url.stopAccessingSecurityScopedResource() } }
                let fileData = try Data(contentsOf: url)
                guard let csvString = String(data: fileData, encoding: .utf8) else {
                    throw NSError(domain: "", code: -1, userInfo: [NSLocalizedDescriptionKey: "Cannot read file"])
                }
                let entries = parseAssociationCSV(csvString)
                let imported = try await NextWordService.shared.batchImportAssociations(entries: entries.map {
                    (prevWord: $0.prevWord, prevTl: $0.prevTl, nextWord: $0.nextWord, nextTl: $0.nextTl, count: $0.count)
                })
                return (imported: imported, skipped: entries.count - imported)
            },
            resultFormat: Tab3Texts.importResultFormat,
            onComplete: { await loadData() },
        )
    }

    // MARK: - CSV Helpers

    private func parseAssociationCSV(_ csv: String) -> [(prevWord: String, prevTl: String, nextWord: String, nextTl: String, count: Int)] {
        let lines = csv.components(separatedBy: .newlines)
        var entries: [(prevWord: String, prevTl: String, nextWord: String, nextTl: String, count: Int)] = []
        for line in lines {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty else { continue }
            let columns = CSVDocument.parseLine(trimmed)
            guard columns.count >= 5 else { continue }
            let prevWord = columns[0].trimmingCharacters(in: .whitespacesAndNewlines)
            let prevTl = columns[1].trimmingCharacters(in: .whitespacesAndNewlines)
            let nextWord = columns[2].trimmingCharacters(in: .whitespacesAndNewlines)
            let nextTl = columns[3].trimmingCharacters(in: .whitespacesAndNewlines)
            guard let count = Int(columns[4].trimmingCharacters(in: .whitespacesAndNewlines)),
                  count > 0, !nextWord.isEmpty else { continue }
            entries.append((prevWord: prevWord, prevTl: prevTl, nextWord: nextWord, nextTl: nextTl, count: count))
        }
        return entries
    }
}

// MARK: - Identifiable Extension

extension NextWordService.AssociationEntry: Identifiable {
    public var id: String {
        "\(prevWord)\t\(prevTl)\t\(nextWord)\t\(nextTl)"
    }
}
