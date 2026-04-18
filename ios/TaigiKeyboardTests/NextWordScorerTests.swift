@testable import TaigiKeyboard
import XCTest

/// Pins the cross-platform ranking invariant. Any change here must be
/// mirrored in Android's `NextWordService.kt`.
final class NextWordScorerTests: XCTestCase {
    // MARK: - Dict Scoring

    func testScoreDict_isCountTimesDictWeight() {
        XCTAssertEqual(NextWordScorer.scoreDict(count: 0), 0)
        XCTAssertEqual(NextWordScorer.scoreDict(count: 1), NextWordScorer.dictWeight)
        XCTAssertEqual(NextWordScorer.scoreDict(count: 7), 7 * NextWordScorer.dictWeight)
    }

    // MARK: - User Scoring

    func testCalculateUserScore_atEpoch_hitsLearningBonusFloor() {
        // nowMs == lastUsedMs → decay = 1 (no age), effectiveDecay = 1
        // score = count * userWeight * 1 + learningBonus
        let score = NextWordScorer.calculateUserScore(count: 1, lastUsedMs: 0, nowMs: 0)
        XCTAssertEqual(score, 1 * NextWordScorer.userWeight + NextWordScorer.learningBonus, accuracy: 1e-9)
    }

    func testCalculateUserScore_userEntryOutranksDictEntry() {
        // A very stale, count=1 user entry should still outrank a count=100 dict entry,
        // thanks to learningBonus (300) pulling the score above dict's max (~100).
        let veryOld: Int64 = 0
        let now: Int64 = 10 * 365 * 24 * 3_600_000 // ~10 years
        let userScore = NextWordScorer.calculateUserScore(count: 1, lastUsedMs: veryOld, nowMs: now)
        let dictScore = NextWordScorer.scoreDict(count: 100)
        XCTAssertGreaterThan(userScore, dictScore)
    }

    func testCalculateUserScore_highUsageHasHigherFloor() {
        // Same old timestamp, count=3 (high) vs count=2 (low) → high-usage decay
        // floor (0.95) > low-usage floor (0.3), producing a larger effectiveDecay
        // and therefore a higher per-count score.
        let old: Int64 = 0
        let now: Int64 = 1000 * 365 * 24 * 3_600_000 // huge age → decay ≈ 0
        let highScore = NextWordScorer.calculateUserScore(count: 3, lastUsedMs: old, nowMs: now)
        let lowScore = NextWordScorer.calculateUserScore(count: 2, lastUsedMs: old, nowMs: now)

        // High-count: 3 * 50 * 0.95 + 300 = 442.5
        // Low-count: 2 * 50 * 0.3 + 300 = 330
        XCTAssertEqual(highScore, 3 * NextWordScorer.userWeight * NextWordScorer.highUsageDecayFloor + NextWordScorer.learningBonus, accuracy: 1e-9)
        XCTAssertEqual(lowScore, 2 * NextWordScorer.userWeight * NextWordScorer.lowUsageDecayFloor + NextWordScorer.learningBonus, accuracy: 1e-9)
        XCTAssertGreaterThan(highScore, lowScore)
    }

    // MARK: - Decay

    func testCalculateDecay_atEpochIsOne() {
        XCTAssertEqual(NextWordScorer.calculateDecay(lastUsedMs: 0, nowMs: 0), 1.0, accuracy: 1e-9)
    }

    func testCalculateDecay_atOneHalfLifeIsHalf() {
        let halfLifeMs = Int64(NextWordScorer.decayHalfLifeHours * 3_600_000)
        let decay = NextWordScorer.calculateDecay(lastUsedMs: 0, nowMs: halfLifeMs)
        // exp(-1 * ln(2)) = 0.5 — tolerance reflects 0.693 approximation of ln(2).
        XCTAssertEqual(decay, 0.5, accuracy: 1e-3)
    }

    func testCalculateDecay_atTwoHalfLivesIsQuarter() {
        let twoHalfLivesMs = Int64(NextWordScorer.decayHalfLifeHours * 3_600_000 * 2)
        let decay = NextWordScorer.calculateDecay(lastUsedMs: 0, nowMs: twoHalfLivesMs)
        XCTAssertEqual(decay, 0.25, accuracy: 1e-3)
    }

    func testCalculateDecay_monotonicallyDecreasesWithAge() {
        let now: Int64 = 1_000_000_000_000
        var previous = NextWordScorer.calculateDecay(lastUsedMs: now, nowMs: now)
        for ageHours in stride(from: 1, through: 200, by: 10) {
            let lastUsed = now - Int64(ageHours) * 3_600_000
            let decay = NextWordScorer.calculateDecay(lastUsedMs: lastUsed, nowMs: now)
            XCTAssertLessThan(decay, previous)
            previous = decay
        }
    }
}
