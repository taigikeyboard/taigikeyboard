import SwiftUI
import UniformTypeIdentifiers

/// UTType for .taigi backup files; the UTI is declared in Info.plist exported types.
extension UTType {
    static let taigiBackup = UTType(exportedAs: "tw.taigikeyboard.backup", conformingTo: .json)
}

/// FileDocument wrapper for .taigi backup export via fileExporter
struct BackupDocument: FileDocument {
    static var readableContentTypes: [UTType] {
        [.taigiBackup, .json]
    }

    var data: Data

    init(_ data: Data) {
        self.data = data
    }

    init(configuration: ReadConfiguration) throws {
        if let fileData = configuration.file.regularFileContents {
            data = fileData
        } else {
            data = Data()
        }
    }

    func fileWrapper(configuration _: WriteConfiguration) throws -> FileWrapper {
        FileWrapper(regularFileWithContents: data)
    }
}
