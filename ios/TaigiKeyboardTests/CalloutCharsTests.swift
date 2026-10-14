import KeyboardKit
@testable import TaigiKeyboard
import XCTest

/// Pins `TaigiCallouts.calloutChars` lookup order: layout-specific → symbol →
/// tone, and English mode = symbol callouts only (Android English loads no
/// Taigi popups — mobile parity R3b).
final class CalloutCharsTests: XCTestCase {
    private func chars(_ char: String, _ inputMode: InputMode, _ layoutType: KeyboardLayoutType) -> [String]? {
        TaigiCallouts.calloutChars(for: char, inputMode: inputMode, layoutType: layoutType)
    }

    func testTaigiMode_layoutCalloutsWinOverSymbolCallouts() {
        // trace: MOE1Callouts["."] = 。！？…  /  MOE2Callouts[","] = ，、；：（「  /  TPSCallouts["，"] = 。
        XCTAssertEqual(chars(".", .tl, .moe1), ["。", "！", "？", "…"])
        XCTAssertEqual(chars(",", .poj, .moe2), ["，", "、", "；", "：", "（", "「"])
        XCTAssertEqual(chars("，", .tps, .tps), ["。"])
    }

    func testEnglishMode_skipsLayoutCallouts_keepsSymbolCallouts() {
        // trace: SymbolCallouts["."] = 。‧·  /  [","] = ，、  /  ["-"] = ·–—﹏
        for layoutType in [KeyboardLayoutType.qwerty, .moe1, .moe2, .phahTaigi, .tps] {
            XCTAssertEqual(chars(".", .english, layoutType), ["。", "‧", "·"], "\(layoutType)")
            XCTAssertEqual(chars(",", .english, layoutType), ["，", "、"], "\(layoutType)")
            XCTAssertEqual(chars("-", .english, layoutType), ["·", "–", "—", "﹏"], "\(layoutType)")
        }
    }

    func testEnglishMode_lettersHaveNoToneCallouts() {
        XCTAssertNotNil(chars("a", .tl, .qwerty), "TL tone callouts exist for the control")
        XCTAssertNotNil(chars("o", .poj, .qwerty), "POJ tone callouts exist for the control")
        for letter in ["a", "e", "i", "o", "u", "m", "n"] {
            XCTAssertNil(chars(letter, .english, .qwerty), letter)
        }
    }

    func testEllipsisKey_isUnicodeEllipsis() {
        // Android `western_default.json`: U+2026 with popup ⋯ ‥.
        XCTAssertEqual(chars("…", .tl, .qwerty), ["⋯", "‥"])
        XCTAssertEqual(chars("⋯", .tl, .qwerty), ["…", "‥"])
        XCTAssertNil(TaigiCallouts.SymbolCallouts.actions["..."])
    }
}
