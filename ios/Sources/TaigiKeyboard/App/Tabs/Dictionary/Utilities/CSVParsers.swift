// 中文: CSVDocument 的 decode/encode 擴充 — 詞頻 (word,count) + 聯想詞 5 欄格式。
// 中文: 解析時跳過空行、缺欄、計數非正整數的列;NextWordService.AssociationEntry 為來源型別。

import Foundation

extension CSVDocument {
    // MARK: - Frequency

    // CROSS-PLATFORM INVARIANT — mirrors android/.../ime/dictionary/DictionaryCsvCodec.kt
    // (encode/decodeFrequencyCSV). Drift causes silent divergence; both serialize byte-for-byte.
    // Pins INVARIANT_USER_FREQ_PAIR_KEY (docs/architecture/behavioral-invariants.md §28).

    /// Parse the frequency CSV. The current format is 3 columns
    /// `word,tl,count` carrying the `(漢字, 羅馬字)` pair (Core Principle #7).
    /// A legacy 2-column `word,count` file (pre-roman-column export or a
    /// hand-edited file) decodes with `tl = ""` — the tolerant legacy bucket.
    /// Any other column count is skipped (an exact discriminator, not `>= 2`,
    /// so a malformed row is never silently eaten as legacy).
    // 中文: 解析詞頻 CSV — 3 欄 (word,tl,count) 為現格式;2 欄 (word,count) 舊檔 → tl=""(legacy bucket);其餘欄數跳過。
    static func decodeFrequencyCSV(_ csv: String) -> [(word: String, tl: String, count: Int)] {
        let lines = csv.components(separatedBy: .newlines)
        var entries: [(word: String, tl: String, count: Int)] = []
        for line in lines {
            let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !trimmed.isEmpty else { continue }
            let columns = parseLine(trimmed).map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }
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
                continue
            }
            guard !word.isEmpty, let count = Int(countField), count > 0 else { continue }
            entries.append((word: word, tl: tl, count: count))
        }
        return entries
    }

    // 中文: 把詞頻清單編成 CSV 字串(每列 word,tl,count\n);word 與 tl 走 escape() 轉義。
    static func encodeFrequencyCSV(_ entries: [(word: String, tl: String, count: Int)]) -> String {
        var csv = ""
        for item in entries {
            csv += "\(escape(item.word)),\(escape(item.tl)),\(item.count)\n"
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
