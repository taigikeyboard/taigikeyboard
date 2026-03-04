import XCTest
@testable import TaigiKeyboard

/// ToneRestoration unit tests
final class ToneRestorationTests: XCTestCase {

    // MARK: - Individual Combining Mark Removal

    func testRestore_acuteAccent() {
        let result = ToneRestoration.restore("k\u{00E1}", mode: .tl)  // ká
        XCTAssertEqual(result, "ka")
    }

    func testRestore_graveAccent() {
        let result = ToneRestoration.restore("k\u{00E0}", mode: .tl)  // kà
        XCTAssertEqual(result, "ka")
    }

    func testRestore_circumflex() {
        let result = ToneRestoration.restore("k\u{00E2}", mode: .tl)  // kâ
        XCTAssertEqual(result, "ka")
    }

    func testRestore_macron() {
        let result = ToneRestoration.restore("k\u{0101}", mode: .tl)  // kā
        XCTAssertEqual(result, "ka")
    }

    func testRestore_verticalLine() {
        let result = ToneRestoration.restore("ka\u{030D}", mode: .tl)  // ka̍
        XCTAssertEqual(result, "ka")
    }

    func testRestore_breve() {
        let result = ToneRestoration.restore("k\u{0103}", mode: .poj)  // kă
        XCTAssertEqual(result, "ka")
    }

    func testRestore_doubleAcute() {
        let result = ToneRestoration.restore("ka\u{030B}", mode: .tl)
        XCTAssertEqual(result, "ka")
    }

    // MARK: - Multi-Syllable Behavior

    func testRestore_multiSyllable_removesLastMarkOnly() {
        // tâi-gí -> should remove mark from gí (last mark) -> tâi-gi
        let result = ToneRestoration.restore("t\u{00E2}i-g\u{00ED}", mode: .tl)
        XCTAssertNotNil(result, "restore(\"tâi-gí\") should not be nil")
        // The last mark (acute on í) should be removed
        XCTAssertTrue(result!.contains("gi"), "Last mark should be removed: got \(result!)")
        // The first mark (circumflex on â) should remain
        XCTAssertTrue(
            result!.contains("\u{00E2}") || result!.contains("a\u{0302}"),
            "First mark should be preserved: got \(result!)"
        )
    }

    // MARK: - No Mark / Empty

    func testRestore_noMark_returnsNil() {
        let result = ToneRestoration.restore("ka", mode: .tl)
        XCTAssertNil(result, "restore(\"ka\") should be nil (no tone mark)")
    }

    func testRestore_empty_returnsNil() {
        let result = ToneRestoration.restore("", mode: .tl)
        XCTAssertNil(result, "restore(\"\") should be nil")
    }

    func testRestore_plainDigit_returnsNil() {
        // Digits are not combining marks, so restore returns nil
        let result = ToneRestoration.restore("ka2", mode: .tl)
        XCTAssertNil(result, "restore(\"ka2\") should be nil (digit is not a combining mark)")
    }

    // MARK: - Complex Syllables

    func testRestore_complexSyllable() {
        // tshiū -> remove macron -> tshiu
        let result = ToneRestoration.restore("tshi\u{016B}", mode: .tl)
        XCTAssertEqual(result, "tshiu")
    }
}
