import Foundation

// Custom dictionary entry model — the page's view of one engine entry
// (`CustomDictionaryEntry`, `engine/protos/proto/user_data.proto`); the
// engine owns the stored shape and stamps the times.

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.
struct CustomDictionaryEntry: Identifiable, Equatable, Sendable {
    /// Stable across edits: the engine stores an edit under the same id.
    let id: String
    /// The romanization as saved — what an edit starts from.
    var roman: String
    var hanji: String
    /// `roman` in the input mode the list was read under (the engine's
    /// rendering); empty for an entry no list answered.
    var displayRoman: String

    init(id: String = UUID().uuidString, roman: String, hanji: String, displayRoman: String = "") {
        self.id = id
        self.roman = roman
        self.hanji = hanji
        self.displayRoman = displayRoman
    }

    /// The list row, in the list's input mode: a romanization-only word
    /// shows its romanization alone.
    /// Twin of Android `CustomDictionaryWord.listLabel`.
    var listLabel: String {
        hanji.isEmpty ? displayRoman : "\(displayRoman) → \(hanji)"
    }
}

/// One page of the custom dictionary list, as the engine cut it.
struct CustomDictionaryPage: Equatable, Sendable {
    let entries: [CustomDictionaryEntry]
    /// Every stored word.
    let total: Int
    /// The words the filter matches: what paging runs up to.
    let matchingTotal: Int
    /// The offset the page really starts at: the engine pulls the request
    /// back to a multiple of the limit when it falls in the last page, or the
    /// matches shrank under it (`engine/userdata/src/paging.rs`).
    let offset: Int
}
