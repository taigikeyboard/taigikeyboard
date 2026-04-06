@testable import TaigiKeyboard
import XCTest

/// CustomDictionaryService 純函數測試
///
/// NOTE: 新建的測試檔案，需要手動加入 Xcode TaigiKeyboardTests target
final class CustomDictionaryServiceTests: XCTestCase {
    // MARK: - generateNotone (static)

    func testGenerateNotone_pojNasal() {
        // ⁿ (U+207F) → nn
        XCTAssertEqual(CustomDictionaryService.generateNotone("saⁿ"), "sann")
    }

    func testGenerateNotone_uppercaseNasal() {
        // ᴺ (U+1D3A) → nn
        XCTAssertEqual(CustomDictionaryService.generateNotone("saᴺ"), "sann")
    }

    func testGenerateNotone_toneMarksStripped() {
        XCTAssertEqual(CustomDictionaryService.generateNotone("hó"), "ho")
    }

    func testGenerateNotone_multiSyllable() {
        XCTAssertEqual(CustomDictionaryService.generateNotone("gâu-tsá"), "gautsa")
    }

    func testGenerateNotone_spaceStripped() {
        XCTAssertEqual(CustomDictionaryService.generateNotone("lí hó"), "liho")
    }

    func testGenerateNotone_digitsStripped() {
        XCTAssertEqual(CustomDictionaryService.generateNotone("ho2"), "ho")
    }

    func testGenerateNotone_numericMultiSyllable() {
        XCTAssertEqual(CustomDictionaryService.generateNotone("gau5-tsa2"), "gautsa")
    }

    func testGenerateNotone_mixedNasalAndDigit() {
        XCTAssertEqual(CustomDictionaryService.generateNotone("saⁿ2"), "sann")
    }

    // MARK: - generateAbbrev (static)

    func testGenerateAbbrev_twoSyllables() {
        XCTAssertEqual(CustomDictionaryService.generateAbbrev("gâu-tsá"), "gt")
    }

    func testGenerateAbbrev_nasalSyllable() {
        XCTAssertEqual(CustomDictionaryService.generateAbbrev("saⁿ-á"), "sa")
    }

    func testGenerateAbbrev_spaceSeparated() {
        XCTAssertEqual(CustomDictionaryService.generateAbbrev("lí hó"), "lh")
    }

    func testGenerateAbbrev_singleSyllable_empty() {
        XCTAssertEqual(CustomDictionaryService.generateAbbrev("saⁿ"), "")
    }

    func testGenerateAbbrev_singleWord_empty() {
        XCTAssertEqual(CustomDictionaryService.generateAbbrev("hó"), "")
    }
}
