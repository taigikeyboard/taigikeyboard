// 中文: .taigi 備份檔的 UTType 定義 + FileDocument 封裝,提供 SwiftUI fileExporter 使用。
// 中文: 內容是 JSON 編碼的整體備份(自訂詞庫、詞頻、聯想資料),conforming 到 public.json。

import SwiftUI
import UniformTypeIdentifiers

/// UTType for .taigi backup files
// 中文: 備份檔 UTI 註冊;tw.taigikeyboard.backup 由 Info.plist exported types 宣告。
extension UTType {
    static let taigiBackup = UTType(exportedAs: "tw.taigikeyboard.backup", conformingTo: .json)
}

/// FileDocument wrapper for .taigi backup export via fileExporter
// 中文: SwiftUI fileExporter 用的 FileDocument,單純把 Data 包成可讀寫 wrapper。
struct BackupDocument: FileDocument {
    static var readableContentTypes: [UTType] { [.taigiBackup, .json] }

    var data: Data

    init(_ data: Data) {
        self.data = data
    }

    // 中文: 從檔案讀入時的初始化器;空檔回退為空 Data。
    init(configuration: ReadConfiguration) throws {
        if let fileData = configuration.file.regularFileContents {
            data = fileData
        } else {
            data = Data()
        }
    }

    // 中文: 寫檔時把 Data 包成 regular FileWrapper 給 fileExporter 落地。
    func fileWrapper(configuration: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: data)
    }
}
