@testable import TaigiKeyboard
import XCTest

/// Covers the iOS-side helper retained on `CandidateProcessor`:
/// `isHanzi` (text classification — used outside `LexiconService`).
///
/// Score / sort / tier / dedup invariants moved to the Rust shared core
/// in v3.5.2 (`engine/ranking/`); the cold-start dedup invariants moved
/// in the v3.5.3 follow-up (PR #192) once `LexiconService` cold-start
/// started routing through
/// `RustEngineBridge.processCandidates(..., mergeOrderOnly: true)`. The
/// FFI-boundary parity tests live in `RustEngineBridgeRankingTests`;
/// algorithm tests live in `engine/ranking/` + `engine/phonetics/tests/`.
final class CandidateProcessorTests: XCTestCase {
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
}
