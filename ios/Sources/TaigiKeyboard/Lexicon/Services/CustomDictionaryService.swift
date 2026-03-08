import Foundation
import OSLog

/// Service for managing user custom dictionary
/// Handles CRUD, CSV export, and file import
final class CustomDictionaryService: @unchecked Sendable {

    // MARK: - Properties

    static let shared = CustomDictionaryService()

    private let repository: CustomDictionaryRepository
    private let logger = Logger(
        subsystem: LexiconConstants.Logging.subsystem,
        category: "CustomDictionaryService"
    )

    // MARK: - Initialization

    init(repository: CustomDictionaryRepository = .shared) {
        self.repository = repository
    }

    // MARK: - Default Entries

    private static let defaultEntries: [(id: String, roman: String, hanzi: String)] = [
        ("default-li-ho", "lí hó", "你好😀"),
        ("default-gau-tsa", "gâu-tsá", "𠢕早"),
    ]

    /// Seed default example entries if the dictionary is empty
    func seedDefaultEntryIfEmpty() async throws {
        let entries = try await repository.fetchAll()
        guard entries.isEmpty else { return }
        for entry in Self.defaultEntries {
            let defaultEntry = CustomDictionaryEntry(
                id: entry.id,
                roman: entry.roman,
                hanzi: entry.hanzi
            )
            try await repository.upsert(defaultEntry)
        }
    }

    // MARK: - CRUD

    func fetchAll() async throws -> [CustomDictionaryEntry] {
        try await repository.fetchAll()
    }

    func save(_ entry: CustomDictionaryEntry) async throws {
        try await repository.upsert(entry)
    }

    func delete(id: String) async throws {
        try await repository.delete(id: id)
    }

    func deleteAll() async throws {
        try await repository.deleteAll()
    }

    /// Search by roman prefix (for autocomplete)
    func search(romanPrefix: String, limit: Int = 50) async throws -> [CustomDictionaryEntry] {
        try await repository.search(romanPrefix: romanPrefix, limit: limit)
    }

    // MARK: - Export

    /// Export all entries as CSV string
    func exportCSV() async throws -> String {
        let entries = try await repository.fetchAll()
        var csv = "roman,hanzi\n"
        for entry in entries {
            let escapedRoman = csvEscape(entry.roman)
            let escapedHanzi = csvEscape(entry.hanzi)
            csv += "\(escapedRoman),\(escapedHanzi)\n"
        }
        return csv
    }

    // MARK: - File Import

    struct ImportResult {
        let imported: Int
        let skipped: Int
    }

    /// Import entries from a local CSV file
    func importFromFile(url: URL) async throws -> ImportResult {
        let accessing = url.startAccessingSecurityScopedResource()
        defer {
            if accessing { url.stopAccessingSecurityScopedResource() }
        }

        let data = try Data(contentsOf: url)
        guard let csvString = String(data: data, encoding: .utf8) else {
            throw CustomDictionaryError.invalidCSVData
        }

        let entries = parseCSV(csvString)

        // If the file has non-empty content lines but no valid entries, it's a format error
        let hasContentLines = csvString.components(separatedBy: .newlines)
            .contains { !$0.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }
        if entries.isEmpty && hasContentLines {
            throw CustomDictionaryError.invalidCSVFormat
        }

        guard !entries.isEmpty else {
            return ImportResult(imported: 0, skipped: 0)
        }

        let importedCount = try await repository.batchImport(entries)
        let skipped = entries.count - importedCount
        logger.info("[IMPORT] Imported \(importedCount), skipped \(skipped)")
        return ImportResult(imported: importedCount, skipped: skipped)
    }

    // MARK: - Private Helpers

