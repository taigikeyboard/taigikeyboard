@testable import TaigiKeyboard
import XCTest

/// `test_INVARIANT_*` methods pin Phase 0 §5 (dedup) and §6 (scoring)
/// behavioral invariants (`docs/architecture/behavioral-invariants.md`).
final class CandidateProcessorTests: XCTestCase {
    // MARK: - Fixtures

    private func word(_ roman: String, _ hanzi: String? = nil, lengthScore: Int? = nil, sourceBitmask: UInt16? = nil) -> TaigiWord {
        TaigiWord(id: 0, roman: roman, hanzi: hanzi, lengthScore: lengthScore, sourceBitmask: sourceBitmask)
    }

    /// Tier-bitmask convenience for readability in tier-invariant tests.
    /// Bit positions mirror dictionary/build/10_create_dictionary_bin.py.
    private enum TierBit {
        static let kautian: UInt16 = 1 << 0
        static let taigitv: UInt16 = 1 << 1
        static let itaigi: UInt16 = 1 << 2
        static let kungge: UInt16 = 1 << 6
        static let stti: UInt16 = 1 << 7
    }

    private let fixedNow: Int64 = 1_700_000_000_000 // Arbitrary pinned epoch ms

    // MARK: - isHanzi

    func testIsHanzi_cjkMainRange() {
        XCTAssertTrue(CandidateProcessor.isHanzi("台"))
        XCTAssertTrue(CandidateProcessor.isHanzi("語"))
        XCTAssertTrue(CandidateProcessor.isHanzi("人"))
    }

    func testIsHanzi_extensionB() {
        XCTAssertTrue(CandidateProcessor.isHanzi("\u{20000}"))
    }

    func testIsHanzi_latinOnly() {
        XCTAssertFalse(CandidateProcessor.isHanzi("hello"))
        XCTAssertFalse(CandidateProcessor.isHanzi("abc"))
    }

    func testIsHanzi_empty() {
        XCTAssertFalse(CandidateProcessor.isHanzi(""))
    }

    func testIsHanzi_mixed() {
        XCTAssertTrue(CandidateProcessor.isHanzi("hello台語"))
    }

    func testIsHanzi_digits() {
        XCTAssertFalse(CandidateProcessor.isHanzi("12345"))
    }

    func testIsHanzi_bopomofo() {
        XCTAssertFalse(CandidateProcessor.isHanzi("ㄅ"))
        XCTAssertFalse(CandidateProcessor.isHanzi("ㄆ"))
    }

    // MARK: - removeDuplicates

    func testRemoveDuplicates_exactDuplicate() {
        let words = [word("tâi-gí", "台語"), word("tâi-gí", "台語")]
        XCTAssertEqual(CandidateProcessor.removeDuplicates(words).count, 1)
    }

    func testRemoveDuplicates_toneVariantsPreserved() {
        let words = [word("phuǎnn-tshiú", "伴手"), word("phuānn-tshiú", "伴手")]
        XCTAssertEqual(CandidateProcessor.removeDuplicates(words).count, 2)
    }

    func testRemoveDuplicates_differentHanziPreserved() {
        let words = [word("kau3", "教"), word("kau3", "猴")]
        XCTAssertEqual(CandidateProcessor.removeDuplicates(words).count, 2)
    }

    func testRemoveDuplicates_romanOnlyNoDuplicates() {
        let words = [word("hello"), word("world")]
        XCTAssertEqual(CandidateProcessor.removeDuplicates(words).count, 2)
    }

    func testRemoveDuplicates_romanOnlyDuplicate() {
        let words = [word("hello"), word("hello")]
        XCTAssertEqual(CandidateProcessor.removeDuplicates(words).count, 1)
    }

    // MARK: - removeDisplayDuplicates

    func testRemoveDisplayDuplicates_keepsFirstPerHanzi() {
        // Callers must pre-sort; the dedup keeps the first entry per hanzi.
        let words = [word("phuānn-tshiú", "伴手"), word("phuǎnn-tshiú", "伴手")]
        let result = CandidateProcessor.removeDisplayDuplicates(words)
        XCTAssertEqual(result.count, 1)
        XCTAssertEqual(result.first?.roman, "phuānn-tshiú")
    }

    func testRemoveDisplayDuplicates_keepsWordsWithoutHanzi() {
        let words = [word("hello"), word("world"), word("guá", "我")]
        let result = CandidateProcessor.removeDisplayDuplicates(words)
        XCTAssertEqual(result.count, 3)
    }

