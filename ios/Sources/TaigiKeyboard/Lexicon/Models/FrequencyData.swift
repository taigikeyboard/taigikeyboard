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
struct FrequencyData {
    let count: Int
    let lastUsedMillis: Int64 // Unix timestamp in milliseconds

    static let empty = FrequencyData(count: 0, lastUsedMillis: 0)
}
