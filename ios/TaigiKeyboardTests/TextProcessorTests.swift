import XCTest
@testable import TaigiKeyboard

/// CandidateProcessor unit tests
final class TextProcessorTests: XCTestCase {

    // MARK: - isHanzi

    func testIsHanzi_cjkMainRange() {
        XCTAssertTrue(CandidateProcessor.isHanzi("台"), "isHanzi(\"台\") should be true")
        XCTAssertTrue(CandidateProcessor.isHanzi("語"), "isHanzi(\"語\") should be true")
        XCTAssertTrue(CandidateProcessor.isHanzi("人"), "isHanzi(\"人\") should be true")
    }

    func testIsHanzi_extensionB() {
        XCTAssertTrue(CandidateProcessor.isHanzi("\u{20000}"), "isHanzi(U+20000) should be true (CJK Extension B)")
    }

    func testIsHanzi_latinOnly() {
        XCTAssertFalse(CandidateProcessor.isHanzi("hello"), "isHanzi(\"hello\") should be false")
        XCTAssertFalse(CandidateProcessor.isHanzi("abc"), "isHanzi(\"abc\") should be false")
    }

    func testIsHanzi_empty() {
        XCTAssertFalse(CandidateProcessor.isHanzi(""), "isHanzi(\"\") should be false")
    }

    func testIsHanzi_mixed() {
        XCTAssertTrue(CandidateProcessor.isHanzi("hello台語"), "isHanzi(\"hello台語\") should be true (contains Hanzi)")
    }

    func testIsHanzi_digits() {
        XCTAssertFalse(CandidateProcessor.isHanzi("12345"), "isHanzi(\"12345\") should be false")
    }

    func testIsHanzi_bopomofo() {
        XCTAssertFalse(CandidateProcessor.isHanzi("ㄅ"), "isHanzi(\"ㄅ\") should be false (Bopomofo is not Hanzi)")
        XCTAssertFalse(CandidateProcessor.isHanzi("ㄆ"), "isHanzi(\"ㄆ\") should be false")
    }

    // MARK: - removeDuplicates

    private func word(_ roman: String, _ hanzi: String? = nil) -> TaigiWord {
        TaigiWord(id: 0, roman: roman, hanzi: hanzi, lengthScore: nil)
    }

    func testRemoveDuplicates_exactDuplicate() {
        let words = [word("tâi-gí", "台語"), word("tâi-gí", "台語")]
        let result = CandidateProcessor.removeDuplicates(words)
        XCTAssertEqual(result.count, 1, "Exact duplicate (same roman + hanzi) should be removed")
    }

    func testRemoveDuplicates_toneVariantsPreserved() {
        // Different tones, same hanzi → both must be kept
        let words = [
            word("phuǎnn-tshiú", "伴手"),
            word("phuānn-tshiú", "伴手"),
        ]
        let result = CandidateProcessor.removeDuplicates(words)
        XCTAssertEqual(result.count, 2,
            "Tone variants with same hanzi must both be preserved: got \(result.map(\.roman))")
    }

    func testRemoveDuplicates_differentHanziPreserved() {
        let words = [word("kau3", "教"), word("kau3", "猴")]
        let result = CandidateProcessor.removeDuplicates(words)
        XCTAssertEqual(result.count, 2, "Same roman but different hanzi should both be kept")
    }

    func testRemoveDuplicates_romanOnlyNoDuplicates() {
        let words = [word("hello", nil), word("world", nil)]
        let result = CandidateProcessor.removeDuplicates(words)
        XCTAssertEqual(result.count, 2, "Different roman-only entries should both be kept")
    }

    func testRemoveDuplicates_romanOnlyDuplicate() {
        let words = [word("hello", nil), word("hello", nil)]
        let result = CandidateProcessor.removeDuplicates(words)
        XCTAssertEqual(result.count, 1, "Identical roman-only entries should be deduped")
    }
}
