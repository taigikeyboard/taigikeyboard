// 中文: CSV 匯出/匯入用的 FileDocument 包裝 + 共用 CSV 解析工具。
// 中文: 各 ViewModel 的 encode*CSV / decode*CSV 由本檔的 extension 提供。

import SwiftUI
import UniformTypeIdentifiers

/// FileDocument wrapper for CSV export via fileExporter
// 中文: CSV 文字檔的 FileDocument 包裝 + 共用 line parser / field escaper。
struct CSVDocument: FileDocument {
    static var readableContentTypes: [UTType] {
        [.commaSeparatedText]
    }

    var text: String

    init(_ text: String) {
        self.text = text
    }

    // 中文: 從檔案讀入時的初始化器,UTF-8 解碼;失敗回退為空字串。
    init(configuration: ReadConfiguration) throws {
        if let data = configuration.file.regularFileContents {
            text = String(data: data, encoding: .utf8) ?? ""
        } else {
            text = ""
        }
    }

    // 中文: 寫檔時把字串以 UTF-8 編成 FileWrapper。
    func fileWrapper(configuration _: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: Data(text.utf8))
    }

    // MARK: - Shared CSV Helpers

    /// Parse a single CSV line, handling quoted fields.
    // 中文: 解析單行 CSV,支援雙引號包覆的欄位(含逗號、跳脫雙引號)。
    static func parseLine(_ line: String) -> [String] {
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

    /// Escape a field for CSV output (wrap in quotes if needed).
    // 中文: 把欄位轉義成 CSV 安全文字 — 含逗號 / 雙引號 / 換行時包雙引號並 escape。
    static func escape(_ field: String) -> String {
        if field.contains(",") || field.contains("\"") || field.contains("\n") {
            return "\"\(field.replacingOccurrences(of: "\"", with: "\"\""))\""
        }
        return field
    }
}
