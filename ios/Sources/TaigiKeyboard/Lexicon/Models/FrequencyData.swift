import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Per-word frequency snapshot consumed by ranking. Marshalled to
/// `Taigi_Engine_FrequencyEntry` inside `RustEngineBridge.processCandidates`
/// for `engine/ranking/src/score.rs::calculate_score`.
///
/// Hoisted out of `UserFrequencyRepository` so the bridge marshalling code
/// depends on this Foundation-only value type rather than an iOS-only
/// SQLite repository.
public struct FrequencyData {
    public let count: Int
    public let lastUsedMillis: Int64 // Unix timestamp in milliseconds

    public init(count: Int, lastUsedMillis: Int64) {
        self.count = count
        self.lastUsedMillis = lastUsedMillis
    }

    public static let empty = FrequencyData(count: 0, lastUsedMillis: 0)
}

/// One `user_frequency.db` row in R5 `(word, tl)` pair-key form: the
/// display-text key, its canonical-TL reading, and the snapshot. Returned
/// by `UserFrequencyRepository.frequencyDataBatch` so the engine can build
/// a `(display_text, canonical_tl)`-keyed `FrequencyMap` (Core Principle
/// #7). `tl == ""` is the legacy fallback bucket.
public struct FrequencyRow {
    public let word: String
    public let tl: String
    public let data: FrequencyData

    public init(word: String, tl: String, data: FrequencyData) {
        self.word = word
        self.tl = tl
        self.data = data
    }
}
