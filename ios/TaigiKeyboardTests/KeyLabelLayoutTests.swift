// Pins the hint-above-label key layout: the stack stays inside the key with a margin on
// every key height (landscape keys are ~26 pt), the hint gives way before the label, and
// the hint–label gap is measured from glyph ink so it holds for every keyboard font.

@testable import TaigiKeyboard
import UIKit
import XCTest

final class KeyLabelLayoutTests: XCTestCase {
    /// Digit "0" at 23 pt System (ink ≈ cap height) under a tone-8 hint band.
    private func input(keyHeight: CGFloat) -> KeyLabelLayout.Input {
        KeyLabelLayout.Input(
            keyHeight: keyHeight,
            labelFontSize: 23,
            labelInk: .init(ascent: 16.5, descent: 0.3),
            hintBand: 7,
        )
    }

    /// Top of the reserved hint band, from the key top.
    private func bandTop(_ placement: KeyLabelLayout.Placement, band: CGFloat) -> CGFloat {
        placement.hintInkBottom - band * placement.hintScale
    }

    private func assertInsideKey(_ input: KeyLabelLayout.Input, file: StaticString = #filePath, line: UInt = #line) {
        let placement = KeyLabelLayout.place(input)
        let margin = input.labelFontSize * KeyLabelLayout.edgeRatio * placement.labelScale
        let labelInkBottom = placement.labelBaseline + input.labelInk.descent * placement.labelScale
        XCTAssertGreaterThanOrEqual(bandTop(placement, band: input.hintBand), margin - 0.01, "hint above the key top margin", file: file, line: line)
        XCTAssertLessThanOrEqual(labelInkBottom, input.keyHeight - margin + 0.01, "label below the key bottom margin", file: file, line: line)
        let labelInkTop = placement.labelBaseline - input.labelInk.ascent * placement.labelScale
        XCTAssertEqual(
            labelInkTop - placement.hintInkBottom,
            input.labelFontSize * KeyLabelLayout.gapRatio * placement.labelScale,
            accuracy: 0.01,
            "hint–label gap scales with the label",
            file: file,
            line: line,
        )
    }

    func testPlace_portraitKey_keepsFullSizeAndCentres() {
        // trace: band 7 + gap 2.76 + ink 16.8 = 26.56 ≤ 37 − 2×2.3 → no scaling, top = (37 − 26.56) / 2
        let placement = KeyLabelLayout.place(input(keyHeight: 37))
        XCTAssertEqual(placement.labelScale, 1)
        XCTAssertEqual(placement.hintScale, 1)
        XCTAssertEqual(bandTop(placement, band: 7), (37 - 26.56) / 2, accuracy: 0.01)
        assertInsideKey(input(keyHeight: 37))
    }

    func testPlace_landscapeKey_hintShrinksFirstThenStackFits() {
        // trace: 31.16 needed > 26 → hint 0.6 (floor) → 28.36 still > 26 → scale 26 / 28.36
        let placement = KeyLabelLayout.place(input(keyHeight: 26))
        XCTAssertEqual(placement.labelScale, 26 / 28.36, accuracy: 0.001)
        XCTAssertEqual(placement.hintScale, placement.labelScale * KeyLabelLayout.minimumHintScale, accuracy: 0.001)
        assertInsideKey(input(keyHeight: 26))
    }

    func testPlace_slightlyShortKey_onlyHintShrinks() {
        // trace: room for hint = 29 − 4.6 − 2.76 − 16.8 = 4.84 → hint 4.84 / 7, label unscaled
        let placement = KeyLabelLayout.place(input(keyHeight: 29))
        XCTAssertEqual(placement.labelScale, 1)
        XCTAssertEqual(placement.hintScale, 4.84 / 7, accuracy: 0.001)
        assertInsideKey(input(keyHeight: 29))
    }

    func testPlace_everyKeyHeight_staysInsideKey() {
        for height in stride(from: 20.0, through: 60.0, by: 0.5) {
            assertInsideKey(input(keyHeight: CGFloat(height)))
        }
    }

    /// Real fonts: the tone-8 line (tallest hint) keeps a ≥ 2 pt gap above the digit 8 in
    /// every bundled font (it was ~1 pt with GenYoGothic / Open Huninn).
    func testPlace_realFonts_toneHintClearsDigit() {
        let hintFont = UIFont.systemFont(ofSize: 20)
        let band = KeyLabelLayout.ink(of: "ˈ", font: hintFont).height
        for fontType in FontType.allCases {
            if let name = fontType.customFontName {
                XCTAssertNotNil(UIFont(name: name, size: 23), "\(fontType) is not registered in the test host")
            }
            let labelFont = KeyboardFonts.uiFont(for: fontType, size: 23)
            let placement = KeyLabelLayout.place(KeyLabelLayout.Input(
                keyHeight: 37,
                labelFontSize: 23,
                labelInk: KeyLabelLayout.ink(of: "0", font: labelFont),
                hintBand: band,
            ))
            let digitInkTop = placement.labelBaseline - KeyLabelLayout.ink(of: "8", font: labelFont).ascent
            XCTAssertGreaterThanOrEqual(digitInkTop - placement.hintInkBottom, 2, "\(fontType): tone hint crowds the digit")
            XCTAssertGreaterThanOrEqual(bandTop(placement, band: band), 2.3, "\(fontType): hint crowds the key top")
        }
    }

    /// TPS keys (19 pt bopomofo under 12 pt hints): the stack is centred, so the hint row
    /// keeps the same distance from the key top as the label from the bottom (it sat
    /// 2–3 pt from the top under the old fixed 2 pt padding).
    func testPlace_tpsKey_hintRowCentredAwayFromKeyTop() {
        let labelFont = UIFont.systemFont(ofSize: 19)
        let labelInk = KeyLabelLayout.ink(of: "ㄅ", font: labelFont)
        for keyHeight in [CGFloat(38), 42] {
            let band = KeyLabelLayout.ink(of: "ㄅ", font: .systemFont(ofSize: 12)).height
            let placement = KeyLabelLayout.place(KeyLabelLayout.Input(
                keyHeight: keyHeight,
                labelFontSize: 19,
                labelInk: labelInk,
                hintBand: band,
            ))
            let bottomMargin = keyHeight - (placement.labelBaseline + labelInk.descent * placement.labelScale)
            XCTAssertEqual(placement.labelScale, 1, "TPS label keeps its size on a portrait key")
            XCTAssertEqual(bandTop(placement, band: band), bottomMargin, accuracy: 0.01, "stack centred in a \(keyHeight) pt key")
            XCTAssertGreaterThan(bandTop(placement, band: band), 3, "hint row crowds the key top in a \(keyHeight) pt key")
        }
    }

    func testInk_whitespaceHasNoInk() {
        XCTAssertEqual(KeyLabelLayout.ink(of: " ", font: .systemFont(ofSize: 20)), .init(ascent: 0, descent: 0))
    }
}