    func testRemoveDisplayDuplicates_emptyHanziTreatedAsNoHanzi() {
        // The dedup gate guards against `hanzi == nil || hanzi.isEmpty`.
        let words = [word("one", ""), word("two", "")]
        XCTAssertEqual(CandidateProcessor.removeDisplayDuplicates(words).count, 2)
    }

    // MARK: - INVARIANT wrappers

    func test_INVARIANT_engine_dedup_keys_on_roman_plus_hanzi() {
        let differentHanzi = [word("kau3", "教"), word("kau3", "猴")]
        XCTAssertEqual(CandidateProcessor.removeDuplicates(differentHanzi).count, 2)
        let bothNilHanzi = [word("kau3"), word("kau3")]
        XCTAssertEqual(CandidateProcessor.removeDuplicates(bothNilHanzi).count, 1)
    }

    func test_INVARIANT_display_dedup_runs_after_sort() {
        let alreadySorted = [word("hi-score", "伴手"), word("low-score", "伴手")]
        let result = CandidateProcessor.removeDisplayDuplicates(alreadySorted)
        XCTAssertEqual(result.map(\.roman), ["hi-score"])
    }

    func test_INVARIANT_display_dedup_keeps_words_without_hanzi() {
        let words = [word("a"), word("a"), word("同", "相同"), word("x", "相同")]
        let result = CandidateProcessor.removeDisplayDuplicates(words)
        XCTAssertEqual(result.map(\.roman), ["a", "a", "同"])
    }

    func test_INVARIANT_score_is_deterministic() {
        let w = word("tai5-gi2", "台語", lengthScore: 500)
        let freq = FrequencyData(count: 10, lastUsedMillis: fixedNow - 1)
        let a = CandidateProcessor.calculateScore(word: w, normalizedInput: "tai5gi2", frequencyData: freq, currentTime: fixedNow)
        let b = CandidateProcessor.calculateScore(word: w, normalizedInput: "tai5gi2", frequencyData: freq, currentTime: fixedNow)
        XCTAssertEqual(a.total, b.total)
        XCTAssertEqual(a.userFreqScore, b.userFreqScore)
        XCTAssertEqual(a.closenessBonus, b.closenessBonus)
    }

    func test_INVARIANT_user_freq_dominates_ranking() {
        // A low-frequency lengthy candidate with high base score must still rank below
        // a high-frequency short candidate with zero base score.
        let highFreq = word("tai-gi", "台語", lengthScore: 0)
        let baseOnly = word("tai-gi-bun-hak", "台語文學", lengthScore: 9999)
        let freqMap: [String: FrequencyData] = [
            highFreq.displayText: FrequencyData(count: 100, lastUsedMillis: 0),
        ]
        let sorted = CandidateProcessor.sortByScore(
            [baseOnly, highFreq],
            normalizedInput: "taigi",
            frequencyDataMap: freqMap,
            currentTime: fixedNow,
        )
        XCTAssertEqual(sorted.first?.displayText, "台語")
    }

    func test_INVARIANT_completion_penalty_separates_tiers() {
        let exact = word("tai-gi", "台語", lengthScore: 100)
        let completion = word("tai-gi-bun", "台語文", lengthScore: 100)
        let emptyFreq: [String: FrequencyData] = [:]
        let sorted = CandidateProcessor.sortByScore(
            [completion, exact],
            normalizedInput: "taigi",
            frequencyDataMap: emptyFreq,
            currentTime: fixedNow,
        )
        // Exact match must outrank completion in cold-start — the -1000 penalty proves it.
        XCTAssertEqual(sorted.first?.displayText, "台語")
    }

    func test_INVARIANT_recency_window_is_exactly_1_hour() {
        // Boundary: `(now - lastUsed) < 1h` is strict less-than.
        let oneHourMs: Int64 = 60 * 60 * 1000
        let w = word("hi", "哈", lengthScore: 0)
        let justInside = FrequencyData(count: 1, lastUsedMillis: fixedNow - oneHourMs + 1)
        let exactly = FrequencyData(count: 1, lastUsedMillis: fixedNow - oneHourMs)
        let insideBreakdown = CandidateProcessor.calculateScore(word: w, normalizedInput: "hi", frequencyData: justInside, currentTime: fixedNow)
        let edgeBreakdown = CandidateProcessor.calculateScore(word: w, normalizedInput: "hi", frequencyData: exactly, currentTime: fixedNow)
        XCTAssertEqual(insideBreakdown.recencyBonus, 200)
        XCTAssertEqual(edgeBreakdown.recencyBonus, 0, "Exactly 1 hour old must NOT receive the recency bonus")
    }

