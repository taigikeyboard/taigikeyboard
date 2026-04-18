import Foundation

// MARK: - Shared-Core Candidate

// Pure logic, Foundation-only. Eligible for cross-platform extraction.

/// NextWord 預測排序 — 純函式 + 常數。
///
/// CROSS-PLATFORM INVARIANT — the scoring constants below
/// (userWeight, dictWeight, decayHalfLifeHours, learningBonus,
/// *DecayFloor, *Threshold) MUST mirror Android
/// `android/app/src/main/java/com/siansiansu/taigikeyboard/nextword/NextWordService.kt`.
/// Drift causes silent ranking divergence between platforms.
enum NextWordScorer {
    // MARK: - Ranking Constants

    /// Source weights
    static let userWeight: Double = 50.0
    static let dictWeight: Double = 1.0

    /// Time decay: half-life 168 hours (1 week)
    static let decayHalfLifeHours: Double = 168.0

    /// Memory strength: ensures user entries rank above dict entries
    static let learningBonus: Double = 300.0

    /// count >= threshold: near-permanent
    static let highUsageDecayFloor: Double = 0.95
    /// count < threshold: prevents full decay
    static let lowUsageDecayFloor: Double = 0.3
    static let highUsageThreshold: Int = 3

    // MARK: - Scoring

    /// Score a dict-layer prediction (count × dictWeight).
    static func scoreDict(count: Int) -> Double {
        Double(count) * dictWeight
    }

    /// Calculate user-layer score with decay floor + learning bonus.
    ///
    /// Ensures user entries always rank above dict entries (max ~300).
    /// High-usage entries (count >= highUsageThreshold) get near-permanent retention.
    ///
    /// `nowMs` is injected at the call site so tests can pin the decay window
    /// without reaching for a global clock.
    static func calculateUserScore(count: Int, lastUsedMs: Int64, nowMs: Int64) -> Double {
        let decay = calculateDecay(lastUsedMs: lastUsedMs, nowMs: nowMs)
        let rawScore = Double(count) * userWeight
        let decayFloor = count >= highUsageThreshold
            ? highUsageDecayFloor
            : lowUsageDecayFloor
        let effectiveDecay = max(decayFloor, decay)
        return rawScore * effectiveDecay + learningBonus
    }

    /// Exponential time decay (RIME-style):
    ///   decay = exp(-ageHours / halfLifeHours * ln(2))
    static func calculateDecay(lastUsedMs: Int64, nowMs: Int64) -> Double {
        let ageHours = Double(nowMs - lastUsedMs) / 3_600_000.0
        // ln(2) ≈ 0.693
        return exp(-ageHours / decayHalfLifeHours * 0.693)
    }
}
