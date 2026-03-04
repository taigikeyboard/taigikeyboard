import XCTest
@testable import TaigiKeyboard

/// ToneUtilities unit tests
final class ToneUtilitiesTests: XCTestCase {

    // MARK: - uppercaseToneLetter

    func testUppercaseToneLetter_nasalN() {
        // ⁿ (U+207F) -> ᴺ (U+1D3A)
        XCTAssertEqual(ToneUtilities.uppercaseToneLetter("\u{207F}", mode: .poj), "\u{1D3A}")
    }

    func testUppercaseToneLetter_regularToneLetter() {
        XCTAssertEqual(ToneUtilities.uppercaseToneLetter("a", mode: .tl), "A")
        XCTAssertEqual(ToneUtilities.uppercaseToneLetter("k", mode: .tl), "K")
    }

    func testUppercaseToneLetter_toneMarkedLetter() {
        // á -> Á (Swift uppercased handles combining marks)
        let result = ToneUtilities.uppercaseToneLetter("\u{00E1}", mode: .tl)
        XCTAssertEqual(result, "\u{00C1}")
    }

    func testUppercaseToneLetter_alreadyUppercase() {
        XCTAssertEqual(ToneUtilities.uppercaseToneLetter("A", mode: .tl), "A")
    }

    // MARK: - lowercaseToneLetter

    func testLowercaseToneLetter_nasalN() {
        // ᴺ (U+1D3A) -> ⁿ (U+207F)
        XCTAssertEqual(ToneUtilities.lowercaseToneLetter("\u{1D3A}", mode: .poj), "\u{207F}")
    }

    func testLowercaseToneLetter_regularLetter() {
        XCTAssertEqual(ToneUtilities.lowercaseToneLetter("A", mode: .tl), "a")
        XCTAssertEqual(ToneUtilities.lowercaseToneLetter("K", mode: .tl), "k")
    }

    func testLowercaseToneLetter_toneMarkedLetter() {
        // Á -> á
        let result = ToneUtilities.lowercaseToneLetter("\u{00C1}", mode: .tl)
        XCTAssertEqual(result, "\u{00E1}")
    }

    func testLowercaseToneLetter_alreadyLowercase() {
        XCTAssertEqual(ToneUtilities.lowercaseToneLetter("a", mode: .tl), "a")
    }

    // MARK: - adjustNasalMarkerCase

    func testAdjustNasalMarkerCase_lowercaseVowel_unchanged() {
        // ê lowercase → ⁿ stays
        let result = ToneUtilities.adjustNasalMarkerCase("p\u{00EA}\u{207F}")  // pêⁿ
        XCTAssertEqual(result, "p\u{00EA}\u{207F}", "Lowercase vowel should keep ⁿ")
    }

    func testAdjustNasalMarkerCase_uppercaseVowel_becomesUpperNasal() {
        // Ê uppercase → ⁿ becomes ᴺ
        let result = ToneUtilities.adjustNasalMarkerCase("P\u{00CA}\u{207F}")  // PÊⁿ → PÊᴺ
        XCTAssertEqual(result, "P\u{00CA}\u{1D3A}", "Uppercase vowel should produce ᴺ")
    }

    func testAdjustNasalMarkerCase_alreadyCorrectUpper() {
        // PÊᴺ already correct
        let result = ToneUtilities.adjustNasalMarkerCase("P\u{00CA}\u{1D3A}")  // PÊᴺ
        XCTAssertEqual(result, "P\u{00CA}\u{1D3A}", "Already correct uppercase nasal should stay")
    }

    func testAdjustNasalMarkerCase_wrongCaseUpper_becomesLower() {
        // Pêᴺ → Pêⁿ (ê is lowercase, so nasal should be ⁿ)
        let result = ToneUtilities.adjustNasalMarkerCase("P\u{00EA}\u{1D3A}")  // PêᴺPêⁿ
        XCTAssertEqual(result, "P\u{00EA}\u{207F}", "Lowercase vowel before ᴺ should fix to ⁿ")
    }

    func testAdjustNasalMarkerCase_hyphenatedText() {
        // pêⁿ-á stays unchanged
        let result = ToneUtilities.adjustNasalMarkerCase("p\u{00EA}\u{207F}-\u{00E1}")
        XCTAssertEqual(result, "p\u{00EA}\u{207F}-\u{00E1}", "Hyphen should not affect nasal case")
    }

    func testAdjustNasalMarkerCase_noPrecedingLetter_defaultsToLower() {
        // No preceding letter → default ⁿ
        let result = ToneUtilities.adjustNasalMarkerCase("\u{1D3A}")
        XCTAssertEqual(result, "\u{207F}", "No preceding letter should default to ⁿ")
    }
}

