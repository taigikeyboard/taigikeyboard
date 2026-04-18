import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// Per-word frequency snapshot consumed by ranking (`CandidateProcessor.calculateScore`).
///
/// Hoisted out of `UserFrequencyRepository` so ranking code depends on this
/// Foundation-only value type rather than an iOS-only SQLite repository.
struct FrequencyData {
    let count: Int
    let lastUsedMillis: Int64 // Unix timestamp in milliseconds

    static let empty = FrequencyData(count: 0, lastUsedMillis: 0)
}
