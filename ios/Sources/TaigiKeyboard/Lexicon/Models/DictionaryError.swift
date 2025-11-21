import Foundation

/// 詞典相關錯誤
enum DictionaryError: LocalizedError {
    case databaseNotFound
    case databaseNotAvailable
    case databaseConnectionFailed(String)
    case queryExecutionFailed(String)
    case queryPreparationFailed(String)

    var errorDescription: String? {
        switch self {
        case .databaseNotFound:
            "Dictionary database file not found"
        case .databaseNotAvailable:
            "Dictionary database is not available"
        case let .databaseConnectionFailed(message):
            "Failed to connect to database: \(message)"
        case let .queryExecutionFailed(message):
            "Failed to execute query: \(message)"
        case let .queryPreparationFailed(message):
            "Failed to prepare query: \(message)"
        }
    }
}
