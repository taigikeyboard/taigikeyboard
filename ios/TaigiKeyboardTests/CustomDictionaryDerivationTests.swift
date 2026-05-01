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

    // MARK: - generateRomanNum (delegates to InputNormalizer)

    func testGenerateRomanNum_delegatesToInputNormalizer() {
        // Parity check — generateRomanNum is documented as RustEngineBridge.normalizeInput(_).
        XCTAssertEqual(CustomDictionaryDerivation.generateRomanNum("gâu-tsá"), "gau5tsa2")
    }

    // MARK: - searchPrefix routing

    func testSearchPrefix_toneAware() {
        let (key, toneAware) = CustomDictionaryDerivation.searchPrefix(for: "ho2")
        XCTAssertTrue(toneAware)
        XCTAssertEqual(key, "ho2")
    }

    func testSearchPrefix_toneAware_stripsHyphens() {
        let (key, toneAware) = CustomDictionaryDerivation.searchPrefix(for: "gau5-tsa2")
        XCTAssertTrue(toneAware)
        XCTAssertEqual(key, "gau5tsa2")
    }

    func testSearchPrefix_toneless() {
        let (key, toneAware) = CustomDictionaryDerivation.searchPrefix(for: "hó")
        XCTAssertFalse(toneAware)
        XCTAssertEqual(key, "ho")
    }

    // MARK: - INVARIANT wrappers — Phase 0 §10

    func test_INVARIANT_custom_derivation_matches_input_normalizer() {
        // The `roman_num` key must be exactly what RustEngineBridge.normalizeInput(_)
        // produces, or custom-dictionary entries become invisible through the main search.
        let fixtures = ["gâu-tsá", "tāi-tsì", "hó", "tsiah8-pá"]
        for input in fixtures {
            XCTAssertEqual(
                CustomDictionaryDerivation.generateRomanNum(input),
                RustEngineBridge.normalizeInput(input),
                "Parity failure on \(input)",
            )
        }
    }

    func test_INVARIANT_abbrev_key_is_one_char_per_syllable() {
        XCTAssertEqual(CustomDictionaryDerivation.generateAbbrev("gâu-tsá"), "gt")
        XCTAssertEqual(CustomDictionaryDerivation.generateAbbrev("lí-hó-bô"), "lhb")
        XCTAssertEqual(CustomDictionaryDerivation.generateAbbrev("saⁿ-á-kûn"), "sak")
    }
}
