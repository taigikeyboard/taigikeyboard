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
}
