import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Per-word frequency snapshot consumed by ranking. `FrequencyRow` rows
/// are marshalled to `Taigi_Engine_FrequencyEntry` for the Continuous
/// `FetchAtPos` fetch (`engine/ranking/src/score.rs::build_frequency_map`).
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
