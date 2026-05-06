// 中文: CSVDocument 的 decode/encode 擴充 — 詞頻 (word,count) + 聯想詞 5 欄格式。
// 中文: 解析時跳過空行、缺欄、計數非正整數的列;NextWordService.AssociationEntry 為來源型別。

import Foundation

extension CSVDocument {
    // MARK: - Frequency

    // 中文: 解析詞頻 CSV(2 欄:word,count);過濾掉空 word 與非正整數 count。
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

    // 中文: 把詞頻清單編成 CSV 字串(每列 word,count\n);word 走 escape() 轉義。
    static func encodeFrequencyCSV(_ entries: [(word: String, count: Int)]) -> String {
        var csv = ""
        for item in entries {
            csv += "\(escape(item.word)),\(item.count)\n"
        }
        return csv
    }

    // MARK: - Association

    // 中文: 聯想詞 CSV 一列的具名 tuple — 5 欄:prev/next 漢字 + prev/next TL + count。
    typealias AssociationCSVRow = (
        prevWord: String,
        prevTl: String,
        nextWord: String,
        nextTl: String,
        count: Int
    )

    // 中文: 解析聯想詞 CSV(5 欄);過濾掉空 nextWord 與非正整數 count 的列。
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

    // 中文: 把聯想詞清單編成 CSV(5 欄);每個字串欄都走 escape() 轉義。
    static func encodeAssociationCSV(_ entries: [NextWordService.AssociationEntry]) -> String {
        var csv = ""
        for item in entries {
            csv += "\(escape(item.prevWord)),\(escape(item.prevTl)),\(escape(item.nextWord)),\(escape(item.nextTl)),\(item.count)\n"
        }
        return csv
    }
}
