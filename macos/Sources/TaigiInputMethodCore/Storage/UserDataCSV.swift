// Reading and writing the hand-editable CSV exports of the user's own data.

import Foundation

/// The CSV dialect the three platforms share, and the two row shapes that ride
/// on it.
///
/// CROSS-PLATFORM INVARIANT — mirrors
/// ios/Sources/TaigiKeyboard/App/Tabs/Dictionary/Utilities/CSVDocument.swift
/// (`parseLine` / `escape`) and
/// android/app/src/main/java/com/siansiansu/taigikeyboard/ime/dictionary/DictionaryCsvCodec.kt.
/// Drift means a file exported on one platform imports differently on another,
/// which is the whole point of the format.
///
/// This is a SINGLE-RECORD dialect, not full RFC 4180: quoting inside a line is
/// honoured (including the doubled-quote escape), but a record is always one
/// line, because both mirrors split on newlines before parsing. `escape` can
/// therefore emit a quoted field containing a newline that no decoder here will
/// read back — the same asymmetry the other two platforms have, kept rather
/// than fixed so the three stay byte-compatible.
enum UserDataCSV {
    /// Splits one CSV line into its fields.
    ///
    /// A doubled quote inside a quoted field (`""`) is one literal quote, which
    /// is what makes a round-trip through `escape` lossless. An unbalanced
    /// quote leaves the rest of the line quoted rather than erroring — a
    /// hand-edited file gets a best effort, not a rejection.
    static func parseLine(_ line: String) -> [String] {
        var fields: [String] = []
        var current = ""
        var isInsideQuotes = false
        let characters = Array(line)
        var index = 0
        while index < characters.count {
            let character = characters[index]
            if character == "\"", isInsideQuotes, index + 1 < characters.count,
               characters[index + 1] == "\""
            {
                current.append("\"")
                index += 1
            } else if character == "\"" {
                isInsideQuotes.toggle()
            } else if character == ",", !isInsideQuotes {
                fields.append(current)
                current = ""
            } else {
                current.append(character)
            }
            index += 1
        }
        fields.append(current)
        return fields
    }

    /// Quotes a field only when it would otherwise change the parse. The
    /// trigger set is exactly `,` `"` and newline — a leading space or a `\r`
    /// is left alone, matching both mirrors.
    static func escape(_ field: String) -> String {
        if field.contains(",") || field.contains("\"") || field.contains("\n") {
            return "\"\(field.replacingOccurrences(of: "\"", with: "\"\""))\""
        }
        return field
    }

    // MARK: - 詞頻

    /// One exported frequency row: the word, the reading it was learned under,
    /// and how often it has been committed.
    struct FrequencyCSVRow: Equatable, Sendable {
        let word: String
        let tl: String
        let count: Int
    }

    /// `word,tl,count` per line, no header, every line newline-terminated.
    static func encodeFrequency(_ rows: [FrequencyCSVRow]) -> String {
        rows.reduce(into: "") { csv, row in
            csv += "\(escape(row.word)),\(escape(row.tl)),\(row.count)\n"
        }
    }

    /// Reads `word,tl,count`, and `word,count` from a backup written before the
    /// pair key existed — the two-column form lands in the tolerant `tl == ""`
    /// bucket the engine already falls back to.
    ///
    /// The column count is matched EXACTLY rather than with `>=`, so a
    /// malformed four-column row is skipped instead of being silently eaten as
    /// a legacy two-column one.
    static func decodeFrequency(_ csv: String) -> [FrequencyCSVRow] {
        decodeRows(csv) { columns in
            let word: String
            let tl: String
            let countField: String
            switch columns.count {
            case 3:
                word = columns[0]
                tl = columns[1]
                countField = columns[2]
            case 2:
                word = columns[0]
                tl = ""
                countField = columns[1]
            default:
                return nil
            }
            guard !word.isEmpty, let count = Int(countField), count > 0 else { return nil }
            return FrequencyCSVRow(word: word, tl: tl, count: count)
        }
    }

    // MARK: - 詞關聯

    /// One exported association row. All four identity columns are present
    /// because the pair a bigram is keyed on is
    /// `(prev 漢字, prev TL, next 漢字, next TL)` — dropping either reading
    /// would merge two different words (Core Principle #7).
    struct AssociationCSVRow: Equatable, Sendable {
        let previousWord: String
        let previousTl: String
        let nextWord: String
        let nextTl: String
        let count: Int
    }

    static func encodeAssociation(_ rows: [AssociationCSVRow]) -> String {
        rows.reduce(into: "") { csv, row in
            csv += "\(escape(row.previousWord)),\(escape(row.previousTl)),"
            csv += "\(escape(row.nextWord)),\(escape(row.nextTl)),\(row.count)\n"
        }
    }

    /// Reads five columns and ignores any beyond them — `>=` rather than the
    /// frequency codec's exact match, because that is what both mirrors do and
    /// there is no shorter legacy shape here to be confused with.
    ///
    /// `previousWord` may be empty (a sentence-initial word has no predecessor);
    /// `nextWord` may not.
    static func decodeAssociation(_ csv: String) -> [AssociationCSVRow] {
        decodeRows(csv) { columns in
            guard columns.count >= 5,
                  let count = Int(columns[4]), count > 0,
                  !columns[2].isEmpty
            else { return nil }
            return AssociationCSVRow(
                previousWord: columns[0],
                previousTl: columns[1],
                nextWord: columns[2],
                nextTl: columns[3],
                count: count,
            )
        }
    }

    // MARK: - Private

    /// Splits `csv` into lines, hands each one's trimmed fields to `decodeRow`,
    /// and keeps what it accepts.
    ///
    /// Blank lines are skipped and every field is trimmed before the row
    /// decoder sees it, so a hand-edited file with padding around its commas
    /// still reads. A `nil` from `decodeRow` drops that line and no more — one
    /// malformed row in a backup must not cost the user the rest of the file.
    private static func decodeRows<Row>(
        _ csv: String,
        decodeRow: ([String]) -> Row?,
    ) -> [Row] {
        csv.components(separatedBy: .newlines).compactMap { line in
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty else { return nil }
            return decodeRow(parseLine(trimmed).map {
                $0.trimmingCharacters(in: .whitespacesAndNewlines)
            })
        }
    }
}