    /// Parse CSV string into entries
    /// Expected format: column A = roman, column B = hanzi
    /// First row is treated as header if it contains "roman" (case-insensitive)
    func parseCSV(_ csv: String) -> [CustomDictionaryEntry] {
        let lines = csv.components(separatedBy: .newlines)
        var entries: [CustomDictionaryEntry] = []
        var startIndex = 0

        // Skip header row if present
        if let firstLine = lines.first?.lowercased(),
           firstLine.contains("roman") || firstLine.contains("hanzi") ||
           firstLine.contains("poj") || firstLine.contains("tl") ||
           firstLine.contains("羅馬字") || firstLine.contains("漢字") || firstLine.contains("中文") {
            startIndex = 1
        }

        for i in startIndex..<lines.count {
            let line = lines[i].trimmingCharacters(in: .whitespacesAndNewlines)
            guard !line.isEmpty else { continue }

            let columns = parseCSVLine(line)
            guard columns.count >= 2 else { continue }

            let roman = columns[0].trimmingCharacters(in: .whitespacesAndNewlines)
            let hanzi = columns[1].trimmingCharacters(in: .whitespacesAndNewlines)

            guard !roman.isEmpty, !hanzi.isEmpty else { continue }

            entries.append(CustomDictionaryEntry(roman: roman, hanzi: hanzi))
        }

        return entries
    }

    /// Parse a single CSV line handling quoted fields
    private func parseCSVLine(_ line: String) -> [String] {
        var fields: [String] = []
        var current = ""
        var inQuotes = false

        for char in line {
            if char == "\"" {
                inQuotes.toggle()
            } else if char == "," && !inQuotes {
                fields.append(current)
                current = ""
            } else {
                current.append(char)
            }
        }
        fields.append(current)

        return fields
    }

    /// Escape a field for CSV output
    private func csvEscape(_ field: String) -> String {
        if field.contains(",") || field.contains("\"") || field.contains("\n") {
            return "\"\(field.replacingOccurrences(of: "\"", with: "\"\""))\""
        }
        return field
    }

    // MARK: - Notone / Abbrev Generation

    /// Generate toneless form from romanization.
    /// Strips tone diacritics (via NFD), trailing digits, hyphens, and spaces.
    static func generateNotone(_ roman: String) -> String {
        let decomposed = roman.lowercased().decomposedStringWithCanonicalMapping
        var result = ""
        for scalar in decomposed.unicodeScalars {
            // Skip combining marks (Unicode category Mn)
            if scalar.properties.generalCategory == .nonspacingMark { continue }
            // Skip digits
            if scalar.value >= 0x30 && scalar.value <= 0x39 { continue }
            // Skip hyphens and spaces
            if scalar == "-" || scalar == " " { continue }
            result.unicodeScalars.append(scalar)
        }
        return result.precomposedStringWithCanonicalMapping
    }

    /// Generate abbreviation from romanization.
    /// Takes first letter of each syllable (split by - or space), removes diacritics.
    /// Returns empty string if fewer than 2 syllables.
    static func generateAbbrev(_ roman: String) -> String {
        let syllables = roman.lowercased()
            .components(separatedBy: CharacterSet(charactersIn: "- "))
            .filter { !$0.isEmpty }
        guard syllables.count >= 2 else { return "" }
        return syllables.map { syllable in
            let first = String(syllable.prefix(1))
            let decomposed = first.decomposedStringWithCanonicalMapping
            var bare = ""
            for scalar in decomposed.unicodeScalars {
                if scalar.properties.generalCategory != .nonspacingMark {
                    bare.unicodeScalars.append(scalar)
                }
            }
            return bare.precomposedStringWithCanonicalMapping
        }.joined()
    }
}

// MARK: - Custom Dictionary Errors

enum CustomDictionaryError: LocalizedError {
    case invalidCSVData
    case invalidCSVFormat

    var errorDescription: String? {
        switch self {
        case .invalidCSVData:
            "Invalid CSV data"
        case .invalidCSVFormat:
            LanguageManager.shared.text(Tab3Texts.invalidCSVFormat)
        }
    }
}
