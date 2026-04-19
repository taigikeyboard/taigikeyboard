@testable import TaigiKeyboard
import XCTest

/// ToneConverter unit tests — exercise the parameterized `toneToggles` entry point.
/// Settings are never read here; each test supplies the exact toggle combination.
final class ToneConverterTests: XCTestCase {
    private static let bothOn = ToneToggles(isDoubleTapOOEnabled: true, isDoubleTapNNEnabled: true)
    private static let ooOnly = ToneToggles(isDoubleTapOOEnabled: true, isDoubleTapNNEnabled: false)
    private static let nnOnly = ToneToggles(isDoubleTapOOEnabled: false, isDoubleTapNNEnabled: true)
    private static let bothOff = ToneToggles(isDoubleTapOOEnabled: false, isDoubleTapNNEnabled: false)

    // MARK: - TL Mode (No Preprocessing)

    func testConvertToToneMarks_tlMode_multiSyllable() {
        let result = ToneConverter.convertToToneMarks("ho2-se3", mode: .tl, toneToggles: Self.bothOn)
        XCTAssertEqual(result, "h\u{00F3}-s\u{00E8}") // hó-sè
    }

    func testConvertToToneMarks_tlMode_noPreprocessing() {
        // TL mode ignores the OO/NN toggles entirely.
        let result = ToneConverter.convertToToneMarks("hoo2", mode: .tl, toneToggles: Self.bothOn)
        XCTAssertEqual(result, "h\u{00F3}o") // hóo (tone 2 acute on first o)
    }

    // MARK: - POJ Mode + OO Enabled

    func testConvertToToneMarks_pojMode_ooEnabled() {
        let result = ToneConverter.convertToToneMarks("hoo2", mode: .poj, toneToggles: Self.ooOnly)
        XCTAssertTrue(
            result.unicodeScalars.contains { $0 == "\u{0358}" },
            "POJ with OO enabled should produce o͘: got \(result)",
        )
    }

    // MARK: - POJ Mode + NN Enabled

    func testConvertToToneMarks_pojMode_nnEnabled() {
        let result = ToneConverter.convertToToneMarks("ann2", mode: .poj, toneToggles: Self.nnOnly)
        XCTAssertTrue(
            result.contains("\u{207F}"),
            "POJ with NN enabled should produce ⁿ: got \(result)",
        )
    }

    // MARK: - POJ Mode + OO Disabled

    func testConvertToToneMarks_pojMode_ooDisabled() {
        let result = ToneConverter.convertToToneMarks("hoo2", mode: .poj, toneToggles: Self.bothOff)
        // Even with OO preprocessing off, toPOJ always outputs o͘ (POJ standard
        // output). The toggle only affects keyboard input preprocessing.
        XCTAssertTrue(
            result.unicodeScalars.contains { $0 == "\u{0358}" },
            "POJ output always uses o͘ regardless of OO toggle: got \(result)",
        )
    }

    // MARK: - POJ Mode + NN Disabled

    func testConvertToToneMarks_pojMode_nnDisabled() {
        let result = ToneConverter.convertToToneMarks("ann2", mode: .poj, toneToggles: Self.bothOff)
        // Mirrors the OO case — output always uses ⁿ regardless of NN toggle.
        XCTAssertTrue(
            result.unicodeScalars.contains { $0 == "\u{207F}" },
            "POJ output always uses ⁿ regardless of NN toggle: got \(result)",
        )
    }

    // MARK: - English Passthrough

    func testConvertToToneMarks_englishMode() {
        let result = ToneConverter.convertToToneMarks("hello2", mode: .english, toneToggles: Self.bothOn)
        XCTAssertEqual(result, "hello2", "English mode should pass through without conversion")
    }

    func testConvertToToneMarks_englishMode_multiSyllable() {
        let result = ToneConverter.convertToToneMarks("ka2-lang5", mode: .english, toneToggles: Self.bothOn)
        XCTAssertEqual(result, "ka2-lang5")
    }

    // MARK: - Nasal Marker Case

    func testConvertToToneMarks_pojMode_capsLockNN() {
        let result = ToneConverter.convertToToneMarks("ANN2", mode: .poj, toneToggles: Self.nnOnly)
        // Caps lock: uppercase vowel A → nasal should be ᴺ
        XCTAssertTrue(
            result.contains("\u{1D3A}"),
            "Caps lock ANN2 should produce ᴺ (U+1D3A): got \(result)",
        )
    }

    func testConvertToToneMarks_pojMode_singleShiftNN() {
        let result = ToneConverter.convertToToneMarks("Penn5", mode: .poj, toneToggles: Self.nnOnly)
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
        XCTAssertEqual(ToneConverter.convertToToneMarks("", mode: .tl, toneToggles: Self.bothOn), "")
        XCTAssertEqual(ToneConverter.convertToToneMarks("", mode: .poj, toneToggles: Self.bothOn), "")
    }

    func testConvertToToneMarks_pojMode_uppercaseOO() {
        // Oo -> O͘, OO -> O͘
        let result1 = ToneConverter.convertToToneMarks("Oo2", mode: .poj, toneToggles: Self.ooOnly)
        XCTAssertTrue(
            result1.unicodeScalars.contains { $0 == "\u{0358}" },
            "Uppercase Oo should also convert: got \(result1)",
        )
    }
}
