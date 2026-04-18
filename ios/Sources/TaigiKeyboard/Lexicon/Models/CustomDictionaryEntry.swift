import Foundation

/// Custom dictionary entry model

// MARK: - Shared-Core Candidate
// Pure logic, Foundation-only. Eligible for cross-platform extraction.
struct CustomDictionaryEntry: Identifiable, Equatable {
    let id: String
    var roman: String
    var hanzi: String
    let createdAt: Date
    var updatedAt: Date

    init(
        id: String = UUID().uuidString,
        roman: String,
        hanzi: String,
        createdAt: Date = Date(),
        updatedAt: Date = Date()
    ) {
        self.id = id
        self.roman = roman
        self.hanzi = hanzi
        self.createdAt = createdAt
        self.updatedAt = updatedAt
    }
}
