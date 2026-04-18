@testable import TaigiKeyboard
import XCTest

/// Unit tests for `AutocompleteInputClassifier` pure-logic phase.
final class AutocompleteInputClassifierTests: XCTestCase {
    // MARK: - determineInputType

    func testDetermineInputType_hanzi() {
        XCTAssertEqual(AutocompleteInputClassifier.determineInputType("台語"), .hanzi)
    }

    func testDetermineInputType_romanWithDiacritic() {
        XCTAssertEqual(AutocompleteInputClassifier.determineInputType("guá"), .romanWithTone)
    }

    func testDetermineInputType_numericToneDigits() {
        for tone in ["2", "3", "5", "6", "7", "8", "9"] {
            XCTAssertEqual(
                AutocompleteInputClassifier.determineInputType("gua\(tone)"),
                .romanWithTone,
                "tone digit \(tone) should classify as romanWithTone",
            )
        }
    }

    func testDetermineInputType_tone1And4AreNotTone() {
        // soo1 / peh4 are considered tone-less because 1 and 4 carry no diacritic
        XCTAssertEqual(AutocompleteInputClassifier.determineInputType("soo1"), .romanWithoutTone)
        XCTAssertEqual(AutocompleteInputClassifier.determineInputType("peh4"), .romanWithoutTone)
    }

    func testDetermineInputType_tone0IsNotTone() {
        XCTAssertEqual(AutocompleteInputClassifier.determineInputType("foo0"), .romanWithoutTone)
    }

    func testDetermineInputType_romanWithoutTone() {
        XCTAssertEqual(AutocompleteInputClassifier.determineInputType("gua"), .romanWithoutTone)
    }

    func testDetermineInputType_emptyString() {
        XCTAssertEqual(AutocompleteInputClassifier.determineInputType(""), .romanWithoutTone)
    }

    // MARK: - buildSearchKey

    func testBuildSearchKey_nonTPSReturnedAsIs() {
        XCTAssertEqual(AutocompleteInputClassifier.buildSearchKey(from: "gua2"), "gua2")
        XCTAssertEqual(AutocompleteInputClassifier.buildSearchKey(from: "台"), "台")
    }

    // MARK: - classify end-to-end

    func testClassify_combinesTypeAndSearchKey() {
        let result = AutocompleteInputClassifier.classify(rawInput: "gua2")
        XCTAssertEqual(result.inputType, .romanWithTone)
        XCTAssertEqual(result.searchKey, "gua2")
    }

    func testClassify_hanzi() {
        let result = AutocompleteInputClassifier.classify(rawInput: "我")
        XCTAssertEqual(result.inputType, .hanzi)
        XCTAssertEqual(result.searchKey, "我")
    }
}
