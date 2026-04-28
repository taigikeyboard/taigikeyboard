@testable import TaigiKeyboard
import XCTest

/// Parity coverage for the v3.5.2 ranking slice (Rust shared-core
/// `engine/ranking/`). Mirrors the score / sort / tier-bonus invariants
/// previously exercised against `CandidateProcessor.calculateScore` /
/// `sortByScore` — those Swift helpers are deleted once production
/// routing swaps to `RustEngineBridge.processCandidates`.
///
/// Each test drives the FFI through `processCandidatesDetailed` so the
/// engine's `ScoreBreakdown` payload (six fields summing to the sort
/// key) is asserted directly. The corresponding Rust-side tests live in
/// `engine/ranking/src/score.rs` + `process.rs`; this file is the iOS
/// boundary check that proto marshalling + dispatcher routing don't
/// drift between the platform and the engine.
final class RustEngineBridgeRankingTests: XCTestCase {
    override class func setUp() {
        super.setUp()
        RustEngineBridge.install()
    }

    // MARK: - Fixtures

    private func word(_ roman: String, _ hanzi: String? = nil, lengthScore: Int? = nil, sourceBitmask: UInt16? = nil) -> TaigiWord {
        TaigiWord(id: 0, roman: roman, hanzi: hanzi, lengthScore: lengthScore, sourceBitmask: sourceBitmask)
    }

    /// Tier-bitmask convenience for readability in tier-invariant tests.
    /// Bit positions mirror dictionary/common/source_bits.py.
    private enum TierBit {
        static let kautian: UInt16 = 1 << 0
        static let taigitv: UInt16 = 1 << 1
        static let itaigi: UInt16 = 1 << 2
        static let kungge: UInt16 = 1 << 6
        static let stti: UInt16 = 1 << 7
    }

    private let fixedNow: Int64 = 1_700_000_000_000 // Arbitrary pinned epoch ms

    private func breakdown(
        for word: TaigiWord,
        normalizedInput: String,
        frequencyData: FrequencyData,
        nowMs: Int64,
    ) -> RustEngineBridge.ScoreBreakdown {
        let result = RustEngineBridge.processCandidatesDetailed(
            raw: [word],
            normalizedInput: normalizedInput,
            tpsDedupEnabled: false,
            frequencyData: [word.displayText: frequencyData],
            nowMs: nowMs,
            includeBreakdown: true,
        )
        XCTAssertEqual(result.ranked.count, 1, "single-word fixture should round-trip exactly one ranked entry")
        XCTAssertEqual(result.breakdowns.count, 1, "include_breakdown=true must populate exactly one breakdown")
        return result.breakdowns[0]
    }

    private func sortByScore(
        _ words: [TaigiWord],
        normalizedInput: String,
        frequencyDataMap: [String: FrequencyData] = [:],
        nowMs: Int64,
    ) -> [TaigiWord] {
        RustEngineBridge.processCandidates(
            raw: words,
            normalizedInput: normalizedInput,
            tpsDedupEnabled: false,
            frequencyData: frequencyDataMap,
            nowMs: nowMs,
        )
    }

    // MARK: - INVARIANT: scoring math

    func test_INVARIANT_score_is_deterministic() {
        let w = word("tai5-gi2", "台語", lengthScore: 500)
        let freq = FrequencyData(count: 10, lastUsedMillis: fixedNow - 1)
        let a = breakdown(for: w, normalizedInput: "tai5gi2", frequencyData: freq, nowMs: fixedNow)
        let b = breakdown(for: w, normalizedInput: "tai5gi2", frequencyData: freq, nowMs: fixedNow)
        XCTAssertEqual(a, b)
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
        let sorted = sortByScore(
            [baseOnly, highFreq],
            normalizedInput: "taigi",
            frequencyDataMap: freqMap,
            nowMs: fixedNow,
        )
        XCTAssertEqual(sorted.first?.displayText, "台語")
    }

