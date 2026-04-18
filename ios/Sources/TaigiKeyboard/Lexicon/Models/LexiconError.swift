import Foundation

/// Errors raised by the lexicon engine (SQLite repositories, trie loader,
/// shared infrastructure). Named after the module, not any single data
/// source — these surface from `DictionaryRepository`, `CustomDictionaryRepository`,
/// `UserFrequencyRepository`, `NextWordService`, and `TrieService` alike.

// MARK: - Shared-Core Candidate
// Pure logic, Foundation-only. Eligible for cross-platform extraction.
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
