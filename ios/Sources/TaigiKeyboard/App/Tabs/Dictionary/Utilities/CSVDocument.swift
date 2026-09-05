import SwiftUI
import UniformTypeIdentifiers

/// FileDocument wrapper for CSV export via fileExporter
struct CSVDocument: FileDocument {
    static var readableContentTypes: [UTType] {
        [.commaSeparatedText]
    }

    var text: String

    init(_ text: String) {
        self.text = text
    }

    init(configuration: ReadConfiguration) throws {
        if let data = configuration.file.regularFileContents {
            text = String(data: data, encoding: .utf8) ?? ""
        } else {
            text = ""
        }
    }

    func fileWrapper(configuration _: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: Data(text.utf8))
    }

    // MARK: - Shared CSV Helpers

    /// Parse a single CSV line, handling quoted fields and RFC 4180 doubled
    /// quotes (`""` inside a quoted field → one literal `"`), so round-trip
    /// with `escape()` is lossless.
    // CROSS-PLATFORM INVARIANT — mirrors android/.../ime/dictionary/DictionaryCsvCodec.kt parseLine.
    static func parseLine(_ line: String) -> [String] {
        var fields: [String] = []
        var current = ""
        var inQuotes = false
        let chars = Array(line)
        var index = 0
        while index < chars.count {
            let char = chars[index]
            if char == "\"", inQuotes, index + 1 < chars.count, chars[index + 1] == "\"" {
                // Doubled quote inside a quoted field → literal ".
                current.append("\"")
                index += 1
            } else if char == "\"" {
                inQuotes.toggle()
            } else if char == ",", !inQuotes {
                fields.append(current)
                current = ""
            } else {
                current.append(char)
            }
            index += 1
        }
        fields.append(current)
        return fields
    }

    /// Escape a field for CSV output (wrap in quotes if needed).
    static func escape(_ field: String) -> String {
        if field.contains(",") || field.contains("\"") || field.contains("\n") {
            return "\"\(field.replacingOccurrences(of: "\"", with: "\"\""))\""
        }
        return field
    }
}