    func test_INVARIANT_completion_penalty_separates_tiers() {
        let exact = word("tai-gi", "台語", lengthScore: 100)
        let completion = word("tai-gi-bun", "台語文", lengthScore: 100)
        let sorted = sortByScore(
            [completion, exact],
            normalizedInput: "taigi",
            nowMs: fixedNow,
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
        let insideBreakdown = breakdown(for: w, normalizedInput: "hi", frequencyData: justInside, nowMs: fixedNow)
        let edgeBreakdown = breakdown(for: w, normalizedInput: "hi", frequencyData: exactly, nowMs: fixedNow)
        XCTAssertEqual(insideBreakdown.recencyBonus, 200)
        XCTAssertEqual(edgeBreakdown.recencyBonus, 0, "Exactly 1 hour old must NOT receive the recency bonus")
    }

    func test_INVARIANT_roman_to_base_strips_tones_hyphens_digits() {
        // Probe the engine's `roman_to_base` via the `exactBonus` branch:
        // identical base forms → exactBonus == 100, completionPenalty == 0.
        let hyphenated = word("tāi-tsì", "代誌", lengthScore: 0)
        let bd = breakdown(
            for: hyphenated,
            normalizedInput: "tai3tsi3",
            frequencyData: .empty,
            nowMs: fixedNow,
        )
        XCTAssertEqual(bd.exactBonus, 100)
        XCTAssertEqual(bd.completionPenalty, 0)
    }

    // MARK: - INVARIANT: tier bonus arithmetic

    func test_INVARIANT_tier_bonus_preserves_frequency_ordering() {
        // Assert baseFreqScore arithmetic directly to isolate the tier math from
        // closeness/exact/completion noise. Using identical roman + input
        // neutralizes those components across both candidates.
        // Kautian tier-1 (1.5×): lengthScore=100 → baseFreqScore = 100/10 * 15/10 = 15.
        // Default tier        : lengthScore=160 → baseFreqScore = 160/10 * 10/10 = 16.
        // Max tier bonus (1.5×) cannot invert a strictly >1.5× raw-frequency gap.
        let kautianWord = word("x", "A", lengthScore: 100, sourceBitmask: TierBit.kautian)
        let defaultWord = word("x", "B", lengthScore: 160, sourceBitmask: TierBit.itaigi)
        let kautianBreakdown = breakdown(for: kautianWord, normalizedInput: "x", frequencyData: .empty, nowMs: fixedNow)
        let defaultBreakdown = breakdown(for: defaultWord, normalizedInput: "x", frequencyData: .empty, nowMs: fixedNow)
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
        let bothScore = breakdown(for: both, normalizedInput: "x", frequencyData: .empty, nowMs: fixedNow)
        let kautianScore = breakdown(for: kautianOnly, normalizedInput: "x", frequencyData: .empty, nowMs: fixedNow)
        let kunggeScore = breakdown(for: kunggeOnly, normalizedInput: "x", frequencyData: .empty, nowMs: fixedNow)
        XCTAssertEqual(bothScore.baseFreqScore, kautianScore.baseFreqScore, "kautian+kungge → kautian tier wins")
        XCTAssertGreaterThan(bothScore.baseFreqScore, kunggeScore.baseFreqScore, "kautian+kungge must outrank kungge-only")
    }

    func test_kautian_beats_itaigi_at_comparable_frequency() {
        // Kautian lengthScore=100 → baseFreqScore = 15. Itaigi lengthScore=110 → 11. Kautian wins.
        let kautianWord = word("tl-a", "A", lengthScore: 100, sourceBitmask: TierBit.kautian)
        let itaigiWord = word("tl-b", "B", lengthScore: 110, sourceBitmask: TierBit.itaigi)
        let sorted = sortByScore(
            [itaigiWord, kautianWord],
            normalizedInput: "z",
            nowMs: fixedNow,
        )
        XCTAssertEqual(sorted.first?.hanzi, "A", "Kautian at comparable frequency must outrank itaigi")
    }

    // MARK: - INVARIANT: pipeline orchestration

    func test_INVARIANT_engine_dedup_runs_before_score() {
        // Duplicate roman+hanzi must collapse before scoring; ranked.count
        // proves the engine's `remove_duplicates` ran inside processCandidates.
        let dup = [word("gua", "我", lengthScore: 100), word("gua", "我", lengthScore: 100)]
        let ranked = sortByScore(dup, normalizedInput: "gua", nowMs: fixedNow)
        XCTAssertEqual(ranked.count, 1, "engine must dedup before sorting")
    }

    func test_INVARIANT_tps_dedup_gate_collapses_homographs() {
        // tpsDedupEnabled=true: same hanzi, different roman → keep top-scoring only.
        // The engine sorts then display-dedups, mirroring the legacy
        // `removeDisplayDuplicates(sortByScore(...))` order. Differentiate
        // the two candidates via `lengthScore` so the survivor is
        // identifiable: the higher base-freq entry must win, proving sort
        // happens before display-dedup.
        let highBase = word("phuānn-tshiú", "伴手", lengthScore: 100)
        let lowBase = word("phuǎnn-tshiú", "伴手", lengthScore: 10)
        // Both share displayText "伴手" so they share the same frequency
        // entry; ranking divergence must come from baseFreqScore alone.
        let freqMap = [highBase.displayText: FrequencyData(count: 0, lastUsedMillis: 0)]
        let ranked = RustEngineBridge.processCandidates(
            raw: [lowBase, highBase],
            normalizedInput: "phuannchiu",
            tpsDedupEnabled: true,
            frequencyData: freqMap,
            nowMs: fixedNow,
        )
        XCTAssertEqual(ranked.count, 1, "tpsDedupEnabled must collapse homograph hanzi after sort")
        XCTAssertEqual(ranked.first?.hanzi, "伴手")
        XCTAssertEqual(
            ranked.first?.roman,
            "phuānn-tshiú",
            "higher-base-freq entry must survive — proves sort runs before display-dedup",
        )
    }

    func test_INVARIANT_breakdown_total_equals_sum_of_six_fields() {
        let w = word("tai-gi", "台語", lengthScore: 100, sourceBitmask: TierBit.kautian)
        let freq = FrequencyData(count: 5, lastUsedMillis: fixedNow - 1000)
        let bd = breakdown(for: w, normalizedInput: "taigi", frequencyData: freq, nowMs: fixedNow)
        let manualTotal = bd.userFreqScore + bd.recencyBonus + bd.exactBonus
            + bd.completionPenalty + bd.closenessBonus + bd.baseFreqScore
        XCTAssertEqual(bd.total, manualTotal)
    }
}
