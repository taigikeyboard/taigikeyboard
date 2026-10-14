import KeyboardKit
@testable import TaigiKeyboard
import XCTest

/// Pins `CustomLayoutService.applyInputTypeKey` against the Android `mod/*.json`
/// bottom-row variants: `,` becomes `@` / `/` where the row has one, else the key
/// is added after the emoji key; other input types leave the rows untouched.
final class InputTypeKeyTests: XCTestCase {
    private let layouts = TaigiLayouts.Alphabetic.self

    private func bottomRow(_ rows: [[KeyDef]], _ inputType: Keyboard.InputType) -> [KeyDef] {
        CustomLayoutService.applyInputTypeKey(rows, inputType: inputType).last ?? []
    }

    func testCommaRows_swapComma() {
        XCTAssertEqual(
            bottomRow(layouts.qwerty_TL_iPhone, .emailAddress),
            [.numeric, .emoji, .char("@"), .space, .char("-"), .translate, .return],
        )
        XCTAssertEqual(
            bottomRow(layouts.qwerty_POJ_withGlobe, .url),
            [.numeric, .globe, .emoji, .char("/"), .space, .char("-"), .translate, .return],
        )
        XCTAssertEqual(
            bottomRow(layouts.moe2_TL_iPhone, .url),
            [.numeric, .emoji, .char("/"), .space, .char(".", fullWidth: "。"), .translate, .return],
        )
    }

    func testRowsWithoutComma_addKeyAfterEmoji() {
        XCTAssertEqual(
            bottomRow(layouts.moe1_TL_iPhone, .emailAddress),
            [.numeric, .emoji, .char("@"), .space, .translate, .return],
        )
        XCTAssertEqual(
            bottomRow(layouts.phahTaigi_withGlobe, .url),
            [.numeric, .globe, .emoji, .char("/"), .space, .translate, .return],
        )
        XCTAssertEqual(
            bottomRow(layouts.qwerty_English_iPhone, .emailAddress),
            [.numeric, .emoji, .char("@"), .space, .return],
        )
    }

    func testOnlyBottomRowChanges() {
        // phahTaigi / MOE1 carry `,` on row 4 — it must stay.
        for rows in [layouts.phahTaigi_iPhone, layouts.moe1_POJ_iPhone] {
            let result = CustomLayoutService.applyInputTypeKey(rows, inputType: .emailAddress)
            XCTAssertEqual(Array(result.dropLast()), Array(rows.dropLast()))
        }
    }

    func testOtherInputTypes_leaveRowsUnchanged() {
        for inputType in [Keyboard.InputType.text, .webSearch, .emojiSearch] {
            for rows in [layouts.qwerty_TL_iPhone, layouts.moe1_TL_iPhone, layouts.qwerty_English_iPhone] {
                XCTAssertEqual(CustomLayoutService.applyInputTypeKey(rows, inputType: inputType), rows, "\(inputType)")
            }
        }
    }
}
