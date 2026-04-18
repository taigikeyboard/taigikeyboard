@testable import TaigiKeyboard
import XCTest

/// ToneConverter unit tests
final class ToneConverterTests: XCTestCase {
    override func setUp() {
        super.setUp()
        SharedSettings.shared.isDoubleTapOOEnabled = true
        SharedSettings.shared.isDoubleTapNNEnabled = true
    }

    override func tearDown() {
        SharedSettings.shared.isDoubleTapOOEnabled = true
        SharedSettings.shared.isDoubleTapNNEnabled = true
        super.tearDown()
    }

    // MARK: - TL Mode (No Preprocessing)

    func testConvertToToneMarks_tlMode_multiSyllable() {
        let result = ToneConverter.convertToToneMarks("ho2-se3", mode: .tl)
        XCTAssertEqual(result, "h\u{00F3}-s\u{00E8}") // hó-sè
    }

    func testConvertToToneMarks_tlMode_noPreprocessing() {
        // In TL mode, "oo" stays "oo" (no o͘ conversion), tone 2 = acute
        let result = ToneConverter.convertToToneMarks("hoo2", mode: .tl)
        XCTAssertEqual(result, "h\u{00F3}o") // hóo (tone 2 acute on first o)
    }

    // MARK: - POJ Mode + OO Enabled

    func testConvertToToneMarks_pojMode_ooEnabled() {
        SharedSettings.shared.isDoubleTapOOEnabled = true
        let result = ToneConverter.convertToToneMarks("hoo2", mode: .poj)
        // oo -> o͘ first, then tone mark applied
        XCTAssertTrue(
            result.unicodeScalars.contains { $0 == "\u{0358}" },
            "POJ with OO enabled should produce o͘: got \(result)",
        )
    }

    // MARK: - POJ Mode + NN Enabled

    func testConvertToToneMarks_pojMode_nnEnabled() {
        SharedSettings.shared.isDoubleTapNNEnabled = true
        let result = ToneConverter.convertToToneMarks("ann2", mode: .poj)
        XCTAssertTrue(
            result.contains("\u{207F}"),
            "POJ with NN enabled should produce ⁿ: got \(result)",
        )
    }

    // MARK: - POJ Mode + OO Disabled

    func testConvertToToneMarks_pojMode_ooDisabled() {
        SharedSettings.shared.isDoubleTapOOEnabled = false
        let result = ToneConverter.convertToToneMarks("hoo2", mode: .poj)
        // Even with OO disabled, toPOJ always outputs o͘ (POJ standard format).
        // The setting only affects keyboard input preprocessing, not output format.
        XCTAssertTrue(
            result.unicodeScalars.contains { $0 == "\u{0358}" },
            "POJ output always uses o͘ regardless of OO setting: got \(result)",
        )
    }

    // MARK: - POJ Mode + NN Disabled

    func testConvertToToneMarks_pojMode_nnDisabled() {
        SharedSettings.shared.isDoubleTapNNEnabled = false
        let result = ToneConverter.convertToToneMarks("ann2", mode: .poj)
        // Even with NN disabled, toPOJ always outputs ⁿ (POJ standard format).
        // The setting only affects keyboard input preprocessing, not output format.
        XCTAssertTrue(
            result.unicodeScalars.contains { $0 == "\u{207F}" },
            "POJ output always uses ⁿ regardless of NN setting: got \(result)",
        )
    }

    // MARK: - English Passthrough

    func testConvertToToneMarks_englishMode() {
        let result = ToneConverter.convertToToneMarks("hello2", mode: .english)
        XCTAssertEqual(result, "hello2", "English mode should pass through without conversion")
    }

    func testConvertToToneMarks_englishMode_multiSyllable() {
        let result = ToneConverter.convertToToneMarks("ka2-lang5", mode: .english)
        XCTAssertEqual(result, "ka2-lang5")
    }

    // MARK: - Nasal Marker Case

    func testConvertToToneMarks_pojMode_capsLockNN() {
        SharedSettings.shared.isDoubleTapNNEnabled = true
        let result = ToneConverter.convertToToneMarks("ANN2", mode: .poj)
        // Caps lock: uppercase vowel A → nasal should be ᴺ
        XCTAssertTrue(
            result.contains("\u{1D3A}"),
            "Caps lock ANN2 should produce ᴺ (U+1D3A): got \(result)",
        )
    }

    func testConvertToToneMarks_pojMode_singleShiftNN() {
        SharedSettings.shared.isDoubleTapNNEnabled = true
        let result = ToneConverter.convertToToneMarks("Penn5", mode: .poj)
        // Single shift: P uppercase but e lowercase → nasal follows e → ⁿ
        XCTAssertTrue(
            result.contains("\u{207F}"),
            "Single shift Penn5 should produce ⁿ (U+207F): got \(result)",
        )
        XCTAssertFalse(
            result.contains("\u{1D3A}"),
            "Single shift Penn5 should NOT produce ᴺ: got \(result)",
        )
    }

    // MARK: - Edge Cases

    func testConvertToToneMarks_emptyString() {
        XCTAssertEqual(ToneConverter.convertToToneMarks("", mode: .tl), "")
        XCTAssertEqual(ToneConverter.convertToToneMarks("", mode: .poj), "")
    }

    func testConvertToToneMarks_pojMode_uppercaseOO() {
        SharedSettings.shared.isDoubleTapOOEnabled = true
        // Oo -> O͘, OO -> O͘
        let result1 = ToneConverter.convertToToneMarks("Oo2", mode: .poj)
        XCTAssertTrue(
            result1.unicodeScalars.contains { $0 == "\u{0358}" },
            "Uppercase Oo should also convert: got \(result1)",
        )
    }
}