    func test_INVARIANT_roman_to_base_strips_tones_hyphens_digits() {
        // Probe private `romanToBase` via the `exactBonus` branch of `calculateScore`:
        // identical base forms → exactBonus == 100, completionPenalty == 0.
        let hyphenated = word("tāi-tsì", "代誌", lengthScore: 0)
        let breakdown = CandidateProcessor.calculateScore(
            word: hyphenated,
            normalizedInput: "tai3tsi3",
            frequencyData: .empty,
            currentTime: fixedNow,
        )
        XCTAssertEqual(breakdown.exactBonus, 100)
        XCTAssertEqual(breakdown.completionPenalty, 0)
    }

    // MARK: - Tier bonus invariants

    func test_INVARIANT_tier_bonus_preserves_frequency_ordering() {
        // Assert baseFreqScore arithmetic directly to isolate the tier math from
        // closeness/exact/completion noise. Using identical roman + input
        // neutralizes those components across both candidates.
        // Kautian tier-1 (1.5×): lengthScore=100 → baseFreqScore = 100/10 * 15/10 = 15.
        // Default tier        : lengthScore=160 → baseFreqScore = 160/10 * 10/10 = 16.
        // Max tier bonus (1.5×) cannot invert a strictly >1.5× raw-frequency gap.
        let kautianWord = word("x", "A", lengthScore: 100, sourceBitmask: TierBit.kautian)
        let defaultWord = word("x", "B", lengthScore: 160, sourceBitmask: TierBit.itaigi)
        let kautianBreakdown = CandidateProcessor.calculateScore(
            word: kautianWord, normalizedInput: "x", frequencyData: .empty, currentTime: fixedNow,
        )
        let defaultBreakdown = CandidateProcessor.calculateScore(
            word: defaultWord, normalizedInput: "x", frequencyData: .empty, currentTime: fixedNow,
        )
        XCTAssertEqual(kautianBreakdown.baseFreqScore, 15)
        XCTAssertEqual(defaultBreakdown.baseFreqScore, 16)
        XCTAssertGreaterThan(defaultBreakdown.baseFreqScore, kautianBreakdown.baseFreqScore)
    }

    func test_INVARIANT_tier_bonus_first_match_wins() {
        // Bitmask with kautian (bit 0) + kungge (bit 6) set. First-match-wins → kautian (1.5×) applies.
        // A kautian+kungge word with lengthScore=100 → baseFreqScore = 15 (not 11).
        // A pure kungge word with lengthScore=100 → baseFreqScore = 11.
        // A kautian-only word with lengthScore=100 → baseFreqScore = 15.
        // Both-bits word must tie with kautian-only, strictly beat kungge-only.
        let both = word("tl-both", "Both", lengthScore: 100, sourceBitmask: TierBit.kautian | TierBit.kungge)
        let kautianOnly = word("tl-kau", "Kau", lengthScore: 100, sourceBitmask: TierBit.kautian)
        let kunggeOnly = word("tl-kun", "Kun", lengthScore: 100, sourceBitmask: TierBit.kungge)
        let bothScore = CandidateProcessor.calculateScore(word: both, normalizedInput: "x", frequencyData: .empty, currentTime: fixedNow)
        let kautianScore = CandidateProcessor.calculateScore(word: kautianOnly, normalizedInput: "x", frequencyData: .empty, currentTime: fixedNow)
        let kunggeScore = CandidateProcessor.calculateScore(word: kunggeOnly, normalizedInput: "x", frequencyData: .empty, currentTime: fixedNow)
        XCTAssertEqual(bothScore.baseFreqScore, kautianScore.baseFreqScore, "kautian+kungge → kautian tier wins")
        XCTAssertGreaterThan(bothScore.baseFreqScore, kunggeScore.baseFreqScore, "kautian+kungge must outrank kungge-only")
    }

    func test_kautian_beats_itaigi_at_comparable_frequency() {
        // Kautian lengthScore=100 → baseFreqScore = 15. Itaigi lengthScore=110 → 11. Kautian wins.
        let kautianWord = word("tl-a", "A", lengthScore: 100, sourceBitmask: TierBit.kautian)
        let itaigiWord = word("tl-b", "B", lengthScore: 110, sourceBitmask: TierBit.itaigi)
        let sorted = CandidateProcessor.sortByScore(
            [itaigiWord, kautianWord],
            normalizedInput: "z",
            frequencyDataMap: [:],
            currentTime: fixedNow,
        )
        XCTAssertEqual(sorted.first?.hanzi, "A", "Kautian at comparable frequency must outrank itaigi")
    }
}
