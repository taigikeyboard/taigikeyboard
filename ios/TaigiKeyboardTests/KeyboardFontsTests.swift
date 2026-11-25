// Pins that every keyboard font is fixed-size: a custom font must not follow Dynamic
// Type (the candidate bar and keys have fixed heights; System never scales).

import SwiftUI
@testable import TaigiKeyboard
import XCTest

final class KeyboardFontsTests: XCTestCase {
    func testFont_customFonts_areFixedSize() {
        for fontType in FontType.allCases {
            guard let name = fontType.customFontName else { continue }
            XCTAssertEqual(
                KeyboardFonts.font(for: fontType, size: 21),
                Font.custom(name, fixedSize: 21),
                "\(fontType) must not scale with Dynamic Type",
            )
        }
    }

    func testFont_system_isFixedSystemFont() {
        XCTAssertEqual(KeyboardFonts.font(for: .system, size: 16), Font.system(size: 16))
    }
}
