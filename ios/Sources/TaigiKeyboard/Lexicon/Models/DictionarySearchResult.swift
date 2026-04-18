import Foundation

/// Search result with source information for dictionary exploration (Tab 3).
struct DictionarySearchResult {
    /// Sentinel id for results synthesised from the user's custom dictionary.
    static let customDictMarkerId = -2

    let id: Int
    let roman: String // Display form (POJ or TL based on user setting)
    let tl: String // Raw TL from database (for external lookup URLs)
    let hanzi: String?
    let frequency: Int
    let sources: [DictionarySource]

    /// Chhoe Taigi dictionary lookup URL for this result's TL form.
    var chhoeURL: URL? {
        ExternalLookupURLBuilder.chhoeURL(forTL: tl)
    }

    /// MOE Sutian dictionary lookup URL for this result's TL form.
    var moeURL: URL? {
        ExternalLookupURLBuilder.moeURL(forTL: tl)
    }

    /// Unique source display-name tags for badge rendering.
    /// Preserves `sources` order; skips empty names and duplicates.
    var uniqueTagNames: [String] {
        var seen = Set<String>()
        return sources.compactMap { source in
            let name = source.displayName
            guard !name.isEmpty, seen.insert(name).inserted else { return nil }
            return name
        }
    }
}
