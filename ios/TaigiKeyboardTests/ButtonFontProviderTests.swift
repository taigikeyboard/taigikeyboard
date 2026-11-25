// Pins that key labels and long-press callouts in a custom font are fixed-size: a
// Dynamic Type–relative font outgrew the key at XXXL (labels touching the key edges,
// tone hints pushed against the key top) while System keys never scale.

import KeyboardKit
@testable import TaigiKeyboard
import XCTest

@MainActor
final class ButtonFontProviderTests: XCTestCase {
    private func snapshot(fontType: FontType, inputMode: InputMode = .poj) -> SettingsSnapshot {
        SettingsSnapshot(
            inputMode: inputMode,
            fontType: fontType,
            keyboardLayoutType: .qwerty,
            activeCustomLayout: nil,
            isHanjiFirst: false,
            isTpsOrMappedToER: false,
            isNasalMarkerUppercaseEnabled: false,
            keyFontSizeScale: 1,
            keyCornerRadius: 5,
            colorSettings: .default,
            candidateTextSizeScale: 1,
            keyBorderWidth: 0,
            keyShadowIntensity: nil,
        )
    }

    /// Custom fonts are fixed-size (a Dynamic Type–relative `.custom` fails the switch), and
    /// `buttonUIFont`, which `KeyHintStack` measures with, is the font `buttonKeyboardFont` renders.
    func testButtonFonts_fixedSizeAndUIFontMatches() {
        let context = KeyboardContext(settings: KeyboardSettings())
        for fontType in FontType.allCases {
            let provider = ButtonFontProvider(keyboardContext: context, settings: snapshot(fontType: fontType))
            for action in [KeyboardAction.character("W"), .character("2"), .character("tsh"), .character(",")] {
                let uiFont = provider.buttonUIFont(for: action)
                let font = provider.buttonKeyboardFont(for: action)
                switch font.type {
                case let .customFixed(name, size):
                    XCTAssertEqual(font, KeyboardFont.custom(name, fixedSize: size, weight: .regular))
                    XCTAssertEqual(uiFont.fontName, name)
                    XCTAssertEqual(uiFont.pointSize, size)
                case let .system(size):
                    XCTAssertEqual(fontType, .system, "custom font fell back to System")
                    XCTAssertEqual(uiFont.pointSize, size)
                default:
                    XCTFail("\(fontType) \(action): not fixed-size, got \(font.type)")
                }
            }
        }
    }

    func testCalloutStyle_customFonts_areFixedSize() {
        for fontType in FontType.allCases {
            guard fontType.customFontName != nil else { continue }
            let style = KeyboardCalloutStyle.taigi(for: fontType)
            XCTAssertNotNil(style.actionItemFont.fixedSize, "\(fontType) action callout font scales")
            XCTAssertNotNil(style.inputItemFont.fixedSize, "\(fontType) input callout font scales")
        }
    }
}

private extension KeyboardFont {
    /// The point size when the font is a fixed custom font, nil for any Dynamic Type–relative one.
    var fixedSize: CGFloat? {
        if case let .customFixed(_, size) = type {
            return size
        }
        return nil
    }
}
