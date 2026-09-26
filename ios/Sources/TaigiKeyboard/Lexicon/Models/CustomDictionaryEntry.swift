import Foundation

// Custom dictionary entry model — the page's view of one engine entry
// (`CustomDictionaryEntry`, `engine/protos/proto/user_data.proto`); the
// engine owns the stored shape and stamps the times.

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.
struct CustomDictionaryEntry: Identifiable, Equatable, Sendable {
    /// Stable across edits: the engine stores an edit under the same id.
    let id: String
    var roman: String
    var hanzi: String

    init(id: String = UUID().uuidString, roman: String, hanzi: String) {
        self.id = id
        self.roman = roman
        self.hanzi = hanzi
    }
}
