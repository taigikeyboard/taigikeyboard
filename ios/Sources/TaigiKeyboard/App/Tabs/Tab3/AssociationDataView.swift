import SwiftUI
import UniformTypeIdentifiers

/// Association data sub-page
/// Shows word association list with toggle, import/export, and clear option
struct AssociationDataView: View {
    @StateObject private var languageManager = LanguageManager.shared

    @State private var isAssociationRecordingEnabled: Bool
    @State private var allData: [NextWordService.AssociationEntry] = []
    @State private var total = 0
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
        _isAssociationRecordingEnabled = State(initialValue: SharedSettings.shared.associationRecordingEnabled)
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
                            Text(languageManager.text(Tab3Texts.associationRecordingEnabled))
                            SettingInfoButton(description: languageManager.text(Tab3Texts.associationRecordingEnabledInfo))
                        }
                    }
                    .onChange(of: isAssociationRecordingEnabled) { _, newValue in
                        settings.associationRecordingEnabled = newValue
                    }
                }

                // Import/Export
                Section {
                    Text(languageManager.text(Tab3Texts.associationDescription))
                        .font(AppStyle.bodyFont)
                        .foregroundColor(.primary)
                    Button {
                        exportAssociationCSV()
                    } label: {
                        Label(
                            languageManager.text(Tab3Texts.associationExportCSV),
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
                                languageManager.text(Tab3Texts.associationImportCSV),
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
                        Text(languageManager.text(Tab3Texts.clearAllAssociation))
                    }
                }

                // Privacy warning
                Section {
                    Text(languageManager.text(Tab3Texts.associationPrivacyWarning))
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
                                        total = max(total - 1, 0)
                                    }
                                } label: {
                                    Image(systemName: "trash")
                                }
                            }
                        }
                    }
                } header: {
                    HStack {
                        Text(languageManager.text(Tab3Texts.associationManagement))
                            .font(AppStyle.sectionHeaderFont)
                        SettingInfoButton(description: languageManager.text(Tab3Texts.filterHint))
                    }
                }
            }
        }
        .safeAreaInset(edge: .bottom) {
            VStack(spacing: 0) {
                Divider()
                HStack {
                    Image(systemName: "magnifyingglass")
                        .foregroundStyle(.secondary)
                    TextField(
                        languageManager.text(Tab3Texts.searchPlaceholder),
                        text: $filterText,
                    )
                    .autocorrectionDisabled()
                    .textInputAutocapitalization(.never)
                    if !filterText.isEmpty {
                        Button {
                            filterText = ""
                        } label: {
                            Image(systemName: "xmark.circle.fill")
                                .foregroundStyle(.secondary)
                        }
                        .buttonStyle(.plain)
                    }
                }
                .padding(.horizontal, 12)
                .padding(.vertical, 8)
                .background(Color(.tertiarySystemFill))
                .clipShape(RoundedRectangle(cornerRadius: 12, style: .continuous))
                .padding(.horizontal, 16)
                .padding(.vertical, 8)
            }
            .background(Color(.systemBackground))
            .padding(.bottom, 8)
        }
        .navigationTitle(languageManager.text(Tab3Texts.associationManagement))
        .navigationBarTitleDisplayMode(.large)
        .alert(languageManager.text(Tab3Texts.clearAllAssociation), isPresented: $showClearAlert) {
            Button(languageManager.text(Tab3Texts.cancel), role: .cancel) {}
            Button(languageManager.text(Tab3Texts.clear), role: .destructive) {
                clearData()
            }
        } message: {
            Text(languageManager.text(Tab3Texts.clearAssociationMessage))
        }
        .fileImporter(
            isPresented: $showFileImporter,
            allowedContentTypes: [.commaSeparatedText, .plainText],
            allowsMultipleSelection: false,
        ) { result in
            handleAssociationImport(result)
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
        .alert(languageManager.text(Tab3Texts.associationImportCSV), isPresented: $showImportResultAlert) {
            Button(languageManager.text(Tab3Texts.ok)) {}
        } message: {
            Text(importResultMessage)
        }
        .alert(languageManager.text(Tab3Texts.associationExportCSV), isPresented: $showExportSuccessAlert) {
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
        let assoc = await NextWordService.shared.allAssociations()
        await MainActor.run {
            total = assoc.count
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
                total = 0
            }
        }
    }

    // MARK: - Export/Import

    private func exportFilename() -> String {
        let f = DateFormatter()
        f.dateFormat = "yyyy-MM-dd"
        return "詞關聯紀錄_\(f.string(from: Date())).csv"
    }

    private func exportAssociationCSV() {
        Task {
            let allData = await NextWordService.shared.allAssociations()
            var csv = ""
            for item in allData {
                csv += "\(csvEscape(item.prevWord)),\(csvEscape(item.prevTl)),\(csvEscape(item.nextWord)),\(csvEscape(item.nextTl)),\(item.count)\n"
            }
            await MainActor.run {
                csvDocument = CSVDocument(csv)
                showFileExporter = true
            }
        }
    }

    private func handleAssociationImport(_ result: Result<[URL], Error>) {
        switch result {
        case let .success(urls):
            guard let url = urls.first else { return }
            isImporting = true
            Task {
                defer { Task { @MainActor in isImporting = false } }
                do {
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
                    let skipped = entries.count - imported
                    await MainActor.run {
                        importResultMessage = String(
                            format: languageManager.text(Tab3Texts.associationImportResult),
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

    private func parseAssociationCSV(_ csv: String) -> [(prevWord: String, prevTl: String, nextWord: String, nextTl: String, count: Int)] {
        let lines = csv.components(separatedBy: .newlines)
        var entries: [(prevWord: String, prevTl: String, nextWord: String, nextTl: String, count: Int)] = []
        for line in lines {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty else { continue }
            let columns = parseCSVLine(trimmed)
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

    private func parseCSVLine(_ line: String) -> [String] {
        var fields: [String] = []
        var current = ""
        var inQuotes = false
        for char in line {
            if char == "\"" { inQuotes.toggle() }
            else if char == ",", !inQuotes { fields.append(current); current = "" }
            else { current.append(char) }
        }
        fields.append(current)
        return fields
    }

    private func csvEscape(_ field: String) -> String {
        if field.contains(",") || field.contains("\"") || field.contains("\n") {
            return "\"\(field.replacingOccurrences(of: "\"", with: "\"\""))\""
        }
        return field
    }
}

// MARK: - Identifiable Extension

extension NextWordService.AssociationEntry: Identifiable {
    public var id: String {
        "\(prevWord)\t\(prevTl)\t\(nextWord)\t\(nextTl)"
    }
}
