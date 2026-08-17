// Reading and writing the two-column CSV the custom dictionary exports.

import Foundation

/// Why a custom-dictionary CSV could not be imported.
enum CustomDictionaryCSVError: Error, CustomStringConvertible {
    /// Bigger than `maxFileSizeBytes`. Checked before the file is read, so a
    /// huge file costs nothing to refuse.
    case fileTooLarge(limitBytes: Int)
    /// Not UTF-8.
    case notUTF8
    /// The file has content but not one usable row — a wrong-format file
    /// rather than an empty one, and worth saying so.
    case noUsableRows
    /// More rows than the dictionary can ever hold.
    case tooManyRows(limit: Int)

    var description: String {
        switch self {
        case let .fileTooLarge(limit): "file is larger than \(limit / (1024 * 1024)) MB"
        case .notUTF8: "file is not UTF-8 text"
        case .noUsableRows: "no usable rows in the file"
        case let .tooManyRows(limit): "file holds more than \(limit) entries"
        }
    }
}

/// The `roman,hanzi` CSV the 自訂詞庫 page reads and writes.
///
/// Uses the shared `UserDataCSV` quoting rather than a parser of its own. iOS
/// has a private second parser here that does NOT unescape doubled quotes, so
/// an entry containing a quote does not survive its own export/import round
/// trip; that is a bug to fix there, not a behaviour to reproduce.
enum CustomDictionaryCSV {
    /// CROSS-PLATFORM INVARIANT — mirrors
    /// ios/Sources/TaigiKeyboard/Lexicon/Services/CustomDictionaryService.swift:90.
    static let maxFileSizeBytes = 5 * 1024 * 1024

    static func encode(_ rows: [CustomDictionaryRow]) -> String {
        rows.reduce(into: "") { csv, row in
            csv += "\(UserDataCSV.escape(row.roman)),\(UserDataCSV.escape(row.hanzi))\n"
        }
    }

    /// Parses `csv` into rows, or explains why it could not.
    ///
    /// A row needs a romanization; the 漢字 column may be empty, because a
    /// romanization-only entry is legitimate. Rows the parser cannot use are
    /// dropped rather than failing the file — a hand-edited CSV with one bad
    /// line should import the rest — but a file that is entirely unusable is
    /// reported instead of silently importing nothing.
    static func decode(_ csv: String, entryLimit: Int) throws -> [CustomDictionaryRow] {
        var contentLines = 0
        var rows: [CustomDictionaryRow] = []
        for line in csv.components(separatedBy: .newlines) {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty else { continue }
            contentLines += 1
            let columns = UserDataCSV.parseLine(trimmed).map {
                $0.trimmingCharacters(in: .whitespacesAndNewlines)
            }
            guard columns.count >= 2, !columns[0].isEmpty else { continue }
            rows.append(CustomDictionaryRow(roman: columns[0], hanzi: columns[1]))
        }
        guard contentLines == 0 || !rows.isEmpty else { throw CustomDictionaryCSVError.noUsableRows }
        guard rows.count <= entryLimit else {
            throw CustomDictionaryCSVError.tooManyRows(limit: entryLimit)
        }
        return rows
    }

    /// Reads a file the user picked, refusing it before the read when it is
    /// too big to be a word list.
    static func decodeFile(at url: URL, entryLimit: Int) throws -> [CustomDictionaryRow] {
        let size = try url.resourceValues(forKeys: [.fileSizeKey]).fileSize ?? 0
        guard size <= maxFileSizeBytes else {
            throw CustomDictionaryCSVError.fileTooLarge(limitBytes: maxFileSizeBytes)
        }
        guard let text = try String(data: Data(contentsOf: url), encoding: .utf8) else {
            throw CustomDictionaryCSVError.notUTF8
        }
        return try decode(text, entryLimit: entryLimit)
    }
}
