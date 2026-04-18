import Foundation

/// Service for managing user custom dictionary
/// Handles CRUD, CSV export, and file import
final class CustomDictionaryService: @unchecked Sendable {
    // MARK: - Properties

    static let shared = CustomDictionaryService()

    private let repository: CustomDictionaryRepository
    private let logger = DebugLogger(category: "CustomDictionaryService")

    // MARK: - Initialization

    init(repository: CustomDictionaryRepository = .shared) {
        self.repository = repository
    }

    // MARK: - Default Entries

    private static let defaultEntries: [(id: String, roman: String, hanzi: String)] = [
        ("default-gau-tsa", "gâu-tsá", "𠢕早"),
        ("default-tsiah-pa-bue", "tsia̍h-pá--buē", "食飽未"),
    ]

    /// Seed default example entries if the dictionary is empty
    func seedDefaultEntryIfEmpty() async throws {
        let entries = try await repository.fetchAll()
        guard entries.isEmpty else { return }
        for entry in Self.defaultEntries {
            let defaultEntry = CustomDictionaryEntry(
                id: entry.id,
                roman: entry.roman,
                hanzi: entry.hanzi,
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

    /// Search by prefix (for autocomplete)
    func search(prefix: String, isToneAware: Bool, limit: Int = 50) async throws -> [CustomDictionaryEntry] {
        try await repository.search(prefix: prefix, isToneAware: isToneAware, limit: limit)
    }

    // MARK: - Export

    /// Export all entries as CSV string
    func exportCSV() async throws -> String {
        let entries = try await repository.fetchAll()
        var csv = ""
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

    private static let maxFileSize = 5 * 1024 * 1024 // 5 MB
    private static let maxEntryCount = 30000

    /// Import entries from a local CSV file
    func importFromFile(url: URL) async throws -> ImportResult {
        let accessing = url.startAccessingSecurityScopedResource()
        defer {
            if accessing { url.stopAccessingSecurityScopedResource() }
        }

        // Pre-validate file size
        let resourceValues = try url.resourceValues(forKeys: [.fileSizeKey])
        if let fileSize = resourceValues.fileSize, fileSize > Self.maxFileSize {
            throw CustomDictionaryError.fileTooLarge
        }

        let data = try Data(contentsOf: url)
        guard let csvString = String(data: data, encoding: .utf8) else {
            throw CustomDictionaryError.invalidCSVData
        }

        let entries = parseCSV(csvString)

        // Pre-validate entry count
        if entries.count > Self.maxEntryCount {
            throw CustomDictionaryError.tooManyEntries
        }

        // If the file has non-empty content lines but no valid entries, it's a format error
        let hasContentLines = csvString.components(separatedBy: .newlines)
            .contains { !$0.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }
        if entries.isEmpty, hasContentLines {
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
    func parseCSV(_ csv: String) -> [CustomDictionaryEntry] {
        let lines = csv.components(separatedBy: .newlines)
        var entries: [CustomDictionaryEntry] = []

        for i in 0 ..< lines.count {
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
            } else if char == ",", !inQuotes {
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
        // Convert POJ nasal markers ⁿ (U+207F) / ᴺ (U+1D3A) → nn
        let withNasalConverted = roman.lowercased()
            .replacingOccurrences(of: "\u{207F}", with: "nn")
            .replacingOccurrences(of: "\u{1D3A}", with: "nn")
        let decomposed = withNasalConverted.decomposedStringWithCanonicalMapping
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

    /// Generate numeric-toned form from romanization (for tone-aware search).
    /// Converts diacritics to tone digits and strips hyphens.
    /// Example: "gâu-tsá" → "gau5tsa2"
    static func generateRomanNum(_ roman: String) -> String {
        InputNormalizer.normalize(roman, mode: .tl)
    }
}

// MARK: - Custom Dictionary Errors

enum CustomDictionaryError: LocalizedError {
    case invalidCSVData
    case invalidCSVFormat
    case fileTooLarge
    case tooManyEntries

    var errorDescription: String? {
        switch self {
        case .invalidCSVData:
            "Invalid CSV data"
        case .invalidCSVFormat:
            DictionaryTexts.invalidCSVFormat
        case .fileTooLarge:
            DictionaryTexts.fileTooLarge
        case .tooManyEntries:
            DictionaryTexts.tooManyEntries
        }
    }
}
