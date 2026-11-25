// Renders `KeyHintStack` (the alignment-guide path `KeyLabelLayout` tests cannot see) and
// checks the ink: inside a landscape-height key with a margin, and a spread TPS hint pair
// kept apart on a narrow key.

import SwiftUI
@testable import TaigiKeyboard
import XCTest

@MainActor
final class KeyHintStackRenderTests: XCTestCase {
    private let scale: CGFloat = 3

    /// Dark-ink mask of the rendered view, [row][column] at `scale` pixels per point.
    private func inkMask(_ view: some View, width: CGFloat, height: CGFloat) throws -> [[Bool]] {
        let renderer = ImageRenderer(content: view.frame(width: width, height: height).background(Color.white))
        renderer.scale = scale
        let image = try XCTUnwrap(renderer.cgImage)
        let w = image.width, h = image.height
        var pixels = [UInt8](repeating: 0, count: w * h * 4)
        let context = try XCTUnwrap(CGContext(
            data: &pixels, width: w, height: h, bitsPerComponent: 8, bytesPerRow: w * 4,
            space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue,
        ))
        context.draw(image, in: CGRect(x: 0, y: 0, width: w, height: h))
        return (0 ..< h).map { y in (0 ..< w).map { x in pixels[(y * w + x) * 4] < 200 } }
    }

    func testToneKey_landscapeHeight_inkStaysInsideWithMargin() throws {
        let stack = KeyHintStack(
            hints: ["ˊ"],
            label: "2",
            labelFont: .systemFont(ofSize: 23),
            hintFont: .systemFont(ofSize: 20),
            labelReference: "0",
            hintReference: "ˈ",
            alignsEachHintByInk: true,
            hintColor: .black,
            hintOpacity: 1,
        )
        let mask = try inkMask(stack, width: 40, height: 26)
        let inkRows = mask.indices.filter { mask[$0].contains(true) }
        let first = try XCTUnwrap(inkRows.first), last = try XCTUnwrap(inkRows.last)
        // ≥ 1.5 pt clear at the top and bottom of a 26 pt key (it spilled above the top before).
        XCTAssertGreaterThanOrEqual(CGFloat(first) / scale, 1.5, "hint ink at the key top")
        XCTAssertLessThanOrEqual(CGFloat(last + 1) / scale, 26 - 1.5, "label ink at the key bottom")
        // Hint and label are two separate ink runs (a blank row between them).
        XCTAssertTrue((first ... last).contains { !mask[$0].contains(true) }, "hint touches the digit")
    }

    func testTPSHintPair_narrowKey_hintsDoNotMeet() throws {
        let stack = KeyHintStack(
            hints: ["ㆭ", "ㄥ"],
            label: "ㄫ",
            labelFont: .systemFont(ofSize: 19),
            hintFont: .systemFont(ofSize: 12),
            labelReference: "ㄅ",
            hintReference: "ㄅ",
            hintSideInsetRatio: 3 / 19,
            hintColor: .black,
            hintOpacity: 1,
        )
        // ~ one-handed TPS key width; both 12 pt hints plus insets need ~30 pt.
        let mask = try inkMask(stack, width: 22, height: 38)
        let inkRows = mask.indices.filter { mask[$0].contains(true) }
        let first = try XCTUnwrap(inkRows.first), last = try XCTUnwrap(inkRows.last)
        // The hint row is the top third of the ink stack (hint band ≈ 10 pt of ≈ 28 pt).
        let hintRows = inkRows.filter { $0 <= first + (last - first) / 3 }
        XCTAssertFalse(hintRows.isEmpty, "no hint ink rendered")
        let middle = mask[0].count / 2
        XCTAssertFalse(hintRows.contains { mask[$0][middle] }, "the two hints meet in the middle of the key")
    }
}
