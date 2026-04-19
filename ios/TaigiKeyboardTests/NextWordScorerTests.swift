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

    // MARK: - INVARIANT wrappers — Phase 0 §7 + §8 labels

    /// Age far beyond any reasonable retention — pushes raw decay ≈ 0 so the
    /// high/low decay floors are the only remaining contribution.
    private static let saturatedAgeMs: Int64 = 1000 * 365 * 24 * 3_600_000

    func test_INVARIANT_decay_half_life_is_168_hours() {
        let halfLifeMs = Int64(NextWordScorer.decayHalfLifeHours * 3_600_000)
        XCTAssertEqual(NextWordScorer.calculateDecay(lastUsedMs: 0, nowMs: halfLifeMs), 0.5, accuracy: 1e-3)
    }

    func test_INVARIANT_high_usage_decay_floor_95() {
        let score = NextWordScorer.calculateUserScore(count: 3, lastUsedMs: 0, nowMs: Self.saturatedAgeMs)
        let expected = 3 * NextWordScorer.userWeight * NextWordScorer.highUsageDecayFloor
            + NextWordScorer.learningBonus
        XCTAssertEqual(score, expected, accuracy: 1e-9)
    }

    func test_INVARIANT_low_usage_decay_floor_30() {
        let score = NextWordScorer.calculateUserScore(count: 2, lastUsedMs: 0, nowMs: Self.saturatedAgeMs)
        let expected = 2 * NextWordScorer.userWeight * NextWordScorer.lowUsageDecayFloor
            + NextWordScorer.learningBonus
        XCTAssertEqual(score, expected, accuracy: 1e-9)
    }

    /// Pin test — literal values here are deliberate. Changing any constant in
    /// `NextWordScorer` requires a mirrored change in
    /// `android/app/src/main/java/com/siansiansu/taigikeyboard/nextword/NextWordService.kt`.
    func test_INVARIANT_scorer_constants_match_android() {
        XCTAssertEqual(NextWordScorer.userWeight, 50.0)
        XCTAssertEqual(NextWordScorer.dictWeight, 1.0)
        XCTAssertEqual(NextWordScorer.learningBonus, 300.0)
        XCTAssertEqual(NextWordScorer.decayHalfLifeHours, 168.0)
        XCTAssertEqual(NextWordScorer.highUsageDecayFloor, 0.95)
        XCTAssertEqual(NextWordScorer.lowUsageDecayFloor, 0.3)
        XCTAssertEqual(NextWordScorer.highUsageThreshold, 3)
    }

    func test_INVARIANT_user_weight_is_50() {
        // Differencing two count values cancels learningBonus and isolates userWeight.
        let one = NextWordScorer.calculateUserScore(count: 1, lastUsedMs: 0, nowMs: 0)
        let two = NextWordScorer.calculateUserScore(count: 2, lastUsedMs: 0, nowMs: 0)
        XCTAssertEqual(two - one, NextWordScorer.userWeight, accuracy: 1e-9)
    }

    func test_INVARIANT_dict_weight_is_1() {
        XCTAssertEqual(NextWordScorer.scoreDict(count: 0), 0.0)
        XCTAssertEqual(NextWordScorer.scoreDict(count: 1), NextWordScorer.dictWeight)
        XCTAssertEqual(NextWordScorer.scoreDict(count: 42), 42 * NextWordScorer.dictWeight)
    }

    func test_INVARIANT_learning_bonus_is_300() {
        // count=0 zeroes the userWeight term; at epoch effectiveDecay == 1.
        XCTAssertEqual(
            NextWordScorer.calculateUserScore(count: 0, lastUsedMs: 0, nowMs: 0),
            NextWordScorer.learningBonus,
        )
    }

    func test_INVARIANT_user_entry_outranks_dict_entry() {
        // Very stale count=1 user entry vs count=100 dict entry — learningBonus wins.
        let tenYearsMs: Int64 = 10 * 365 * 24 * 3_600_000
        let userScore = NextWordScorer.calculateUserScore(count: 1, lastUsedMs: 0, nowMs: tenYearsMs)
        let dictScore = NextWordScorer.scoreDict(count: 100)
        XCTAssertGreaterThan(userScore, dictScore)
    }
}
