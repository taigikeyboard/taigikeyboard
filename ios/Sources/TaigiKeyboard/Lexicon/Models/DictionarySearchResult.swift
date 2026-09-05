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
}
