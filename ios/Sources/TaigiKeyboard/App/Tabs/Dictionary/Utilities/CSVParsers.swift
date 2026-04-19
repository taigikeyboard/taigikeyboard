import Foundation

extension CSVDocument {
    // MARK: - Frequency

    static func decodeFrequencyCSV(_ csv: String) -> [(word: String, count: Int)] {
        let lines = csv.components(separatedBy: .newlines)
        var entries: [(word: String, count: Int)] = []
        for line in lines {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty else { continue }
            let columns = parseLine(trimmed)
            guard columns.count >= 2 else { continue }
            let word = columns[0].trimmingCharacters(in: .whitespacesAndNewlines)
            guard !word.isEmpty,
                  let count = Int(columns[1].trimmingCharacters(in: .whitespacesAndNewlines)),
                  count > 0 else { continue }
            entries.append((word: word, count: count))
        }
        return entries
    }

    static func encodeFrequencyCSV(_ entries: [(word: String, count: Int)]) -> String {
        var csv = ""
        for item in entries {
            csv += "\(escape(item.word)),\(item.count)\n"
        }
        return csv
    }

    // MARK: - Association

    typealias AssociationCSVRow = (
        prevWord: String,
        prevTl: String,
        nextWord: String,
        nextTl: String,
        count: Int
    )

    static func decodeAssociationCSV(_ csv: String) -> [AssociationCSVRow] {
        let lines = csv.components(separatedBy: .newlines)
        var entries: [AssociationCSVRow] = []
        for line in lines {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty else { continue }
            let columns = parseLine(trimmed)
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

    static func encodeAssociationCSV(_ entries: [NextWordService.AssociationEntry]) -> String {
        var csv = ""
        for item in entries {
            csv += "\(escape(item.prevWord)),\(escape(item.prevTl)),\(escape(item.nextWord)),\(escape(item.nextTl)),\(item.count)\n"
        }
        return csv
    }
}
