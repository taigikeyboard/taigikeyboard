import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Search result with source information for dictionary search (Manage Dictionaries page).
struct DictionarySearchResult: Equatable {
    let roman: String // Display form (POJ or TL based on user setting)
    let tl: String // TL the web dictionaries are asked with (a custom entry's own romanization)
    let hanji: String?
    let sources: [DictionarySource]

    /// Chhoe Taigi dictionary lookup URL for this result's TL form.
    var chhoeURL: URL? {
        ExternalLookupURLBuilder.chhoeURL(forTL: tl)
    }

    /// MOE Sutian dictionary lookup URL for this result's TL form.
    var moeURL: URL? {
        ExternalLookupURLBuilder.moeURL(forTL: tl)
    }
}
