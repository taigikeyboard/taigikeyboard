@testable import TaigiKeyboard
import XCTest

/// Covers the iOS-side helpers retained on `CandidateProcessor`:
/// `isHanzi` (text classification — used outside `LexiconService`) and
/// the cold-start fallback dedup helpers (`removeDuplicates`,
/// `removeDisplayDuplicates`) reached when the user-frequency DB has not
/// yet warmed up.
///
/// The score / sort / tier-bonus invariants moved to
/// `RustEngineBridgeRankingTests` once production ranking swapped to
/// `RustEngineBridge.processCandidates` in v3.5.2.
final class CandidateProcessorTests: XCTestCase {
    // MARK: - Fixtures

    private func word(_ roman: String, _ hanzi: String? = nil, lengthScore: Int? = nil, sourceBitmask: UInt16? = nil) -> TaigiWord {
        TaigiWord(id: 0, roman: roman, hanzi: hanzi, lengthScore: lengthScore, sourceBitmask: sourceBitmask)
    }

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

    // MARK: - removeDuplicates (cold-start fallback)

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

    // MARK: - removeDisplayDuplicates (cold-start fallback)

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

    // MARK: - INVARIANT wrappers (cold-start dedup invariants)

    func test_INVARIANT_cold_start_dedup_keys_on_roman_plus_hanzi() {
        let differentHanzi = [word("kau3", "教"), word("kau3", "猴")]
        XCTAssertEqual(CandidateProcessor.removeDuplicates(differentHanzi).count, 2)
        let bothNilHanzi = [word("kau3"), word("kau3")]
        XCTAssertEqual(CandidateProcessor.removeDuplicates(bothNilHanzi).count, 1)
    }

    func test_INVARIANT_cold_start_display_dedup_runs_after_sort() {
        let alreadySorted = [word("hi-score", "伴手"), word("low-score", "伴手")]
        let result = CandidateProcessor.removeDisplayDuplicates(alreadySorted)
        XCTAssertEqual(result.map(\.roman), ["hi-score"])
    }

    func test_INVARIANT_cold_start_display_dedup_keeps_words_without_hanzi() {
        let words = [word("a"), word("a"), word("同", "相同"), word("x", "相同")]
        let result = CandidateProcessor.removeDisplayDuplicates(words)
        XCTAssertEqual(result.map(\.roman), ["a", "a", "同"])
    }
}
