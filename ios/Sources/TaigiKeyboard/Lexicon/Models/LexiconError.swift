// lexicon 模組的錯誤型別 — 由所有 user-data SQLite repository 共用。
// 唯讀辭典的錯誤走 Rust 引擎,經 RustEngineBridge 回傳,不在這裡。

import Foundation

// Errors raised by the lexicon module (user-data SQLite repositories +
// shared infrastructure). Named after the module, not any single data
// source — these surface from `CustomDictionaryRepository`,
// `UserFrequencyRepository`, `NextWordRepository`, and `NextWordService`
// alike. (Read-only dictionary errors live in the Rust engine and surface
// through `RustEngineBridge` instead.)

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.
// lexicon 共用錯誤 enum — 連線、查詢準備、查詢執行、Trie 未載入等。
enum LexiconError: LocalizedError {
    case databaseNotFound
    case databaseNotAvailable
    case databaseConnectionFailed(String)
    case queryExecutionFailed(String)
    case queryPreparationFailed(String)
    case trieNotLoaded

    var errorDescription: String? {
        switch self {
        case .databaseNotFound:
            "Database file not found"
        case .databaseNotAvailable:
            "Database is not available"
        case let .databaseConnectionFailed(message):
            "Failed to connect to database: \(message)"
        case let .queryExecutionFailed(message):
            "Failed to execute query: \(message)"
        case let .queryPreparationFailed(message):
            "Failed to prepare query: \(message)"
        case .trieNotLoaded:
            "Trie index not loaded"
        }
    }
}
