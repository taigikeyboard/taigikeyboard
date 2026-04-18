@testable import TaigiKeyboard
import XCTest

/// Pure-function tests for `CustomDictionaryDerivation`.
final class CustomDictionaryDerivationTests: XCTestCase {
    // MARK: - generateNotone

    func testGenerateNotone_pojNasal() {
        // ⁿ (U+207F) → nn
        XCTAssertEqual(CustomDictionaryDerivation.generateNotone("saⁿ"), "sann")
    }

    func testGenerateNotone_uppercaseNasal() {
        // ᴺ (U+1D3A) → nn
        XCTAssertEqual(CustomDictionaryDerivation.generateNotone("saᴺ"), "sann")
    }

    func testGenerateNotone_toneMarksStripped() {
        XCTAssertEqual(CustomDictionaryDerivation.generateNotone("hó"), "ho")
    }

    func testGenerateNotone_multiSyllable() {
        XCTAssertEqual(CustomDictionaryDerivation.generateNotone("gâu-tsá"), "gautsa")
    }

    func testGenerateNotone_spaceStripped() {
        XCTAssertEqual(CustomDictionaryDerivation.generateNotone("lí hó"), "liho")
    }

    func testGenerateNotone_digitsStripped() {
        XCTAssertEqual(CustomDictionaryDerivation.generateNotone("ho2"), "ho")
    }

    func testGenerateNotone_numericMultiSyllable() {
        XCTAssertEqual(CustomDictionaryDerivation.generateNotone("gau5-tsa2"), "gautsa")
    }

    func testGenerateNotone_mixedNasalAndDigit() {
        XCTAssertEqual(CustomDictionaryDerivation.generateNotone("saⁿ2"), "sann")
    }

    // MARK: - generateAbbrev

    func testGenerateAbbrev_twoSyllables() {
        XCTAssertEqual(CustomDictionaryDerivation.generateAbbrev("gâu-tsá"), "gt")
    }

    func testGenerateAbbrev_nasalSyllable() {
        XCTAssertEqual(CustomDictionaryDerivation.generateAbbrev("saⁿ-á"), "sa")
    }

    func testGenerateAbbrev_spaceSeparated() {
        XCTAssertEqual(CustomDictionaryDerivation.generateAbbrev("lí hó"), "lh")
    }

    func testGenerateAbbrev_singleSyllable_empty() {
        XCTAssertEqual(CustomDictionaryDerivation.generateAbbrev("saⁿ"), "")
    }

    func testGenerateAbbrev_singleWord_empty() {
        XCTAssertEqual(CustomDictionaryDerivation.generateAbbrev("hó"), "")
    }
}
